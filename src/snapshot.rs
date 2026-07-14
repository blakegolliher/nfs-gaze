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

use crate::types::{DeltaStats, DeltaXprtStats, NFSMount};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema version written into new [`Report`] files. Consumers should
/// accept any value they recognise and reject (or warn on) newer
/// values they do not understand.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Serde default for [`XprtReport::nconnect`]: reports written before
/// the field existed were produced by a parser that kept exactly one
/// connection's counters, so 1 is the accurate backfill.
fn default_nconnect() -> i64 {
    1
}

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
    /// Number of non-seed intervals whose deltas were folded into at
    /// least one mount's aggregation. Intervals dropped whole — a
    /// counter reset on the only monitored mount, the `-m` target
    /// being absent, a parse failure — do not count, so this may be
    /// less than `duration_sec / interval_sec`. With several mounts
    /// this is a session-level count, not per-mount; per-mount
    /// coverage is [`MountReport::covered_sec`].
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
    /// Seconds of wall-clock actually covered by recorded samples —
    /// the sum of each interval's measured elapsed time, including
    /// op-idle intervals. This is the denominator behind every
    /// `ops_per_sec` in the report; it is smaller than the requested
    /// duration (the seed sample covers none of it) and reflects
    /// scheduling jitter. Zero when read from reports written before
    /// the field existed.
    #[serde(default)]
    pub covered_sec: f64,
    /// Mount-level totals across all operations.
    pub summary: SummaryStats,
    /// Per-operation aggregated stats, sorted by `ops` descending so
    /// the hottest operations appear first when the report is printed.
    pub operations: Vec<OpReport>,
    /// Aggregated RPC transport stats, if the mount had an `xprt:`
    /// line in its mountstats. Emitted only when present so old
    /// snapshot files written before xprt support deserialise
    /// unchanged, and so snapshots of UDP/RDMA mounts (which the
    /// parser currently leaves as None) do not pad the report with
    /// an empty placeholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xprt: Option<XprtReport>,
}

/// Session-level summary of RPC transport statistics for a mount.
///
/// All counter fields are the sum of the per-interval deltas observed
/// across the capture, so they represent "work done during this
/// session" rather than the kernel's lifetime totals. The per-request
/// averages are session-weighted: `sum(Δ_x) / sum(Δreq)`, not the
/// arithmetic mean of the per-interval ratios, so busy and idle
/// intervals are weighted fairly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XprtReport {
    pub protocol: String,
    /// Number of transport connections behind the aggregated numbers
    /// (`nconnect=N` mounts have N). Defaults to 1 when reading
    /// reports written before this field existed — those aggregates
    /// genuinely described a single connection.
    #[serde(default = "default_nconnect")]
    pub nconnect: i64,
    /// High-water mark of slots actually used, across the session.
    /// On multi-connection mounts this is the per-connection maximum
    /// (the slot cap applies per transport). Carried forward from
    /// the kernel gauge rather than derived from deltas (it is
    /// monotonically non-decreasing by design).
    pub max_slots: i64,
    /// Configured cap from `/proc/sys/sunrpc/tcp_max_slot_table_entries`
    /// at capture time, when readable. Omitted from the JSON
    /// entirely when unknown so a consumer can tell "capped at N"
    /// from "cap unknown" without a sentinel value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_cap: Option<i64>,
    pub sends: i64,
    pub recvs: i64,
    pub bad_xids: i64,
    pub bklog_per_req: f64,
    pub sending_per_req: f64,
    pub pending_per_req: f64,
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
/// loop starts, push each interval's deltas (with the interval's
/// measured elapsed time) via [`Self::record`], and call
/// [`Self::finalise`] once to produce the final report.
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
    /// Wall-clock seconds covered by recorded samples. See
    /// [`MountReport::covered_sec`].
    covered_seconds: f64,
    ops: HashMap<String, OpAccumulator>,
    /// Lazily populated on the first `record_xprt` call. Stays
    /// `None` for mounts whose xprt deltas never arrived (UDP/RDMA
    /// or counter reset), so `finalise` can emit `xprt: None` in
    /// the report rather than a misleading zeroed block.
    xprt: Option<XprtAccumulator>,
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

/// Streaming accumulator for [`DeltaXprtStats`] samples.
///
/// Tracks session totals for the cumulative counters and the
/// running high-water mark for `max_slots`. Per-request averages
/// are derived at finalise time from the total sums, giving a
/// session-weighted rather than interval-averaged figure.
struct XprtAccumulator {
    protocol: String,
    sends_total: i64,
    recvs_total: i64,
    bad_xids_total: i64,
    req_total: i64,
    bklog_total: i64,
    sending_total: i64,
    pending_total: i64,
    max_slots_hwm: i64,
    /// Highest connection count seen across the session. Constant in
    /// practice (nconnect is fixed at mount time); max() is a cheap
    /// defence if a remount mid-session changes it.
    nconnect_max: i64,
}

impl MountAggregator {
    /// Start a new aggregator for the given mount. Identity fields —
    /// including fstype and mount options — are carried over from the
    /// parsed mount so the finalised report describes the mount as
    /// the kernel reported it at session start.
    pub fn new(mount: &NFSMount) -> Self {
        Self {
            device: mount.device.clone(),
            mount_point: mount.mount_point.clone(),
            fstype: mount.fstype.clone(),
            options: mount.options.clone(),
            covered_seconds: 0.0,
            ops: HashMap::new(),
            xprt: None,
        }
    }

    /// Fold one interval's xprt delta into the session totals.
    ///
    /// On first call the inner accumulator is lazily created from
    /// the protocol tag of the delta. Subsequent calls must use the
    /// same protocol — a mismatch is treated as a dropped sample
    /// because mixing protocols within a session would produce
    /// numbers that do not describe any real transport. This
    /// shouldn't happen in practice (the delta function already
    /// drops protocol-change samples) but the defence is cheap.
    pub fn record_xprt(&mut self, delta: &DeltaXprtStats) {
        let acc = self.xprt.get_or_insert_with(|| XprtAccumulator {
            protocol: delta.protocol.clone(),
            sends_total: 0,
            recvs_total: 0,
            bad_xids_total: 0,
            req_total: 0,
            bklog_total: 0,
            sending_total: 0,
            pending_total: 0,
            max_slots_hwm: 0,
            nconnect_max: 0,
        });
        if acc.protocol != delta.protocol {
            return;
        }
        acc.sends_total += delta.delta_sends;
        acc.recvs_total += delta.delta_recvs;
        acc.bad_xids_total += delta.delta_bad_xids;
        acc.req_total += delta.delta_req;
        acc.bklog_total += delta.delta_bklog;
        acc.sending_total += delta.delta_sending;
        acc.pending_total += delta.delta_pending;
        if delta.max_slots > acc.max_slots_hwm {
            acc.max_slots_hwm = delta.max_slots;
        }
        if delta.nconnect > acc.nconnect_max {
            acc.nconnect_max = delta.nconnect;
        }
    }

    /// Fold one interval's worth of deltas into the aggregator.
    ///
    /// `elapsed_seconds` is the interval's *measured* wall-clock
    /// length; it accumulates into the covered time that later
    /// serves as the rate denominator. Callers must record every
    /// interval — including ones with an empty `sample` — so idle
    /// time counts: dividing by requested duration (the old
    /// behaviour) understated every rate by interval/duration, and
    /// dividing by busy-time-only would overstate rates on idle
    /// mounts.
    ///
    /// Ops with `delta_ops == 0` are already filtered out upstream by
    /// `calculate_delta_stats`, but the aggregator tolerates them
    /// anyway: they are added to the zero-initialised accumulator
    /// without affecting the ops-weighted RTT or the per-interval
    /// min/max bounds.
    pub fn record(&mut self, sample: &[DeltaStats], elapsed_seconds: f64) {
        if elapsed_seconds > 0.0 {
            self.covered_seconds += elapsed_seconds;
        }
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
    /// Rates are derived over the accumulated covered time (the sum
    /// of measured interval lengths), not over a caller-supplied
    /// duration: a `-d 60 -i 10` capture covers roughly 50 seconds
    /// of samples, and dividing by 60 would understate every rate
    /// by interval/duration. Zero covered time (no samples) yields
    /// zero rates rather than dividing by zero. `slot_cap` is
    /// stamped into the emitted [`XprtReport`] (if any); it is
    /// accepted here rather than held on the aggregator because it
    /// is a session-constant external value.
    pub fn finalise(self, slot_cap: Option<i64>) -> MountReport {
        let denom = self.covered_seconds;
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

        // Fold the xprt accumulator, if any, into its final form.
        // Session-weighted per-request averages divide the summed
        // deltas by the summed request count, matching the
        // interval-level semantics of calculate_xprt_delta but
        // across the whole session.
        let xprt = self.xprt.map(|acc| {
            let req_f = acc.req_total as f64;
            let safe = |n: i64| if req_f > 0.0 { n as f64 / req_f } else { 0.0 };
            XprtReport {
                protocol: acc.protocol,
                nconnect: acc.nconnect_max.max(1),
                max_slots: acc.max_slots_hwm,
                slot_cap,
                sends: acc.sends_total,
                recvs: acc.recvs_total,
                bad_xids: acc.bad_xids_total,
                bklog_per_req: safe(acc.bklog_total),
                sending_per_req: safe(acc.sending_total),
                pending_per_req: safe(acc.pending_total),
            }
        });

        MountReport {
            device: self.device,
            mount_point: self.mount_point,
            fstype: self.fstype,
            options: self.options,
            covered_sec: self.covered_seconds,
            summary,
            operations,
            xprt,
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
            covered_sec: 0.0,
            summary: SummaryStats {
                total_ops: 0,
                ops_per_sec: 0.0,
                retrans: 0,
                timeouts: 0,
                errors: 0,
            },
            operations: Vec::new(),
            xprt: None,
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
            fstype: "nfs4".to_string(),
            options: "rw,vers=4.1".to_string(),
            age: 0,
            operations: HashMap::new(),
            events: None,
            bytes_read: 0,
            bytes_write: 0,
            xprt: None,
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
        let mut agg = MountAggregator::new(&mount);

        // Three intervals: (100 ops, 100ms rtt), (200 ops, 240ms rtt), (50 ops, 30ms rtt)
        // Ops-weighted mean RTT = (100+240+30) / (100+200+50) = 370/350 ≈ 1.057ms
        agg.record(&[delta("READ", 100, 100, 0, 0)], 4.0);
        agg.record(&[delta("READ", 200, 240, 0, 0)], 3.0);
        agg.record(&[delta("READ", 50, 30, 0, 0)], 3.0);

        let report = agg.finalise(None);
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
        let mut agg = MountAggregator::new(&mount);

        // Per-interval averages: 1.0, 5.0, 0.5 → min 0.5, max 5.0
        agg.record(&[delta("READ", 10, 10, 0, 0)], 1.0); // avg 1.0
        agg.record(&[delta("READ", 10, 50, 0, 0)], 1.0); // avg 5.0
        agg.record(&[delta("READ", 10, 5, 0, 0)], 1.0); // avg 0.5

        let report = agg.finalise(None);
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
        let mut agg = MountAggregator::new(&mount);

        agg.record(
            &[delta("READ", 100, 50, 0, 0), delta("GETATTR", 0, 0, 0, 0)],
            10.0,
        );

        let report = agg.finalise(None);
        assert_eq!(report.operations.len(), 1, "silent op should be dropped");
        assert_eq!(report.operations[0].name, "READ");
        assert_eq!(report.summary.total_ops, 100);
    }

    #[test]
    fn aggregator_covered_time_includes_idle_intervals() {
        // A mount busy for one second and idle for nine must report
        // rates over all ten seconds. Idle intervals carry an empty
        // sample but real elapsed time.
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount);
        agg.record(&[delta("READ", 100, 50, 0, 0)], 1.0);
        for _ in 0..9 {
            agg.record(&[], 1.0);
        }

        let report = agg.finalise(None);
        assert!((report.covered_sec - 10.0).abs() < 1e-9);
        assert!(
            (report.summary.ops_per_sec - 10.0).abs() < 1e-9,
            "100 ops over 10 covered seconds must be 10 ops/s, got {}",
            report.summary.ops_per_sec
        );
        assert!((report.operations[0].ops_per_sec - 10.0).abs() < 1e-9);
    }

    #[test]
    fn aggregator_rates_use_measured_elapsed_not_nominal_interval() {
        // Intervals rarely last exactly the nominal -i value; the
        // denominator must be the sum of what was measured.
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount);
        agg.record(&[delta("READ", 100, 50, 0, 0)], 1.05);
        agg.record(&[delta("READ", 100, 50, 0, 0)], 1.10);
        agg.record(&[delta("READ", 100, 50, 0, 0)], 0.85);

        let report = agg.finalise(None);
        let expected = 300.0 / 3.0;
        assert!((report.covered_sec - 3.0).abs() < 1e-9);
        assert!(
            (report.summary.ops_per_sec - expected).abs() < 1e-9,
            "expected {expected} ops/s over measured time, got {}",
            report.summary.ops_per_sec
        );
    }

    #[test]
    fn covered_sec_defaults_to_zero_for_old_reports_and_round_trips() {
        // Reports written before covered_sec existed (e.g. the
        // nfs-monitor fixture) must deserialise with 0.0; new
        // reports must carry the field through JSON.
        let old: Report = serde_json::from_str(NFS_MONITOR_FIXTURE).expect("fixture deserialise");
        assert_eq!(old.mounts[0].covered_sec, 0.0);

        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount);
        agg.record(&[delta("READ", 10, 5, 0, 0)], 2.5);
        let report = agg.finalise(None);
        let json = serde_json::to_string(&report).expect("serialise");
        assert!(
            json.contains("\"covered_sec\":2.5"),
            "covered_sec must be serialised: {json}"
        );
    }

    #[test]
    fn aggregator_handles_zero_covered_time_without_dividing_by_zero() {
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount);
        agg.record(&[delta("READ", 50, 25, 0, 0)], 0.0);

        let report = agg.finalise(None);
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
        let mut agg = MountAggregator::new(&mount);
        agg.record(&[delta("READ", 100, 50, 3, 7)], 1.0);

        let report = agg.finalise(None);
        assert_eq!(report.operations[0].retrans, 3);
        assert_eq!(report.operations[0].timeouts, 7);
        assert_eq!(report.summary.retrans, 3);
        assert_eq!(report.summary.timeouts, 7);
    }

    #[test]
    fn aggregator_sorts_operations_by_ops_descending() {
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount);
        agg.record(
            &[
                delta("GETATTR", 500, 250, 0, 0),
                delta("READ", 2000, 1000, 0, 0),
                delta("WRITE", 800, 400, 0, 0),
            ],
            10.0,
        );

        let report = agg.finalise(None);
        let names: Vec<&str> = report.operations.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, vec!["READ", "WRITE", "GETATTR"]);
    }

    fn xprt_delta(
        delta_sends: i64,
        delta_req: i64,
        delta_bklog: i64,
        delta_sending: i64,
        delta_pending: i64,
        max_slots: i64,
    ) -> DeltaXprtStats {
        DeltaXprtStats {
            protocol: "tcp".to_string(),
            delta_sends,
            delta_recvs: delta_sends,
            delta_bad_xids: 0,
            delta_req,
            delta_bklog,
            delta_sending,
            delta_pending,
            max_slots,
            nconnect: 1,
            bklog_per_req: if delta_req > 0 {
                delta_bklog as f64 / delta_req as f64
            } else {
                0.0
            },
            sending_per_req: if delta_req > 0 {
                delta_sending as f64 / delta_req as f64
            } else {
                0.0
            },
            pending_per_req: if delta_req > 0 {
                delta_pending as f64 / delta_req as f64
            } else {
                0.0
            },
        }
    }

    #[test]
    fn aggregator_folds_xprt_samples_with_session_weighting() {
        // Two intervals. First: 1000 req, bklog 0, sending 500.
        // Second: 500 req, bklog 100, sending 500. Session-weighted
        // bklog/req = 100 / 1500 = 0.0666..., sending/req = 1000/1500 = 0.6666...
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount);
        agg.record_xprt(&xprt_delta(1000, 1000, 0, 500, 100, 16));
        agg.record_xprt(&xprt_delta(500, 500, 100, 500, 200, 32));

        let report = agg.finalise(Some(65536));
        let xprt = report.xprt.as_ref().expect("xprt report should be present");

        assert_eq!(xprt.protocol, "tcp");
        assert_eq!(xprt.max_slots, 32, "high-water mark should carry forward");
        assert_eq!(xprt.slot_cap, Some(65536));
        assert_eq!(xprt.sends, 1500);
        assert_eq!(xprt.recvs, 1500);
        // Session-weighted: 100 / 1500 ≈ 0.0667
        assert!(
            (xprt.bklog_per_req - (100.0 / 1500.0)).abs() < 1e-9,
            "bklog_per_req should be session-weighted, got {}",
            xprt.bklog_per_req
        );
        assert!((xprt.sending_per_req - (1000.0 / 1500.0)).abs() < 1e-9);
        assert!((xprt.pending_per_req - (300.0 / 1500.0)).abs() < 1e-9);
    }

    #[test]
    fn aggregator_without_xprt_emits_none_not_empty_block() {
        // No record_xprt calls → the finalised report should have
        // xprt: None, which serde drops from the JSON entirely via
        // the `skip_serializing_if` attribute. A zeroed XprtReport
        // would be misleading (would look like "no slot pressure")
        // so the distinction is load-bearing.
        let mount = test_mount();
        let agg = MountAggregator::new(&mount);
        let report = agg.finalise(Some(65536));
        assert!(report.xprt.is_none());

        let json = serde_json::to_string(&report).expect("serialise");
        assert!(
            !json.contains("\"xprt\""),
            "xprt field should be skipped when None, got: {json}"
        );
    }

    #[test]
    fn aggregator_ignores_xprt_samples_with_mismatched_protocol() {
        // A protocol change mid-session is already rejected upstream
        // by calculate_xprt_delta, but the aggregator defends in
        // depth: if the caller forces through a mismatched delta,
        // it drops the sample rather than polluting the accumulator
        // with values from a different transport.
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount);
        agg.record_xprt(&xprt_delta(1000, 1000, 0, 500, 100, 16));
        // Second delta with a different protocol tag:
        let mut bogus = xprt_delta(9999, 9999, 9999, 9999, 9999, 9999);
        bogus.protocol = "rdma".to_string();
        agg.record_xprt(&bogus);

        let report = agg.finalise(None);
        let xprt = report
            .xprt
            .as_ref()
            .expect("first sample seeds the accumulator");
        assert_eq!(xprt.protocol, "tcp");
        // Only the first sample's numbers should have landed.
        assert_eq!(xprt.sends, 1000);
        assert_eq!(xprt.max_slots, 16);
    }

    #[test]
    fn xprt_report_round_trips_through_json_with_slot_cap_skipped_when_none() {
        // slot_cap is Option and should disappear from the JSON
        // when None — consumers tell "capped at N" from "cap
        // unknown" by presence/absence of the field.
        let report = XprtReport {
            protocol: "tcp".into(),
            nconnect: 1,
            max_slots: 42,
            slot_cap: None,
            sends: 100,
            recvs: 100,
            bad_xids: 0,
            bklog_per_req: 0.0,
            sending_per_req: 0.0,
            pending_per_req: 0.0,
        };
        let json = serde_json::to_string(&report).expect("serialise");
        assert!(
            !json.contains("slot_cap"),
            "slot_cap should be omitted when None: {json}"
        );
        let parsed: XprtReport = serde_json::from_str(&json).expect("round-trip");
        assert_eq!(parsed, report);
    }

    #[test]
    fn xprt_report_nconnect_defaults_to_one_for_old_json() {
        // Reports written before the nconnect field existed must
        // deserialise with nconnect = 1: the old parser kept exactly
        // one connection's counters, so 1 describes that data.
        let json = r#"{
            "protocol": "tcp",
            "max_slots": 42,
            "sends": 100,
            "recvs": 100,
            "bad_xids": 0,
            "bklog_per_req": 0.0,
            "sending_per_req": 0.0,
            "pending_per_req": 0.0
        }"#;
        let parsed: XprtReport = serde_json::from_str(json).expect("old JSON must deserialise");
        assert_eq!(parsed.nconnect, 1);
    }

    #[test]
    fn aggregator_carries_nconnect_into_report() {
        let mount = test_mount();
        let mut agg = MountAggregator::new(&mount);
        let mut delta = xprt_delta(1000, 1000, 0, 500, 100, 16);
        delta.nconnect = 16;
        agg.record_xprt(&delta);

        let report = agg.finalise(None);
        assert_eq!(report.xprt.as_ref().unwrap().nconnect, 16);
    }

    #[test]
    fn aggregator_empty_session_produces_empty_report() {
        let mount = test_mount();
        let agg = MountAggregator::new(&mount);
        let report = agg.finalise(None);

        assert_eq!(report.summary.total_ops, 0);
        assert_eq!(report.summary.ops_per_sec, 0.0);
        assert!(report.operations.is_empty());
        assert_eq!(report.fstype, "nfs4");
        assert_eq!(report.options, "rw,vers=4.1");
    }
}
