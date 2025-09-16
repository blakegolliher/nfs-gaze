/// Demo showing how to enable Prometheus metrics export
///
/// Usage:
/// cargo run --example prometheus_demo --features prometheus -- --prometheus --prometheus-port 9090

use nfs_gaze::{
    cli::{Args, parse_operations_filter},
    metrics::{MetricsConfig, MetricsManager},
    types::{NFSMount, DeltaStats},
};
use std::collections::HashMap;

fn main() {
    // Simulate CLI args for Prometheus
    let metrics_config = MetricsConfig {
        enable_prometheus: true,
        prometheus_port: 9090,
        enable_opentelemetry: false,
        otel_endpoint: None,
        export_interval: std::time::Duration::from_secs(10),
        include_labels: true,
    };

    // Create metrics manager
    match MetricsManager::new(metrics_config) {
        Ok(manager) => {
            if manager.is_enabled() {
                println!("Prometheus metrics export enabled!");
                println!("Metrics would be available at http://localhost:9090/metrics");

                // Create sample data
                let mut operations = HashMap::new();
                let sample_stats = vec![
                    DeltaStats {
                        operation: "READ".to_string(),
                        delta_ops: 100,
                        delta_bytes: 1024000,
                        delta_sent: 512000,
                        delta_recv: 512000,
                        delta_rtt: 500,
                        delta_exec: 800,
                        delta_queue: 100,
                        delta_errors: 2,
                        delta_retrans: 1,
                        avg_rtt: 5.0,
                        avg_exec: 8.0,
                        avg_queue: 1.0,
                        kb_per_op: 10.0,
                        kb_per_sec: 1000.0,
                        iops: 100.0,
                    }
                ];

                let sample_mount = NFSMount {
                    device: "demo-server:/export".to_string(),
                    mount_point: "/mnt/demo".to_string(),
                    server: "demo-server".to_string(),
                    export: "/export".to_string(),
                    age: 12345,
                    operations,
                    events: None,
                    bytes_read: 10485760,
                    bytes_write: 20971520,
                };

                // Export metrics
                manager.export_metrics(&sample_mount, &sample_stats);
                println!("Sample metrics exported");

                // Show what Prometheus output would look like
                if let Some(metrics_output) = manager.get_prometheus_metrics() {
                    println!("\nSample Prometheus metrics output:");
                    println!("{}", metrics_output);
                } else {
                    println!("Metrics exported (output available via HTTP endpoint)");
                }
            } else {
                println!("Metrics export not enabled");
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize metrics: {}", e);
        }
    }
}