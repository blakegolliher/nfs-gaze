use crate::types::{DeltaStats, NFSMount, Result};
use chrono::{DateTime, Utc};
use std::io::Write;

/// Display statistics in a simple table format
pub fn display_stats_simple<W: Write>(
    writer: &mut W,
    mount: &NFSMount,
    stats: &[DeltaStats],
    show_bandwidth: bool,
    timestamp: &DateTime<Utc>,
) -> Result<()> {
    if stats.is_empty() {
        return Ok(());
    }

    writeln!(writer, "{} mounted on {}", mount.device, mount.mount_point)?;
    writeln!(
        writer,
        "Timestamp: {}",
        timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    )?;
    writeln!(writer)?;

    // Common columns + optional bandwidth columns
    let bw_cols: &[&str] = if show_bandwidth {
        &["MB/s", "KB/op"]
    } else {
        &[]
    };
    write!(
        writer,
        "{:<12} {:>8} {:>10} {:>10}",
        "OP", "IOPS", "RTT", "EXE"
    )?;
    for col in bw_cols {
        write!(writer, " {:>8}", col)?;
    }
    writeln!(writer, " {:>8}", "ERRORS")?;

    let width = if show_bandwidth { 76 } else { 52 };
    writeln!(writer, "{}", "-".repeat(width))?;

    for stat in stats {
        write!(
            writer,
            "{:<12} {:>8} {:>10} {:>10}",
            stat.operation,
            format_rate(stat.iops),
            format_duration(stat.avg_rtt),
            format_duration(stat.avg_exec),
        )?;
        if show_bandwidth {
            write!(
                writer,
                " {:>8} {:>8}",
                format_bandwidth(stat.kb_per_sec),
                format_rate(stat.kb_per_op),
            )?;
        }
        writeln!(writer, " {:>8}", stat.delta_errors)?;
    }

    writeln!(writer)?;
    Ok(())
}

/// Format duration with automatic unit selection.
/// Input is in milliseconds (as reported by Linux mountstats via ktime_to_ms).
pub fn format_duration(ms: f64) -> String {
    if ms == 0.0 {
        "0.00ms".to_string()
    } else if ms < 1.0 {
        format!("{:.2}μs", ms * 1000.0)
    } else if ms < 1000.0 {
        format!("{:.2}ms", ms)
    } else {
        format!("{:.2}s", ms / 1000.0)
    }
}

/// Format rate with appropriate precision
pub fn format_rate(rate: f64) -> String {
    if rate == 0.0 {
        "0.0".to_string()
    } else {
        format!("{:.1}", rate)
    }
}

/// Format bandwidth in MB/s (converting from KB/s)
pub fn format_bandwidth(kb_per_sec: f64) -> String {
    let mb_per_sec = kb_per_sec / 1024.0;
    if mb_per_sec == 0.0 {
        "0.0".to_string()
    } else {
        format!("{:.1}", mb_per_sec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NFSMount;
    use chrono::TimeZone;
    use std::collections::HashMap;

    #[test]
    fn test_display_stats_empty() {
        let mount = NFSMount {
            device: "server:/export".to_string(),
            mount_point: "/mnt/nfs".to_string(),
            server: "server".to_string(),
            export: "/export".to_string(),
            age: 0,
            operations: HashMap::new(),
            events: None,
            bytes_read: 0,
            bytes_write: 0,
            xprt: None,
        };

        let stats = vec![];
        let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let mut buf = Vec::new();

        display_stats_simple(&mut buf, &mount, &stats, false, &timestamp).unwrap();

        assert!(buf.is_empty());
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.0), "0.00ms");
        assert_eq!(format_duration(0.5), "500.00μs");
        assert_eq!(format_duration(0.001), "1.00μs");
        assert_eq!(format_duration(1.0), "1.00ms");
        assert_eq!(format_duration(144.5), "144.50ms");
        assert_eq!(format_duration(999.0), "999.00ms");
        assert_eq!(format_duration(1500.0), "1.50s");
    }

    #[test]
    fn test_format_rate() {
        assert_eq!(format_rate(0.0), "0.0");
        assert_eq!(format_rate(1.5), "1.5");
        assert_eq!(format_rate(100.0), "100.0");
        assert_eq!(format_rate(1000.5), "1000.5");
    }

    #[test]
    fn test_format_bandwidth() {
        assert_eq!(format_bandwidth(0.0), "0.0");
        assert_eq!(format_bandwidth(512.0), "0.5");
        assert_eq!(format_bandwidth(1024.0), "1.0");
        assert_eq!(format_bandwidth(1536.0), "1.5");
        assert_eq!(format_bandwidth(10240.0), "10.0");
    }
}
