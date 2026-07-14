use crate::types::{NFSEvents, NFSMount, NFSOperation, NfsGazeError, Result, XprtStats};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::sync::{Mutex, OnceLock};

// Minimum number of fields required for parsing
const MIN_EVENTS_FIELDS: usize = 25;
const MIN_OPERATION_FIELDS: usize = 8;
const MIN_BYTES_FIELDS: usize = 6;
const MIN_KEY_VALUE_FIELDS: usize = 2;

// Optional field indices
const PNFS_READ_INDEX: usize = 25;
const PNFS_WRITE_INDEX: usize = 26;
const OPERATION_ERRORS_INDEX: usize = 8;

/// Number of whitespace-separated fields on a TCP `xprt:` line after
/// the `xprt:` label itself. Layout (from net/sunrpc/xprtsock.c):
///
/// ```text
/// xprt: tcp <srcport> <bind> <connect> <connect_time> <idle_time>
///           <sends> <recvs> <bad_xids> <req_u> <bklog_u> <max_slots>
///           <sending_u> <pending_u>
/// ```
const TCP_XPRT_FIELD_COUNT: usize = 14;

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

/// Emit a diagnostic for skipped or unparseable input at most once per
/// `key` for the lifetime of the process.
///
/// The monitoring loop re-parses `/proc/self/mountstats` every
/// interval, so an unconditional `eprintln!` for a persistently odd
/// line would repeat once per second forever. Deduplicating on a
/// stable key (the line kind, or the operation name) keeps the signal
/// without the spam. The set is bounded in practice: keys are drawn
/// from line kinds and operation names, not raw line content.
fn warn_once(key: &str, message: &str) {
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let warned = WARNED.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = match warned.lock() {
        Ok(guard) => guard,
        // A panic while holding this lock cannot corrupt a HashSet of
        // Strings in a way that matters for dedup; keep warning.
        Err(poisoned) => poisoned.into_inner(),
    };
    if guard.insert(key.to_string()) {
        eprintln!("nfs-gaze: {}", message);
    }
}

/// Main mountstats parser.
///
/// # Robustness contract
///
/// `/proc/self/mountstats` mixes stat lines this tool consumes with
/// metadata lines it does not (`opts:`, `caps:`, `sec:`, `impl_id:`,
/// `nfsv4:`, `fsc:`, `RPC iostats version:`, ...), and kernels keep
/// adding new ones. The parser therefore *never* fails the whole file
/// because of a line it does not understand:
///
/// - Per-op statistics are only parsed inside the `per-op statistics`
///   section (the kernel has printed that marker since 2006), so
///   metadata lines can never be mistaken for operations.
/// - A malformed line inside a recognised construct (a stats value
///   that does not parse, a truncated per-op line) is skipped with a
///   once-per-key warning on stderr rather than aborting.
/// - Only I/O errors reading the file surface as `Err`.
struct MountstatsParser {
    mounts: HashMap<String, NFSMount>,
    current_mount: Option<NFSMount>,
    /// True once the `per-op statistics` marker of the current mount
    /// block has been seen. Reset on every device line.
    in_per_op_section: bool,
}

impl MountstatsParser {
    fn new() -> Self {
        Self {
            mounts: HashMap::new(),
            current_mount: None,
            in_per_op_section: false,
        }
    }

    /// Flush current_mount into the mounts map.
    ///
    /// Mounts are keyed by mount point. When the same path appears
    /// twice (an over-mount: a second NFS filesystem mounted on top
    /// of an existing one), the kernel lists the older mount first,
    /// so keeping the later entry means keeping the mount that is
    /// actually visible at that path — which is what a user pointing
    /// `-m` at the path expects to monitor. The shadowed mount's
    /// stats are dropped, and that is worth a diagnostic: silently
    /// last-winning looks identical to a bug when the two devices
    /// differ.
    fn flush_current(&mut self) {
        if let Some(mount) = self.current_mount.take() {
            if let Some(shadowed) = self.mounts.insert(mount.mount_point.clone(), mount) {
                let visible = &self.mounts[&shadowed.mount_point];
                if visible.device != shadowed.device {
                    warn_once(
                        &format!("overmount:{}", shadowed.mount_point),
                        &format!(
                            "{} is over-mounted: monitoring {} (topmost); {} is shadowed and not monitored",
                            shadowed.mount_point, visible.device, shadowed.device
                        ),
                    );
                }
            }
        }
    }

    fn parse<R: BufRead>(mut self, reader: R) -> Result<HashMap<String, NFSMount>> {
        for line in reader.lines() {
            let line = line?;
            self.parse_line(&line);
        }
        self.flush_current();
        Ok(self.mounts)
    }

    fn parse_line(&mut self, line: &str) {
        let line = line.trim();
        if line.starts_with("device ") {
            // Every device line — NFS or not — terminates the previous
            // mount's block, so a non-NFS mount between two NFS mounts
            // cannot cause stats lines to bleed into the wrong mount.
            self.flush_current();
            self.in_per_op_section = false;
            self.parse_device_line(line);
        } else if self.current_mount.is_some() {
            self.parse_stats_line(line);
        }
    }

    fn parse_device_line(&mut self, line: &str) {
        // Example: "device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1"
        let parts: Vec<&str> = line.splitn(2, " on ").collect();
        if parts.len() != 2 {
            // Not warned: /proc/self/mountstats routinely lists
            // non-NFS filesystems whose device lines take other
            // shapes, and they are simply not this tool's business.
            return;
        }

        let device_info: Vec<&str> = parts[0].split_whitespace().collect();
        let mount_info: Vec<&str> = parts[1].split_whitespace().collect();

        if device_info.len() < 2 || mount_info.is_empty() {
            return;
        }

        // Only the NFS *client* filesystem types are ours. Matching
        // must be exact: substring checks catch `fstype nfsd` (the
        // server-side export filesystem, present on any NFS server)
        // and arbitrary mounts whose path merely contains "nfs".
        let fstype = mount_info
            .iter()
            .position(|&w| w == "fstype")
            .and_then(|i| mount_info.get(i + 1))
            .copied()
            .unwrap_or("");
        if fstype != "nfs" && fstype != "nfs4" {
            return;
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
            fstype: fstype.to_string(),
            options: String::new(),
            age: 0,
            operations: HashMap::new(),
            events: None,
            bytes_read: 0,
            bytes_write: 0,
            xprt: None,
        });
    }

    fn parse_stats_line(&mut self, line: &str) {
        if line.starts_with("per-op") {
            self.in_per_op_section = true;
        } else if line.starts_with("age:") {
            self.parse_age(line);
        } else if line.starts_with("events:") {
            self.parse_events_line(line);
        } else if line.starts_with("bytes:") {
            self.parse_bytes(line);
        } else if line.starts_with("xprt:") {
            self.parse_xprt(line);
        } else if let Some(opts) = line.strip_prefix("opts:") {
            if let Some(ref mut mount) = self.current_mount {
                mount.options = opts.trim().to_string();
            }
        } else if self.in_per_op_section && line.contains(':') {
            self.parse_operation(line);
        }
        // Anything else — caps:, sec:, impl_id:, nfsv4:, fsc:,
        // "RPC iostats version:", blank lines, and whatever future
        // kernels add — is metadata this tool does not consume, and is
        // ignored without ceremony. Only lines inside the per-op
        // section are expected to be operations.
    }

    fn parse_age(&mut self, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.get(1).and_then(|v| v.parse::<i64>().ok()) {
            Some(age) => {
                if let Some(ref mut mount) = self.current_mount {
                    mount.age = age;
                }
            }
            None => warn_once("age", &format!("skipping malformed age line: {}", line)),
        }
    }

    fn parse_events_line(&mut self, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < MIN_KEY_VALUE_FIELDS {
            warn_once(
                "events",
                &format!("skipping malformed events line: {}", line),
            );
            return;
        }

        let event_parts: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
        match parse_events(&event_parts) {
            Ok(events) => {
                if let Some(ref mut mount) = self.current_mount {
                    mount.events = Some(events);
                }
            }
            Err(e) => warn_once(
                "events",
                &format!("skipping malformed events line ({}): {}", e, line),
            ),
        }
    }

    fn parse_bytes(&mut self, line: &str) {
        // Kernel format: "bytes: normalread normalwrite directread directwrite serverread serverwrite pagesread pageswrite"
        // Index:              1          2           3            4          5           6          7          8
        //
        // We report the *server* pair (indexes 5 and 6): bytes that
        // actually crossed the wire via READ/WRITE RPCs. The normal*
        // pair counts application read()/write() traffic, which both
        // includes page-cache hits that never touched the network and
        // excludes O_DIRECT I/O entirely — an O_DIRECT-heavy workload
        // (databases, hypervisors) shows normalread=0 while moving
        // hundreds of gigabytes. Wire-level is the symmetric,
        // cache-independent truth for a network monitor.
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < MIN_BYTES_FIELDS {
            warn_once("bytes", &format!("skipping malformed bytes line: {}", line));
            return;
        }

        let read = parts[5].parse::<i64>();
        // A short line missing the write field is tolerated as zero;
        // a present-but-unparseable field is not silently zeroed.
        let write = parts.get(6).map(|v| v.parse::<i64>()).transpose();
        match (read, write) {
            (Ok(read), Ok(write)) => {
                if let Some(ref mut mount) = self.current_mount {
                    mount.bytes_read = read;
                    mount.bytes_write = write.unwrap_or(0);
                }
            }
            _ => warn_once("bytes", &format!("skipping malformed bytes line: {}", line)),
        }
    }

    fn parse_xprt(&mut self, line: &str) {
        // The line looks like `xprt:\ttcp 732 1 40 0 0 59381805 ...`
        // — one "xprt:" token, one protocol tag, then the numeric
        // fields. We handle TCP in full; UDP and RDMA have different
        // layouts and are left unparsed so downstream code can tell
        // "no data" apart from "data layout I do not understand".
        //
        // Mounts using `nconnect=N` print one xprt line per
        // connection; each recognised line is folded into the
        // mount-wide aggregate (see [`XprtStats::absorb`]) so the
        // reported transport numbers describe the whole mount, not
        // just whichever connection happened to be printed last.
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return;
        }

        let xprt = match parts[1] {
            "tcp" if parts.len() > TCP_XPRT_FIELD_COUNT => match parse_tcp_xprt(&parts) {
                Some(xprt) => Some(xprt),
                None => {
                    warn_once("xprt", &format!("skipping malformed xprt line: {}", line));
                    None
                }
            },
            // UDP and RDMA have different field layouts, and a TCP
            // line with too few fields cannot be safely indexed.
            _ => None,
        };

        if let (Some(new), Some(mount)) = (xprt, self.current_mount.as_mut()) {
            match mount.xprt.as_mut() {
                None => mount.xprt = Some(new),
                Some(acc) if acc.protocol == new.protocol => acc.absorb(&new),
                // The kernel cannot mix transport protocols within a
                // single mount; if it ever appears to, keep the first
                // protocol's aggregate rather than corrupting it.
                Some(_) => warn_once(
                    "xprt-protocol-mix",
                    &format!("ignoring xprt line with mismatched protocol: {}", line),
                ),
            }
        }
    }

    fn parse_operation(&mut self, line: &str) {
        let Some((op_name, stats_str)) = line.split_once(':') else {
            return;
        };

        let op_name = op_name.trim();
        let stats: Vec<String> = stats_str
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        match parse_nfs_operation(op_name, &stats) {
            Ok(operation) => {
                if let Some(ref mut mount) = self.current_mount {
                    mount.operations.insert(op_name.to_string(), operation);
                }
            }
            Err(e) => warn_once(
                &format!("op:{}", op_name),
                &format!("skipping unparseable per-op line for '{}': {}", op_name, e),
            ),
        }
    }
}

/// Parse the numeric fields of a TCP `xprt:` line.
///
/// `parts[0]` is the `xprt:` label, `parts[1]` the protocol tag, and
/// `parts[2..]` the 13 numeric fields. The fields we care about live
/// at these offsets:
///
/// ```text
///   2   srcport
///   3   bind_count
///   4   connect_count
///   5   connect_time_ms
///   6   idle_time_s
///   7   sends
///   8   recvs
///   9   bad_xids
///   10  req_u
///   11  bklog_u
///   12  max_slots
///   13  sending_u
///   14  pending_u
/// ```
///
/// Returns `None` if any field fails to parse as an integer.
fn parse_tcp_xprt(parts: &[&str]) -> Option<XprtStats> {
    let field = |index: usize| parts[index].parse::<i64>().ok();
    Some(XprtStats {
        protocol: "tcp".to_string(),
        sends: field(7)?,
        recvs: field(8)?,
        bad_xids: field(9)?,
        req_u: field(10)?,
        bklog_u: field(11)?,
        max_slots: field(12)?,
        sending_u: field(13)?,
        pending_u: field(14)?,
        nconnect: 1,
    })
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
bytes: 111 222 333 444 1048576 2097152 777 888
per-op statistics
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
per-op statistics
READ: 10 10 0 100 200 1 2 3 0

device server2:/export2 mounted on /mnt/nfs2 with fstype nfs statvers=1.1
age: 2000
per-op statistics
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
bytes: 11 22 33 44 1048576
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("Should parse mountstats");
        let mount = &mounts["/mnt/nfs"];
        assert_eq!(mount.bytes_read, 1048576);
        assert_eq!(mount.bytes_write, 0);
    }

    #[test]
    fn test_parse_bytes_uses_wire_level_server_fields() {
        // Regression pin for the O_DIRECT blind spot: a direct-I/O
        // workload has normalread/normalwrite = 0 while the server*
        // fields carry the real transfer volume. bytes_read/bytes_write
        // must come from the server (wire-level) pair, indexes 5 and 6.
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1
age: 100
bytes: 0 0 183766089728 152135794688 183766089728 152135794688 0 0
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("Should parse mountstats");
        let mount = &mounts["/mnt/nfs"];
        assert_eq!(mount.bytes_read, 183766089728);
        assert_eq!(mount.bytes_write, 152135794688);
    }

    #[test]
    fn test_fstype_and_options_are_captured() {
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs4 statvers=1.1
	opts:	rw,vers=4.2,rsize=1048576,wsize=1048576,hard,proto=tcp
	age:	500
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("Should parse mountstats");
        let mount = &mounts["/mnt/nfs"];
        assert_eq!(mount.fstype, "nfs4");
        assert_eq!(
            mount.options,
            "rw,vers=4.2,rsize=1048576,wsize=1048576,hard,proto=tcp"
        );
    }

    #[test]
    fn test_nfsd_export_filesystem_is_not_a_mount() {
        // Every NFS *server* has "device nfsd mounted on /proc/fs/nfsd
        // with fstype nfsd" in its mountstats. The old substring check
        // (`line.contains("nfs")`) treated it as a client mount and
        // monitored garbage. fstype matching must be exact.
        let mountstats_data = r#"device nfsd mounted on /proc/fs/nfsd with fstype nfsd
device server:/export mounted on /mnt/data with fstype nfs statvers=1.1
	age:	100
	per-op statistics
	        READ: 10 10 0 100 200 1 2 3 0
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("Should parse mountstats");
        assert_eq!(
            mounts.len(),
            1,
            "only the client mount: {:?}",
            mounts.keys()
        );
        assert!(mounts.contains_key("/mnt/data"));
    }

    #[test]
    fn test_non_nfs_mount_with_nfs_in_path_is_not_a_mount() {
        // An ext4 volume mounted at a path that happens to contain
        // "nfs" also fooled the substring check.
        let mountstats_data = r#"device /dev/sdb1 mounted on /mnt/nfsbackup with fstype ext4
device server:/export mounted on /mnt/data with fstype nfs4 statvers=1.1
	age:	100
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("Should parse mountstats");
        assert_eq!(mounts.len(), 1, "only the NFS mount: {:?}", mounts.keys());
        assert!(mounts.contains_key("/mnt/data"));
    }

    #[test]
    fn test_overmounted_path_keeps_the_topmost_mount() {
        // The kernel lists the shadowed (older) mount first and the
        // over-mount (the one actually visible at the path) second.
        // The visible mount must win, and its stats must be its own —
        // nothing from the shadowed mount may bleed through.
        let mountstats_data = r#"device old-server:/old mounted on /mnt/data with fstype nfs statvers=1.1
	age:	99999
	per-op statistics
	        READ: 500 500 0 1000 2000 5 10 15 0
device new-server:/new mounted on /mnt/data with fstype nfs4 statvers=1.1
	age:	60
	per-op statistics
	       WRITE: 7 7 0 70 140 1 2 3 0
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("Should parse mountstats");
        assert_eq!(mounts.len(), 1);
        let mount = &mounts["/mnt/data"];
        assert_eq!(mount.device, "new-server:/new", "topmost mount must win");
        assert_eq!(mount.fstype, "nfs4");
        assert_eq!(mount.age, 60);
        assert!(
            mount.operations.contains_key("WRITE") && !mount.operations.contains_key("READ"),
            "shadowed mount's stats must not bleed into the visible mount: {:?}",
            mount.operations.keys()
        );
    }

    #[test]
    fn test_metadata_lines_are_skipped() {
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
    fn test_impl_id_line_does_not_abort_parse() {
        // NFSv4.1+ mounts print an impl_id line when the server sends
        // an implementation ID (NetApp, VAST, Isilon, ... all do).
        // This used to hard-fail the entire parse — the tool would not
        // even start. It must be ignored like any other metadata line.
        let mountstats_data = r#"device filer01:/vol/data mounted on /mnt/data with fstype nfs4 statvers=1.1
	opts:	rw,vers=4.1,rsize=1048576,wsize=1048576,namlen=255,hard,proto=tcp
	age:	86400
	impl_id:	name='NetApp Release 9.9.1P3',domain='netapp.com',date='1616445222,0'
	caps:	caps=0x3fff7,wtmult=512,dtsize=32768,bsize=0,namlen=255
	nfsv4:	bm0=0xfdffbfff,bm1=0x40f9be3e,bm2=0x803,acl=0x3,sessions,pnfs=not configured
	sec:	flavor=1,pseudoflavor=1
	per-op statistics
	        READ: 130 130 0 21320 546160 4 318 330 0
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("impl_id must not abort the parse");
        let mount = &mounts["/mnt/data"];
        assert_eq!(mount.age, 86400);
        assert_eq!(mount.operations["READ"].ops, 130);
    }

    #[test]
    fn test_fsc_line_does_not_abort_parse() {
        // fscache-enabled mounts on kernels <= 5.17 (including RHEL /
        // Rocky 9's 5.14) print a five-field "fsc:" stats line. It
        // used to hard-fail the entire parse.
        let mountstats_data = r#"device server:/export mounted on /mnt/cached with fstype nfs statvers=1.1
	age:	5000
	bytes:	1048576 0 0 0 1048576 0 256 0
	fsc:	204 0 0 0 12
	per-op statistics
	        READ: 256 256 0 41984 1049600 4 318 330
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("fsc must not abort the parse");
        assert_eq!(mounts["/mnt/cached"].operations["READ"].ops, 256);
    }

    #[test]
    fn test_ops_outside_per_op_section_are_not_parsed() {
        // Colon-lines before the "per-op statistics" marker are
        // metadata by the kernel's contract, never operations. Pin
        // that so future refactors cannot regress to guess-parsing.
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1
age: 100
READ: 100 100 0 1024 2048 10 20 30 0
per-op statistics
WRITE: 50 50 0 512 0 5 15 25 1
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("parse should succeed");
        let mount = &mounts["/mnt/nfs"];
        assert!(
            !mount.operations.contains_key("READ"),
            "pre-marker colon-line must not be treated as an operation"
        );
        assert!(mount.operations.contains_key("WRITE"));
    }

    #[test]
    fn test_bad_op_line_is_skipped_without_dropping_others() {
        // A corrupt line inside the per-op section must lose only
        // itself: every other operation still parses, and the parse
        // as a whole succeeds.
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1
age: 100
per-op statistics
READ: 100 100 0 1024 2048 10 20 30 0
BROKEN: 1 2 three 4
WRITE: 50 50 0 512 0 5 15 25 1
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("one bad op line must not abort");
        let mount = &mounts["/mnt/nfs"];
        assert_eq!(mount.operations.len(), 2);
        assert!(mount.operations.contains_key("READ"));
        assert!(mount.operations.contains_key("WRITE"));
        assert!(!mount.operations.contains_key("BROKEN"));
    }

    #[test]
    fn test_malformed_stat_lines_lose_only_their_field() {
        // Malformed age / events / bytes lines skip the field but keep
        // the mount and everything else in the file.
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1
age: not-a-number
events: 1 2 3
bytes: garbage 0 0 0 0 0 0 0
per-op statistics
READ: 100 100 0 1024 2048 10 20 30 0
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("malformed stat lines must not abort");
        let mount = &mounts["/mnt/nfs"];
        assert_eq!(mount.age, 0);
        assert!(mount.events.is_none());
        assert_eq!(mount.bytes_read, 0);
        assert_eq!(mount.operations["READ"].ops, 100);
    }

    #[test]
    fn test_non_nfs_device_line_terminates_previous_block() {
        // A non-NFS device line between two NFS mounts must close the
        // first mount's block so no stats bleed across, and must not
        // itself become a mount.
        let mountstats_data = r#"device server1:/a mounted on /mnt/a with fstype nfs statvers=1.1
age: 100
per-op statistics
READ: 10 10 0 100 200 1 2 3 0
device /dev/sda1 mounted on /boot with fstype ext4
device server2:/b mounted on /mnt/b with fstype nfs statvers=1.1
age: 200
per-op statistics
WRITE: 20 20 0 300 400 2 3 4 0
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("parse should succeed");
        assert_eq!(mounts.len(), 2);
        assert!(mounts["/mnt/a"].operations.contains_key("READ"));
        assert!(!mounts["/mnt/a"].operations.contains_key("WRITE"));
        assert!(mounts["/mnt/b"].operations.contains_key("WRITE"));
    }

    #[test]
    fn test_malformed_device_line_skips_block_but_keeps_other_mounts() {
        // A device line that matches the NFS filter but cannot be
        // split into device/mountpoint should drop only its own block.
        let mountstats_data = r#"device nfs-garbage-without-mount-marker
age: 999
per-op statistics
READ: 10 10 0 100 200 1 2 3 0
device server:/b mounted on /mnt/b with fstype nfs statvers=1.1
age: 200
per-op statistics
WRITE: 20 20 0 300 400 2 3 4 0
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("parse should succeed");
        assert_eq!(mounts.len(), 1, "only the well-formed mount should survive");
        assert_eq!(mounts["/mnt/b"].age, 200);
    }

    #[test]
    fn test_parse_operation_without_errors_field() {
        // Operations with only 8 fields (no errors) should still parse
        let stats: Vec<String> = ["100", "95", "5", "1024", "2048", "10", "20", "30"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let op = parse_nfs_operation("READ", &stats).expect("Should parse without errors field");
        assert_eq!(op.ops, 100);
        assert_eq!(op.errors, 0);
    }

    #[test]
    fn test_parse_xprt_tcp_line_populates_all_fields() {
        // Real xprt line captured from /proc/self/mountstats on this
        // host. The layout is documented at TCP_XPRT_FIELD_COUNT.
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1
age: 12345
xprt:	tcp 732 1 40 0 0 59381805 59381803 2 84476199495 0 7091 820821642 83982296098
per-op statistics
READ: 100 95 5 1024 2048 10 20 30 2
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("parse should succeed");
        let mount = &mounts["/mnt/nfs"];
        let xprt = mount
            .xprt
            .as_ref()
            .expect("xprt should have been populated from TCP line");

        assert_eq!(xprt.protocol, "tcp");
        assert_eq!(xprt.sends, 59381805);
        assert_eq!(xprt.recvs, 59381803);
        assert_eq!(xprt.bad_xids, 2);
        assert_eq!(xprt.req_u, 84476199495);
        assert_eq!(xprt.bklog_u, 0);
        assert_eq!(xprt.max_slots, 7091);
        assert_eq!(xprt.sending_u, 820821642);
        assert_eq!(xprt.pending_u, 83982296098);
    }

    #[test]
    fn test_parse_xprt_nconnect_lines_are_summed_not_last_wins() {
        // nconnect=N mounts print one xprt line per connection. The
        // mount-wide aggregate must sum the cumulative counters, take
        // the max of the per-connection slot HWMs, and count the
        // connections. (The original bug kept only the last line,
        // silently dropping N-1 transports.)
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1
age: 100
xprt:	tcp 904 1 8 0 0 1000 990 1 1010 5 4 30 500
xprt:	tcp 905 1 8 0 0 2000 1980 2 2020 10 6 40 600
xprt:	tcp 906 1 8 0 0 3000 2970 3 3030 15 2 50 700
per-op statistics
READ: 100 95 5 1024 2048 10 20 30 2
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("parse should succeed");
        let xprt = mounts["/mnt/nfs"].xprt.as_ref().expect("xprt aggregated");

        assert_eq!(xprt.nconnect, 3);
        assert_eq!(xprt.sends, 6000, "sends must be summed across connections");
        assert_eq!(xprt.recvs, 5940);
        assert_eq!(xprt.bad_xids, 6);
        assert_eq!(xprt.req_u, 6060);
        assert_eq!(xprt.bklog_u, 30);
        assert_eq!(xprt.sending_u, 120);
        assert_eq!(xprt.pending_u, 1800);
        assert_eq!(
            xprt.max_slots, 6,
            "slot HWM is the per-connection max, not a sum — the cap applies per transport"
        );
    }

    #[test]
    fn test_parse_xprt_single_line_has_nconnect_one() {
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1
age: 100
xprt:	tcp 904 1 8 0 0 1000 990 1 1010 5 4 30 500
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("parse should succeed");
        assert_eq!(mounts["/mnt/nfs"].xprt.as_ref().unwrap().nconnect, 1);
    }

    #[test]
    fn test_parse_xprt_udp_is_unparsed_but_not_an_error() {
        // UDP has a different field layout. We intentionally set xprt
        // to None rather than partially populate it — downstream code
        // can tell "unknown layout" from "no xprt line" by the None.
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1
age: 100
xprt:	udp 1234 1 5000 5000 0 1000 0 10 50 100
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("parse should succeed");
        let mount = &mounts["/mnt/nfs"];
        assert!(
            mount.xprt.is_none(),
            "UDP xprt should currently be parsed as None, not an error"
        );
    }

    #[test]
    fn test_parse_xprt_truncated_tcp_line_is_treated_as_unparseable() {
        // A truncated line (too few fields for TCP) should not panic
        // or return Err — it should leave xprt as None, because we
        // cannot safely extract the higher-index fields.
        let mountstats_data = r#"device server:/export mounted on /mnt/nfs with fstype nfs statvers=1.1
age: 100
xprt:	tcp 732 1 40 0 0 59381805 59381803
"#;
        let cursor = Cursor::new(mountstats_data);
        let mounts = parse_mountstats_reader(cursor).expect("parse should succeed");
        assert!(mounts["/mnt/nfs"].xprt.is_none());
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
