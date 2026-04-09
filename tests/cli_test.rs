use clap::Parser;
use nfs_gaze::cli::Args;

#[test]
fn test_cli_default_flags() {
    let args = Args::try_parse_from(["nfs-gaze"]).expect("Should parse default args");

    assert_eq!(args.mount_point, None);
    assert_eq!(args.operations, None);
    assert_eq!(args.interval, 1);
    assert_eq!(args.count, 0);
    assert_eq!(args.duration, None);
    assert!(!args.show_bandwidth);
    assert!(!args.clear_screen);
    assert_eq!(args.mountstats_path, "/proc/self/mountstats");
}

#[test]
fn test_cli_with_duration() {
    let args =
        Args::try_parse_from(["nfs-gaze", "-d", "30"]).expect("should accept duration in seconds");
    assert_eq!(args.duration, Some(30));
    assert_eq!(args.count, 0);
}

#[test]
fn test_cli_duration_and_count_are_mutually_exclusive() {
    // Passing both --count and --duration must be a clap error. The
    // two limits would race otherwise, and we decided against
    // "first-wins" ambiguity.
    let err = Args::try_parse_from(["nfs-gaze", "-c", "5", "-d", "30"])
        .expect_err("clap should reject -c with -d");
    let rendered = err.to_string();
    assert!(
        rendered.contains("cannot be used with") || rendered.contains("conflict"),
        "unexpected clap error: {rendered}"
    );
}

#[test]
fn test_cli_with_output() {
    use std::path::PathBuf;
    let args = Args::try_parse_from(["nfs-gaze", "-d", "5", "-o", "/tmp/report.json"])
        .expect("should accept -o with a path");
    assert_eq!(args.output, Some(PathBuf::from("/tmp/report.json")));
    assert_eq!(args.duration, Some(5));
}

#[test]
fn test_cli_compare_subcommand_parses_positional_args() {
    use nfs_gaze::cli::Command;
    use std::path::PathBuf;

    let args = Args::try_parse_from([
        "nfs-gaze",
        "compare",
        "baseline.json",
        "new.json",
        "BASELINE",
        "NEW",
    ])
    .expect("compare subcommand with four positional args should parse");

    match args.command {
        Some(Command::Compare(c)) => {
            assert_eq!(c.file1, PathBuf::from("baseline.json"));
            assert_eq!(c.file2, PathBuf::from("new.json"));
            assert_eq!(c.label1.as_deref(), Some("BASELINE"));
            assert_eq!(c.label2.as_deref(), Some("NEW"));
        }
        other => panic!("expected Command::Compare, got {other:?}"),
    }
}

#[test]
fn test_cli_compare_subcommand_labels_are_optional() {
    use nfs_gaze::cli::Command;

    let args = Args::try_parse_from(["nfs-gaze", "compare", "a.json", "b.json"])
        .expect("compare with only the two file arguments should parse");

    match args.command {
        Some(Command::Compare(c)) => {
            assert!(c.label1.is_none());
            assert!(c.label2.is_none());
        }
        other => panic!("expected Command::Compare, got {other:?}"),
    }
}

#[test]
fn test_cli_with_mount_point() {
    let args = Args::try_parse_from(["nfs-gaze", "-m", "/mnt/nfs"])
        .expect("Should parse with mount point");

    assert_eq!(args.mount_point, Some("/mnt/nfs".to_string()));
    assert_eq!(args.operations, None);
    assert_eq!(args.interval, 1);
    assert_eq!(args.count, 0);
    assert!(!args.show_bandwidth);
    assert!(!args.clear_screen);
    assert_eq!(args.mountstats_path, "/proc/self/mountstats");
}

#[test]
fn test_cli_with_operations_filter() {
    let args = Args::try_parse_from(["nfs-gaze", "--ops", "READ,WRITE"])
        .expect("Should parse with operations filter");

    assert_eq!(args.mount_point, None);
    assert_eq!(args.operations, Some("READ,WRITE".to_string()));
    assert_eq!(args.interval, 1);
    assert_eq!(args.count, 0);
    assert!(!args.show_bandwidth);
    assert!(!args.clear_screen);
    assert_eq!(args.mountstats_path, "/proc/self/mountstats");
}

#[test]
fn test_cli_with_custom_interval() {
    let args =
        Args::try_parse_from(["nfs-gaze", "-i", "5"]).expect("Should parse with custom interval");

    assert_eq!(args.mount_point, None);
    assert_eq!(args.operations, None);
    assert_eq!(args.interval, 5);
    assert_eq!(args.count, 0);
    assert!(!args.show_bandwidth);
    assert!(!args.clear_screen);
    assert_eq!(args.mountstats_path, "/proc/self/mountstats");
}

#[test]
fn test_cli_with_all_flags() {
    let args = Args::try_parse_from([
        "nfs-gaze", "-m", "/mnt/nfs", "--ops", "READ", "-i", "2", "-c", "10", "--bw", "--clear",
    ])
    .expect("Should parse with all flags");

    assert_eq!(args.mount_point, Some("/mnt/nfs".to_string()));
    assert_eq!(args.operations, Some("READ".to_string()));
    assert_eq!(args.interval, 2);
    assert_eq!(args.count, 10);
    assert!(args.show_bandwidth);
    assert!(args.clear_screen);
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
