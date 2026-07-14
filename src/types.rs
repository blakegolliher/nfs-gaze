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
    /// Failure creating or writing the output report file. Kept
    /// distinct from [`MountstatsRead`] so "tool could not read its
    /// own input" and "tool could not write its own output" surface
    /// as different diagnostics in logs.
    #[error("Failed to write report to {path}: {source}")]
    ReportWrite {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// Failure inside `serde_json` while encoding the report. In
    /// practice this is nearly impossible for the current schema
    /// (all fields are trivially serialisable) but is surfaced
    /// explicitly so future schema changes cannot swallow the error.
    #[error("Failed to serialise report: {0}")]
    ReportSerialize(#[from] serde_json::Error),
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
    pub queue_time: i64,   // milliseconds
    pub rtt: i64,          // milliseconds
    pub execute_time: i64, // milliseconds
    pub errors: i64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NFSEvents {
    pub inode_revalidate: i64,  // index 0
    pub dentry_revalidate: i64, // index 1
    pub data_invalidate: i64,   // index 2
    pub attr_invalidate: i64,   // index 3
    pub vfs_open: i64,          // index 4
    pub vfs_lookup: i64,        // index 5
    pub vfs_access: i64,        // index 6
    pub vfs_update_page: i64,   // index 7
    pub vfs_read_page: i64,     // index 8
    pub vfs_read_pages: i64,    // index 9
    pub vfs_write_page: i64,    // index 10
    pub vfs_write_pages: i64,   // index 11
    pub vfs_getdents: i64,      // index 12
    pub vfs_setattr: i64,       // index 13
    pub vfs_flush: i64,         // index 14
    pub vfs_fsync: i64,         // index 15
    pub vfs_lock: i64,          // index 16
    pub vfs_release: i64,       // index 17
    pub congestion_wait: i64,   // index 18
    pub setattr_trunc: i64,     // index 19
    pub extend_write: i64,      // index 20
    pub silly_rename: i64,      // index 21
    pub short_read: i64,        // index 22
    pub short_write: i64,       // index 23
    pub delay: i64,             // index 24
    pub pnfs_read: i64,         // index 25
    pub pnfs_write: i64,        // index 26
}

/// Number of event counters in [`NFSEvents`], including the optional
/// trailing pNFS pair.
pub const NFS_EVENTS_COUNT: usize = 27;

impl NFSEvents {
    /// The counters in kernel index order (the order of the fields on
    /// the `events:` line, documented on each struct field).
    fn to_array(&self) -> [i64; NFS_EVENTS_COUNT] {
        [
            self.inode_revalidate,
            self.dentry_revalidate,
            self.data_invalidate,
            self.attr_invalidate,
            self.vfs_open,
            self.vfs_lookup,
            self.vfs_access,
            self.vfs_update_page,
            self.vfs_read_page,
            self.vfs_read_pages,
            self.vfs_write_page,
            self.vfs_write_pages,
            self.vfs_getdents,
            self.vfs_setattr,
            self.vfs_flush,
            self.vfs_fsync,
            self.vfs_lock,
            self.vfs_release,
            self.congestion_wait,
            self.setattr_trunc,
            self.extend_write,
            self.silly_rename,
            self.short_read,
            self.short_write,
            self.delay,
            self.pnfs_read,
            self.pnfs_write,
        ]
    }

    fn from_array(values: [i64; NFS_EVENTS_COUNT]) -> Self {
        Self {
            inode_revalidate: values[0],
            dentry_revalidate: values[1],
            data_invalidate: values[2],
            attr_invalidate: values[3],
            vfs_open: values[4],
            vfs_lookup: values[5],
            vfs_access: values[6],
            vfs_update_page: values[7],
            vfs_read_page: values[8],
            vfs_read_pages: values[9],
            vfs_write_page: values[10],
            vfs_write_pages: values[11],
            vfs_getdents: values[12],
            vfs_setattr: values[13],
            vfs_flush: values[14],
            vfs_fsync: values[15],
            vfs_lock: values[16],
            vfs_release: values[17],
            congestion_wait: values[18],
            setattr_trunc: values[19],
            extend_write: values[20],
            silly_rename: values[21],
            short_read: values[22],
            short_write: values[23],
            delay: values[24],
            pnfs_read: values[25],
            pnfs_write: values[26],
        }
    }

    /// Per-interval delta of two cumulative event samples.
    ///
    /// Returns `None` if any counter moved backwards — the kernel
    /// reset the per-mount stats (remount) and no meaningful delta
    /// exists for this interval. Mirrors the reset handling of the
    /// per-operation delta path.
    pub fn delta(current: &Self, previous: &Self) -> Option<Self> {
        let cur = current.to_array();
        let prev = previous.to_array();
        let mut delta = [0i64; NFS_EVENTS_COUNT];
        for i in 0..NFS_EVENTS_COUNT {
            if cur[i] < prev[i] {
                return None;
            }
            delta[i] = cur[i] - prev[i];
        }
        Some(Self::from_array(delta))
    }
}

/// RPC transport statistics parsed from the `xprt:` line in
/// `/proc/self/mountstats`.
///
/// The field set here matches the TCP variant, which is by far the
/// most common transport for NFSv3/v4 in Linux since the mid-2000s.
/// UDP and RDMA use different field layouts, so the parser sets
/// [`NFSMount::xprt`] to `None` for those protocols rather than
/// attempting a best-effort map — a partially populated struct is
/// harder to reason about than a missing one.
///
/// The fields with a `_u` suffix in the kernel (`req_u`, `bklog_u`,
/// `sending_u`, `pending_u`) are renamed to clearer names here;
/// their kernel semantics are:
///
/// - `req_u` — cumulative count of requests for slot accounting.
///   Used as the denominator for per-request averages of the
///   other `_u` fields. Roughly equal to [`Self::sends`] in a
///   healthy system.
/// - `bklog_u` — cumulative length of the backlog queue across
///   every enqueue. If this is climbing between samples, the
///   client is slot-starved: RPCs are queueing up waiting for a
///   free slot before they can even hit the wire. This is the
///   smoking gun for slot exhaustion.
/// - `max_slots` — high water mark of slots actually used. If this
///   equals the configured cap (`tcp_max_slot_table_entries`),
///   the client has hit the ceiling.
/// - `sending_u` — cumulative time (weighted by request count) in
///   the "sending" state. High values mean the socket send queue
///   is backing up, often a single-connection bottleneck that
///   `nconnect` can fix.
/// - `pending_u` — cumulative time waiting for a reply from the
///   server. High pending + low sending means the bottleneck is
///   the server or the network, not the client.
///
/// Mounts using `nconnect=N` open N connections and the kernel prints
/// one `xprt:` line per connection. This struct holds the *mount-wide
/// aggregate*: cumulative counters are summed across connections,
/// `max_slots` is the maximum of the per-connection high-water marks
/// (the slot cap applies per transport, so max — not sum — is what
/// compares against the cap), and [`Self::nconnect`] records how many
/// transport lines were folded in.
#[derive(Debug, Clone, PartialEq)]
pub struct XprtStats {
    /// Transport protocol tag: `"tcp"`, `"udp"`, or `"rdma"`.
    pub protocol: String,
    /// Cumulative number of RPC requests sent over this transport.
    pub sends: i64,
    /// Cumulative number of RPC replies received.
    pub recvs: i64,
    /// RPC replies with an XID that did not match any outstanding
    /// request. Usually zero; any non-zero value means the server
    /// is confused or the connection is corrupt.
    pub bad_xids: i64,
    /// Cumulative request count used as the denominator for
    /// per-request averages of `bklog_u`, `sending_u`, `pending_u`.
    pub req_u: i64,
    /// Cumulative backlog queue length — see type-level docs.
    pub bklog_u: i64,
    /// High water mark of slots used (not a cumulative counter; it
    /// only moves upward over the lifetime of the mount). Across
    /// multiple connections this is the per-connection maximum.
    pub max_slots: i64,
    /// Cumulative "sending" state dwell — see type-level docs.
    pub sending_u: i64,
    /// Cumulative "pending" state dwell — see type-level docs.
    pub pending_u: i64,
    /// Number of transport connections folded into this aggregate
    /// (`nconnect=N` mounts have N `xprt:` lines; plain mounts 1).
    pub nconnect: i64,
}

impl XprtStats {
    /// Fold another connection's stats into this mount-wide aggregate:
    /// cumulative counters add, the slot high-water mark takes the
    /// max, and the connection count grows. Callers must ensure both
    /// sides use the same protocol (the kernel cannot mix transports
    /// within one mount).
    pub(crate) fn absorb(&mut self, other: &XprtStats) {
        self.sends += other.sends;
        self.recvs += other.recvs;
        self.bad_xids += other.bad_xids;
        self.req_u += other.req_u;
        self.bklog_u += other.bklog_u;
        self.sending_u += other.sending_u;
        self.pending_u += other.pending_u;
        self.max_slots = self.max_slots.max(other.max_slots);
        self.nconnect += other.nconnect;
    }
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
    /// Cumulative bytes read from the server over the wire
    /// (`serverreadbytes` from the kernel's `bytes:` line). Wire-level
    /// rather than application-level: page-cache hits are excluded and
    /// O_DIRECT reads are included.
    pub bytes_read: i64,
    /// Cumulative bytes written to the server over the wire
    /// (`serverwritebytes`). Same wire-level semantics as
    /// [`Self::bytes_read`].
    pub bytes_write: i64,
    /// RPC transport statistics, when the parser recognised the
    /// `xprt:` line layout. `None` for unrecognised protocols (UDP,
    /// RDMA) or for mounts that did not have an `xprt:` line at all.
    pub xprt: Option<XprtStats>,
}

/// Per-interval delta of the RPC transport statistics.
///
/// Cumulative counters are subtracted to yield the work that
/// happened in the last sample interval, and the per-request
/// averages are derived from those deltas rather than re-reading
/// the cumulative values — otherwise a long-running session would
/// have its averages dominated by ancient history.
///
/// The three `_per_req` fields are the signal most worth watching
/// for slot pressure:
///
/// - `bklog_per_req > 0` means at least one request waited for a
///   free slot in this interval. Anything above ~0 is worth
///   investigating; sustained double-digit values indicate the
///   client is slot-starved.
/// - `sending_per_req` high + `pending_per_req` low means the
///   socket is the bottleneck (consider `nconnect`).
/// - `sending_per_req` low + `pending_per_req` high means the
///   server or the network is the bottleneck, not the client.
#[derive(Debug, Clone, PartialEq)]
pub struct DeltaXprtStats {
    pub protocol: String,
    pub delta_sends: i64,
    pub delta_recvs: i64,
    pub delta_bad_xids: i64,
    pub delta_req: i64,
    pub delta_bklog: i64,
    pub delta_sending: i64,
    pub delta_pending: i64,
    /// Current high-water mark for slots actually used (max across
    /// connections on nconnect mounts). This is a monotonic gauge —
    /// not a delta — and is carried forward so callers can compare
    /// against the configured slot cap.
    pub max_slots: i64,
    /// Number of transport connections behind these numbers, carried
    /// forward from the current sample.
    pub nconnect: i64,
    pub bklog_per_req: f64,
    pub sending_per_req: f64,
    pub pending_per_req: f64,
}

impl DeltaXprtStats {
    /// True when the transport did any work this interval: RPCs were
    /// sent or received, requests entered the transport, the backlog
    /// queue moved, or a reply arrived for an unknown XID. This is
    /// deliberately independent of per-op completions — a mount whose
    /// every op is stuck in flight completes nothing, but its
    /// transport counters keep moving, and that divergence is the
    /// stall signal.
    pub fn has_activity(&self) -> bool {
        self.delta_sends > 0
            || self.delta_recvs > 0
            || self.delta_req > 0
            || self.delta_bklog > 0
            || self.delta_bad_xids > 0
    }
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
    pub delta_timeouts: i64,
    pub avg_rtt: f64,
    pub avg_exec: f64,
    pub avg_queue: f64,
    pub kb_per_op: f64,
    pub kb_per_sec: f64,
    pub iops: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet_xprt_delta() -> DeltaXprtStats {
        DeltaXprtStats {
            protocol: "tcp".to_string(),
            delta_sends: 0,
            delta_recvs: 0,
            delta_bad_xids: 0,
            delta_req: 0,
            delta_bklog: 0,
            delta_sending: 0,
            delta_pending: 0,
            max_slots: 16,
            nconnect: 1,
            bklog_per_req: 0.0,
            sending_per_req: 0.0,
            pending_per_req: 0.0,
        }
    }

    #[test]
    fn xprt_delta_with_no_movement_has_no_activity() {
        assert!(!quiet_xprt_delta().has_activity());
    }

    #[test]
    fn any_moving_transport_counter_counts_as_activity() {
        for field in ["sends", "recvs", "req", "bklog", "bad_xids"] {
            let mut d = quiet_xprt_delta();
            match field {
                "sends" => d.delta_sends = 1,
                "recvs" => d.delta_recvs = 1,
                "req" => d.delta_req = 1,
                "bklog" => d.delta_bklog = 1,
                _ => d.delta_bad_xids = 1,
            }
            assert!(
                d.has_activity(),
                "delta_{field} alone must count as activity"
            );
        }
    }

    #[test]
    fn slot_gauge_alone_is_not_activity() {
        // max_slots is a carried-forward gauge, not a delta; a high
        // watermark from earlier in the session must not make an
        // otherwise idle interval look active.
        let mut d = quiet_xprt_delta();
        d.max_slots = 4096;
        assert!(!d.has_activity());
    }
}
