use clap::{Args as ClapArgs, Parser, Subcommand};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "nfs-gaze")]
#[command(version)]
#[command(about = "NFS I/O Statistics Monitor")]
// When a subcommand is present, suppress the top-level "run mode"
// flags so that e.g. `nfs-gaze compare a.json b.json` does not error
// out on missing required-for-run defaults. Top-level flags remain
// valid when no subcommand is given, preserving the original CLI
// shape.
#[command(args_conflicts_with_subcommands = true)]
#[command(long_about = r#"
NFS I/O Statistics Monitor

Monitor NFS mount point I/O statistics in real-time by parsing /proc/self/mountstats.
Displays operations per second, latency metrics, and bandwidth statistics.

Examples:
  # Monitor all NFS mounts
  nfs-gaze

  # Monitor a specific mount point
  nfs-gaze -m /mnt/nfs

  # Monitor specific operations with bandwidth
  nfs-gaze -m /mnt/nfs --ops READ,WRITE --bw

  # Capture a 5-minute JSON snapshot for later analysis
  nfs-gaze -d 300 -o /tmp/baseline.json

  # Compare two snapshots
  nfs-gaze compare baseline.json new-run.json BASELINE NEW
"#)]
pub struct Args {
    /// Mount point to monitor
    #[arg(short = 'm', long)]
    pub mount_point: Option<String>,

    /// Comma-separated list of operations to monitor
    #[arg(long = "ops")]
    pub operations: Option<String>,

    /// Update interval in seconds
    #[arg(short = 'i', long, default_value = "1")]
    pub interval: u64,

    /// Number of measured intervals (0 = infinite). The seed sample
    /// taken at startup is not counted, so `-c 1` produces exactly
    /// one measurement.
    #[arg(short = 'c', long, default_value = "0")]
    pub count: usize,

    /// Total capture duration in seconds (mutually exclusive with --count)
    #[arg(short = 'd', long, conflicts_with = "count")]
    pub duration: Option<u64>,

    /// Write a JSON snapshot report to this path at end of session.
    /// Suppresses the live per-interval table and replaces it with a
    /// `Sampling...` progress line on stderr.
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// Show bandwidth statistics
    #[arg(long = "bw")]
    pub show_bandwidth: bool,

    /// Clear screen between iterations
    #[arg(long = "clear")]
    pub clear_screen: bool,

    /// Path to mountstats file
    #[arg(short = 'f', long, default_value = "/proc/self/mountstats")]
    pub mountstats_path: String,

    /// Enable Prometheus metrics export
    #[cfg(feature = "prometheus")]
    #[arg(long)]
    pub prometheus: bool,

    /// Prometheus metrics server bind address
    #[cfg(feature = "prometheus")]
    #[arg(long, default_value = "127.0.0.1")]
    pub prometheus_bind: String,

    /// Prometheus metrics server port
    #[cfg(feature = "prometheus")]
    #[arg(long, default_value = "9100")]
    pub prometheus_port: u16,

    /// Metrics export interval in seconds
    #[arg(long, default_value = "10")]
    pub metrics_interval: u64,

    /// Optional subcommand. When present, the top-level run-mode
    /// flags above are ignored.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Subcommands that branch away from the default "live monitor" mode.
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Compare two nfs-gaze JSON snapshot reports side by side
    Compare(CompareArgs),
}

/// Arguments for the `compare` subcommand.
#[derive(ClapArgs, Debug, Clone)]
pub struct CompareArgs {
    /// First report file (baseline)
    pub file1: PathBuf,
    /// Second report file (comparison)
    pub file2: PathBuf,
    /// Optional display label for `file1` (defaults to "File1")
    pub label1: Option<String>,
    /// Optional display label for `file2` (defaults to "File2")
    pub label2: Option<String>,
}

impl Args {
    /// Convert CLI args to MetricsConfig
    pub fn to_metrics_config(&self) -> crate::metrics::MetricsConfig {
        crate::metrics::MetricsConfig {
            #[cfg(feature = "prometheus")]
            enable_prometheus: self.prometheus,
            #[cfg(not(feature = "prometheus"))]
            enable_prometheus: false,

            #[cfg(feature = "prometheus")]
            prometheus_bind: self.prometheus_bind.clone(),
            #[cfg(not(feature = "prometheus"))]
            prometheus_bind: "127.0.0.1".to_string(),

            #[cfg(feature = "prometheus")]
            prometheus_port: self.prometheus_port,
            #[cfg(not(feature = "prometheus"))]
            prometheus_port: 9100,

            export_interval: std::time::Duration::from_secs(self.metrics_interval),
            include_labels: true,
        }
    }
}

/// Parse operations filter string into a HashSet of operation names
pub fn parse_operations_filter(operations: Option<String>) -> HashSet<String> {
    match operations {
        Some(ops_str) if !ops_str.trim().is_empty() => ops_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => HashSet::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_operations_filter_empty() {
        let filter = parse_operations_filter(None);
        assert!(filter.is_empty());

        let filter = parse_operations_filter(Some("".to_string()));
        assert!(filter.is_empty());

        let filter = parse_operations_filter(Some("   ".to_string()));
        assert!(filter.is_empty());
    }

    #[test]
    fn test_parse_operations_filter_single() {
        let filter = parse_operations_filter(Some("READ".to_string()));
        assert_eq!(filter.len(), 1);
        assert!(filter.contains("READ"));
    }

    #[test]
    fn test_parse_operations_filter_multiple() {
        let filter = parse_operations_filter(Some("READ,WRITE,GETATTR".to_string()));
        assert_eq!(filter.len(), 3);
        assert!(filter.contains("READ"));
        assert!(filter.contains("WRITE"));
        assert!(filter.contains("GETATTR"));
    }

    #[test]
    fn test_parse_operations_filter_whitespace() {
        let filter = parse_operations_filter(Some(" READ , WRITE , GETATTR ".to_string()));
        assert_eq!(filter.len(), 3);
        assert!(filter.contains("READ"));
        assert!(filter.contains("WRITE"));
        assert!(filter.contains("GETATTR"));
    }
}
