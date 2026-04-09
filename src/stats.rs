use crate::types::{DeltaStats, NFSMount, NFSOperation};

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
pub fn calculate_delta_stats(
    previous: &NFSMount,
    current: &NFSMount,
    elapsed_seconds: f64,
) -> Vec<DeltaStats> {
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
    let delta_sent = current.bytes_sent - previous.bytes_sent;
    let delta_recv = current.bytes_recv - previous.bytes_recv;
    let delta_bytes = delta_sent + delta_recv;
    let delta_rtt = current.rtt - previous.rtt;
    let delta_exec = current.execute_time - previous.execute_time;
    let delta_queue = current.queue_time - previous.queue_time;
    let delta_errors = current.errors - previous.errors;
    let delta_retrans = current.timeouts - previous.timeouts;

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
        avg_rtt,
        avg_exec,
        avg_queue,
        kb_per_op,
        kb_per_sec,
        iops,
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
}
