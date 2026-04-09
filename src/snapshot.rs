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

use crate::types::{DeltaStats, NFSMount};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Streaming aggregator that folds per-interval [`DeltaStats`] into a
/// single [`MountReport`] at the end of a capture session.
///
/// Callers construct one aggregator per monitored mount before the
/// loop starts, push each interval's deltas via [`Self::record`], and
/// call [`Self::finalise`] once with the total capture duration to
/// produce the final report.
///
/// The aggregation matches `nfs-monitor`'s semantics so cutover
/// reports stay comparable:
///
/// - `rtt_avg_ms` is the ops-weighted mean across all samples, i.e.
///   `sum(delta_rtt) / sum(delta_ops)`. This is intentionally *not*
///   the same as averaging the per-interval averages, which would
///   give busy intervals and idle intervals equal weight.
/// - `rtt_min_ms` and `rtt_max_ms` are the min/max of the
///   *per-interval* average RTT (not of individual RPCs, which
///   mountstats does not expose). An interval contributes to these
///   bounds only if it had `delta_ops > 0`.
/// - Operations with `ops_total == 0` at the end of the session are
///   excluded from [`MountReport::operations`] but still contribute
///   (as zero) to [`SummaryStats`]. This mirrors nfs-monitor's
///   `buildReport` behaviour exactly.
pub struct MountAggregator {
    device: String,
    mount_point: String,
    fstype: String,
    options: String,
    ops: HashMap<String, OpAccumulator>,
}

#[derive(Default)]
struct OpAccumulator {
    ops_total: i64,
    retrans_total: i64,
    timeouts_total: i64,
    errors_total: i64,
    /// Raw sum of `delta_rtt` in ms across samples. Divided by
    /// `ops_total` at finalise to give the ops-weighted mean.
    rtt_sum_ms: i64,
    /// Minimum per-interval average RTT seen so far, in ms. `None`
    /// until the first non-empty interval is recorded.
    rtt_min_avg_ms: Option<f64>,
    /// Maximum per-interval average RTT seen so far, in ms.
    rtt_max_avg_ms: Option<f64>,
}

impl MountAggregator {
    /// Start a new aggregator for the given mount. `fstype` and
    /// `options` are accepted as caller-supplied strings rather than
    /// read from `mount` because [`NFSMount`] does not yet parse them
    /// from `/proc/self/mountstats`; pass empty strings if unknown.
    pub fn new(mount: &NFSMount, fstype: String, options: String) -> Self {
        Self {
            device: mount.device.clone(),
            mount_point: mount.mount_point.clone(),
            fstype,
            options,
            ops: HashMap::new(),
        }
    }

    /// Fold one interval's worth of deltas into the aggregator.
    ///
    /// Ops with `delta_ops == 0` are already filtered out upstream by
    /// `calculate_delta_stats`, but the aggregator tolerates them
    /// anyway: they are added to the zero-initialised accumulator
    /// without affecting the ops-weighted RTT or the per-interval
    /// min/max bounds.
    pub fn record(&mut self, sample: &[DeltaStats]) {
        for delta in sample {
            let acc = self.ops.entry(delta.operation.clone()).or_default();
            acc.ops_total += delta.delta_ops;
            acc.retrans_total += delta.delta_retrans;
            acc.timeouts_total += delta.delta_timeouts;
            acc.errors_total += delta.delta_errors;
            acc.rtt_sum_ms += delta.delta_rtt;

            if delta.delta_ops > 0 {
                let interval_avg = delta.avg_rtt;
                acc.rtt_min_avg_ms = Some(match acc.rtt_min_avg_ms {
                    Some(cur) if cur <= interval_avg => cur,
                    _ => interval_avg,
                });
                acc.rtt_max_avg_ms = Some(match acc.rtt_max_avg_ms {
                    Some(cur) if cur >= interval_avg => cur,
                    _ => interval_avg,
                });
            }
        }
    }

    /// Consume the aggregator and produce a [`MountReport`].
    ///
    /// `duration_sec` is the total capture length used to derive
    /// `ops_per_sec` fields. A zero duration is handled gracefully:
    /// all derived rates become zero rather than dividing by zero.
    pub fn finalise(self, duration_sec: u64) -> MountReport {
        let denom = duration_sec as f64;
        let mut total_ops: i64 = 0;
        let mut total_retrans: i64 = 0;
        let mut total_timeouts: i64 = 0;
        let mut total_errors: i64 = 0;

        let mut operations: Vec<OpReport> = self
            .ops
            .into_iter()
            .map(|(name, acc)| {
                total_ops += acc.ops_total;
                total_retrans += acc.retrans_total;
                total_timeouts += acc.timeouts_total;
                total_errors += acc.errors_total;

                let rtt_avg_ms = if acc.ops_total > 0 {
                    acc.rtt_sum_ms as f64 / acc.ops_total as f64
                } else {
                    0.0
                };
                let ops_per_sec = if denom > 0.0 {
                    acc.ops_total as f64 / denom
                } else {
                    0.0
                };
                OpReport {
                    name,
                    ops: acc.ops_total,
                    ops_per_sec,
                    retrans: acc.retrans_total,
                    timeouts: acc.timeouts_total,
                    errors: acc.errors_total,
                    rtt_avg_ms,
                    rtt_min_ms: acc.rtt_min_avg_ms.unwrap_or(0.0),
                    rtt_max_ms: acc.rtt_max_avg_ms.unwrap_or(0.0),
                }
            })
            .collect();

        // Drop silent ops from the per-op list but keep their
        // contribution (zero) in the summary totals computed above.
        operations.retain(|op| op.ops > 0);

        // Sort by ops descending; break ties lexicographically so the
        // output order is deterministic for snapshot tests.
        operations.sort_by(|a, b| b.ops.cmp(&a.ops).then_with(|| a.name.cmp(&b.name)));

        let summary = SummaryStats {
            total_ops,
            ops_per_sec: if denom > 0.0 {
                total_ops as f64 / denom
            } else {
                0.0
            },
            retrans: total_retrans,
            timeouts: total_timeouts,
            errors: total_errors,
        };

        MountReport {
            device: self.device,
            mount_point: self.mount_point,
            fstype: self.fstype,
            options: self.options,
            summary,
            operations,
        }
    }
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

    // --- MountAggregator tests ---

    fn test_mount() -> NFSMount {
        NFSMount {
            device: "server:/export".to_string(),
            mount_point: "/mnt/nfs".to_string(),
            server: "server".to_string(),
            export: "/export".to_string(),
            age: 0,
            operations: HashMap::new(),
            events: None,
            bytes_read: 0,
            bytes_write: 0,
        }
    }

    /// Build a `DeltaStats` with only the fields the aggregator reads,
    /// so test cases can focus on what they are actually asserting.
    fn delta(op: &str, ops: i64, delta_rtt_ms: i64, retrans: i64, timeouts: i64) -> DeltaStats {
        let avg_rtt = if ops > 0 {
            delta_rtt_ms as f64 / ops as f64
        } else {
            0.0
        };
        DeltaStats {
            operation: op.to_string(),
            delta_ops: ops,
            delta_bytes: 0,
            delta_sent: 0,
            delta_recv: 0,
            delta_rtt: delta_rtt_ms,
            delta_exec: 0,
            delta_queue: 0,
            delta_errors: 0,
            delta_retrans: retrans,
            delta_timeouts: timeouts,
            avg_rtt,
            avg_exec: 0.0,
            avg_queue: 0.0,
            kb_per_op: 0.0,
            kb_per_sec: 0.0,
            iops: 0.0,
        }
    }

    #[test]
    fn aggregator_sums_totals_across_samples() {
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount, "nfs4".into(), "rw,vers=4.1".into());

        // Three intervals: (100 ops, 100ms rtt), (200 ops, 240ms rtt), (50 ops, 30ms rtt)
        // Ops-weighted mean RTT = (100+240+30) / (100+200+50) = 370/350 ≈ 1.057ms
        agg.record(&[delta("READ", 100, 100, 0, 0)]);
        agg.record(&[delta("READ", 200, 240, 0, 0)]);
        agg.record(&[delta("READ", 50, 30, 0, 0)]);

        let report = agg.finalise(10);
        assert_eq!(report.summary.total_ops, 350);
        assert!((report.summary.ops_per_sec - 35.0).abs() < 1e-9);
        assert_eq!(report.operations.len(), 1);

        let op = &report.operations[0];
        assert_eq!(op.name, "READ");
        assert_eq!(op.ops, 350);
        assert!((op.ops_per_sec - 35.0).abs() < 1e-9);
        let expected_avg = 370.0 / 350.0;
        assert!(
            (op.rtt_avg_ms - expected_avg).abs() < 1e-9,
            "expected weighted mean {expected_avg}, got {}",
            op.rtt_avg_ms
        );
    }

    #[test]
    fn aggregator_tracks_per_interval_rtt_min_and_max() {
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount, String::new(), String::new());

        // Per-interval averages: 1.0, 5.0, 0.5 → min 0.5, max 5.0
        agg.record(&[delta("READ", 10, 10, 0, 0)]); // avg 1.0
        agg.record(&[delta("READ", 10, 50, 0, 0)]); // avg 5.0
        agg.record(&[delta("READ", 10, 5, 0, 0)]); // avg 0.5

        let report = agg.finalise(3);
        let op = &report.operations[0];
        assert!((op.rtt_min_ms - 0.5).abs() < 1e-9);
        assert!((op.rtt_max_ms - 5.0).abs() < 1e-9);
    }

    #[test]
    fn aggregator_excludes_silent_ops_from_operations_but_not_summary() {
        // An op that appeared in a sample with delta_ops == 0 (unusual
        // because calculate_delta_stats filters those out, but we
        // defend in depth). It should not appear in the operations
        // list but the summary totals should still be consistent.
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount, String::new(), String::new());

        agg.record(&[delta("READ", 100, 50, 0, 0), delta("GETATTR", 0, 0, 0, 0)]);

        let report = agg.finalise(10);
        assert_eq!(report.operations.len(), 1, "silent op should be dropped");
        assert_eq!(report.operations[0].name, "READ");
        assert_eq!(report.summary.total_ops, 100);
    }

    #[test]
    fn aggregator_handles_zero_duration_without_dividing_by_zero() {
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount, String::new(), String::new());
        agg.record(&[delta("READ", 50, 25, 0, 0)]);

        let report = agg.finalise(0);
        assert_eq!(report.summary.ops_per_sec, 0.0);
        assert_eq!(report.operations[0].ops_per_sec, 0.0);
        // The ops-weighted mean RTT is well-defined even at zero
        // duration because it divides by ops_total, not duration.
        assert!((report.operations[0].rtt_avg_ms - 0.5).abs() < 1e-9);
    }

    #[test]
    fn aggregator_retransmissions_come_from_delta_retrans_not_timeouts() {
        // Regression guard matching the stats.rs test of the same
        // name. The aggregator must not conflate the two counters
        // when building the report.
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount, String::new(), String::new());
        agg.record(&[delta("READ", 100, 50, 3, 7)]);

        let report = agg.finalise(1);
        assert_eq!(report.operations[0].retrans, 3);
        assert_eq!(report.operations[0].timeouts, 7);
        assert_eq!(report.summary.retrans, 3);
        assert_eq!(report.summary.timeouts, 7);
    }

    #[test]
    fn aggregator_sorts_operations_by_ops_descending() {
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount, String::new(), String::new());
        agg.record(&[
            delta("GETATTR", 500, 250, 0, 0),
            delta("READ", 2000, 1000, 0, 0),
            delta("WRITE", 800, 400, 0, 0),
        ]);

        let report = agg.finalise(10);
        let names: Vec<&str> = report.operations.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, vec!["READ", "WRITE", "GETATTR"]);
    }

    #[test]
    fn aggregator_empty_session_produces_empty_report() {
        let mount = test_mount();
        let agg = MountAggregator::new(&mount, "nfs4".into(), "rw".into());
        let report = agg.finalise(10);

        assert_eq!(report.summary.total_ops, 0);
        assert_eq!(report.summary.ops_per_sec, 0.0);
        assert!(report.operations.is_empty());
        assert_eq!(report.fstype, "nfs4");
        assert_eq!(report.options, "rw");
    }
}
