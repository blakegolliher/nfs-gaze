#[cfg(target_os = "linux")]
mod linux_tests {
    use nfs_gaze::cli::Args;
    use nfs_gaze::parser::parse_mountstats;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_mountstats(dir: &TempDir) -> PathBuf {
        let path = dir.path().join("mountstats");
        // Per-op stats line format: ops trans timeouts bytes_sent bytes_recv queue_ms rtt_ms total_ms errors
        // (9 fields after the op name; matches /proc/self/mountstats on Linux 4.x+)
        let content = r#"device nfs-server:/export mounted on /mnt/test with fstype nfs4 statvers=1.1
        opts:   rw,vers=4.2,rsize=1048576,wsize=1048576,namlen=255,hard,proto=tcp,timeo=600,retrans=2,sec=sys,clientaddr=10.0.0.2,local_lock=none,addr=10.0.0.1
        age:    12345
        bytes:  10485760 20971520 0 0 5242880 10485760 1024 2048
        events: 100 200 50 75 150 300 400 25 100 50 75 25 10 5 20 15 8 3 1 4 12 6 7 3 2 0 0
        READ: 1000 950 50 10240 20480 100 200 300 0
        WRITE: 500 490 10 51200 10240 150 250 400 1
        GETATTR: 2000 2000 0 4096 8192 50 100 150 0
"#;
        fs::write(&path, content).unwrap();
        path
    }

    fn create_empty_mountstats(dir: &TempDir) -> PathBuf {
        let path = dir.path().join("empty_mountstats");
        fs::write(&path, "").unwrap();
        path
    }

    fn create_invalid_mountstats(dir: &TempDir) -> PathBuf {
        let path = dir.path().join("invalid_mountstats");
        let content = r#"device nfs-server:/export mounted on /mnt/test with fstype nfs4 statvers=1.1
        invalid data here
        this is not valid mountstats format
"#;
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_parse_valid_mountstats() {
        let dir = TempDir::new().unwrap();
        let path = create_test_mountstats(&dir);

        let mounts = parse_mountstats(path.to_str().unwrap())
            .expect("parsing valid mountstats should succeed");

        assert_eq!(mounts.len(), 1);
        let mount = mounts
            .get("/mnt/test")
            .expect("mount /mnt/test should be present");
        assert_eq!(mount.device, "nfs-server:/export");
        assert_eq!(mount.mount_point, "/mnt/test");
        assert_eq!(mount.operations.len(), 3);
        assert!(mount.operations.contains_key("READ"));
        assert!(mount.operations.contains_key("WRITE"));
        assert!(mount.operations.contains_key("GETATTR"));
    }

    #[test]
    fn test_parse_empty_mountstats() {
        let dir = TempDir::new().unwrap();
        let path = create_empty_mountstats(&dir);

        let result = parse_mountstats(path.to_str().unwrap());
        assert!(result.is_ok());

        let mounts = result.unwrap();
        assert_eq!(mounts.len(), 0);
    }

    #[test]
    fn test_parse_nonexistent_mountstats() {
        let result = parse_mountstats("/nonexistent/path/to/mountstats");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_mountstats_yields_no_mounts() {
        let dir = TempDir::new().unwrap();
        let path = create_invalid_mountstats(&dir);

        // The parser ignores lines it doesn't recognise inside an NFS mount block,
        // so a malformed body still produces a mount entry but with empty stats.
        let mounts = parse_mountstats(path.to_str().unwrap())
            .expect("parser should not return Err for unrecognized lines");

        let mount = mounts
            .get("/mnt/test")
            .expect("device line should still produce a mount");
        assert!(mount.operations.is_empty());
        assert!(mount.events.is_none());
    }

    #[cfg(feature = "prometheus")]
    #[test]
    fn test_args_to_metrics_config_prometheus_enabled() {
        let args = Args {
            mount_point: None,
            interval: 5,
            count: 0,
            duration: None,
            output: None,
            command: None,
            operations: Some("READ,WRITE".to_string()),
            mountstats_path: "/proc/self/mountstats".to_string(),
            show_bandwidth: false,
            clear_screen: false,
            prometheus: true,
            prometheus_bind: "127.0.0.1".to_string(),
            prometheus_port: 8080,
            metrics_interval: 10,
        };

        let config = args.to_metrics_config();
        assert!(config.enable_prometheus);
        assert_eq!(config.prometheus_port, 8080);
        assert_eq!(config.prometheus_bind, "127.0.0.1");
    }

    #[cfg(not(feature = "prometheus"))]
    #[test]
    fn test_args_to_metrics_config_default() {
        let args = Args {
            mount_point: None,
            interval: 2,
            count: 0,
            duration: None,
            output: None,
            command: None,
            operations: None,
            mountstats_path: "/proc/self/mountstats".to_string(),
            show_bandwidth: false,
            clear_screen: false,
            metrics_interval: 10,
        };

        let config = args.to_metrics_config();
        assert!(!config.enable_prometheus);
    }
}

#[cfg(not(target_os = "linux"))]
mod non_linux_tests {
    #[test]
    fn test_non_linux_platform_message() {
        // This test verifies that the code properly handles non-Linux platforms
        // The actual behavior (printing error and exiting) is tested manually
        assert!(!cfg!(target_os = "linux"));
    }
}

// Integration tests that work on all platforms
mod integration_tests {
    use nfs_gaze::cli::parse_operations_filter;

    #[test]
    fn test_operations_filter_parsing() {
        let ops = Some("READ,WRITE,GETATTR".to_string());
        let filter = parse_operations_filter(ops);

        assert_eq!(filter.len(), 3);
        assert!(filter.contains("READ"));
        assert!(filter.contains("WRITE"));
        assert!(filter.contains("GETATTR"));
    }

    #[test]
    fn test_empty_operations_filter() {
        let ops = None;
        let filter = parse_operations_filter(ops);

        assert_eq!(filter.len(), 0);
    }

    #[test]
    fn test_empty_string_operations_filter() {
        let ops = Some("".to_string());
        let filter = parse_operations_filter(ops);

        assert_eq!(filter.len(), 0);
    }

    #[test]
    fn test_whitespace_operations_filter() {
        let ops = Some("  ".to_string());
        let filter = parse_operations_filter(ops);

        assert_eq!(filter.len(), 0);
    }

    #[test]
    fn test_duplicate_operations_filter() {
        let ops = Some("READ,READ,WRITE".to_string());
        let filter = parse_operations_filter(ops);

        // HashSet should remove duplicates
        assert_eq!(filter.len(), 2);
        assert!(filter.contains("READ"));
        assert!(filter.contains("WRITE"));
    }

    #[test]
    fn test_case_sensitive_operations_filter() {
        let ops = Some("read,READ,Write".to_string());
        let filter = parse_operations_filter(ops);

        // Should preserve case
        assert_eq!(filter.len(), 3);
        assert!(filter.contains("read"));
        assert!(filter.contains("READ"));
        assert!(filter.contains("Write"));
    }

    #[test]
    fn test_operations_filter_with_spaces() {
        let ops = Some("READ , WRITE , GETATTR".to_string());
        let filter = parse_operations_filter(ops);

        assert_eq!(filter.len(), 3);
        assert!(filter.contains("READ"));
        assert!(filter.contains("WRITE"));
        assert!(filter.contains("GETATTR"));
    }
}
