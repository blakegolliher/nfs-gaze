use nfs_gaze::{DeltaStats, NFSMount};
use nfs_gaze::display::display_stats_simple;
use std::collections::HashMap;
use chrono::{Utc, TimeZone};
use std::io::{self, Write};

// Mock writer to capture output instead of writing to stdout
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
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_display_stats_simple_without_bandwidth() {
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

    let stats = vec![
        DeltaStats {
            operation: "READ".to_string(),
            delta_ops: 100,
            delta_bytes: 0,
            delta_sent: 0,
            delta_recv: 0,
            delta_rtt: 0,
            delta_exec: 0,
            delta_queue: 0,
            delta_errors: 0,
            delta_retrans: 0,
            avg_rtt: 1.5,
            avg_exec: 2.0,
            avg_queue: 0.0,
            kb_per_op: 10.24,
            kb_per_sec: 1024.0,
            iops: 100.0,
        },
        DeltaStats {
            operation: "WRITE".to_string(),
            delta_ops: 50,
            delta_bytes: 0,
            delta_sent: 0,
            delta_recv: 0,
            delta_rtt: 0,
            delta_exec: 0,
            delta_queue: 0,
            delta_errors: 0,
            delta_retrans: 0,
            avg_rtt: 2.5,
            avg_exec: 3.0,
            avg_queue: 0.0,
            kb_per_op: 10.24,
            kb_per_sec: 512.0,
            iops: 50.0,
        },
    ];

    let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let mut writer = MockWriter::new();

    display_stats_simple(&mut writer, &mount, &stats, false, &timestamp).unwrap();
    let output = writer.to_string();

    assert!(output.contains("READ"), "Output should contain READ operation");
    assert!(output.contains("WRITE"), "Output should contain WRITE operation");
    assert!(output.contains("100.0"), "Output should contain IOPS for READ");
    assert!(!output.contains("MB/s"), "Output should not contain bandwidth columns");
}

#[test]
fn test_display_stats_simple_with_bandwidth() {
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

    let stats = vec![
        DeltaStats {
            operation: "READ".to_string(),
            delta_ops: 100,
            delta_bytes: 0,
            delta_sent: 0,
            delta_recv: 0,
            delta_rtt: 0,
            delta_exec: 0,
            delta_queue: 0,
            delta_errors: 0,
            delta_retrans: 0,
            avg_rtt: 1.5,
            avg_exec: 2.0,
            avg_queue: 0.0,
            kb_per_op: 10.24,
            kb_per_sec: 1024.0,
            iops: 100.0,
        },
        DeltaStats {
            operation: "WRITE".to_string(),
            delta_ops: 50,
            delta_bytes: 0,
            delta_sent: 0,
            delta_recv: 0,
            delta_rtt: 0,
            delta_exec: 0,
            delta_queue: 0,
            delta_errors: 0,
            delta_retrans: 0,
            avg_rtt: 2.5,
            avg_exec: 3.0,
            avg_queue: 0.0,
            kb_per_op: 10.24,
            kb_per_sec: 512.0,
            iops: 50.0,
        },
    ];

    let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let mut writer = MockWriter::new();

    display_stats_simple(&mut writer, &mount, &stats, true, &timestamp).unwrap();
    let output = writer.to_string();

    assert!(output.contains("READ"), "Output should contain READ operation");
    assert!(output.contains("WRITE"), "Output should contain WRITE operation");
    assert!(output.contains("MB/s"), "Output should contain bandwidth column");
    assert!(output.contains("KB/op"), "Output should contain KB/op column");
}

#[test]
fn test_display_stats_simple_empty_stats() {
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

    let stats: Vec<DeltaStats> = vec![];
    let timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let mut writer = MockWriter::new();

    display_stats_simple(&mut writer, &mount, &stats, false, &timestamp).unwrap();
    let output = writer.to_string();

    // Should have minimal output for empty stats
    assert!(output.is_empty() || output.trim().is_empty(), "Output should be empty for empty stats");
}

#[test]
fn test_format_duration() {
    use nfs_gaze::display::format_duration;

    assert_eq!(format_duration(500), "0.5ms");
    assert_eq!(format_duration(1500), "1.5ms");
    assert_eq!(format_duration(10000), "10.0ms");
    assert_eq!(format_duration(0), "0.0ms");
}

#[test]
fn test_format_rate() {
    use nfs_gaze::display::format_rate;

    assert_eq!(format_rate(0.0), "0.0");
    assert_eq!(format_rate(1.5), "1.5");
    assert_eq!(format_rate(100.0), "100.0");
    assert_eq!(format_rate(1000.5), "1000.5");
}

#[test]
fn test_format_bandwidth() {
    use nfs_gaze::display::format_bandwidth;

    // Test KB/s
    assert_eq!(format_bandwidth(512.0), "0.5");
    assert_eq!(format_bandwidth(1024.0), "1.0");
    assert_eq!(format_bandwidth(1536.0), "1.5");

    // Test zero
    assert_eq!(format_bandwidth(0.0), "0.0");

    // Test large values
    assert_eq!(format_bandwidth(10240.0), "10.0");
}