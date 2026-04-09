use nfs_gaze::{
    metrics::{MetricsConfig, MetricsManager},
    types::{DeltaStats, NFSEvents, NFSMount, NFSOperation},
};
use std::collections::HashMap;
use std::time::Duration;

fn create_test_mount(device: &str, mount_point: &str) -> NFSMount {
    let mut operations = HashMap::new();

    operations.insert(
        "READ".to_string(),
        NFSOperation {
            name: "READ".to_string(),
            ops: 1000,
            ntrans: 950,
            timeouts: 50,
            bytes_sent: 10240,
            bytes_recv: 20480,
            queue_time: 100,
            rtt: 200,
            execute_time: 300,
            errors: 10,
        },
    );

    operations.insert(
        "WRITE".to_string(),
        NFSOperation {
            name: "WRITE".to_string(),
            ops: 500,
            ntrans: 490,
            timeouts: 10,
            bytes_sent: 51200,
            bytes_recv: 10240,
            queue_time: 150,
            rtt: 250,
            execute_time: 400,
            errors: 5,
        },
    );

    operations.insert(
        "GETATTR".to_string(),
        NFSOperation {
            name: "GETATTR".to_string(),
            ops: 2000,
            ntrans: 2000,
            timeouts: 0,
            bytes_sent: 4096,
            bytes_recv: 8192,
            queue_time: 50,
            rtt: 100,
            execute_time: 150,
            errors: 0,
        },
    );

    NFSMount {
        device: device.to_string(),
        mount_point: mount_point.to_string(),
        server: "test-server".to_string(),
        export: "/test/export".to_string(),
        age: 3600,
        operations,
        events: Some(NFSEvents {
            inode_revalidate: 100,
            dentry_revalidate: 200,
            data_invalidate: 50,
            attr_invalidate: 75,
            vfs_open: 150,
            vfs_lookup: 300,
            vfs_access: 400,
            vfs_update_page: 25,
            vfs_read_page: 100,
            vfs_read_pages: 50,
            vfs_write_page: 75,
            vfs_write_pages: 25,
            vfs_getdents: 10,
            vfs_setattr: 5,
            vfs_flush: 20,
            vfs_fsync: 15,
            vfs_lock: 8,
            vfs_release: 3,
            congestion_wait: 1,
            setattr_trunc: 4,
            extend_write: 12,
            silly_rename: 6,
            short_read: 7,
            short_write: 3,
            delay: 2,
            pnfs_read: 0,
            pnfs_write: 0,
        }),
        bytes_read: 10485760,
        bytes_write: 20971520,
        xprt: None,
    }
}

fn create_test_delta_stats() -> Vec<DeltaStats> {
    vec![
        DeltaStats {
            operation: "READ".to_string(),
            delta_ops: 100,
            delta_bytes: 10240,
            delta_sent: 5120,
            delta_recv: 5120,
            delta_rtt: 1000,
            delta_exec: 2000,
            delta_queue: 500,
            delta_errors: 2,
            delta_retrans: 5,
            delta_timeouts: 3,
            avg_rtt: 10.0,
            avg_exec: 20.0,
            avg_queue: 5.0,
            kb_per_op: 10.0,
            kb_per_sec: 100.0,
            iops: 100.0,
        },
        DeltaStats {
            operation: "WRITE".to_string(),
            delta_ops: 50,
            delta_bytes: 51200,
            delta_sent: 25600,
            delta_recv: 25600,
            delta_rtt: 2000,
            delta_exec: 3000,
            delta_queue: 1000,
            delta_errors: 1,
            delta_retrans: 2,
            delta_timeouts: 1,
            avg_rtt: 40.0,
            avg_exec: 60.0,
            avg_queue: 20.0,
            kb_per_op: 100.0,
            kb_per_sec: 500.0,
            iops: 50.0,
        },
    ]
}

#[test]
fn test_metrics_manager_disabled_by_default() {
    let config = MetricsConfig::default();
    let manager = MetricsManager::new(config).expect("Failed to create metrics manager");

    assert!(!manager.is_enabled());
    assert!(manager.get_prometheus_metrics().is_none());
}

#[test]
fn test_metrics_manager_export_with_empty_stats() {
    let config = MetricsConfig::default();
    let manager = MetricsManager::new(config).expect("Failed to create metrics manager");
    let mount = create_test_mount("server:/export", "/mnt/test");
    let empty_stats: Vec<DeltaStats> = vec![];

    // Should handle empty stats gracefully
    manager.export_metrics(&mount, &empty_stats);
}

#[test]
fn test_metrics_manager_export_with_multiple_mounts() {
    let config = MetricsConfig::default();
    let manager = MetricsManager::new(config).expect("Failed to create metrics manager");

    let mount1 = create_test_mount("server1:/export1", "/mnt/test1");
    let mount2 = create_test_mount("server2:/export2", "/mnt/test2");
    let stats = create_test_delta_stats();

    // Should handle multiple mounts without issues
    manager.export_metrics(&mount1, &stats);
    manager.export_metrics(&mount2, &stats);
}

#[test]
fn test_metrics_config_custom_values() {
    let config = MetricsConfig {
        enable_prometheus: true,
        prometheus_port: 8080,
        prometheus_bind: "0.0.0.0".to_string(),
        export_interval: Duration::from_secs(30),
        include_labels: false,
    };

    assert!(config.enable_prometheus);
    assert_eq!(config.prometheus_port, 8080);
    assert_eq!(config.prometheus_bind, "0.0.0.0");
    assert_eq!(config.export_interval, Duration::from_secs(30));
    assert!(!config.include_labels);
}

#[test]
fn test_metrics_manager_handles_missing_events() {
    let config = MetricsConfig::default();
    let manager = MetricsManager::new(config).expect("Failed to create metrics manager");

    let mut mount = create_test_mount("server:/export", "/mnt/test");
    mount.events = None;
    let stats = create_test_delta_stats();

    // Should handle missing events gracefully
    manager.export_metrics(&mount, &stats);
}

#[test]
fn test_metrics_manager_handles_high_volume_stats() {
    let config = MetricsConfig::default();
    let manager = MetricsManager::new(config).expect("Failed to create metrics manager");
    let mount = create_test_mount("server:/export", "/mnt/test");

    // Create a large number of delta stats
    let mut large_stats = Vec::new();
    for i in 0..100 {
        large_stats.push(DeltaStats {
            operation: format!("OP_{}", i),
            delta_ops: i as i64 * 10,
            delta_bytes: i as i64 * 1024,
            delta_sent: i as i64 * 512,
            delta_recv: i as i64 * 512,
            delta_rtt: i as i64 * 100,
            delta_exec: i as i64 * 200,
            delta_queue: i as i64 * 50,
            delta_errors: i as i64,
            delta_retrans: i as i64 * 2,
            delta_timeouts: i as i64,
            avg_rtt: i as f64 * 10.0,
            avg_exec: i as f64 * 20.0,
            avg_queue: i as f64 * 5.0,
            kb_per_op: i as f64,
            kb_per_sec: i as f64 * 10.0,
            iops: i as f64 * 100.0,
        });
    }

    // Should handle large numbers of stats without issues
    manager.export_metrics(&mount, &large_stats);
}

#[cfg(feature = "prometheus")]
mod prometheus_tests {
    use super::*;

    #[test]
    fn test_prometheus_enabled_manager() {
        let config = MetricsConfig {
            enable_prometheus: true,
            prometheus_port: 9091,
            ..Default::default()
        };

        let manager = MetricsManager::new(config).expect("Failed to create metrics manager");
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_prometheus_metrics_output() {
        let config = MetricsConfig {
            enable_prometheus: true,
            prometheus_port: 9092,
            ..Default::default()
        };

        let manager = MetricsManager::new(config).expect("Failed to create metrics manager");
        let mount = create_test_mount("server:/export", "/mnt/test");
        let stats = create_test_delta_stats();

        manager.export_metrics(&mount, &stats);

        // Get metrics output
        let metrics = manager.get_prometheus_metrics();
        assert!(metrics.is_some());

        let metrics_text = metrics.unwrap();
        assert!(metrics_text.contains("nfs_operations_total"));
        assert!(metrics_text.contains("nfs_operation_duration_seconds"));
    }

    #[test]
    fn test_prometheus_xprt_metrics_exported() {
        use nfs_gaze::DeltaXprtStats;

        let config = MetricsConfig {
            enable_prometheus: true,
            prometheus_port: 9094,
            ..Default::default()
        };

        let manager = MetricsManager::new(config).expect("Failed to create metrics manager");
        let mount = create_test_mount("server:/export", "/mnt/test");

        // Craft a delta with non-zero counters and a non-zero HWM so
        // the gauges get populated and the counters all fire their
        // inc_by guards.
        let delta = DeltaXprtStats {
            protocol: "tcp".to_string(),
            delta_sends: 1000,
            delta_recvs: 1000,
            delta_bad_xids: 2,
            delta_req: 1000,
            delta_bklog: 50,
            delta_sending: 820,
            delta_pending: 8398,
            max_slots: 7091,
            bklog_per_req: 0.05,
            sending_per_req: 0.82,
            pending_per_req: 8.398,
        };

        manager.export_xprt(&mount, &delta, Some(65536));

        let text = manager
            .get_prometheus_metrics()
            .expect("metrics output should be available");

        // Spot-check one counter and one gauge. Both carry the
        // protocol label so we also confirm the label plumbing.
        assert!(
            text.contains("nfs_xprt_sends_total"),
            "missing nfs_xprt_sends_total in metrics output"
        );
        assert!(
            text.contains("nfs_xprt_max_slots"),
            "missing nfs_xprt_max_slots gauge"
        );
        assert!(
            text.contains("nfs_xprt_slot_cap"),
            "missing nfs_xprt_slot_cap gauge"
        );
        assert!(
            text.contains("nfs_xprt_backlog_total"),
            "missing nfs_xprt_backlog_total counter"
        );
        assert!(
            text.contains("protocol=\"tcp\""),
            "missing protocol label on xprt metrics"
        );
    }

    #[tokio::test]
    async fn test_prometheus_server_start() {
        let config = MetricsConfig {
            enable_prometheus: true,
            prometheus_port: 9093,
            ..Default::default()
        };

        let manager = MetricsManager::new(config).expect("Failed to create metrics manager");
        let server_handle = manager.start_prometheus_server();

        assert!(server_handle.is_some());

        // Clean up
        if let Some(handle) = server_handle {
            handle.abort();
        }
    }
}

#[test]
fn test_concurrent_metrics_export() {
    use std::sync::Arc;
    use std::thread;

    let config = MetricsConfig::default();
    let manager = Arc::new(MetricsManager::new(config).expect("Failed to create metrics manager"));

    let mut handles = vec![];

    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            let mount =
                create_test_mount(&format!("server{}:/export", i), &format!("/mnt/test{}", i));
            let stats = create_test_delta_stats();
            manager_clone.export_metrics(&mount, &stats);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_metrics_manager_with_special_characters() {
    let config = MetricsConfig::default();
    let manager = MetricsManager::new(config).expect("Failed to create metrics manager");

    let mut mount = create_test_mount("server:/export/with spaces", "/mnt/test-with-dashes");
    mount.device = "server:/export/with\"quotes\"and'apostrophes'".to_string();
    mount.mount_point = "/mnt/test/with/many/slashes".to_string();

    let stats = create_test_delta_stats();

    // Should handle special characters gracefully
    manager.export_metrics(&mount, &stats);
}
