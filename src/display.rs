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

    // Write header information
    writeln!(writer, "{} mounted on {}", mount.device, mount.mount_point)?;
    writeln!(
        writer,
        "Timestamp: {}",
        timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    )?;
    writeln!(writer)?;

    // Write table headers
    if show_bandwidth {
        writeln!(
            writer,
            "{:<12} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
            "OP", "IOPS", "RTT(ms)", "EXE(ms)", "MB/s", "KB/op", "ERRORS"
        )?;
    } else {
        writeln!(
            writer,
            "{:<12} {:>8} {:>8} {:>8} {:>8}",
            "OP", "IOPS", "RTT(ms)", "EXE(ms)", "ERRORS"
        )?;
    }

    // Write separator line
    if show_bandwidth {
        writeln!(writer, "{}", "-".repeat(72))?;
    } else {
        writeln!(writer, "{}", "-".repeat(48))?;
    }

    // Write data rows
    for stat in stats {
        if show_bandwidth {
            writeln!(
                writer,
                "{:<12} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
                stat.operation,
                format_rate(stat.iops),
                format_duration(stat.avg_rtt as i64),
                format_duration(stat.avg_exec as i64),
                format_bandwidth(stat.kb_per_sec),
                format_rate(stat.kb_per_op),
                stat.delta_errors
            )?;
        } else {
            writeln!(
                writer,
                "{:<12} {:>8} {:>8} {:>8} {:>8}",
                stat.operation,
                format_rate(stat.iops),
                format_duration(stat.avg_rtt as i64),
                format_duration(stat.avg_exec as i64),
                stat.delta_errors
            )?;
        }
    }

    writeln!(writer)?;
    Ok(())
}

/// Format duration in milliseconds with appropriate precision
pub fn format_duration(ms: i64) -> String {
    if ms == 0 {
        "0.0ms".to_string()
    } else {
        format!("{:.1}ms", ms as f64 / 1000.0)
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

    struct MockWriter {
        pub buffer: Vec<u8>,
    }

    impl MockWriter {
        fn new() -> Self {
            Self { buffer: Vec::new() }
        }

        fn to_string(&self) -> String {
            String::from_utf8(self.buffer.clone()).unwrap()
        }
    }

    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buffer.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

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
        };

        let stats = vec![];
        let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let mut writer = MockWriter::new();

        display_stats_simple(&mut writer, &mount, &stats, false, &timestamp).unwrap();
        let output = writer.to_string();

        assert!(output.is_empty());
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "0.0ms");
        assert_eq!(format_duration(500), "0.5ms");
        assert_eq!(format_duration(1500), "1.5ms");
        assert_eq!(format_duration(10000), "10.0ms");
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
