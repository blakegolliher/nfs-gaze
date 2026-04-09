use crate::display::{display_stats_simple, display_xprt_summary};
use crate::parser::parse_mountstats;
use crate::snapshot::{MountAggregator, MountReport, Report, CURRENT_SCHEMA_VERSION};
use crate::stats::{calculate_delta_stats, calculate_xprt_delta, filter_operations};
use crate::types::{NFSMount, NfsGazeError, Result};
use chrono::Utc;
use signal_hook::{consts::SIGINT, consts::SIGTERM, iterator::Signals};
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Maximum number of consecutive `parse_mountstats` failures the
/// monitoring loop will tolerate before giving up. Without this
/// circuit breaker a permanent failure (e.g. `/proc` becoming
/// unreadable) would loop forever printing the same error each
/// interval.
const MAX_CONSECUTIVE_PARSE_FAILURES: u32 = 10;

/// Read the configured TCP RPC slot table cap from the kernel.
///
/// Returns `None` if the sysctl is unreadable for any reason (not a
/// Linux kernel, `/proc/sys` not mounted, sunrpc module missing,
/// unusual permissions). The value is purely informational — it is
/// used alongside the per-mount `max_slots` high-water mark to let
/// users tell "lots of headroom" from "at the ceiling" — and any
/// consumer must be prepared for a `None` fallback rather than
/// failing the whole capture session on it.
pub fn read_tcp_slot_cap() -> Option<i64> {
    let s = std::fs::read_to_string("/proc/sys/sunrpc/tcp_max_slot_table_entries").ok()?;
    s.trim().parse::<i64>().ok()
}

/// Configuration for the monitoring loop.
///
/// Collected into a struct so `monitoring_loop` can be called with a
/// single argument instead of a long positional list.
pub struct MonitorConfig<'a> {
    pub mountstats_path: &'a str,
    pub monitor_mounts: Vec<NFSMount>,
    pub operations_filter: HashSet<String>,
    pub interval: Duration,
    /// Maximum iteration count (0 = unlimited). Mutually exclusive with
    /// `duration` at the CLI layer; this struct only enforces that the
    /// first limit to trip wins.
    pub count: usize,
    /// Optional wall-clock termination. `None` means "no time limit".
    /// When `Some`, the loop exits once `Instant::now()` crosses the
    /// start instant plus this `Duration`.
    pub duration: Option<Duration>,
    /// When `Some`, aggregate each interval's deltas into a
    /// [`Report`] and write it to this path at end of session. In
    /// this mode the live per-interval table is suppressed and the
    /// loop prints a `Sampling...` progress line to stderr instead.
    pub output: Option<PathBuf>,
    /// Configured cap from `/proc/sys/sunrpc/tcp_max_slot_table_entries`,
    /// read once at session start. Used only to render the xprt
    /// one-liner; `None` is a soft fallback that displays as `?`.
    pub slot_cap: Option<i64>,
    pub show_bandwidth: bool,
    pub clear_screen: bool,
    pub metrics_manager: Option<&'a crate::metrics::MetricsManager>,
}

/// Main monitoring structure
pub struct Monitor {
    pub running: Arc<AtomicBool>,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Setup signal handling for graceful shutdown
    pub fn setup_signal_handling(&self) {
        let running = self.running.clone();

        thread::spawn(move || {
            let mut signals = Signals::new([SIGINT, SIGTERM]).unwrap();
            if let Some(_sig) = signals.forever().next() {
                running.store(false, Ordering::SeqCst);
            }
        });
    }

    /// Get mounts to monitor based on mount point filter
    pub fn get_mounts_to_monitor(
        mount_point: Option<String>,
        available_mounts: &HashMap<String, NFSMount>,
    ) -> Result<Vec<NFSMount>> {
        match mount_point {
            Some(target) => {
                if let Some(mount) = available_mounts.get(&target) {
                    Ok(vec![mount.clone()])
                } else {
                    Err(crate::types::NfsGazeError::MountNotFound(target))
                }
            }
            None => Ok(available_mounts.values().cloned().collect()),
        }
    }

    /// Print initial summary of monitored mounts
    pub fn print_initial_summary<W: Write>(
        writer: &mut W,
        mount_point: &Option<String>,
        mounts: &[NFSMount],
        operations_filter: &HashSet<String>,
    ) -> io::Result<()> {
        writeln!(writer, "NFS I/O Statistics Monitor")?;
        writeln!(writer, "==========================")?;
        writeln!(writer)?;

        if let Some(mp) = mount_point {
            writeln!(writer, "Monitoring mount point: {}", mp)?;
        } else {
            writeln!(writer, "Monitoring {} NFS mount(s):", mounts.len())?;
            for mount in mounts {
                writeln!(writer, "  {} -> {}", mount.device, mount.mount_point)?;
            }
        }

        if !operations_filter.is_empty() {
            writeln!(writer, "Filtering operations: {:?}", operations_filter)?;
        }

        writeln!(writer)?;
        Ok(())
    }

    /// Main monitoring loop
    pub fn monitoring_loop<W: Write>(
        &self,
        writer: &mut W,
        config: MonitorConfig<'_>,
    ) -> Result<()> {
        let MonitorConfig {
            mountstats_path,
            monitor_mounts,
            operations_filter,
            interval,
            count,
            duration,
            output,
            slot_cap,
            show_bandwidth,
            clear_screen,
            metrics_manager,
        } = config;

        let mut previous_mounts: HashMap<String, NFSMount> = monitor_mounts
            .iter()
            .map(|m| (m.mount_point.clone(), m.clone()))
            .collect();

        // When -o is in play, allocate one aggregator per monitored
        // mount up-front. We key on mount_point to match how
        // `previous_mounts` is keyed, so lookups during the loop are
        // direct. Mounts that appear mid-run are ignored for
        // aggregation purposes — matching nfs-monitor, which locks in
        // the target set at session start.
        let output_mode = output.is_some();
        let mut aggregators: HashMap<String, MountAggregator> = if output_mode {
            monitor_mounts
                .iter()
                .map(|m| {
                    (
                        m.mount_point.clone(),
                        // fstype and options are not yet recovered by
                        // the parser; pass empty strings for now. A
                        // follow-up parser change can populate them
                        // without touching this call site.
                        MountAggregator::new(m, String::new(), String::new()),
                    )
                })
                .collect()
        } else {
            HashMap::new()
        };
        let mut samples_collected: u64 = 0;

        let mut iteration = 0;
        let loop_start = Instant::now();
        let mut last_update = loop_start;
        // Compute the wall-clock deadline once at the start of the loop
        // so the total session length is independent of how long each
        // iteration takes. Note that the deadline is checked *before*
        // sleeping for the next interval, so the loop may overshoot by
        // up to one `interval` when `duration` and `interval` are not
        // commensurate — matching the original nfs-monitor semantics.
        let deadline = duration.map(|d| loop_start + d);
        let mut consecutive_parse_failures: u32 = 0;

        while self.running.load(Ordering::SeqCst) {
            // Check if we've reached the iteration limit
            if count > 0 && iteration >= count {
                break;
            }

            // Check if we've reached the wall-clock deadline
            if let Some(end) = deadline {
                if Instant::now() >= end {
                    break;
                }
            }

            // Sleep for the specified interval
            thread::sleep(interval);

            // Parse current mountstats
            let current_mounts = match parse_mountstats(mountstats_path) {
                Ok(mounts) => {
                    consecutive_parse_failures = 0;
                    mounts
                }
                Err(e) => {
                    consecutive_parse_failures += 1;
                    eprintln!(
                        "Error reading mountstats ({}/{}): {}",
                        consecutive_parse_failures, MAX_CONSECUTIVE_PARSE_FAILURES, e
                    );
                    if consecutive_parse_failures >= MAX_CONSECUTIVE_PARSE_FAILURES {
                        return Err(NfsGazeError::ParseError(format!(
                            "Giving up after {} consecutive failures reading {}: {}",
                            MAX_CONSECUTIVE_PARSE_FAILURES, mountstats_path, e
                        )));
                    }
                    continue;
                }
            };

            // Get current monitored mounts
            let current_monitor_mounts = match Self::get_mounts_to_monitor(None, &current_mounts) {
                Ok(mounts) => mounts,
                Err(e) => {
                    eprintln!("Error getting mounts to monitor: {}", e);
                    continue;
                }
            };

            // Calculate elapsed time
            let now = Instant::now();
            let elapsed = now.duration_since(last_update);
            let elapsed_seconds = elapsed.as_secs_f64();
            last_update = now;

            // Skip first iteration (no previous data)
            if iteration == 0 {
                for mount in &current_monitor_mounts {
                    previous_mounts.insert(mount.mount_point.clone(), mount.clone());
                }
                iteration += 1;
                continue;
            }

            // Clear screen if requested — never in output mode, since
            // it would clobber the progress line on stderr and blank
            // the terminal in the middle of a silent capture.
            if clear_screen && !output_mode {
                write!(writer, "\x1B[2J\x1B[1;1H")?;
            }

            let timestamp = Utc::now();

            // Process each monitored mount
            for current_mount in &current_monitor_mounts {
                if let Some(previous_mount) = previous_mounts.get(&current_mount.mount_point) {
                    // Calculate delta statistics
                    let mut delta_stats =
                        calculate_delta_stats(previous_mount, current_mount, elapsed_seconds);

                    // Filter operations if specified
                    delta_stats = filter_operations(delta_stats, &operations_filter);

                    // xprt delta is computed alongside the per-op
                    // deltas but shown in a separate one-liner so
                    // the op table stays uncluttered.
                    let xprt_delta = calculate_xprt_delta(
                        previous_mount.xprt.as_ref(),
                        current_mount.xprt.as_ref(),
                    );

                    // Either aggregate for the end-of-session report
                    // or render the live table — never both, because
                    // output mode is intended as a silent capture.
                    if !delta_stats.is_empty() {
                        if output_mode {
                            if let Some(agg) = aggregators.get_mut(&current_mount.mount_point) {
                                agg.record(&delta_stats);
                                // xprt folds into the same
                                // aggregator so the finalised
                                // MountReport carries a matching
                                // XprtReport. Idle xprt samples (no
                                // delta but still non-None) are
                                // silently added as zeros by
                                // record_xprt.
                                if let Some(ref x) = xprt_delta {
                                    agg.record_xprt(x);
                                }
                            }
                        } else {
                            display_stats_simple(
                                writer,
                                current_mount,
                                &delta_stats,
                                show_bandwidth,
                                &timestamp,
                            )?;
                            // Always print the xprt one-liner under
                            // the op table when we have xprt data
                            // for this mount. It co-appears with the
                            // op table so an idle mount stays quiet.
                            if let Some(ref x) = xprt_delta {
                                display_xprt_summary(writer, x, slot_cap)?;
                            }
                        }

                        // Prometheus metrics export is orthogonal to
                        // on-disk reports; keep exporting in both
                        // modes so long-running scrapers don't go
                        // blind during an -o session.
                        if let Some(manager) = metrics_manager {
                            manager.export_metrics(current_mount, &delta_stats);
                        }
                    }
                }

                // Update previous mount data
                previous_mounts.insert(current_mount.mount_point.clone(), current_mount.clone());
            }

            if output_mode {
                samples_collected += 1;
                let elapsed_secs = loop_start.elapsed().as_secs();
                // Carriage return without newline so each update
                // overwrites the previous progress line. Flush
                // explicitly because stderr is line-buffered when
                // attached to a TTY and otherwise block-buffered.
                eprint!(
                    "\r  Sampling... {}s  ({} samples)",
                    elapsed_secs, samples_collected
                );
                let _ = io::stderr().flush();
            }

            iteration += 1;
        }

        // Terminate the progress line with a newline so whatever
        // prints next (error, final "wrote report" note, shell
        // prompt) starts on a fresh line.
        if output_mode {
            eprintln!();
        }

        // Fold aggregators into a Report and write the JSON file.
        // Done here rather than in main so that the samples/duration
        // metadata stays consistent with the loop state that produced
        // it — main.rs never sees `samples_collected` or `loop_start`.
        if let Some(output_path) = output {
            let interval_sec = interval.as_secs();
            let duration_sec = duration
                .map(|d| d.as_secs())
                .unwrap_or_else(|| loop_start.elapsed().as_secs());

            let mut mount_reports: Vec<MountReport> = aggregators
                .into_values()
                .map(|agg| agg.finalise(duration_sec, slot_cap))
                .collect();
            // Sort by device for deterministic output across runs on
            // the same host; HashMap iteration order is otherwise
            // random and would make diffs noisy.
            mount_reports.sort_by(|a, b| a.device.cmp(&b.device));

            let report = Report {
                schema_version: CURRENT_SCHEMA_VERSION,
                generated_at: Utc::now(),
                duration_sec,
                interval_sec,
                samples: samples_collected,
                mounts: mount_reports,
            };

            let file =
                std::fs::File::create(&output_path).map_err(|e| NfsGazeError::ReportWrite {
                    path: output_path.display().to_string(),
                    source: e,
                })?;
            serde_json::to_writer_pretty(file, &report)?;
            eprintln!("Wrote JSON report to {}", output_path.display());
        }

        Ok(())
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_mount(mount_point: &str, device: &str) -> NFSMount {
        NFSMount {
            device: device.to_string(),
            mount_point: mount_point.to_string(),
            server: "test-server".to_string(),
            export: "/test".to_string(),
            age: 0,
            operations: HashMap::new(),
            events: None,
            bytes_read: 0,
            bytes_write: 0,
            xprt: None,
        }
    }

    #[test]
    fn test_get_mounts_to_monitor_specific() {
        let mut available_mounts = HashMap::new();
        available_mounts.insert(
            "/mnt/nfs1".to_string(),
            create_test_mount("/mnt/nfs1", "server1:/export1"),
        );
        available_mounts.insert(
            "/mnt/nfs2".to_string(),
            create_test_mount("/mnt/nfs2", "server2:/export2"),
        );

        let result =
            Monitor::get_mounts_to_monitor(Some("/mnt/nfs1".to_string()), &available_mounts)
                .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].mount_point, "/mnt/nfs1");
    }

    #[test]
    fn test_get_mounts_to_monitor_all() {
        let mut available_mounts = HashMap::new();
        available_mounts.insert(
            "/mnt/nfs1".to_string(),
            create_test_mount("/mnt/nfs1", "server1:/export1"),
        );
        available_mounts.insert(
            "/mnt/nfs2".to_string(),
            create_test_mount("/mnt/nfs2", "server2:/export2"),
        );

        let result = Monitor::get_mounts_to_monitor(None, &available_mounts).unwrap();

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_get_mounts_to_monitor_not_found() {
        let available_mounts = HashMap::new();

        let result =
            Monitor::get_mounts_to_monitor(Some("/mnt/nonexistent".to_string()), &available_mounts);

        assert!(result.is_err());
    }

    #[test]
    fn test_monitoring_loop_circuit_breaker_on_persistent_parse_failure() {
        // Pointing the loop at a path that does not exist makes every
        // parse_mountstats call fail. The circuit breaker must trip
        // after MAX_CONSECUTIVE_PARSE_FAILURES iterations rather than
        // looping forever.
        let monitor = Monitor::new();
        let result = monitor.monitoring_loop(
            &mut Vec::<u8>::new(),
            MonitorConfig {
                mountstats_path: "/definitely/not/a/real/path/mountstats",
                monitor_mounts: vec![],
                operations_filter: HashSet::new(),
                interval: Duration::from_millis(1),
                // count=0 means "infinite iterations"; we rely on the
                // breaker to terminate, not the count limit.
                count: 0,
                duration: None,
                output: None,
                slot_cap: None,
                show_bandwidth: false,
                clear_screen: false,
                metrics_manager: None,
            },
        );

        let err = result.expect_err("expected the monitoring loop to bail out");
        let msg = format!("{err}");
        assert!(
            msg.contains("Giving up after"),
            "unexpected error message: {msg}"
        );
        assert!(
            msg.contains(&MAX_CONSECUTIVE_PARSE_FAILURES.to_string()),
            "error should mention the failure threshold: {msg}"
        );
    }

    #[test]
    fn test_monitoring_loop_respects_duration() {
        use tempfile::NamedTempFile;

        // A valid-but-empty mountstats file keeps the parser happy so
        // the circuit breaker does not trip; we rely on the wall-clock
        // deadline to terminate the loop. NamedTempFile already creates
        // the file on disk, so we do not need to write to it.
        let tmp = NamedTempFile::new().expect("create tempfile");
        let path = tmp
            .path()
            .to_str()
            .expect("tempfile path is valid utf-8")
            .to_string();

        let target = Duration::from_millis(80);
        let slack = Duration::from_millis(400);

        let monitor = Monitor::new();
        let start = Instant::now();
        let result = monitor.monitoring_loop(
            &mut Vec::<u8>::new(),
            MonitorConfig {
                mountstats_path: &path,
                monitor_mounts: vec![],
                operations_filter: HashSet::new(),
                interval: Duration::from_millis(20),
                count: 0,
                duration: Some(target),
                output: None,
                slot_cap: None,
                show_bandwidth: false,
                clear_screen: false,
                metrics_manager: None,
            },
        );
        let elapsed = start.elapsed();

        result.expect("loop should exit cleanly when duration elapses");
        assert!(
            elapsed >= target,
            "loop exited before the deadline: {:?} < {:?}",
            elapsed,
            target
        );
        assert!(
            elapsed < target + slack,
            "loop ran far longer than its deadline: {:?} vs target {:?}",
            elapsed,
            target
        );
    }

    #[test]
    fn test_monitoring_loop_writes_report_in_output_mode() {
        use tempfile::TempDir;

        // Arrange a temp directory holding both an empty mountstats
        // file (so parsing succeeds with zero mounts) and the path
        // the loop should write its JSON report to.
        let dir = TempDir::new().expect("create tempdir");
        let mountstats_path = dir.path().join("mountstats");
        std::fs::write(&mountstats_path, "").expect("write empty mountstats");
        let output_path = dir.path().join("report.json");

        let monitor = Monitor::new();
        let result = monitor.monitoring_loop(
            &mut Vec::<u8>::new(),
            MonitorConfig {
                mountstats_path: mountstats_path.to_str().expect("utf-8 path"),
                monitor_mounts: vec![],
                operations_filter: HashSet::new(),
                interval: Duration::from_millis(10),
                count: 0,
                // Short-lived session so the test is fast; the
                // deadline is the only reason the loop terminates.
                duration: Some(Duration::from_millis(80)),
                output: Some(output_path.clone()),
                slot_cap: None,
                show_bandwidth: false,
                clear_screen: false,
                metrics_manager: None,
            },
        );
        result.expect("loop should exit cleanly under duration");

        // The file must exist and round-trip through the Report
        // deserialiser. We do not assert samples/duration_sec
        // because both are sub-second and would round to zero.
        let json = std::fs::read_to_string(&output_path).expect("read report file");
        let report: crate::snapshot::Report =
            serde_json::from_str(&json).expect("report must be valid JSON");
        assert_eq!(
            report.schema_version,
            crate::snapshot::CURRENT_SCHEMA_VERSION
        );
        assert!(
            report.mounts.is_empty(),
            "no monitor_mounts passed → no MountReports expected, got {:?}",
            report.mounts
        );
    }

    #[test]
    fn test_print_initial_summary() {
        let mut buffer = Vec::new();
        let mounts = vec![
            create_test_mount("/mnt/nfs1", "server1:/export1"),
            create_test_mount("/mnt/nfs2", "server2:/export2"),
        ];
        let operations_filter = HashSet::new();

        Monitor::print_initial_summary(&mut buffer, &None, &mounts, &operations_filter).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("NFS I/O Statistics Monitor"));
        assert!(output.contains("Monitoring 2 NFS mount(s)"));
        assert!(output.contains("server1:/export1"));
        assert!(output.contains("server2:/export2"));
    }

    #[test]
    fn test_print_initial_summary_with_filter() {
        let mut buffer = Vec::new();
        let mounts = vec![create_test_mount("/mnt/nfs", "server:/export")];
        let mut operations_filter = HashSet::new();
        operations_filter.insert("READ".to_string());
        operations_filter.insert("WRITE".to_string());

        Monitor::print_initial_summary(
            &mut buffer,
            &Some("/mnt/nfs".to_string()),
            &mounts,
            &operations_filter,
        )
        .unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("Monitoring mount point: /mnt/nfs"));
        assert!(output.contains("Filtering operations"));
    }
}
