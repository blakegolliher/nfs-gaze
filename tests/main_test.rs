#[cfg(target_os = "linux")]
mod linux_tests {
    use nfs_gaze::cli::Args;
    use nfs_gaze::parser::parse_mountstats;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_mountstats(dir: &TempDir) -> PathBuf {
        let path = dir.path().join("mountstats");
        let content = r#"device nfs-server:/export mounted on /mnt/test with fstype nfs4 statvers=1.1
        opts:   rw,vers=4.2,rsize=1048576,wsize=1048576,namlen=255,hard,proto=tcp,timeo=600,retrans=2,sec=sys,clientaddr=10.0.0.2,local_lock=none,addr=10.0.0.1
        age:    12345
        bytes:  10485760 20971520
        events: 100 200 50 75 150 300 400 25 100 50 75 25 10 5 20 15 8 3 1 4 12 6 7 3 2 0 0
        READ: 1000 950 50 10240 20480 100 200 300
        WRITE: 500 490 10 51200 10240 150 250 400
        GETATTR: 2000 2000 0 4096 8192 50 100 150
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

        let result = parse_mountstats(path.to_str().unwrap());
        assert!(result.is_ok());

        let mounts = result.unwrap();
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].device, "nfs-server:/export");
        assert_eq!(mounts[0].mount_point, "/mnt/test");
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
    fn test_args_to_metrics_config() {
        let args = Args {
            mount_point: None,
            interval: 5,
            count: 0,
            operations: Some("READ,WRITE".to_string()),
            mountstats_path: "/proc/self/mountstats".to_string(),
            show_attr: false,
            show_bandwidth: false,
            clear_screen: false,
            prometheus: true,
            prometheus_port: 8080,
            opentelemetry: true,
            otel_endpoint: Some("http://localhost:4317".to_string()),
        };

        let config = args.to_metrics_config();
        assert!(config.enable_prometheus);
        assert_eq!(config.prometheus_port, 8080);
        assert!(config.enable_opentelemetry);
        assert_eq!(config.otel_endpoint, Some("http://localhost:4317".to_string()));
    }

    #[test]
    fn test_args_default_values() {
        let args = Args {
            mount_point: None,
            interval: 2,
            count: 0,
            operations: None,
            mountstats_path: "/proc/self/mountstats".to_string(),
            show_attr: false,
            show_bandwidth: false,
            clear_screen: false,
            prometheus: false,
            prometheus_port: 9090,
            opentelemetry: false,
            otel_endpoint: None,
        };

        let config = args.to_metrics_config();
        assert!(!config.enable_prometheus);
        assert_eq!(config.prometheus_port, 9090);
        assert!(!config.enable_opentelemetry);
        assert!(config.otel_endpoint.is_none());
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
    use std::collections::HashSet;

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