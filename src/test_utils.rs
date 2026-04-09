//! Shared test utilities for creating test data
//!
//! This module provides common test helper functions to avoid duplication
//! across different test modules.

#![cfg(test)]

use crate::types::{DeltaStats, NFSEvents, NFSMount, NFSOperation};
use std::collections::HashMap;

/// Creates a basic test mount with default values
pub fn create_test_mount() -> NFSMount {
    create_test_mount_with_params("server:/export", "/mnt/nfs")
}

/// Creates a test mount with specified device and mount point
pub fn create_test_mount_with_params(device: &str, mount_point: &str) -> NFSMount {
    let (server, export) = parse_device(device);

    NFSMount {
        device: device.to_string(),
        mount_point: mount_point.to_string(),
        server,
        export,
        age: 3600,
        operations: create_default_operations(),
        events: Some(create_default_events()),
        bytes_read: 1048576,
        bytes_write: 2097152,
        xprt: None,
    }
}

/// Creates a test mount with custom operations
pub fn create_test_mount_with_operations(operations: HashMap<String, NFSOperation>) -> NFSMount {
    NFSMount {
        device: "server:/export".to_string(),
        mount_point: "/mnt/nfs".to_string(),
        server: "server".to_string(),
        export: "/export".to_string(),
        age: 3600,
        operations,
        events: None,
        bytes_read: 0,
        bytes_write: 0,
        xprt: None,
    }
}

/// Creates a basic NFS operation for testing
pub fn create_test_operation(name: &str) -> NFSOperation {
    create_test_operation_with_stats(name, 100, 1024, 2048, 20, 30)
}

/// Creates an NFS operation with specific stats
pub fn create_test_operation_with_stats(
    name: &str,
    ops: i64,
    bytes_sent: i64,
    bytes_recv: i64,
    rtt: i64,
    execute_time: i64,
) -> NFSOperation {
    NFSOperation {
        name: name.to_string(),
        ops,
        ntrans: ops - 5, // Simulate some retransmissions
        timeouts: 0,
        bytes_sent,
        bytes_recv,
        queue_time: 10,
        rtt,
        execute_time,
        errors: 0,
    }
}

/// Creates default NFS events for testing
pub fn create_default_events() -> NFSEvents {
    NFSEvents {
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
    }
}

/// Creates a default set of operations (READ, WRITE, GETATTR)
pub fn create_default_operations() -> HashMap<String, NFSOperation> {
    let mut operations = HashMap::new();

    operations.insert(
        "READ".to_string(),
        create_test_operation_with_stats("READ", 1000, 10240, 20480, 200, 300),
    );

    operations.insert(
        "WRITE".to_string(),
        create_test_operation_with_stats("WRITE", 500, 51200, 10240, 250, 400),
    );

    operations.insert(
        "GETATTR".to_string(),
        create_test_operation_with_stats("GETATTR", 2000, 4096, 8192, 100, 150),
    );

    operations
}

/// Creates test delta stats
pub fn create_test_delta_stats() -> Vec<DeltaStats> {
    vec![
        create_test_delta_stat("READ", 100, 10240),
        create_test_delta_stat("WRITE", 50, 51200),
    ]
}

/// Creates a single delta stat for testing
pub fn create_test_delta_stat(operation: &str, delta_ops: i64, delta_bytes: i64) -> DeltaStats {
    DeltaStats {
        operation: operation.to_string(),
        delta_ops,
        delta_bytes,
        delta_sent: delta_bytes / 2,
        delta_recv: delta_bytes / 2,
        delta_rtt: delta_ops * 10,
        delta_exec: delta_ops * 20,
        delta_queue: delta_ops * 5,
        delta_errors: 0,
        delta_retrans: 0,
        delta_timeouts: 0,
        avg_rtt: 10.0,
        avg_exec: 20.0,
        avg_queue: 5.0,
        kb_per_op: (delta_bytes as f64 / delta_ops as f64) / 1024.0,
        kb_per_sec: 100.0,
        iops: delta_ops as f64,
    }
}

/// Helper function to parse device string into server and export
fn parse_device(device: &str) -> (String, String) {
    let parts: Vec<&str> = device.splitn(2, ':').collect();
    let server = parts.first().unwrap_or(&"").to_string();
    let export = parts
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "/".to_string());
    (server, export)
}

/// Creates sample mountstats data for testing parser
pub fn create_test_mountstats_data() -> String {
    r#"device server:/export mounted on /mnt/nfs with fstype nfs4 statvers=1.1
        opts:   rw,vers=4.2,rsize=1048576,wsize=1048576,namlen=255,hard,proto=tcp
        age:    3600
        bytes:  1048576 0 0 0 0 2097152 0 0
        events: 100 200 50 75 150 300 400 25 100 50 75 25 10 5 20 15 8 3 1 4 12 6 7 3 2 0 0
        READ: 1000 995 5 10240 20480 100 200 300 10
        WRITE: 500 495 5 51200 10240 150 250 400 5
        GETATTR: 2000 2000 0 4096 8192 50 100 150 0
"#
    .to_string()
}
