use clap::Parser;
use nfs_gaze::cli::Args;

#[test]
fn test_cli_default_flags() {
    let args = Args::try_parse_from(&["nfs-gaze"]).expect("Should parse default args");

    assert_eq!(args.mount_point, None);
    assert_eq!(args.operations, None);
    assert_eq!(args.interval, 1);
    assert_eq!(args.count, 0);
    assert_eq!(args.show_attr, false);
    assert_eq!(args.show_bandwidth, false);
    assert_eq!(args.clear_screen, false);
    assert_eq!(args.mountstats_path, "/proc/self/mountstats");
}

#[test]
fn test_cli_with_mount_point() {
    let args = Args::try_parse_from(&["nfs-gaze", "-m", "/mnt/nfs"])
        .expect("Should parse with mount point");

    assert_eq!(args.mount_point, Some("/mnt/nfs".to_string()));
    assert_eq!(args.operations, None);
    assert_eq!(args.interval, 1);
    assert_eq!(args.count, 0);
    assert_eq!(args.show_attr, false);
    assert_eq!(args.show_bandwidth, false);
    assert_eq!(args.clear_screen, false);
    assert_eq!(args.mountstats_path, "/proc/self/mountstats");
}

#[test]
fn test_cli_with_operations_filter() {
    let args = Args::try_parse_from(&["nfs-gaze", "--ops", "READ,WRITE"])
        .expect("Should parse with operations filter");

    assert_eq!(args.mount_point, None);
    assert_eq!(args.operations, Some("READ,WRITE".to_string()));
    assert_eq!(args.interval, 1);
    assert_eq!(args.count, 0);
    assert_eq!(args.show_attr, false);
    assert_eq!(args.show_bandwidth, false);
    assert_eq!(args.clear_screen, false);
    assert_eq!(args.mountstats_path, "/proc/self/mountstats");
}

#[test]
fn test_cli_with_custom_interval() {
    let args = Args::try_parse_from(&["nfs-gaze", "-i", "5"])
        .expect("Should parse with custom interval");

    assert_eq!(args.mount_point, None);
    assert_eq!(args.operations, None);
    assert_eq!(args.interval, 5);
    assert_eq!(args.count, 0);
    assert_eq!(args.show_attr, false);
    assert_eq!(args.show_bandwidth, false);
    assert_eq!(args.clear_screen, false);
    assert_eq!(args.mountstats_path, "/proc/self/mountstats");
}

#[test]
fn test_cli_with_all_flags() {
    let args = Args::try_parse_from(&[
        "nfs-gaze",
        "-m", "/mnt/nfs",
        "--ops", "READ",
        "-i", "2",
        "-c", "10",
        "--attr",
        "--bw",
        "--clear"
    ]).expect("Should parse with all flags");

    assert_eq!(args.mount_point, Some("/mnt/nfs".to_string()));
    assert_eq!(args.operations, Some("READ".to_string()));
    assert_eq!(args.interval, 2);
    assert_eq!(args.count, 10);
    assert_eq!(args.show_attr, true);
    assert_eq!(args.show_bandwidth, true);
    assert_eq!(args.clear_screen, true);
    assert_eq!(args.mountstats_path, "/proc/self/mountstats");
}

#[test]
fn test_parse_operations_filter() {
    use nfs_gaze::cli::parse_operations_filter;

    // Test empty filter
    let filter = parse_operations_filter(None);
    assert!(filter.is_empty());

    // Test single operation
    let filter = parse_operations_filter(Some("READ".to_string()));
    assert_eq!(filter.len(), 1);
    assert!(filter.contains("READ"));

    // Test multiple operations
    let filter = parse_operations_filter(Some("READ,WRITE,GETATTR".to_string()));
    assert_eq!(filter.len(), 3);
    assert!(filter.contains("READ"));
    assert!(filter.contains("WRITE"));
    assert!(filter.contains("GETATTR"));

    // Test operations with whitespace
    let filter = parse_operations_filter(Some(" READ , WRITE , GETATTR ".to_string()));
    assert_eq!(filter.len(), 3);
    assert!(filter.contains("READ"));
    assert!(filter.contains("WRITE"));
    assert!(filter.contains("GETATTR"));

    // Test empty string
    let filter = parse_operations_filter(Some("".to_string()));
    assert!(filter.is_empty());
}

#[test]
fn test_operations_filter_case_sensitivity() {
    use nfs_gaze::cli::parse_operations_filter;

    // Test case sensitivity
    let filter = parse_operations_filter(Some("read,Write,GETATTR".to_string()));
    assert_eq!(filter.len(), 3);
    assert!(filter.contains("read"));
    assert!(filter.contains("Write"));
    assert!(filter.contains("GETATTR"));

    // Should not match different cases
    assert!(!filter.contains("READ"));
    assert!(!filter.contains("WRITE"));
    assert!(!filter.contains("getattr"));
}