use crate::types::{DeltaStats, DeltaXprtStats, NFSEvents, NFSMount, NFSOperation, XprtStats};

fn safe_div(numerator: f64, denominator: f64) -> f64 {
    if denominator > 0.0 {
        numerator / denominator
    } else {
        0.0
    }
}

/// Calculate delta statistics between two measurements.
///
/// Operations whose monotonic counters have moved backwards since the
/// previous sample are treated as having been reset (typically due to
/// a remount or `umount`/`mount` cycle that re-initialised
/// `/proc/self/mountstats`) and dropped from this batch. The next
/// sample will compute a fresh delta against the post-reset values.
///
/// A mount `age` that moved backwards is the authoritative remount
/// signal and invalidates the *whole* sample: the per-op monotonic
/// checks cannot catch a remount whose fresh counters happen to
/// exceed the previous sample's (e.g. a long capture whose previous
/// sample was taken shortly after an earlier remount), which would
/// otherwise fabricate a delta spanning two different mounts.
pub fn calculate_delta_stats(
    previous: &NFSMount,
    current: &NFSMount,
    elapsed_seconds: f64,
) -> Vec<DeltaStats> {
    if current.age < previous.age {
        return Vec::new();
    }

    let mut deltas = Vec::new();

    for (op_name, current_op) in &current.operations {
        let delta = if let Some(previous_op) = previous.operations.get(op_name) {
            calculate_operation_delta(previous_op, current_op, elapsed_seconds)
        } else {
            // New operation seen for the first time — treat the previous
            // sample as all-zeros so the first delta reflects the entire
            // observed history.
            let zero_op = NFSOperation {
                name: op_name.clone(),
                ops: 0,
                ntrans: 0,
                timeouts: 0,
                bytes_sent: 0,
                bytes_recv: 0,
                queue_time: 0,
                rtt: 0,
                execute_time: 0,
                errors: 0,
            };
            calculate_operation_delta(&zero_op, current_op, elapsed_seconds)
        };

        if let Some(delta) = delta {
            if delta.delta_ops > 0 {
                deltas.push(delta);
            }
        }
    }

    // Sort by operation name for consistent output
    deltas.sort_by(|a, b| a.operation.cmp(&b.operation));
    deltas
}

/// Calculate delta statistics for a single operation.
///
/// Returns `None` if any underlying counter has moved backwards
/// relative to the previous sample, which indicates the kernel reset
/// the per-mount stats (remount, `umount`/`mount`, or container
/// restart). Callers should treat a `None` result as "skip this
/// sample" rather than "no activity".
fn calculate_operation_delta(
    previous: &NFSOperation,
    current: &NFSOperation,
    elapsed_seconds: f64,
) -> Option<DeltaStats> {
    // Detect counter reset: if any monotonic field went backwards we
    // cannot compute a meaningful delta, and feeding negative values
    // into Prometheus's `Counter::inc_by` would panic. Skip this
    // sample entirely; the next one will rebase against the
    // post-reset values.
    if current.ops < previous.ops
        || current.ntrans < previous.ntrans
        || current.bytes_sent < previous.bytes_sent
        || current.bytes_recv < previous.bytes_recv
        || current.rtt < previous.rtt
        || current.execute_time < previous.execute_time
        || current.queue_time < previous.queue_time
        || current.errors < previous.errors
        || current.timeouts < previous.timeouts
    {
        return None;
    }

    let delta_ops = current.ops - previous.ops;
    let delta_ntrans = current.ntrans - previous.ntrans;
    let delta_sent = current.bytes_sent - previous.bytes_sent;
    let delta_recv = current.bytes_recv - previous.bytes_recv;
    let delta_bytes = delta_sent + delta_recv;
    let delta_rtt = current.rtt - previous.rtt;
    let delta_exec = current.execute_time - previous.execute_time;
    let delta_queue = current.queue_time - previous.queue_time;
    let delta_errors = current.errors - previous.errors;
    let delta_timeouts = current.timeouts - previous.timeouts;
    // Retransmissions are transmissions beyond the initial one per op:
    // ntrans counts every RPC send (initial + retries), ops counts
    // unique completed operations. Clamp to zero defensively; in real
    // data delta_ntrans >= delta_ops always.
    let delta_retrans = (delta_ntrans - delta_ops).max(0);

    let ops_f = delta_ops as f64;
    let iops = safe_div(ops_f, elapsed_seconds);
    let avg_rtt = safe_div(delta_rtt as f64, ops_f);
    let avg_exec = safe_div(delta_exec as f64, ops_f);
    let avg_queue = safe_div(delta_queue as f64, ops_f);
    let kb_total = delta_bytes as f64 / 1024.0;
    let kb_per_op = safe_div(kb_total, ops_f);
    let kb_per_sec = safe_div(kb_total, elapsed_seconds);

    Some(DeltaStats {
        operation: current.name.clone(),
        delta_ops,
        delta_bytes,
        delta_sent,
        delta_recv,
        delta_rtt,
        delta_exec,
        delta_queue,
        delta_errors,
        delta_retrans,
        delta_timeouts,
        avg_rtt,
        avg_exec,
        avg_queue,
        kb_per_op,
        kb_per_sec,
        iops,
    })
}

/// Compute the per-interval xprt delta between two samples.
///
/// Returns `None` in three cases:
///
/// 1. Either side lacks an [`XprtStats`] — nothing to diff.
/// 2. The protocol tag changed between samples — almost certainly a
///    remount, safest to drop the interval rather than produce
///    nonsensical deltas across different transport layouts.
/// 3. Any cumulative counter moved backwards, indicating a kernel
///    counter reset. Matches the behaviour of
///    [`calculate_delta_stats`] for per-operation deltas so the two
///    pipelines stay in sync.
///
/// `max_slots` is a high-water mark rather than a cumulative counter
/// so it is carried forward rather than subtracted; in practice it
/// only moves upward over the lifetime of a mount. We still check
/// that it did not shrink as a defence against future kernel
/// bookkeeping changes.
pub fn calculate_xprt_delta(
    previous: Option<&XprtStats>,
    current: Option<&XprtStats>,
) -> Option<DeltaXprtStats> {
    let previous = previous?;
    let current = current?;

    if previous.protocol != current.protocol {
        return None;
    }

    // A change in connection count means the mount was reconfigured
    // (nconnect is fixed at mount time), so the two aggregates do not
    // describe the same set of transports. Drop the interval; the
    // next sample rebases cleanly.
    if previous.nconnect != current.nconnect {
        return None;
    }

    if current.sends < previous.sends
        || current.recvs < previous.recvs
        || current.bad_xids < previous.bad_xids
        || current.req_u < previous.req_u
        || current.bklog_u < previous.bklog_u
        || current.sending_u < previous.sending_u
        || current.pending_u < previous.pending_u
        || current.max_slots < previous.max_slots
    {
        return None;
    }

    let delta_sends = current.sends - previous.sends;
    let delta_recvs = current.recvs - previous.recvs;
    let delta_bad_xids = current.bad_xids - previous.bad_xids;
    let delta_req = current.req_u - previous.req_u;
    let delta_bklog = current.bklog_u - previous.bklog_u;
    let delta_sending = current.sending_u - previous.sending_u;
    let delta_pending = current.pending_u - previous.pending_u;

    let req_f = delta_req as f64;
    let bklog_per_req = safe_div(delta_bklog as f64, req_f);
    let sending_per_req = safe_div(delta_sending as f64, req_f);
    let pending_per_req = safe_div(delta_pending as f64, req_f);

    Some(DeltaXprtStats {
        protocol: current.protocol.clone(),
        delta_sends,
        delta_recvs,
        delta_bad_xids,
        delta_req,
        delta_bklog,
        delta_sending,
        delta_pending,
        max_slots: current.max_slots,
        nconnect: current.nconnect,
        bklog_per_req,
        sending_per_req,
        pending_per_req,
    })
}

/// Per-interval delta of the mount-level cumulative counters: the
/// wire-level byte totals from the `bytes:` line and the VFS event
/// counters from the `events:` line.
///
/// These exist for consumers that need *rates* (Prometheus counters,
/// bandwidth displays). Feeding the raw cumulative values to a
/// counter once per interval — the original bug — inflates it
/// quadratically: after N intervals the counter holds roughly
/// N x cumulative instead of the true total.
#[derive(Debug, Clone, PartialEq)]
pub struct MountDeltas {
    pub delta_bytes_read: i64,
    pub delta_bytes_write: i64,
    /// Per-interval VFS event deltas, present only when both samples
    /// carried an `events:` line.
    pub delta_events: Option<NFSEvents>,
}

/// Compute the mount-level deltas between two samples.
///
/// Returns `None` when the mount age moved backwards (remount — see
/// [`calculate_delta_stats`]) or when any counter moved backwards
/// (kernel reset the per-mount stats), matching the all-or-nothing
/// reset semantics of the per-operation path: a reset invalidates
/// the whole sample, not individual fields.
pub fn calculate_mount_deltas(previous: &NFSMount, current: &NFSMount) -> Option<MountDeltas> {
    if current.age < previous.age {
        return None;
    }

    if current.bytes_read < previous.bytes_read || current.bytes_write < previous.bytes_write {
        return None;
    }

    let delta_events = match (&previous.events, &current.events) {
        (Some(prev), Some(cur)) => match NFSEvents::delta(cur, prev) {
            Some(delta) => Some(delta),
            // Events went backwards while bytes did not: still a
            // reset — drop the whole sample.
            None => return None,
        },
        _ => None,
    };

    Some(MountDeltas {
        delta_bytes_read: current.bytes_read - previous.bytes_read,
        delta_bytes_write: current.bytes_write - previous.bytes_write,
        delta_events,
    })
}

/// Filter operations based on a set of allowed operation names
pub fn filter_operations(
    stats: Vec<DeltaStats>,
    filter: &std::collections::HashSet<String>,
) -> Vec<DeltaStats> {
    if filter.is_empty() {
        stats
    } else {
        stats
            .into_iter()
            .filter(|stat| filter.contains(&stat.operation))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{create_test_mount_with_operations, create_test_operation_with_stats};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_calculate_delta_stats() {
        let mut prev_ops = HashMap::new();
        prev_ops.insert(
            "READ".to_string(),
            create_test_operation_with_stats("READ", 100, 1024, 2048, 1000, 2000),
        );

        let mut curr_ops = HashMap::new();
        curr_ops.insert(
            "READ".to_string(),
            create_test_operation_with_stats("READ", 200, 2048, 4096, 2000, 4000),
        );

        let previous = create_test_mount_with_operations(prev_ops);
        let current = create_test_mount_with_operations(curr_ops);

        let deltas = calculate_delta_stats(&previous, &current, 1.0);

        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];
        assert_eq!(delta.operation, "READ");
        assert_eq!(delta.delta_ops, 100);
        assert_eq!(delta.delta_bytes, 1024 + 2048); // delta_sent + delta_recv
        assert_eq!(delta.iops, 100.0);
        assert_eq!(delta.avg_rtt, 10.0); // delta_rtt / delta_ops
        assert_eq!(delta.avg_exec, 20.0); // delta_exec / delta_ops
    }

    #[test]
    fn test_counter_reset_drops_sample() {
        // Simulate a remount: the "current" sample's counters are
        // smaller than the previous sample for the same operation.
        // calculate_delta_stats must drop the operation entirely
        // (not return a row with negative deltas, which would later
        // panic inside Prometheus's Counter::inc_by).
        let mut prev_ops = HashMap::new();
        prev_ops.insert(
            "READ".to_string(),
            create_test_operation_with_stats("READ", 10_000, 1_048_576, 2_097_152, 5_000, 8_000),
        );

        let mut curr_ops = HashMap::new();
        curr_ops.insert(
            "READ".to_string(),
            create_test_operation_with_stats("READ", 50, 4_096, 8_192, 10, 20),
        );

        let previous = create_test_mount_with_operations(prev_ops);
        let current = create_test_mount_with_operations(curr_ops);

        let deltas = calculate_delta_stats(&previous, &current, 1.0);

        assert!(
            deltas.is_empty(),
            "expected counter-reset sample to be dropped, got {:?}",
            deltas
        );
    }

    #[test]
    fn test_age_reset_drops_sample_even_when_counters_grew() {
        // The dangerous remount: the fresh mount's counters happen to
        // exceed the previous sample's, so per-op monotonic checks
        // see nothing wrong — but the delta would span two different
        // mounts. The age going backwards must invalidate the whole
        // sample.
        let mut prev_ops = HashMap::new();
        prev_ops.insert(
            "READ".to_string(),
            create_test_operation_with_stats("READ", 100, 1_024, 2_048, 50, 80),
        );
        let mut prev = create_test_mount_with_operations(prev_ops);
        prev.age = 500_000;

        let mut curr_ops = HashMap::new();
        curr_ops.insert(
            "READ".to_string(),
            create_test_operation_with_stats("READ", 100_000, 10_485_760, 20_971_520, 5_000, 8_000),
        );
        let mut curr = create_test_mount_with_operations(curr_ops);
        curr.age = 10;

        let deltas = calculate_delta_stats(&prev, &curr, 1.0);
        assert!(
            deltas.is_empty(),
            "an age reset must drop every op delta, got {:?}",
            deltas
        );
    }

    #[test]
    fn test_mount_deltas_none_on_age_reset_even_when_bytes_grew() {
        let mut prev = mount_with_bytes_and_events(1_000, 2_000, 100);
        prev.age = 500_000;
        let mut curr = mount_with_bytes_and_events(1_000_000, 2_000_000, 999);
        curr.age = 10;
        assert!(calculate_mount_deltas(&prev, &curr).is_none());
    }

    #[test]
    fn test_counter_reset_recovers_on_next_sample() {
        // After a reset, the very next sample (where counters are
        // monotonically growing again) must produce a normal delta.
        let mut reset_ops = HashMap::new();
        reset_ops.insert(
            "READ".to_string(),
            create_test_operation_with_stats("READ", 50, 4_096, 8_192, 10, 20),
        );

        let mut next_ops = HashMap::new();
        next_ops.insert(
            "READ".to_string(),
            create_test_operation_with_stats("READ", 150, 8_192, 16_384, 30, 60),
        );

        let previous = create_test_mount_with_operations(reset_ops);
        let current = create_test_mount_with_operations(next_ops);

        let deltas = calculate_delta_stats(&previous, &current, 1.0);

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].operation, "READ");
        assert_eq!(deltas[0].delta_ops, 100);
        assert_eq!(deltas[0].delta_bytes, 4_096 + 8_192);
    }

    #[test]
    fn test_calculate_delta_stats_new_operation() {
        let prev_ops = HashMap::new();

        let mut curr_ops = HashMap::new();
        curr_ops.insert(
            "READ".to_string(),
            create_test_operation_with_stats("READ", 100, 1024, 2048, 1000, 2000),
        );

        let previous = create_test_mount_with_operations(prev_ops);
        let current = create_test_mount_with_operations(curr_ops);

        let deltas = calculate_delta_stats(&previous, &current, 1.0);

        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];
        assert_eq!(delta.operation, "READ");
        assert_eq!(delta.delta_ops, 100);
        assert_eq!(delta.iops, 100.0);
    }

    #[test]
    fn test_filter_operations() {
        let stats = vec![
            DeltaStats {
                operation: "READ".to_string(),
                delta_ops: 100,
                delta_bytes: 0,
                delta_sent: 0,
                delta_recv: 0,
                delta_rtt: 0,
                delta_exec: 0,
                delta_queue: 0,
                delta_errors: 0,
                delta_retrans: 0,
                delta_timeouts: 0,
                avg_rtt: 0.0,
                avg_exec: 0.0,
                avg_queue: 0.0,
                kb_per_op: 0.0,
                kb_per_sec: 0.0,
                iops: 100.0,
            },
            DeltaStats {
                operation: "WRITE".to_string(),
                delta_ops: 50,
                delta_bytes: 0,
                delta_sent: 0,
                delta_recv: 0,
                delta_rtt: 0,
                delta_exec: 0,
                delta_queue: 0,
                delta_errors: 0,
                delta_retrans: 0,
                delta_timeouts: 0,
                avg_rtt: 0.0,
                avg_exec: 0.0,
                avg_queue: 0.0,
                kb_per_op: 0.0,
                kb_per_sec: 0.0,
                iops: 50.0,
            },
        ];

        // Test empty filter (should return all)
        let empty_filter = HashSet::new();
        let filtered = filter_operations(stats.clone(), &empty_filter);
        assert_eq!(filtered.len(), 2);

        // Test specific filter
        let mut filter = HashSet::new();
        filter.insert("READ".to_string());
        let filtered = filter_operations(stats, &filter);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].operation, "READ");
    }

    #[test]
    fn test_calculate_delta_stats_zero_ops() {
        // When delta_ops is 0, the operation should be filtered out
        // entirely (delta_ops == 0 means no activity this interval).
        let mut prev_ops = HashMap::new();
        prev_ops.insert(
            "READ".to_string(),
            create_test_operation_with_stats("READ", 100, 1024, 2048, 1000, 2000),
        );

        // Current ops unchanged — delta_ops = 0
        let curr_ops = prev_ops.clone();

        let previous = create_test_mount_with_operations(prev_ops);
        let current = create_test_mount_with_operations(curr_ops);

        let deltas = calculate_delta_stats(&previous, &current, 1.0);

        assert_eq!(deltas.len(), 0);
    }

    #[test]
    fn test_calculate_delta_stats_zero_elapsed() {
        // elapsed_seconds = 0 must not panic. Rates (iops, kb_per_sec)
        // should be 0.0 because safe_div rejects a zero denominator,
        // while per-op averages (avg_rtt) are still well-defined since
        // their denominator is delta_ops, not elapsed_seconds.
        let mut prev_ops = HashMap::new();
        prev_ops.insert(
            "READ".to_string(),
            create_test_operation_with_stats("READ", 100, 1024, 2048, 1000, 2000),
        );

        let mut curr_ops = HashMap::new();
        curr_ops.insert(
            "READ".to_string(),
            create_test_operation_with_stats("READ", 200, 2048, 4096, 2000, 4000),
        );

        let previous = create_test_mount_with_operations(prev_ops);
        let current = create_test_mount_with_operations(curr_ops);

        let deltas = calculate_delta_stats(&previous, &current, 0.0);

        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];
        assert_eq!(delta.iops, 0.0);
        assert_eq!(delta.kb_per_sec, 0.0);
        assert!(delta.avg_rtt > 0.0);
    }

    // --- calculate_mount_deltas tests ---

    fn mount_with_bytes_and_events(read: i64, write: i64, event_base: i64) -> NFSMount {
        let mut mount = create_test_mount_with_operations(HashMap::new());
        mount.bytes_read = read;
        mount.bytes_write = write;
        // Distinct value per field (base, base+1, ...) so index mixups
        // between fields are caught, not masked.
        mount.events = Some(NFSEvents {
            inode_revalidate: event_base,
            dentry_revalidate: event_base + 1,
            vfs_open: event_base + 4,
            vfs_fsync: event_base + 15,
            pnfs_write: event_base + 26,
            ..NFSEvents::default()
        });
        mount
    }

    #[test]
    fn test_mount_deltas_are_differences_not_cumulative() {
        let prev = mount_with_bytes_and_events(1_000, 2_000, 100);
        let curr = mount_with_bytes_and_events(1_500, 2_600, 130);

        let deltas = calculate_mount_deltas(&prev, &curr).expect("monotonic sample");
        assert_eq!(deltas.delta_bytes_read, 500);
        assert_eq!(deltas.delta_bytes_write, 600);

        let events = deltas.delta_events.expect("both samples had events");
        assert_eq!(events.inode_revalidate, 30);
        assert_eq!(events.dentry_revalidate, 30);
        assert_eq!(events.vfs_open, 30);
        assert_eq!(events.vfs_fsync, 30);
        assert_eq!(events.pnfs_write, 30);
    }

    #[test]
    fn test_mount_deltas_conservation_across_intervals() {
        // Nothing dropped, nothing double counted: the sum of interval
        // deltas over a monotonic counter walk must equal final minus
        // initial.
        let walk = [
            (0i64, 0i64),
            (100, 50),
            (100, 700),
            (250, 700),
            (900, 1_000),
        ];
        let mounts: Vec<NFSMount> = walk
            .iter()
            .map(|&(r, w)| mount_with_bytes_and_events(r, w, r))
            .collect();

        let mut sum_read = 0;
        let mut sum_write = 0;
        for pair in mounts.windows(2) {
            let d = calculate_mount_deltas(&pair[0], &pair[1]).expect("monotonic walk");
            sum_read += d.delta_bytes_read;
            sum_write += d.delta_bytes_write;
        }
        assert_eq!(sum_read, 900, "sum of deltas == final - initial");
        assert_eq!(sum_write, 1_000);
    }

    #[test]
    fn test_mount_deltas_none_on_bytes_reset() {
        let prev = mount_with_bytes_and_events(1_000, 2_000, 100);
        let curr = mount_with_bytes_and_events(10, 2_600, 130);
        assert!(calculate_mount_deltas(&prev, &curr).is_none());
    }

    #[test]
    fn test_mount_deltas_none_on_events_reset_even_if_bytes_grew() {
        let prev = mount_with_bytes_and_events(1_000, 2_000, 100);
        let curr = mount_with_bytes_and_events(1_500, 2_600, 5);
        assert!(
            calculate_mount_deltas(&prev, &curr).is_none(),
            "a reset invalidates the whole sample, not just the events"
        );
    }

    #[test]
    fn test_mount_deltas_without_events_still_deltas_bytes() {
        let mut prev = mount_with_bytes_and_events(1_000, 2_000, 100);
        prev.events = None;
        let curr = mount_with_bytes_and_events(1_500, 2_600, 130);

        let deltas = calculate_mount_deltas(&prev, &curr).expect("bytes are monotonic");
        assert_eq!(deltas.delta_bytes_read, 500);
        assert!(
            deltas.delta_events.is_none(),
            "events delta needs both samples"
        );
    }

    #[test]
    fn test_events_delta_roundtrip_covers_every_field() {
        // Feed a strictly increasing value into every one of the 27
        // fields and check the delta reproduces each field exactly —
        // guards the to_array/from_array mapping against transposition.
        let prev_parts: Vec<String> = (0..27).map(|i| (i * 10).to_string()).collect();
        let curr_parts: Vec<String> = (0..27).map(|i| (i * 10 + i + 1).to_string()).collect();
        let prev = crate::parser::parse_events(&prev_parts).unwrap();
        let curr = crate::parser::parse_events(&curr_parts).unwrap();

        let delta = NFSEvents::delta(&curr, &prev).expect("monotonic events");
        assert_eq!(delta.inode_revalidate, 1);
        assert_eq!(delta.dentry_revalidate, 2);
        assert_eq!(delta.data_invalidate, 3);
        assert_eq!(delta.attr_invalidate, 4);
        assert_eq!(delta.vfs_open, 5);
        assert_eq!(delta.vfs_lookup, 6);
        assert_eq!(delta.vfs_access, 7);
        assert_eq!(delta.vfs_update_page, 8);
        assert_eq!(delta.vfs_read_page, 9);
        assert_eq!(delta.vfs_read_pages, 10);
        assert_eq!(delta.vfs_write_page, 11);
        assert_eq!(delta.vfs_write_pages, 12);
        assert_eq!(delta.vfs_getdents, 13);
        assert_eq!(delta.vfs_setattr, 14);
        assert_eq!(delta.vfs_flush, 15);
        assert_eq!(delta.vfs_fsync, 16);
        assert_eq!(delta.vfs_lock, 17);
        assert_eq!(delta.vfs_release, 18);
        assert_eq!(delta.congestion_wait, 19);
        assert_eq!(delta.setattr_trunc, 20);
        assert_eq!(delta.extend_write, 21);
        assert_eq!(delta.silly_rename, 22);
        assert_eq!(delta.short_read, 23);
        assert_eq!(delta.short_write, 24);
        assert_eq!(delta.delay, 25);
        assert_eq!(delta.pnfs_read, 26);
        assert_eq!(delta.pnfs_write, 27);
    }

    // --- calculate_xprt_delta tests ---

    fn make_xprt(
        sends: i64,
        recvs: i64,
        req_u: i64,
        bklog_u: i64,
        sending_u: i64,
        pending_u: i64,
        max_slots: i64,
    ) -> XprtStats {
        XprtStats {
            protocol: "tcp".to_string(),
            sends,
            recvs,
            bad_xids: 0,
            req_u,
            bklog_u,
            max_slots,
            sending_u,
            pending_u,
            nconnect: 1,
        }
    }

    #[test]
    fn test_xprt_delta_computes_per_request_averages() {
        // Between the two samples: 1000 new requests, 200 units of
        // cumulative backlog, 500 units of sending, 300 of pending.
        // Per-request averages should be 0.2 / 0.5 / 0.3.
        let prev = make_xprt(10_000, 9_998, 10_000, 100, 50_000, 200_000, 32);
        let curr = make_xprt(11_000, 10_998, 11_000, 300, 50_500, 200_300, 48);

        let delta =
            calculate_xprt_delta(Some(&prev), Some(&curr)).expect("delta should be computed");

        assert_eq!(delta.delta_sends, 1000);
        assert_eq!(delta.delta_recvs, 1000);
        assert_eq!(delta.delta_req, 1000);
        assert_eq!(delta.delta_bklog, 200);
        assert_eq!(delta.delta_sending, 500);
        assert_eq!(delta.delta_pending, 300);
        assert_eq!(delta.max_slots, 48); // high-water mark carried forward
        assert!((delta.bklog_per_req - 0.2).abs() < 1e-9);
        assert!((delta.sending_per_req - 0.5).abs() < 1e-9);
        assert!((delta.pending_per_req - 0.3).abs() < 1e-9);
    }

    #[test]
    fn test_xprt_delta_returns_none_when_either_side_missing() {
        let xprt = make_xprt(100, 100, 100, 0, 0, 0, 4);
        assert!(calculate_xprt_delta(None, Some(&xprt)).is_none());
        assert!(calculate_xprt_delta(Some(&xprt), None).is_none());
        assert!(calculate_xprt_delta(None, None).is_none());
    }

    #[test]
    fn test_xprt_delta_returns_none_on_counter_reset() {
        // sends went backwards — almost certainly a remount. Must
        // drop the sample so we do not produce a nonsense negative
        // delta that would later panic inside Prometheus.
        let prev = make_xprt(10_000, 9_998, 10_000, 100, 50_000, 200_000, 32);
        let curr = make_xprt(500, 500, 500, 0, 1_000, 2_000, 4);
        assert!(calculate_xprt_delta(Some(&prev), Some(&curr)).is_none());
    }

    #[test]
    fn test_xprt_delta_returns_none_on_protocol_change() {
        // Transport swap between samples implies the mount was
        // reconfigured; dropping the sample is safer than producing
        // a struct labelled with the new protocol but whose
        // numbers mix the two.
        let prev = XprtStats {
            protocol: "tcp".to_string(),
            ..make_xprt(10_000, 9_998, 10_000, 100, 50_000, 200_000, 32)
        };
        let curr = XprtStats {
            protocol: "rdma".to_string(),
            ..make_xprt(11_000, 10_998, 11_000, 100, 50_000, 200_000, 32)
        };
        assert!(calculate_xprt_delta(Some(&prev), Some(&curr)).is_none());
    }

    #[test]
    fn test_xprt_delta_returns_none_on_nconnect_change() {
        // nconnect is fixed at mount time, so a change means the two
        // aggregates describe different transport sets (remount).
        // Even with counters that look monotonic, the interval must
        // be dropped.
        let prev = make_xprt(10_000, 9_998, 10_000, 100, 50_000, 200_000, 32);
        let curr = XprtStats {
            nconnect: 16,
            ..make_xprt(11_000, 10_998, 11_000, 300, 50_500, 200_300, 48)
        };
        assert!(calculate_xprt_delta(Some(&prev), Some(&curr)).is_none());
    }

    #[test]
    fn test_xprt_delta_carries_nconnect_forward() {
        let prev = XprtStats {
            nconnect: 16,
            ..make_xprt(10_000, 9_998, 10_000, 100, 50_000, 200_000, 32)
        };
        let curr = XprtStats {
            nconnect: 16,
            ..make_xprt(11_000, 10_998, 11_000, 300, 50_500, 200_300, 48)
        };
        let delta = calculate_xprt_delta(Some(&prev), Some(&curr)).expect("delta computed");
        assert_eq!(delta.nconnect, 16);
        assert_eq!(delta.delta_sends, 1000);
    }

    #[test]
    fn test_xprt_delta_handles_zero_request_delta_without_nan() {
        // Zero requests between samples — per-request averages must
        // be 0.0 (not NaN from divide-by-zero), because downstream
        // display code formats them as floats and NaN would corrupt
        // the output.
        let prev = make_xprt(1000, 1000, 1000, 42, 500, 800, 16);
        let curr = prev.clone();
        let delta = calculate_xprt_delta(Some(&prev), Some(&curr))
            .expect("equal samples should still produce an all-zeros delta");
        assert_eq!(delta.delta_req, 0);
        assert_eq!(delta.bklog_per_req, 0.0);
        assert_eq!(delta.sending_per_req, 0.0);
        assert_eq!(delta.pending_per_req, 0.0);
        assert!(
            !delta.bklog_per_req.is_nan(),
            "per-req averages must never be NaN"
        );
    }

    #[test]
    fn test_delta_retrans_is_ntrans_minus_ops_not_timeouts() {
        // Retransmissions must be derived from ntrans minus ops, not
        // from timeouts. ntrans counts every RPC send (initial plus
        // retries); ops counts unique completed operations; timeouts
        // is an independent counter. Conflating retrans with timeouts
        // was the original bug.
        let previous = NFSOperation {
            name: "READ".to_string(),
            ops: 50,
            ntrans: 52,
            timeouts: 1,
            bytes_sent: 0,
            bytes_recv: 0,
            queue_time: 0,
            rtt: 0,
            execute_time: 0,
            errors: 0,
        };
        let current = NFSOperation {
            name: "READ".to_string(),
            ops: 100,
            ntrans: 105,
            timeouts: 2,
            bytes_sent: 0,
            bytes_recv: 0,
            queue_time: 0,
            rtt: 0,
            execute_time: 0,
            errors: 0,
        };

        let mut prev_ops = HashMap::new();
        prev_ops.insert("READ".to_string(), previous);
        let mut curr_ops = HashMap::new();
        curr_ops.insert("READ".to_string(), current);

        let previous_mount = create_test_mount_with_operations(prev_ops);
        let current_mount = create_test_mount_with_operations(curr_ops);

        let deltas = calculate_delta_stats(&previous_mount, &current_mount, 1.0);
        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];

        // delta_ntrans (53) - delta_ops (50) = 3 retransmissions.
        assert_eq!(
            delta.delta_retrans, 3,
            "retrans should be delta_ntrans - delta_ops"
        );
        // delta_timeouts is reported separately and must not be
        // conflated with retransmissions.
        assert_eq!(delta.delta_timeouts, 1);
    }
}
