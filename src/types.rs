use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NfsGazeError {
    #[error("Failed to read mountstats: {0}")]
    MountstatsRead(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Mount point not found: {0}")]
    MountNotFound(String),
    #[error("Invalid number of parts for events: {0}")]
    InvalidEventsParts(usize),
    #[error("Error parsing {field}: {source}")]
    FieldParseError {
        field: String,
        #[source]
        source: std::num::ParseIntError,
    },
}

pub type Result<T> = std::result::Result<T, NfsGazeError>;

#[derive(Debug, Clone, PartialEq)]
pub struct NFSOperation {
    pub name: String,
    pub ops: i64,
    pub ntrans: i64,
    pub timeouts: i64,
    pub bytes_sent: i64,
    pub bytes_recv: i64,
    pub queue_time: i64,  // milliseconds
    pub rtt: i64,         // milliseconds
    pub execute_time: i64, // milliseconds
    pub errors: i64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NFSEvents {
    pub inode_revalidate: i64,    // index 0
    pub dentry_revalidate: i64,   // index 1
    pub data_invalidate: i64,     // index 2
    pub attr_invalidate: i64,     // index 3
    pub vfs_open: i64,            // index 4
    pub vfs_lookup: i64,          // index 5
    pub vfs_access: i64,          // index 6
    pub vfs_update_page: i64,     // index 7
    pub vfs_read_page: i64,       // index 8
    pub vfs_read_pages: i64,      // index 9
    pub vfs_write_page: i64,      // index 10
    pub vfs_write_pages: i64,     // index 11
    pub vfs_getdents: i64,        // index 12
    pub vfs_setattr: i64,         // index 13
    pub vfs_flush: i64,           // index 14
    pub vfs_fsync: i64,           // index 15
    pub vfs_lock: i64,            // index 16
    pub vfs_release: i64,         // index 17
    pub congestion_wait: i64,     // index 18
    pub setattr_trunc: i64,       // index 19
    pub extend_write: i64,        // index 20
    pub silly_rename: i64,        // index 21
    pub short_read: i64,          // index 22
    pub short_write: i64,         // index 23
    pub delay: i64,               // index 24
    pub pnfs_read: i64,           // index 25
    pub pnfs_write: i64,          // index 26
}

#[derive(Debug, Clone, PartialEq)]
pub struct NFSMount {
    pub device: String,
    pub mount_point: String,
    pub server: String,
    pub export: String,
    pub age: i64,
    pub operations: HashMap<String, NFSOperation>,
    pub events: Option<NFSEvents>,
    pub bytes_read: i64,
    pub bytes_write: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeltaStats {
    pub operation: String,
    pub delta_ops: i64,
    pub delta_bytes: i64,
    pub delta_sent: i64,
    pub delta_recv: i64,
    pub delta_rtt: i64,
    pub delta_exec: i64,
    pub delta_queue: i64,
    pub delta_errors: i64,
    pub delta_retrans: i64,
    pub avg_rtt: f64,
    pub avg_exec: f64,
    pub avg_queue: f64,
    pub kb_per_op: f64,
    pub kb_per_sec: f64,
    pub iops: f64,
}