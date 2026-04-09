use crate::display::display_stats_simple;
use crate::parser::parse_mountstats;
use crate::stats::{calculate_delta_stats, filter_operations};
use crate::types::{NFSMount, NfsGazeError, Result};
use chrono::Utc;
use signal_hook::{consts::SIGINT, consts::SIGTERM, iterator::Signals};
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
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

/// Configuration for the monitoring loop.
///
/// Collected into a struct so `monitoring_loop` can be called with a
/// single argument instead of a long positional list.
pub struct MonitorConfig<'a> {
    pub mountstats_path: &'a str,
    pub monitor_mounts: Vec<NFSMount>,
    pub operations_filter: HashSet<String>,
    pub interval: Duration,
    pub count: usize,
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
            show_bandwidth,
            clear_screen,
            metrics_manager,
        } = config;

        let mut previous_mounts: HashMap<String, NFSMount> = monitor_mounts
            .iter()
            .map(|m| (m.mount_point.clone(), m.clone()))
            .collect();

        let mut iteration = 0;
        let mut last_update = Instant::now();
        let mut consecutive_parse_failures: u32 = 0;

        while self.running.load(Ordering::SeqCst) {
            // Check if we've reached the iteration limit
            if count > 0 && iteration >= count {
                break;
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

            // Clear screen if requested
            if clear_screen {
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

                    // Display stats if we have any
                    if !delta_stats.is_empty() {
                        display_stats_simple(
                            writer,
                            current_mount,
                            &delta_stats,
                            show_bandwidth,
                            &timestamp,
                        )?;

                        // Export metrics if enabled
                        if let Some(manager) = metrics_manager {
                            manager.export_metrics(current_mount, &delta_stats);
                        }
                    }
                }

                // Update previous mount data
                previous_mounts.insert(current_mount.mount_point.clone(), current_mount.clone());
            }

            iteration += 1;
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
