use clap::Parser;
use std::collections::HashSet;

#[derive(Parser, Debug)]
#[command(name = "nfs-gaze")]
#[command(version)]
#[command(about = "NFS I/O Statistics Monitor")]
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

  # Clear screen between iterations
  nfs-gaze -m /mnt/nfs --clear
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

    /// Number of iterations (0 = infinite)
    #[arg(short = 'c', long, default_value = "0")]
    pub count: usize,

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
