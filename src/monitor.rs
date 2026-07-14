use crate::display::{display_stats_simple, display_xprt_only, display_xprt_summary};
use crate::parser::parse_mountstats;
use crate::snapshot::{MountAggregator, MountReport, Report, CURRENT_SCHEMA_VERSION};
use crate::stats::{
    calculate_delta_stats, calculate_mount_deltas, calculate_xprt_delta, filter_operations,
};
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
    /// The `-m` mount-point filter. When `Some`, only this mount is
    /// displayed/aggregated/exported each interval; when `None`, all
    /// NFS mounts are monitored (including ones that appear mid-run).
    /// An interval where the selected mount is absent (unmounted) is
    /// quietly skipped rather than treated as an error.
    pub mount_point: Option<String>,
    pub operations_filter: HashSet<String>,
    pub interval: Duration,
    /// Maximum number of *measured* intervals (0 = unlimited). The
    /// seed sample taken at startup is not counted, so `count: 1`
    /// produces exactly one displayed measurement. Mutually exclusive
    /// with `duration` at the CLI layer; this struct only enforces
    /// that the first limit to trip wins.
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
            mount_point,
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
                .map(|m| (m.mount_point.clone(), MountAggregator::new(m)))
                .collect()
        } else {
            HashMap::new()
        };
        let mut samples_collected: u64 = 0;

        // `iteration` only distinguishes the seed pass from the rest;
        // `measured_intervals` counts intervals that actually produced
        // a measurement. `count` bounds the latter, so `-c 1` yields
        // exactly one displayed interval instead of burning the whole
        // count on the seed.
        let mut iteration = 0;
        let mut measured_intervals: usize = 0;
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
            // Check if we've reached the measured-interval limit
            if count > 0 && measured_intervals >= count {
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

            // Select this interval's mounts, honouring the -m filter.
            // A selected mount that is currently absent (unmounted)
            // yields an empty set for the interval — no error spam;
            // when it returns, the reset detection rebases cleanly.
            let mut current_monitor_mounts: Vec<NFSMount> = match &mount_point {
                Some(target) => current_mounts.get(target).cloned().into_iter().collect(),
                None => current_mounts.values().cloned().collect(),
            };
            // Sort for a stable display order across intervals —
            // HashMap iteration order would otherwise shuffle the
            // mounts on every refresh.
            current_monitor_mounts.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));

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

            // Process each monitored mount. `interval_recorded`
            // tracks whether this interval actually folded into at
            // least one aggregator — intervals dropped whole (age
            // reset, target mount absent) must not count as samples.
            let mut interval_recorded = false;
            for current_mount in &current_monitor_mounts {
                if let Some(previous_mount) = previous_mounts.get(&current_mount.mount_point) {
                    // Age moving backwards means the mount was
                    // re-created since the previous sample: every
                    // kernel counter — ops, bytes, events, and the
                    // xprt transports (which carry no age of their
                    // own) — rebased. Skip the whole interval for
                    // this mount and rebase against the fresh
                    // counters for the next one.
                    if current_mount.age < previous_mount.age {
                        previous_mounts
                            .insert(current_mount.mount_point.clone(), current_mount.clone());
                        continue;
                    }

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

                    // Mount-level byte/event deltas feed the metrics
                    // exporter. None means a counter reset invalidated
                    // this interval.
                    let mount_deltas = calculate_mount_deltas(previous_mount, current_mount);

                    // Either aggregate for the end-of-session report
                    // or render the live table — never both, because
                    // output mode is intended as a silent capture.
                    if output_mode {
                        if let Some(agg) = aggregators.get_mut(&current_mount.mount_point) {
                            // Every interval is recorded, including
                            // op-idle ones, so covered time tracks
                            // real wall clock: a mount idle for 90%
                            // of a session must not report rates as
                            // if it had been busy throughout. An
                            // idle sample contributes elapsed time
                            // and nothing else.
                            agg.record(&delta_stats, elapsed_seconds);
                            interval_recorded = true;
                            // xprt folds into the same aggregator so
                            // the finalised MountReport carries a
                            // matching XprtReport. Recorded even
                            // when no op completed: transports can
                            // move (sends, backlog) while every op
                            // is stuck in flight, which is exactly
                            // the slot-pressure signal.
                            if let Some(ref x) = xprt_delta {
                                agg.record_xprt(x);
                            }
                        }
                    } else if !delta_stats.is_empty() {
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
                    } else if let Some(ref x) = xprt_delta {
                        // No op completed, but the transport moved:
                        // the stall case. A truly idle mount (no op
                        // and no transport activity) still renders
                        // nothing.
                        if x.has_activity() {
                            display_xprt_only(writer, current_mount, x, slot_cap, &timestamp)?;
                        }
                    }

                    // Metrics export is NOT gated on op activity: a
                    // stalled mount (ops frozen in-flight, backlog
                    // climbing) is exactly when the xprt pressure
                    // counters matter most, and gauges must keep
                    // refreshing on idle mounts. Empty delta_stats
                    // just means the op counters do not move. This is
                    // orthogonal to on-disk reports, so scrapers stay
                    // live during an -o session too.
                    if let Some(manager) = metrics_manager {
                        manager.export_metrics(current_mount, &delta_stats, mount_deltas.as_ref());
                        if let Some(ref x) = xprt_delta {
                            manager.export_xprt(current_mount, x, slot_cap);
                        }
                    }
                }

                // Update previous mount data
                previous_mounts.insert(current_mount.mount_point.clone(), current_mount.clone());
            }

            if output_mode {
                if interval_recorded {
                    samples_collected += 1;
                }
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
            measured_intervals += 1;
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
                .map(|agg| agg.finalise(slot_cap))
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
            fstype: "nfs".to_string(),
            options: String::new(),
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
                mount_point: None,
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
                mount_point: None,
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
                mount_point: None,
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

    /// Write a two-mount mountstats file whose counters scale with
    /// `k`, atomically (write + rename) so a concurrently running
    /// monitoring loop never reads a torn file.
    fn write_two_mount_stats(path: &std::path::Path, k: i64) {
        let content = format!(
            r#"device serverA:/exportA mounted on /mnt/a with fstype nfs statvers=1.1
	opts:	rw,vers=3,proto=tcp
	age:	{age_a}
	bytes:	0 0 0 0 {rd_a} {wr_a} 0 0
	xprt:	tcp 900 1 8 0 0 {x_a} {x_a} 0 {x_a} 0 4 {snd_a} {pnd_a}
	per-op statistics
	        READ: {ops_a} {ops_a} 0 {sent_a} {recv_a} {q_a} {rtt_a} {exe_a} 0
device serverB:/exportB mounted on /mnt/b with fstype nfs statvers=1.1
	age:	{age_b}
	bytes:	0 0 0 0 {rd_b} {wr_b} 0 0
	xprt:	tcp 901 1 8 0 0 {x_b} {x_b} 0 {x_b} 0 8 {snd_b} {pnd_b}
	per-op statistics
	       WRITE: {ops_b} {ops_b} 0 {sent_b} {recv_b} {q_b} {rtt_b} {exe_b} 0
"#,
            age_a = 1000 + k,
            rd_a = k * 1_048_576,
            wr_a = k * 524_288,
            x_a = k * 100,
            snd_a = k * 30,
            pnd_a = k * 500,
            ops_a = k * 100,
            sent_a = k * 13_200,
            recv_a = k * 3_276_800,
            q_a = k * 10,
            rtt_a = k * 150,
            exe_a = k * 170,
            age_b = 2000 + k,
            rd_b = k * 2_097_152,
            wr_b = k * 1_048_576,
            x_b = k * 200,
            snd_b = k * 60,
            pnd_b = k * 900,
            ops_b = k * 300,
            sent_b = k * 9_830_400,
            recv_b = k * 42_000,
            q_b = k * 30,
            rtt_b = k * 900,
            exe_b = k * 1_000,
        );
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, content).expect("write temp mountstats");
        std::fs::rename(&tmp, path).expect("rename mountstats into place");
    }

    /// Run the monitoring loop against a live-updated two-mount file
    /// and return everything it wrote to the display writer. The file
    /// is bumped every 50 ms so every 200 ms sampling interval is
    /// guaranteed to observe activity on both mounts.
    fn run_loop_against_bumped_file(
        mount_point: Option<String>,
        count: usize,
        duration: Option<Duration>,
    ) -> String {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("create tempdir");
        let path = dir.path().join("mountstats");
        write_two_mount_stats(&path, 1);

        let bump_path = path.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let bump_stop = stop.clone();
        let bumper = thread::spawn(move || {
            let mut k = 2;
            while !bump_stop.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(50));
                write_two_mount_stats(&bump_path, k);
                k += 1;
            }
        });

        let monitor = Monitor::new();
        let mut output = Vec::<u8>::new();
        let result = monitor.monitoring_loop(
            &mut output,
            MonitorConfig {
                mountstats_path: path.to_str().expect("utf-8 path"),
                monitor_mounts: vec![],
                mount_point,
                operations_filter: HashSet::new(),
                interval: Duration::from_millis(200),
                count,
                duration,
                output: None,
                slot_cap: Some(128),
                show_bandwidth: false,
                clear_screen: false,
                metrics_manager: None,
            },
        );
        stop.store(true, Ordering::SeqCst);
        bumper.join().expect("bumper thread");
        result.expect("loop should exit cleanly");

        String::from_utf8(output).expect("display output is utf-8")
    }

    #[test]
    fn test_monitoring_loop_count_one_yields_one_measurement() {
        // The README sells "-c 1" as a single measurement for
        // monitoring systems. The seed sample must not consume the
        // count: exactly one interval must render. The generous
        // duration is a safety net so a regression cannot hang the
        // test — it would fail the assertion instead.
        let output = run_loop_against_bumped_file(
            Some("/mnt/a".to_string()),
            1,
            Some(Duration::from_secs(5)),
        );
        let headers = output.matches(" mounted on /mnt/a").count();
        assert_eq!(
            headers, 1,
            "-c 1 must render exactly one interval: {output}"
        );
    }

    #[test]
    fn test_monitoring_loop_count_two_yields_two_measurements() {
        let output = run_loop_against_bumped_file(
            Some("/mnt/a".to_string()),
            2,
            Some(Duration::from_secs(5)),
        );
        let headers = output.matches(" mounted on /mnt/a").count();
        assert_eq!(
            headers, 2,
            "-c 2 must render exactly two intervals: {output}"
        );
    }

    #[test]
    fn test_output_mode_rates_use_covered_time_not_requested_duration() {
        use tempfile::TempDir;

        // The bumper advances /mnt/a's READ counter by 100 ops every
        // 50 ms — a true rate of 2000 ops/s. The seed sample covers
        // none of the capture, so dividing by the requested duration
        // instead of the measured covered time understates the rate
        // by interval/duration (~33% here). The generous band still
        // catches that regression (~1333 ops/s) while tolerating
        // scheduler jitter and the 50 ms bump granularity.
        let dir = TempDir::new().expect("create tempdir");
        let path = dir.path().join("mountstats");
        let report_path = dir.path().join("report.json");
        write_two_mount_stats(&path, 1);

        // Mirror main(): the aggregation target set is locked in from
        // an initial parse before the loop starts.
        let initial_mounts = crate::parser::parse_mountstats(path.to_str().expect("utf-8 path"))
            .expect("initial parse");
        let monitor_mounts =
            Monitor::get_mounts_to_monitor(Some("/mnt/a".to_string()), &initial_mounts)
                .expect("mount filter matches");

        let bump_path = path.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let bump_stop = stop.clone();
        let bumper = thread::spawn(move || {
            let mut k = 2;
            while !bump_stop.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(50));
                write_two_mount_stats(&bump_path, k);
                k += 1;
            }
        });

        let monitor = Monitor::new();
        let mut output = Vec::<u8>::new();
        let result = monitor.monitoring_loop(
            &mut output,
            MonitorConfig {
                mountstats_path: path.to_str().expect("utf-8 path"),
                monitor_mounts,
                mount_point: Some("/mnt/a".to_string()),
                operations_filter: HashSet::new(),
                interval: Duration::from_millis(400),
                count: 0,
                duration: Some(Duration::from_millis(1200)),
                output: Some(report_path.clone()),
                slot_cap: None,
                show_bandwidth: false,
                clear_screen: false,
                metrics_manager: None,
            },
        );
        stop.store(true, Ordering::SeqCst);
        bumper.join().expect("bumper thread");
        result.expect("loop should exit cleanly");

        let json = std::fs::read_to_string(&report_path).expect("read report file");
        let report: crate::snapshot::Report =
            serde_json::from_str(&json).expect("report must be valid JSON");
        let mount = report
            .mounts
            .iter()
            .find(|m| m.mount_point == "/mnt/a")
            .expect("report must include the monitored mount");

        assert!(
            mount.covered_sec > 0.0,
            "covered_sec must reflect measured intervals: {json}"
        );
        assert!(
            report.samples >= 1,
            "recorded intervals must be counted as samples: {json}"
        );
        assert_eq!(
            mount.fstype, "nfs",
            "fstype from the device line must land in the report: {json}"
        );
        assert_eq!(
            mount.options, "rw,vers=3,proto=tcp",
            "options from the opts: line must land in the report: {json}"
        );
        assert!(
            (1500.0..=2700.0).contains(&mount.summary.ops_per_sec),
            "true rate is 2000 ops/s; requested-duration denominators would \
             report ~1333. got {} over covered_sec {}",
            mount.summary.ops_per_sec,
            mount.covered_sec
        );
        // The report must be internally consistent: rate × covered
        // time reproduces the op total exactly.
        assert!(
            (mount.summary.ops_per_sec * mount.covered_sec - mount.summary.total_ops as f64).abs()
                < 1.0,
            "ops_per_sec ({}) × covered_sec ({}) must equal total_ops ({})",
            mount.summary.ops_per_sec,
            mount.covered_sec,
            mount.summary.total_ops
        );
    }

    #[test]
    fn test_report_samples_counts_only_intervals_that_folded() {
        use tempfile::TempDir;

        // The -m target exists at session start (so the aggregator is
        // allocated) and then vanishes from mountstats for the entire
        // capture. Nothing can fold, so the report must say 0 samples
        // and 0 covered seconds — the old per-iteration counter would
        // have reported one "sample" per loop pass, promising data
        // the report does not contain.
        let dir = TempDir::new().expect("create tempdir");
        let path = dir.path().join("mountstats");
        let report_path = dir.path().join("report.json");
        write_single_mount_stats(&path, 1000, 100);

        let initial_mounts = crate::parser::parse_mountstats(path.to_str().expect("utf-8 path"))
            .expect("initial parse");
        let monitor_mounts =
            Monitor::get_mounts_to_monitor(Some("/mnt/a".to_string()), &initial_mounts)
                .expect("mount filter matches");

        // Unmount simulation: only a non-NFS filesystem remains.
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, "device /dev/sda1 mounted on /boot with fstype ext4\n")
            .expect("write temp mountstats");
        std::fs::rename(&tmp, &path).expect("rename mountstats into place");

        let monitor = Monitor::new();
        let mut output = Vec::<u8>::new();
        monitor
            .monitoring_loop(
                &mut output,
                MonitorConfig {
                    mountstats_path: path.to_str().expect("utf-8 path"),
                    monitor_mounts,
                    mount_point: Some("/mnt/a".to_string()),
                    operations_filter: HashSet::new(),
                    interval: Duration::from_millis(100),
                    count: 0,
                    duration: Some(Duration::from_millis(500)),
                    output: Some(report_path.clone()),
                    slot_cap: None,
                    show_bandwidth: false,
                    clear_screen: false,
                    metrics_manager: None,
                },
            )
            .expect("loop should exit cleanly");

        let json = std::fs::read_to_string(&report_path).expect("read report file");
        let report: crate::snapshot::Report =
            serde_json::from_str(&json).expect("report must be valid JSON");
        assert_eq!(
            report.samples, 0,
            "no interval folded, so no interval may be counted: {json}"
        );
        let mount = report
            .mounts
            .iter()
            .find(|m| m.mount_point == "/mnt/a")
            .expect("aggregator was allocated at session start");
        assert_eq!(mount.covered_sec, 0.0, "nothing recorded: {json}");
        assert_eq!(mount.summary.total_ops, 0, "nothing recorded: {json}");
    }

    /// Single-mount writer simulating a stalled mount: the READ op
    /// counters are frozen (no RPC ever completes) while the
    /// transport keeps working — sends, requests, and the backlog
    /// queue all climb with `k`, and recvs stay flat because no
    /// replies are coming back.
    fn write_stalled_mount_stats(path: &std::path::Path, k: i64) {
        let content = format!(
            r#"device serverA:/exportA mounted on /mnt/a with fstype nfs statvers=1.1
	age:	{age}
	xprt:	tcp 900 1 8 0 0 {sends} 500 0 {req} {bklog} 128 4000 90000
	per-op statistics
	        READ: 500 500 0 66000 16384000 50 500 1000 0
"#,
            age = 1000 + k,
            sends = k * 100,
            req = k * 100,
            bklog = k * 50,
        );
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, content).expect("write temp mountstats");
        std::fs::rename(&tmp, path).expect("rename mountstats into place");
    }

    /// Run the loop against a stalled mount and return (display
    /// output, report JSON if `capture` is set).
    fn run_loop_against_stalled_mount(capture: bool) -> (String, Option<String>) {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("create tempdir");
        let path = dir.path().join("mountstats");
        let report_path = dir.path().join("report.json");
        write_stalled_mount_stats(&path, 1);

        let initial_mounts = crate::parser::parse_mountstats(path.to_str().expect("utf-8 path"))
            .expect("initial parse");
        let monitor_mounts =
            Monitor::get_mounts_to_monitor(Some("/mnt/a".to_string()), &initial_mounts)
                .expect("mount filter matches");

        let bump_path = path.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let bump_stop = stop.clone();
        let bumper = thread::spawn(move || {
            let mut k = 2;
            while !bump_stop.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(50));
                write_stalled_mount_stats(&bump_path, k);
                k += 1;
            }
        });

        let monitor = Monitor::new();
        let mut output = Vec::<u8>::new();
        let result = monitor.monitoring_loop(
            &mut output,
            MonitorConfig {
                mountstats_path: path.to_str().expect("utf-8 path"),
                monitor_mounts,
                mount_point: Some("/mnt/a".to_string()),
                operations_filter: HashSet::new(),
                interval: Duration::from_millis(200),
                count: 0,
                duration: Some(Duration::from_millis(900)),
                output: capture.then(|| report_path.clone()),
                slot_cap: Some(128),
                show_bandwidth: false,
                clear_screen: false,
                metrics_manager: None,
            },
        );
        stop.store(true, Ordering::SeqCst);
        bumper.join().expect("bumper thread");
        result.expect("loop should exit cleanly");

        let display = String::from_utf8(output).expect("display output is utf-8");
        let json =
            capture.then(|| std::fs::read_to_string(&report_path).expect("read report file"));
        (display, json)
    }

    #[test]
    fn test_live_mode_shows_xprt_pressure_when_ops_are_stalled() {
        // Ops frozen + transport churning is the signature of a
        // stalled mount. The pre-fix loop rendered nothing at all in
        // that state — silent exactly when the slot-pressure signal
        // mattered most.
        let (output, _) = run_loop_against_stalled_mount(false);

        assert!(
            output.contains("serverA:/exportA mounted on /mnt/a"),
            "stall intervals must identify the mount: {output}"
        );
        assert!(
            output.contains("no operations completed this interval; transport still active"),
            "stall intervals must render the stall note: {output}"
        );
        assert!(
            output.contains("xprt tcp slots"),
            "stall intervals must render the xprt summary: {output}"
        );
        assert!(
            !output.lines().any(|l| l.trim_start().starts_with("READ")),
            "no op completed, so no op rows may render: {output}"
        );
    }

    #[test]
    fn test_output_mode_report_carries_xprt_work_from_stalled_intervals() {
        // Every interval in this capture is op-idle, but the
        // transport moved. The report must still carry the xprt
        // session totals (record_xprt is not gated on op activity)
        // and account the covered wall-clock time.
        let (_, json) = run_loop_against_stalled_mount(true);
        let json = json.expect("capture mode writes a report");
        let report: crate::snapshot::Report =
            serde_json::from_str(&json).expect("report must be valid JSON");
        let mount = report
            .mounts
            .iter()
            .find(|m| m.mount_point == "/mnt/a")
            .expect("report must include the monitored mount");

        assert_eq!(
            mount.summary.total_ops, 0,
            "the op counters never moved: {json}"
        );
        assert!(
            mount.covered_sec > 0.0,
            "op-idle intervals still cover wall-clock time: {json}"
        );
        let xprt = mount
            .xprt
            .as_ref()
            .expect("transport activity must produce an XprtReport");
        assert!(
            xprt.sends > 0,
            "sends from op-idle intervals must land in the report: {json}"
        );
    }

    /// Single-mount mountstats writer for remount simulations: `age`
    /// and the READ op counters are fully caller-controlled.
    fn write_single_mount_stats(path: &std::path::Path, age: i64, ops: i64) {
        let content = format!(
            r#"device serverA:/exportA mounted on /mnt/a with fstype nfs statvers=1.1
	age:	{age}
	per-op statistics
	        READ: {ops} {ops} 0 {sent} {recv} {queue} {rtt} {exe} 0
"#,
            age = age,
            ops = ops,
            sent = ops * 132,
            recv = ops * 32_768,
            queue = ops / 10,
            rtt = ops,
            exe = ops * 2,
        );
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, content).expect("write temp mountstats");
        std::fs::rename(&tmp, path).expect("rename mountstats into place");
    }

    #[test]
    fn test_monitoring_loop_drops_interval_spanning_a_remount() {
        use tempfile::TempDir;

        let dir = TempDir::new().expect("create tempdir");
        let path = dir.path().join("mountstats");
        write_single_mount_stats(&path, 1001, 100);

        // Normal activity, then a remount whose fresh counters are far
        // LARGER than the previous sample's — the case the per-op
        // monotonic checks cannot catch — then normal activity again.
        // Without the age check, the remount-spanning interval would
        // fabricate a delta of ~50k ops and display a monster IOPS
        // spike.
        let schedule: Vec<(i64, i64)> = vec![
            (1002, 200),
            (1003, 300),
            (1004, 400),
            (2, 50_000),
            (3, 50_100),
            (4, 50_200),
            (5, 50_300),
            (6, 50_400),
            (7, 50_500),
        ];
        let bump_path = path.clone();
        let bumper = thread::spawn(move || {
            for (age, ops) in schedule {
                thread::sleep(Duration::from_millis(100));
                write_single_mount_stats(&bump_path, age, ops);
            }
        });

        let monitor = Monitor::new();
        let mut output = Vec::<u8>::new();
        monitor
            .monitoring_loop(
                &mut output,
                MonitorConfig {
                    mountstats_path: path.to_str().expect("utf-8 path"),
                    monitor_mounts: vec![],
                    mount_point: Some("/mnt/a".to_string()),
                    operations_filter: HashSet::new(),
                    interval: Duration::from_millis(200),
                    count: 0,
                    duration: Some(Duration::from_millis(1300)),
                    output: None,
                    slot_cap: None,
                    show_bandwidth: false,
                    clear_screen: false,
                    metrics_manager: None,
                },
            )
            .expect("loop should exit cleanly on duration");
        bumper.join().expect("bumper thread");

        let output = String::from_utf8(output).expect("utf-8");
        for line in output
            .lines()
            .filter(|l| l.trim_start().starts_with("READ"))
        {
            let iops: f64 = line
                .split_whitespace()
                .nth(1)
                .expect("IOPS column present")
                .parse()
                .expect("IOPS parses as a number");
            assert!(
                iops < 10_000.0,
                "a remount-spanning interval leaked a bogus spike: {line}\nfull output:\n{output}"
            );
        }
    }

    #[test]
    fn test_monitoring_loop_honors_mount_filter() {
        // Both mounts are active every interval; with -m /mnt/a only
        // /mnt/a may appear in the display output. The pre-fix loop
        // re-listed all mounts each interval and leaked /mnt/b from
        // the second interval onward.
        let output = run_loop_against_bumped_file(
            Some("/mnt/a".to_string()),
            0,
            Some(Duration::from_millis(1300)),
        );

        assert!(
            output.contains("/mnt/a"),
            "the selected mount must be displayed (harness produced no activity?): {output}"
        );
        assert!(
            !output.contains("/mnt/b"),
            "-m /mnt/a must not display other mounts: {output}"
        );
    }

    #[test]
    fn test_monitoring_loop_displays_all_mounts_in_stable_order() {
        // Without -m, both active mounts appear — and in sorted order
        // within every interval, not HashMap order.
        let output = run_loop_against_bumped_file(None, 0, Some(Duration::from_millis(1300)));

        let headers: Vec<&str> = output
            .lines()
            .filter(|l| l.contains(" mounted on "))
            .collect();
        assert!(
            headers.len() >= 2,
            "expected at least one interval showing both mounts: {output}"
        );
        for pair in headers.chunks(2) {
            if let [first, second] = pair {
                assert!(
                    first.ends_with("/mnt/a") && second.ends_with("/mnt/b"),
                    "mounts must render in sorted order every interval, got {first} then {second}"
                );
            }
        }
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
