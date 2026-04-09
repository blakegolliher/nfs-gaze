use crate::types::{NFSEvents, NFSMount, NFSOperation, NfsGazeError, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};

// Minimum number of fields required for parsing
const MIN_EVENTS_FIELDS: usize = 25;
const MIN_OPERATION_FIELDS: usize = 8;
const MIN_BYTES_FIELDS: usize = 6;
const MIN_KEY_VALUE_FIELDS: usize = 2;

// Optional field indices
const PNFS_READ_INDEX: usize = 25;
const PNFS_WRITE_INDEX: usize = 26;
const OPERATION_ERRORS_INDEX: usize = 8;

/// Parse the events line into an NFSEvents struct
pub fn parse_events(parts: &[String]) -> Result<NFSEvents> {
    if parts.len() < MIN_EVENTS_FIELDS {
        return Err(NfsGazeError::InvalidEventsParts(parts.len()));
    }

    let parse_int = |index: usize, field: &str| -> Result<i64> {
        parts
            .get(index)
            .ok_or_else(|| {
                NfsGazeError::ParseError(format!("Missing field {} at index {}", field, index))
            })?
            .parse::<i64>()
            .map_err(|e| NfsGazeError::FieldParseError {
                field: field.to_string(),
                source: e,
            })
    };

    let mut events = NFSEvents {
        inode_revalidate: parse_int(0, "InodeRevalidate")?,
        dentry_revalidate: parse_int(1, "DentryRevalidate")?,
        data_invalidate: parse_int(2, "DataInvalidate")?,
        attr_invalidate: parse_int(3, "AttrInvalidate")?,
        vfs_open: parse_int(4, "VFSOpen")?,
        vfs_lookup: parse_int(5, "VFSLookup")?,
        vfs_access: parse_int(6, "VFSAccess")?,
        vfs_update_page: parse_int(7, "VFSUpdatePage")?,
        vfs_read_page: parse_int(8, "VFSReadPage")?,
        vfs_read_pages: parse_int(9, "VFSReadPages")?,
        vfs_write_page: parse_int(10, "VFSWritePage")?,
        vfs_write_pages: parse_int(11, "VFSWritePages")?,
        vfs_getdents: parse_int(12, "VFSGetdents")?,
        vfs_setattr: parse_int(13, "VFSSetattr")?,
        vfs_flush: parse_int(14, "VFSFlush")?,
        vfs_fsync: parse_int(15, "VFSFsync")?,
        vfs_lock: parse_int(16, "VFSLock")?,
        vfs_release: parse_int(17, "VFSRelease")?,
        congestion_wait: parse_int(18, "CongestionWait")?,
        setattr_trunc: parse_int(19, "SetattrTrunc")?,
        extend_write: parse_int(20, "ExtendWrite")?,
        silly_rename: parse_int(21, "SillyRename")?,
        short_read: parse_int(22, "ShortRead")?,
        short_write: parse_int(23, "ShortWrite")?,
        delay: parse_int(24, "Delay")?,
        pnfs_read: 0,
        pnfs_write: 0,
    };

    // Optional pNFS fields
    if parts.len() > PNFS_READ_INDEX {
        events.pnfs_read = parse_int(PNFS_READ_INDEX, "PNFSRead")?;
    }
    if parts.len() > PNFS_WRITE_INDEX {
        events.pnfs_write = parse_int(PNFS_WRITE_INDEX, "PNFSWrite")?;
    }

    Ok(events)
}

/// Parse NFS operation statistics from a stats line
pub fn parse_nfs_operation(op_name: &str, stats: &[String]) -> Result<NFSOperation> {
    if stats.len() < MIN_OPERATION_FIELDS {
        return Err(NfsGazeError::ParseError(format!(
            "insufficient stats for operation {}: got {}, need {}",
            op_name,
            stats.len(),
            MIN_OPERATION_FIELDS
        )));
    }

    let parse_int = |index: usize, field: &str| -> Result<i64> {
        stats
            .get(index)
            .ok_or_else(|| {
                NfsGazeError::ParseError(format!(
                    "Missing field {}_{} at index {}",
                    op_name, field, index
                ))
            })?
            .parse::<i64>()
            .map_err(|e| NfsGazeError::FieldParseError {
                field: format!("{}_{}", op_name, field),
                source: e,
            })
    };

    let mut operation = NFSOperation {
        name: op_name.to_string(),
        ops: parse_int(0, "ops")?,
        ntrans: parse_int(1, "ntrans")?,
        timeouts: parse_int(2, "timeouts")?,
        bytes_sent: parse_int(3, "bytes_sent")?,
        bytes_recv: parse_int(4, "bytes_recv")?,
        queue_time: parse_int(5, "queue_time")?,
        rtt: parse_int(6, "rtt")?,
        execute_time: parse_int(7, "execute_time")?,
        errors: 0,
    };

    // Optional errors field
    if stats.len() > OPERATION_ERRORS_INDEX {
        operation.errors = parse_int(OPERATION_ERRORS_INDEX, "errors")?;
    }

    Ok(operation)
}

/// Helper to parse a field value from a whitespace-split line
fn parse_field(value: &str, field: &str) -> Result<i64> {
    value.parse().map_err(|e| NfsGazeError::FieldParseError {
        field: field.to_string(),
        source: e,
    })
}

/// Lines that look like "OP: stats..." but aren't actual NFS operations
const IGNORED_PREFIXES: &[&str] = &[
    "RPC", "xprt", "per-op", "opts", "caps", "sec", "nfsv4", "nfsv3",
];

/// Main mountstats parser
struct MountstatsParser {
    mounts: HashMap<String, NFSMount>,
    current_mount: Option<NFSMount>,
}

impl MountstatsParser {
    fn new() -> Self {
        Self {
            mounts: HashMap::new(),
            current_mount: None,
        }
    }

    /// Flush current_mount into the mounts map
    fn flush_current(&mut self) {
        if let Some(mount) = self.current_mount.take() {
            self.mounts.insert(mount.mount_point.clone(), mount);
        }
    }

    fn parse<R: BufRead>(mut self, reader: R) -> Result<HashMap<String, NFSMount>> {
        for line in reader.lines() {
            let line = line?;
            self.parse_line(&line)?;
        }
        self.flush_current();
        Ok(self.mounts)
    }

    fn parse_line(&mut self, line: &str) -> Result<()> {
        let line = line.trim();
        if line.starts_with("device") && line.contains("nfs") {
            self.flush_current();
            self.parse_device_line(line)?;
        } else if self.current_mount.is_some() {
            self.parse_stats_line(line)?;
        }
        Ok(())
    }

    fn parse_device_line(&mut self, line: &str) -> Result<()> {
        // Example: "device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1"
        let parts: Vec<&str> = line.splitn(2, " on ").collect();
        if parts.len() != 2 {
            return Err(NfsGazeError::ParseError(format!(
                "Invalid device line: {}",
                line
            )));
        }

        let device_info: Vec<&str> = parts[0].split_whitespace().collect();
        let mount_info: Vec<&str> = parts[1].split_whitespace().collect();

        if device_info.len() < 2 || mount_info.is_empty() {
            return Err(NfsGazeError::ParseError(format!(
                "Invalid device info: {}",
                line
            )));
        }

        let server_export = device_info[1];
        let mount_point = mount_info[0];

        let (server, export) = match server_export.split_once(':') {
            Some((s, e)) => (s.to_string(), e.to_string()),
            None => (server_export.to_string(), "/".to_string()),
        };

        self.current_mount = Some(NFSMount {
            device: server_export.to_string(),
            mount_point: mount_point.to_string(),
            server,
            export,
            age: 0,
            operations: HashMap::new(),
            events: None,
            bytes_read: 0,
            bytes_write: 0,
        });
        Ok(())
    }

    fn parse_stats_line(&mut self, line: &str) -> Result<()> {
        if line.starts_with("age:") {
            self.parse_age(line)
        } else if line.starts_with("events:") {
            self.parse_events_line(line)
        } else if line.starts_with("bytes:") {
            self.parse_bytes(line)
        } else if line.contains(':') && !IGNORED_PREFIXES.iter().any(|p| line.starts_with(p)) {
            self.parse_operation(line)
        } else {
            Ok(())
        }
    }

    fn parse_age(&mut self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < MIN_KEY_VALUE_FIELDS {
            return Err(NfsGazeError::ParseError(format!(
                "Invalid age line: {}",
                line
            )));
        }

        if let Some(ref mut mount) = self.current_mount {
            mount.age = parse_field(parts[1], "age")?;
        }
        Ok(())
    }

    fn parse_events_line(&mut self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < MIN_KEY_VALUE_FIELDS {
            return Err(NfsGazeError::ParseError(format!(
                "Invalid events line: {}",
                line
            )));
        }

        let event_parts: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
        let events = parse_events(&event_parts)?;

        if let Some(ref mut mount) = self.current_mount {
            mount.events = Some(events);
        }
        Ok(())
    }

    fn parse_bytes(&mut self, line: &str) -> Result<()> {
        // Kernel format: "bytes: normalread normalwrite directread directwrite serverread serverwrite pagesread pageswrite"
        // Index:              1          2           3            4          5           6          7          8
        // bytes_read = index 1 (normalread), bytes_write = index 6 (serverwrite)
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < MIN_BYTES_FIELDS {
            return Err(NfsGazeError::ParseError(format!(
                "Invalid bytes line: {}",
                line
            )));
        }

        if let Some(ref mut mount) = self.current_mount {
            mount.bytes_read = parse_field(parts[1], "bytes_read")?;
            mount.bytes_write = match parts.get(6) {
                Some(val) => parse_field(val, "bytes_write")?,
                None => 0,
            };
        }
        Ok(())
    }

    fn parse_operation(&mut self, line: &str) -> Result<()> {
        let (op_name, stats_str) = line
            .split_once(':')
            .ok_or_else(|| NfsGazeError::ParseError(format!("Invalid operation line: {}", line)))?;

        let op_name = op_name.trim();
        let stats: Vec<String> = stats_str
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let operation = parse_nfs_operation(op_name, &stats)?;

        if let Some(ref mut mount) = self.current_mount {
            mount.operations.insert(op_name.to_string(), operation);
        }
        Ok(())
    }
}

/// Parse mountstats from a file path
pub fn parse_mountstats(path: &str) -> Result<HashMap<String, NFSMount>> {
    let file = File::open(path)?;
    parse_mountstats_reader(file)
}

/// Parse mountstats from a reader (for testing)
pub fn parse_mountstats_reader<R: Read>(reader: R) -> Result<HashMap<String, NFSMount>> {
    let buf_reader = BufReader::new(reader);
    let parser = MountstatsParser::new();
    parser.parse(buf_reader)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_events_valid() {
        let parts: Vec<String> = (1..=27).map(|i| i.to_string()).collect();
        let events = parse_events(&parts).expect("Should parse valid events");

        assert_eq!(events.inode_revalidate, 1);
        assert_eq!(events.dentry_revalidate, 2);
        assert_eq!(events.pnfs_write, 27);
    }

    #[test]
    fn test_parse_events_insufficient_parts() {
        let parts: Vec<String> = vec!["1".to_string(), "2".to_string(), "3".to_string()];
        let result = parse_events(&parts);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_events_invalid_number() {
        let mut parts: Vec<String> = (1..=27).map(|i| i.to_string()).collect();
        if !parts.is_empty() {
            parts[0] = "invalid".to_string();
        }
        let result = parse_events(&parts);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_nfs_operation_valid() {
        let stats = vec![
            "100".to_string(),
            "95".to_string(),
            "5".to_string(),
            "1024".to_string(),
            "2048".to_string(),
            "10".to_string(),
            "20".to_string(),
            "30".to_string(),
            "2".to_string(),
        ];

        let op = parse_nfs_operation("READ", &stats).expect("Should parse valid operation");

        assert_eq!(op.name, "READ");
        assert_eq!(op.ops, 100);
        assert_eq!(op.ntrans, 95);
        assert_eq!(op.timeouts, 5);
        assert_eq!(op.bytes_sent, 1024);
        assert_eq!(op.bytes_recv, 2048);
        assert_eq!(op.queue_time, 10);
        assert_eq!(op.rtt, 20);
        assert_eq!(op.execute_time, 30);
        assert_eq!(op.errors, 2);
    }

    #[test]
    fn test_parse_nfs_operation_insufficient_stats() {
        let stats = vec!["100".to_string(), "95".to_string()];
        let result = parse_nfs_operation("READ", &stats);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_nfs_operation_invalid_number() {
        let stats = vec![
            "invalid".to_string(),
            "95".to_string(),
            "5".to_string(),
            "1024".to_string(),
            "2048".to_string(),
            "10".to_string(),
            "20".to_string(),
            "30".to_string(),
            "2".to_string(),
        ];

        let result = parse_nfs_operation("READ", &stats);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mountstats_reader() {
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1
age: 12345
events: 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27
bytes: 1048576 0 0 0 0 2097152 0 0
READ: 100 95 5 1024 2048 10 20 30 2
WRITE: 50 50 0 512 0 5 15 25 1
"#;

        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("Should parse mountstats");

        assert_eq!(mounts.len(), 1);
        let mount = &mounts["/mnt/nfs"];
        assert_eq!(mount.device, "server:/export");
        assert_eq!(mount.mount_point, "/mnt/nfs");
        assert_eq!(mount.server, "server");
        assert_eq!(mount.export, "/export");
        assert_eq!(mount.age, 12345);
        assert_eq!(mount.bytes_read, 1048576);
        assert_eq!(mount.bytes_write, 2097152);
        assert_eq!(mount.operations.len(), 2);

        let read_op = &mount.operations["READ"];
        assert_eq!(read_op.ops, 100);
        assert_eq!(read_op.bytes_sent, 1024);
        assert_eq!(read_op.bytes_recv, 2048);
    }

    #[test]
    fn test_parse_mountstats_multiple_mounts() {
        let mountstats_data = r#"device server1:/export1 mounted on /mnt/nfs1 with fstype nfs statvers=1.1
age: 1000
READ: 10 10 0 100 200 1 2 3 0

device server2:/export2 mounted on /mnt/nfs2 with fstype nfs statvers=1.1
age: 2000
WRITE: 20 20 0 300 400 2 3 4 0
"#;

        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("Should parse mountstats");

        assert_eq!(mounts.len(), 2);
        assert!(mounts.contains_key("/mnt/nfs1"));
        assert!(mounts.contains_key("/mnt/nfs2"));

        let mount1 = &mounts["/mnt/nfs1"];
        assert_eq!(mount1.age, 1000);
        assert!(mount1.operations.contains_key("READ"));

        let mount2 = &mounts["/mnt/nfs2"];
        assert_eq!(mount2.age, 2000);
        assert!(mount2.operations.contains_key("WRITE"));
    }

    #[test]
    fn test_parse_bytes_short_line_defaults_write_to_zero() {
        // bytes line with only 6 fields (no index 6) — bytes_write should default to 0
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1
age: 100
bytes: 1048576 0 0 0 0
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("Should parse mountstats");
        let mount = &mounts["/mnt/nfs"];
        assert_eq!(mount.bytes_read, 1048576);
        assert_eq!(mount.bytes_write, 0);
    }

    #[test]
    fn test_ignored_prefixes_are_skipped() {
        // Real mountstats contain RPC, xprt, opts, caps, sec lines that should be skipped
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs4 statvers=1.1
        opts:   rw,vers=4.2,rsize=1048576,wsize=1048576
        age:    500
        caps:   caps=0x3ffdf,wtmult=512,dtsize=32768
        sec:    flavor=1,pseudoflavor=1
        RPC: some rpc stats here
        xprt:  tcp 771 0 1 0 0 12345 12345 0 0
        per-op statistics
        READ: 100 100 0 1024 2048 10 20 30 0
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("Should parse mountstats");
        let mount = &mounts["/mnt/nfs"];
        assert_eq!(mount.age, 500);
        assert_eq!(mount.operations.len(), 1);
        assert!(mount.operations.contains_key("READ"));
    }

    #[test]
    fn test_parse_operation_without_errors_field() {
        // Operations with only 8 fields (no errors) should still parse
        let stats: Vec<String> = vec!["100", "95", "5", "1024", "2048", "10", "20", "30"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let op = parse_nfs_operation("READ", &stats).expect("Should parse without errors field");
        assert_eq!(op.ops, 100);
        assert_eq!(op.errors, 0);
    }

    #[test]
    fn test_parse_device_without_colon() {
        // Server export without colon — export should default to "/"
        let mountstats_data = r#"device serveronly mounted on /mnt/nfs with fstype nfs statvers=1.1
age: 100
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("Should parse mountstats");
        let mount = &mounts["/mnt/nfs"];
        assert_eq!(mount.server, "serveronly");
        assert_eq!(mount.export, "/");
    }
}
