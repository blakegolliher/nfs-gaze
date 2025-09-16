use crate::types::{NFSEvents, NFSMount, NFSOperation, NfsGazeError, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};

/// Parse the events line into an NFSEvents struct
pub fn parse_events(parts: &[String]) -> Result<NFSEvents> {
    if parts.len() < 25 {
        return Err(NfsGazeError::InvalidEventsParts(parts.len()));
    }

    let parse_int = |index: usize, field: &str| -> Result<i64> {
        parts[index]
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
    if parts.len() > 25 {
        events.pnfs_read = parse_int(25, "PNFSRead")?;
    }
    if parts.len() > 26 {
        events.pnfs_write = parse_int(26, "PNFSWrite")?;
    }

    Ok(events)
}

/// Parse NFS operation statistics from a stats line
pub fn parse_nfs_operation(op_name: &str, stats: &[String]) -> Result<NFSOperation> {
    if stats.len() < 9 {
        return Err(NfsGazeError::ParseError(format!(
            "insufficient stats for operation {}: got {}, need 9",
            op_name,
            stats.len()
        )));
    }

    let parse_int = |index: usize, field: &str| -> Result<i64> {
        stats[index]
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
    if stats.len() > 8 {
        operation.errors = parse_int(8, "errors")?;
    }

    Ok(operation)
}

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

    fn parse<R: BufRead>(&mut self, reader: R) -> Result<HashMap<String, NFSMount>> {
        for line in reader.lines() {
            let line = line?;
            self.parse_line(&line)?;
        }
        Ok(self.mounts.clone())
    }

    fn parse_line(&mut self, line: &str) -> Result<()> {
        let line = line.trim();
        if line.starts_with("device") && line.contains("nfs") {
            self.parse_device_line(line)?
        } else if self.current_mount.is_some() {
            self.parse_stats_line(line)?
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

        let server_parts: Vec<&str> = server_export.splitn(2, ':').collect();
        let server = server_parts[0].to_string();
        let export = if server_parts.len() > 1 {
            server_parts[1].to_string()
        } else {
            "/".to_string()
        };

        let mount = NFSMount {
            device: server_export.to_string(),
            mount_point: mount_point.to_string(),
            server,
            export,
            age: 0,
            operations: HashMap::new(),
            events: None,
            bytes_read: 0,
            bytes_write: 0,
        };

        self.mounts.insert(mount_point.to_string(), mount.clone());
        self.current_mount = Some(mount);
        Ok(())
    }

    fn parse_stats_line(&mut self, line: &str) -> Result<()> {
        if line.starts_with("age:") {
            self.parse_age(line)
        } else if line.starts_with("events:") {
            self.parse_events_line(line)
        } else if line.starts_with("bytes:") {
            self.parse_bytes(line)
        } else if line.contains(':')
            && !line.starts_with("RPC")
            && !line.starts_with("xprt")
            && !line.starts_with("per-op")
            && !line.starts_with("opts")
            && !line.starts_with("caps")
            && !line.starts_with("sec")
            && !line.starts_with("nfsv4")
            && !line.starts_with("nfsv3")
        {
            self.parse_operation(line)
        } else {
            Ok(())
        }
    }

    fn parse_age(&mut self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(NfsGazeError::ParseError(format!(
                "Invalid age line: {}",
                line
            )));
        }

        if let Some(ref mut mount) = self.current_mount {
            mount.age = parts[1]
                .parse()
                .map_err(|e| NfsGazeError::FieldParseError {
                    field: "age".to_string(),
                    source: e,
                })?;

            // Update in mounts map
            if let Some(existing_mount) = self.mounts.get_mut(&mount.mount_point) {
                existing_mount.age = mount.age;
            }
        }
        Ok(())
    }

    fn parse_events_line(&mut self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(NfsGazeError::ParseError(format!(
                "Invalid events line: {}",
                line
            )));
        }

        let event_parts: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
        let events = parse_events(&event_parts)?;

        if let Some(ref mut mount) = self.current_mount {
            mount.events = Some(events.clone());

            // Update in mounts map
            if let Some(existing_mount) = self.mounts.get_mut(&mount.mount_point) {
                existing_mount.events = Some(events);
            }
        }
        Ok(())
    }

    fn parse_bytes(&mut self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            return Err(NfsGazeError::ParseError(format!(
                "Invalid bytes line: {}",
                line
            )));
        }

        if let Some(ref mut mount) = self.current_mount {
            mount.bytes_read = parts[1]
                .parse()
                .map_err(|e| NfsGazeError::FieldParseError {
                    field: "bytes_read".to_string(),
                    source: e,
                })?;
            // Handle different formats - try both index 5 and 6
            mount.bytes_write = if parts.len() > 6 && parts[6] != "0" {
                parts[6]
                    .parse()
                    .map_err(|e| NfsGazeError::FieldParseError {
                        field: "bytes_write".to_string(),
                        source: e,
                    })?
            } else if parts.len() > 5 {
                parts[5]
                    .parse()
                    .map_err(|e| NfsGazeError::FieldParseError {
                        field: "bytes_write".to_string(),
                        source: e,
                    })?
            } else {
                0
            };

            // Update in mounts map
            if let Some(existing_mount) = self.mounts.get_mut(&mount.mount_point) {
                existing_mount.bytes_read = mount.bytes_read;
                existing_mount.bytes_write = mount.bytes_write;
            }
        }
        Ok(())
    }

    fn parse_operation(&mut self, line: &str) -> Result<()> {
        let op_parts: Vec<&str> = line.splitn(2, ':').collect();
        if op_parts.len() != 2 {
            return Err(NfsGazeError::ParseError(format!(
                "Invalid operation line: {}",
                line
            )));
        }

        let op_name = op_parts[0].trim();
        let stats: Vec<String> = op_parts[1]
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let operation = parse_nfs_operation(op_name, &stats)?;

        if let Some(ref mut mount) = self.current_mount {
            mount
                .operations
                .insert(op_name.to_string(), operation.clone());

            // Update in mounts map
            if let Some(existing_mount) = self.mounts.get_mut(&mount.mount_point) {
                existing_mount
                    .operations
                    .insert(op_name.to_string(), operation);
            }
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
    let mut parser = MountstatsParser::new();
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
        parts[0] = "invalid".to_string();
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
}
