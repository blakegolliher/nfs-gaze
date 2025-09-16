use clap::Parser;
use std::collections::HashSet;

#[derive(Parser, Debug)]
#[command(name = "nfs-gaze")]
#[command(about = "NFS I/O Statistics Monitor")]
#[command(long_about = r#"
NFS I/O Statistics Monitor

Monitor NFS mount point I/O statistics in real-time by parsing /proc/self/mountstats.
Displays operations per second, latency metrics, and bandwidth statistics.

Examples:
  # Monitor with attribute cache statistics
  nfs-gaze /mnt/nfs --attr

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

    /// Show attribute cache statistics
    #[arg(long = "attr")]
    pub show_attr: bool,

    /// Show bandwidth statistics
    #[arg(long = "bw")]
    pub show_bandwidth: bool,

    /// Clear screen between iterations
    #[arg(long = "clear")]
    pub clear_screen: bool,

    /// Path to mountstats file
    #[arg(short = 'f', long, default_value = "/proc/self/mountstats")]
    pub mountstats_path: String,
}

/// Parse operations filter string into a HashSet of operation names
pub fn parse_operations_filter(operations: Option<String>) -> HashSet<String> {
    match operations {
        Some(ops_str) if !ops_str.trim().is_empty() => {
            ops_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
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