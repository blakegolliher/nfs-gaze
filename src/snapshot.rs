//! JSON snapshot report types.
//!
//! A capture session can be serialised to a [`Report`] and written to
//! disk with `--output`, then later fed to the `compare` subcommand
//! (or to any other consumer that understands the schema). The field
//! names and layout intentionally match the JSON schema produced by
//! the older `nfs-monitor` Go tool, so files generated before the
//! cutover remain usable after it.
//!
//! The canonical on-wire shape is documented in the struct field
//! attributes below. New fields added here must either be backwards
//! compatible (via `#[serde(default)]`) or accompanied by a bump of
//! [`CURRENT_SCHEMA_VERSION`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Schema version written into new [`Report`] files. Consumers should
/// accept any value they recognise and reject (or warn on) newer
/// values they do not understand.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// A full capture-session report.
///
/// Produced by aggregating per-interval [`crate::types::DeltaStats`]
/// samples over the lifetime of a `nfs-gaze -o ...` invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// Schema version the writer was built against. See
    /// [`CURRENT_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Wall-clock time the report was finalised, in UTC.
    pub generated_at: DateTime<Utc>,
    /// Total capture duration in seconds. For a `--duration`-bounded
    /// run this mirrors the requested duration; for a `--count`-bounded
    /// run it reflects the actual elapsed time.
    pub duration_sec: u64,
    /// Sampling interval in seconds.
    pub interval_sec: u64,
    /// Number of non-seed samples folded into the aggregation. May be
    /// less than `duration_sec / interval_sec` if some intervals were
    /// dropped due to counter resets or parse errors.
    pub samples: u64,
    /// One entry per monitored mount, in a stable order (sorted by
    /// device string).
    pub mounts: Vec<MountReport>,
}

/// Aggregated statistics for a single monitored mount.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MountReport {
    /// The `server:/export` device string from `/proc/self/mountstats`.
    pub device: String,
    /// Local mount point path.
    #[serde(rename = "mountpoint")]
    pub mount_point: String,
    /// Filesystem type string (e.g. `nfs4`). May be empty if the
    /// parser was unable to recover it.
    pub fstype: String,
    /// Mount options string. May be empty.
    pub options: String,
    /// Mount-level totals across all operations.
    pub summary: SummaryStats,
    /// Per-operation aggregated stats, sorted by `ops` descending so
    /// the hottest operations appear first when the report is printed.
    pub operations: Vec<OpReport>,
}

/// Mount-level totals summed across every operation and every sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummaryStats {
    pub total_ops: i64,
    pub ops_per_sec: f64,
    #[serde(rename = "retransmissions")]
    pub retrans: i64,
    pub timeouts: i64,
    pub errors: i64,
}

/// Per-operation aggregated stats. RTT fields are in milliseconds and
/// describe the *per-interval* average RTT: `rtt_avg_ms` is the
/// ops-weighted mean across samples; `rtt_min_ms` / `rtt_max_ms` are
/// the minimum and maximum of the per-interval averages (not the
/// absolute fastest/slowest individual RPC observed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpReport {
    pub name: String,
    pub ops: i64,
    pub ops_per_sec: f64,
    #[serde(rename = "retransmissions")]
    pub retrans: i64,
    pub timeouts: i64,
    pub errors: i64,
    pub rtt_avg_ms: f64,
    pub rtt_min_ms: f64,
    pub rtt_max_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The canonical nfs-monitor example report, embedded at compile
    /// time. If this file moves, update the path below.
    const NFS_MONITOR_FIXTURE: &str = include_str!("../tests/fixtures/nfs_monitor_report.json");

    #[test]
    fn fixture_from_nfs_monitor_deserialises() {
        let report: Report = serde_json::from_str(NFS_MONITOR_FIXTURE)
            .expect("nfs-monitor fixture should deserialise into Report");

        assert_eq!(report.schema_version, 1);
        assert_eq!(report.duration_sec, 180);
        assert_eq!(report.interval_sec, 1);
        assert_eq!(report.samples, 180);
        assert_eq!(report.mounts.len(), 1);

        let mount = &report.mounts[0];
        assert_eq!(mount.device, "server02:/export/data");
        assert_eq!(mount.mount_point, "/mnt/nfs1");
        assert_eq!(mount.fstype, "nfs4");
        assert!(mount.options.contains("vers=4.1"));
        assert_eq!(mount.summary.total_ops, 52890);
        assert_eq!(mount.summary.retrans, 3);
        assert_eq!(mount.summary.timeouts, 0);

        // Operations are ordered by ops descending in the fixture.
        let first = &mount.operations[0];
        assert_eq!(first.name, "READ");
        assert_eq!(first.ops, 34200);
        assert!((first.rtt_avg_ms - 0.72).abs() < 1e-9);
    }

    #[test]
    fn report_round_trips_through_json() {
        // Deserialise, re-serialise, deserialise again and compare
        // structurally. This catches accidental field renames on
        // either the serialize or deserialize side — for example, if
        // a future refactor loses a `#[serde(rename)]` attribute.
        let first: Report = serde_json::from_str(NFS_MONITOR_FIXTURE).expect("fixture deserialise");
        let encoded = serde_json::to_string(&first).expect("serialise");
        let second: Report = serde_json::from_str(&encoded).expect("reparse");
        assert_eq!(first, second);
    }

    #[test]
    fn mountpoint_field_matches_nfs_monitor_name() {
        // The Go struct uses `json:"mountpoint"` (no underscore). The
        // Rust field is `mount_point` for idiom but must serialise to
        // `"mountpoint"` so cross-tool interchange keeps working.
        let mount = MountReport {
            device: "s:/e".into(),
            mount_point: "/mnt/x".into(),
            fstype: "nfs4".into(),
            options: String::new(),
            summary: SummaryStats {
                total_ops: 0,
                ops_per_sec: 0.0,
                retrans: 0,
                timeouts: 0,
                errors: 0,
            },
            operations: Vec::new(),
        };
        let json = serde_json::to_string(&mount).expect("serialise");
        assert!(
            json.contains("\"mountpoint\":\"/mnt/x\""),
            "expected `mountpoint` field, got: {json}"
        );
        assert!(
            !json.contains("mount_point"),
            "mount_point must not leak into the wire format: {json}"
        );
    }

    #[test]
    fn retransmissions_field_uses_long_name() {
        // Same story as mountpoint: Go uses `json:"retransmissions"`,
        // Rust uses `retrans` for brevity internally. The rename must
        // stick on both ends.
        let summary = SummaryStats {
            total_ops: 0,
            ops_per_sec: 0.0,
            retrans: 5,
            timeouts: 0,
            errors: 0,
        };
        let json = serde_json::to_string(&summary).expect("serialise");
        assert!(
            json.contains("\"retransmissions\":5"),
            "expected `retransmissions` field, got: {json}"
        );
    }
}
