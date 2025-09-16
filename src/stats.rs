use crate::types::{DeltaStats, NFSMount, NFSOperation};

/// Calculate delta statistics between two measurements
pub fn calculate_delta_stats(
    previous: &NFSMount,
    current: &NFSMount,
    elapsed_seconds: f64,
) -> Vec<DeltaStats> {
    let mut deltas = Vec::new();

    for (op_name, current_op) in &current.operations {
        if let Some(previous_op) = previous.operations.get(op_name) {
            let delta = calculate_operation_delta(previous_op, current_op, elapsed_seconds);
            if delta.delta_ops > 0 {
                deltas.push(delta);
            }
        } else {
            // New operation, treat previous as zeros
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
            let delta = calculate_operation_delta(&zero_op, current_op, elapsed_seconds);
            if delta.delta_ops > 0 {
                deltas.push(delta);
            }
        }
    }

    // Sort by operation name for consistent output
    deltas.sort_by(|a, b| a.operation.cmp(&b.operation));
    deltas
}

/// Calculate delta statistics for a single operation
fn calculate_operation_delta(
    previous: &NFSOperation,
    current: &NFSOperation,
    elapsed_seconds: f64,
) -> DeltaStats {
    let delta_ops = current.ops - previous.ops;
    let delta_sent = current.bytes_sent - previous.bytes_sent;
    let delta_recv = current.bytes_recv - previous.bytes_recv;
    let delta_bytes = delta_sent + delta_recv;
    let delta_rtt = current.rtt - previous.rtt;
    let delta_exec = current.execute_time - previous.execute_time;
    let delta_queue = current.queue_time - previous.queue_time;
    let delta_errors = current.errors - previous.errors;
    let delta_retrans = current.timeouts - previous.timeouts;

    // Calculate averages and rates
    let iops = if elapsed_seconds > 0.0 {
        delta_ops as f64 / elapsed_seconds
    } else {
        0.0
    };

    let avg_rtt = if delta_ops > 0 {
        delta_rtt as f64 / delta_ops as f64
    } else {
        0.0
    };

    let avg_exec = if delta_ops > 0 {
        delta_exec as f64 / delta_ops as f64
    } else {
        0.0
    };

    let avg_queue = if delta_ops > 0 {
        delta_queue as f64 / delta_ops as f64
    } else {
        0.0
    };

    let kb_per_op = if delta_ops > 0 {
        (delta_bytes as f64 / 1024.0) / delta_ops as f64
    } else {
        0.0
    };

    let kb_per_sec = if elapsed_seconds > 0.0 {
        (delta_bytes as f64 / 1024.0) / elapsed_seconds
    } else {
        0.0
    };

    DeltaStats {
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
    }
}

/// Filter operations based on a set of allowed operation names
pub fn filter_operations(stats: Vec<DeltaStats>, filter: &std::collections::HashSet<String>) -> Vec<DeltaStats> {
    if filter.is_empty() {
        stats
    } else {
        stats.into_iter()
            .filter(|stat| filter.contains(&stat.operation))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    fn create_test_mount(operations: HashMap<String, NFSOperation>) -> NFSMount {
        NFSMount {
            device: "server:/export".to_string(),
            mount_point: "/mnt/nfs".to_string(),
            server: "server".to_string(),
            export: "/export".to_string(),
            age: 0,
            operations,
            events: None,
            bytes_read: 0,
            bytes_write: 0,
        }
    }

    fn create_test_operation(name: &str, ops: i64, bytes_sent: i64, bytes_recv: i64, rtt: i64, exec: i64) -> NFSOperation {
        NFSOperation {
            name: name.to_string(),
            ops,
            ntrans: ops,
            timeouts: 0,
            bytes_sent,
            bytes_recv,
            queue_time: 0,
            rtt,
            execute_time: exec,
            errors: 0,
        }
    }

    #[test]
    fn test_calculate_delta_stats() {
        let mut prev_ops = HashMap::new();
        prev_ops.insert("READ".to_string(), create_test_operation("READ", 100, 1024, 2048, 1000, 2000));

        let mut curr_ops = HashMap::new();
        curr_ops.insert("READ".to_string(), create_test_operation("READ", 200, 2048, 4096, 2000, 4000));

        let previous = create_test_mount(prev_ops);
        let current = create_test_mount(curr_ops);

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
    fn test_calculate_delta_stats_new_operation() {
        let prev_ops = HashMap::new();

        let mut curr_ops = HashMap::new();
        curr_ops.insert("READ".to_string(), create_test_operation("READ", 100, 1024, 2048, 1000, 2000));

        let previous = create_test_mount(prev_ops);
        let current = create_test_mount(curr_ops);

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
}