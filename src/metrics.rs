use crate::types::{DeltaStats, NFSEvents, NFSMount};
use std::time::Duration;

#[cfg(feature = "prometheus")]
use prometheus::{
    CounterVec, Encoder, GaugeVec, HistogramOpts, HistogramVec, Opts, Registry, TextEncoder,
};

/// Metrics exporter trait for different backends
pub trait MetricsExporter: Send + Sync {
    fn export_nfs_operation_metrics(&self, mount: &NFSMount, stats: &[DeltaStats]);
    fn export_nfs_events_metrics(&self, mount: &NFSMount, events: &NFSEvents);
    fn export_mount_info_metrics(&self, mount: &NFSMount);
    fn get_metrics_output(&self) -> Option<String>;
}

/// Configuration for metrics export
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub enable_prometheus: bool,
    pub prometheus_port: u16,
    pub prometheus_bind: String,
    pub export_interval: Duration,
    pub include_labels: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_prometheus: false,
            prometheus_port: 9100,
            prometheus_bind: "127.0.0.1".to_string(),
            export_interval: Duration::from_secs(10),
            include_labels: true,
        }
    }
}

/// Prometheus metrics exporter
#[cfg(feature = "prometheus")]
pub struct PrometheusExporter {
    registry: Registry,
    // NFS Operation metrics with labels
    nfs_operations_total: CounterVec,
    nfs_operation_duration_seconds: HistogramVec,
    nfs_operation_bytes_total: CounterVec,
    nfs_operation_errors_total: CounterVec,
    nfs_operation_timeouts_total: CounterVec,

    // VFS Event metrics - separate counters for each event type
    nfs_vfs_open_total: CounterVec,
    nfs_vfs_lookup_total: CounterVec,
    nfs_vfs_access_total: CounterVec,
    nfs_vfs_read_page_total: CounterVec,
    nfs_vfs_write_page_total: CounterVec,
    nfs_vfs_getdents_total: CounterVec,
    nfs_vfs_setattr_total: CounterVec,
    nfs_vfs_flush_total: CounterVec,
    nfs_vfs_fsync_total: CounterVec,

    // Mount metrics with labels
    nfs_mount_age_seconds: GaugeVec,
    nfs_mount_bytes_read_total: CounterVec,
    nfs_mount_bytes_written_total: CounterVec,
}

#[cfg(feature = "prometheus")]
impl PrometheusExporter {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let registry = Registry::new();

        // Define label names for metrics
        let operation_labels = &["mount_point", "server", "operation"];
        let mount_labels = &["mount_point", "server"];

        // Create NFS operation metrics with labels
        let nfs_operations_total = CounterVec::new(
            Opts::new(
                "nfs_operations_total",
                "Total number of NFS operations performed",
            ),
            operation_labels,
        )?;

        let nfs_operation_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "nfs_operation_duration_seconds",
                "Duration of NFS operations in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            operation_labels,
        )?;

        let nfs_operation_bytes_total = CounterVec::new(
            Opts::new(
                "nfs_operation_bytes_total",
                "Total bytes transferred in NFS operations",
            ),
            operation_labels,
        )?;

        let nfs_operation_errors_total = CounterVec::new(
            Opts::new(
                "nfs_operation_errors_total",
                "Total number of NFS operation errors",
            ),
            operation_labels,
        )?;

        let nfs_operation_timeouts_total = CounterVec::new(
            Opts::new(
                "nfs_operation_timeouts_total",
                "Total number of NFS operation timeouts",
            ),
            operation_labels,
        )?;

        // Create separate VFS event metrics with labels
        let nfs_vfs_open_total = CounterVec::new(
            Opts::new("nfs_vfs_open_total", "Total number of VFS open events"),
            mount_labels,
        )?;

        let nfs_vfs_lookup_total = CounterVec::new(
            Opts::new("nfs_vfs_lookup_total", "Total number of VFS lookup events"),
            mount_labels,
        )?;

        let nfs_vfs_access_total = CounterVec::new(
            Opts::new("nfs_vfs_access_total", "Total number of VFS access events"),
            mount_labels,
        )?;

        let nfs_vfs_read_page_total = CounterVec::new(
            Opts::new(
                "nfs_vfs_read_page_total",
                "Total number of VFS read page events",
            ),
            mount_labels,
        )?;

        let nfs_vfs_write_page_total = CounterVec::new(
            Opts::new(
                "nfs_vfs_write_page_total",
                "Total number of VFS write page events",
            ),
            mount_labels,
        )?;

        let nfs_vfs_getdents_total = CounterVec::new(
            Opts::new(
                "nfs_vfs_getdents_total",
                "Total number of VFS getdents events",
            ),
            mount_labels,
        )?;

        let nfs_vfs_setattr_total = CounterVec::new(
            Opts::new(
                "nfs_vfs_setattr_total",
                "Total number of VFS setattr events",
            ),
            mount_labels,
        )?;

        let nfs_vfs_flush_total = CounterVec::new(
            Opts::new("nfs_vfs_flush_total", "Total number of VFS flush events"),
            mount_labels,
        )?;

        let nfs_vfs_fsync_total = CounterVec::new(
            Opts::new("nfs_vfs_fsync_total", "Total number of VFS fsync events"),
            mount_labels,
        )?;

        // Create mount metrics with labels
        let nfs_mount_age_seconds = GaugeVec::new(
            Opts::new("nfs_mount_age_seconds", "Age of NFS mount in seconds"),
            mount_labels,
        )?;

        let nfs_mount_bytes_read_total = CounterVec::new(
            Opts::new(
                "nfs_mount_bytes_read_total",
                "Total bytes read from NFS mount",
            ),
            mount_labels,
        )?;

        let nfs_mount_bytes_written_total = CounterVec::new(
            Opts::new(
                "nfs_mount_bytes_written_total",
                "Total bytes written to NFS mount",
            ),
            mount_labels,
        )?;

        // Register metrics
        registry.register(Box::new(nfs_operations_total.clone()))?;
        registry.register(Box::new(nfs_operation_duration_seconds.clone()))?;
        registry.register(Box::new(nfs_operation_bytes_total.clone()))?;
        registry.register(Box::new(nfs_operation_errors_total.clone()))?;
        registry.register(Box::new(nfs_operation_timeouts_total.clone()))?;

        // Register VFS event metrics
        registry.register(Box::new(nfs_vfs_open_total.clone()))?;
        registry.register(Box::new(nfs_vfs_lookup_total.clone()))?;
        registry.register(Box::new(nfs_vfs_access_total.clone()))?;
        registry.register(Box::new(nfs_vfs_read_page_total.clone()))?;
        registry.register(Box::new(nfs_vfs_write_page_total.clone()))?;
        registry.register(Box::new(nfs_vfs_getdents_total.clone()))?;
        registry.register(Box::new(nfs_vfs_setattr_total.clone()))?;
        registry.register(Box::new(nfs_vfs_flush_total.clone()))?;
        registry.register(Box::new(nfs_vfs_fsync_total.clone()))?;

        // Register mount metrics
        registry.register(Box::new(nfs_mount_age_seconds.clone()))?;
        registry.register(Box::new(nfs_mount_bytes_read_total.clone()))?;
        registry.register(Box::new(nfs_mount_bytes_written_total.clone()))?;

        Ok(Self {
            registry,
            nfs_operations_total,
            nfs_operation_duration_seconds,
            nfs_operation_bytes_total,
            nfs_operation_errors_total,
            nfs_operation_timeouts_total,
            nfs_vfs_open_total,
            nfs_vfs_lookup_total,
            nfs_vfs_access_total,
            nfs_vfs_read_page_total,
            nfs_vfs_write_page_total,
            nfs_vfs_getdents_total,
            nfs_vfs_setattr_total,
            nfs_vfs_flush_total,
            nfs_vfs_fsync_total,
            nfs_mount_age_seconds,
            nfs_mount_bytes_read_total,
            nfs_mount_bytes_written_total,
        })
    }

    pub async fn start_server(
        registry: Registry,
        bind: &str,
        port: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use http_body_util::Full;
        use hyper::body::{Bytes, Incoming};
        use hyper::server::conn::http1;
        use hyper::service::service_fn;
        use hyper::{Method, Request, Response, StatusCode};
        use hyper_util::rt::TokioIo;
        use std::net::SocketAddr;
        use std::sync::Arc;
        use tokio::net::TcpListener;

        let addr: SocketAddr = format!("{bind}:{port}").parse()?;
        let listener = TcpListener::bind(addr).await?;
        let registry = Arc::new(registry);

        eprintln!("Prometheus metrics server listening on http://{addr}/metrics");

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let registry = Arc::clone(&registry);

            tokio::spawn(async move {
                let service = service_fn(move |req: Request<Incoming>| {
                    let registry = Arc::clone(&registry);
                    async move {
                        let response = match (req.method(), req.uri().path()) {
                            (&Method::GET, "/metrics") => {
                                let encoder = TextEncoder::new();
                                let metric_families = registry.gather();
                                let mut buffer = Vec::new();

                                match encoder.encode(&metric_families, &mut buffer) {
                                    Ok(_) => Response::builder()
                                        .status(StatusCode::OK)
                                        .header("Content-Type", encoder.format_type())
                                        .body(Full::new(Bytes::from(buffer)))
                                        .unwrap(),
                                    Err(e) => {
                                        let error_msg = format!("Failed to encode metrics: {}", e);
                                        Response::builder()
                                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                                            .body(Full::new(Bytes::from(error_msg)))
                                            .unwrap()
                                    }
                                }
                            }
                            (&Method::GET, "/") => {
                                let body = "nfs-gaze metrics exporter\n\nAvailable endpoints:\n  /metrics - Prometheus metrics\n  /health  - Health check\n";
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .body(Full::new(Bytes::from(body)))
                                    .unwrap()
                            }
                            (&Method::GET, "/health") => Response::builder()
                                .status(StatusCode::OK)
                                .body(Full::new(Bytes::from("OK\n")))
                                .unwrap(),
                            _ => Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Full::new(Bytes::from("Not Found\n")))
                                .unwrap(),
                        };
                        Ok::<_, hyper::Error>(response)
                    }
                });

                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

#[cfg(feature = "prometheus")]
impl MetricsExporter for PrometheusExporter {
    fn export_nfs_operation_metrics(&self, mount: &NFSMount, stats: &[DeltaStats]) {
        for stat in stats {
            // Create labels for the current operation
            let labels = &[
                mount.mount_point.as_str(),
                mount.server.as_str(),
                stat.operation.as_str(),
            ];

            // Add operation count
            self.nfs_operations_total
                .with_label_values(labels)
                .inc_by(stat.delta_ops as f64);

            // Add duration histogram (convert ms to seconds)
            if stat.avg_rtt > 0.0 {
                self.nfs_operation_duration_seconds
                    .with_label_values(labels)
                    .observe(stat.avg_rtt / 1000.0);
            }

            // Add bytes transferred
            self.nfs_operation_bytes_total
                .with_label_values(labels)
                .inc_by(stat.delta_bytes as f64);

            // Add errors
            if stat.delta_errors > 0 {
                self.nfs_operation_errors_total
                    .with_label_values(labels)
                    .inc_by(stat.delta_errors as f64);
            }

            // Add timeouts
            if stat.delta_retrans > 0 {
                self.nfs_operation_timeouts_total
                    .with_label_values(labels)
                    .inc_by(stat.delta_retrans as f64);
            }
        }
    }

    fn export_nfs_events_metrics(&self, mount: &NFSMount, events: &NFSEvents) {
        // Create labels for VFS events
        let labels = &[mount.mount_point.as_str(), mount.server.as_str()];

        // Export each VFS event type to its own metric with labels
        if events.vfs_open > 0 {
            self.nfs_vfs_open_total
                .with_label_values(labels)
                .inc_by(events.vfs_open as f64);
        }
        if events.vfs_lookup > 0 {
            self.nfs_vfs_lookup_total
                .with_label_values(labels)
                .inc_by(events.vfs_lookup as f64);
        }
        if events.vfs_access > 0 {
            self.nfs_vfs_access_total
                .with_label_values(labels)
                .inc_by(events.vfs_access as f64);
        }
        if events.vfs_read_page > 0 {
            self.nfs_vfs_read_page_total
                .with_label_values(labels)
                .inc_by(events.vfs_read_page as f64);
        }
        if events.vfs_write_page > 0 {
            self.nfs_vfs_write_page_total
                .with_label_values(labels)
                .inc_by(events.vfs_write_page as f64);
        }
        if events.vfs_getdents > 0 {
            self.nfs_vfs_getdents_total
                .with_label_values(labels)
                .inc_by(events.vfs_getdents as f64);
        }
        if events.vfs_setattr > 0 {
            self.nfs_vfs_setattr_total
                .with_label_values(labels)
                .inc_by(events.vfs_setattr as f64);
        }
        if events.vfs_flush > 0 {
            self.nfs_vfs_flush_total
                .with_label_values(labels)
                .inc_by(events.vfs_flush as f64);
        }
        if events.vfs_fsync > 0 {
            self.nfs_vfs_fsync_total
                .with_label_values(labels)
                .inc_by(events.vfs_fsync as f64);
        }
        // Note: We could add more VFS event types here as needed
    }

    fn export_mount_info_metrics(&self, mount: &NFSMount) {
        // Create labels for mount metrics
        let labels = &[mount.mount_point.as_str(), mount.server.as_str()];

        self.nfs_mount_age_seconds
            .with_label_values(labels)
            .set(mount.age as f64);
        self.nfs_mount_bytes_read_total
            .with_label_values(labels)
            .inc_by(mount.bytes_read as f64);
        self.nfs_mount_bytes_written_total
            .with_label_values(labels)
            .inc_by(mount.bytes_write as f64);
    }

    fn get_metrics_output(&self) -> Option<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut output = Vec::new();

        if encoder.encode(&metric_families, &mut output).is_ok() {
            String::from_utf8(output).ok()
        } else {
            None
        }
    }
}

/// Combined metrics manager that exports to Prometheus.
pub struct MetricsManager {
    #[cfg(feature = "prometheus")]
    exporters: Vec<Box<dyn MetricsExporter>>,
    #[allow(dead_code)]
    config: MetricsConfig,
    #[cfg(feature = "prometheus")]
    prometheus_registry: Option<Registry>,
}

impl MetricsManager {
    pub fn new(config: MetricsConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(feature = "prometheus")]
        {
            let mut exporters: Vec<Box<dyn MetricsExporter>> = Vec::new();
            let mut prometheus_registry = None;

            if config.enable_prometheus {
                let exporter = PrometheusExporter::new()?;
                prometheus_registry = Some(exporter.registry.clone());
                exporters.push(Box::new(exporter));
            }

            Ok(Self {
                exporters,
                config,
                prometheus_registry,
            })
        }

        #[cfg(not(feature = "prometheus"))]
        {
            Ok(Self { config })
        }
    }

    #[cfg(feature = "prometheus")]
    pub fn start_prometheus_server(&self) -> Option<tokio::task::JoinHandle<()>> {
        if let Some(registry) = &self.prometheus_registry {
            let registry = registry.clone();
            let bind = self.config.prometheus_bind.clone();
            let port = self.config.prometheus_port;

            Some(tokio::spawn(async move {
                if let Err(e) = PrometheusExporter::start_server(registry, &bind, port).await {
                    eprintln!("Prometheus server error: {e}");
                }
            }))
        } else {
            None
        }
    }

    pub fn export_metrics(&self, mount: &NFSMount, stats: &[DeltaStats]) {
        #[cfg(feature = "prometheus")]
        {
            for exporter in &self.exporters {
                exporter.export_nfs_operation_metrics(mount, stats);

                if let Some(events) = &mount.events {
                    exporter.export_nfs_events_metrics(mount, events);
                }

                exporter.export_mount_info_metrics(mount);
            }
        }

        #[cfg(not(feature = "prometheus"))]
        {
            // No-op when the prometheus feature is disabled.
            let _ = (mount, stats);
        }
    }

    pub fn get_prometheus_metrics(&self) -> Option<String> {
        #[cfg(feature = "prometheus")]
        {
            for exporter in &self.exporters {
                if let Some(output) = exporter.get_metrics_output() {
                    return Some(output);
                }
            }
        }
        None
    }

    pub fn is_enabled(&self) -> bool {
        #[cfg(feature = "prometheus")]
        {
            self.config.enable_prometheus
        }

        #[cfg(not(feature = "prometheus"))]
        {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NFSEvents, NFSOperation};
    use std::collections::HashMap;

    fn create_test_mount() -> NFSMount {
        let mut operations = HashMap::new();
        operations.insert(
            "READ".to_string(),
            NFSOperation {
                name: "READ".to_string(),
                ops: 100,
                ntrans: 95,
                timeouts: 5,
                bytes_sent: 1024,
                bytes_recv: 2048,
                queue_time: 10,
                rtt: 20,
                execute_time: 30,
                errors: 2,
            },
        );

        NFSMount {
            device: "server:/export".to_string(),
            mount_point: "/mnt/nfs".to_string(),
            server: "server".to_string(),
            export: "/export".to_string(),
            age: 12345,
            operations,
            events: Some(NFSEvents::default()),
            bytes_read: 1048576,
            bytes_write: 2097152,
        }
    }

    fn create_test_delta_stats() -> Vec<DeltaStats> {
        vec![DeltaStats {
            operation: "READ".to_string(),
            delta_ops: 10,
            delta_bytes: 1024,
            delta_sent: 512,
            delta_recv: 512,
            delta_rtt: 100,
            delta_exec: 200,
            delta_queue: 50,
            delta_errors: 1,
            delta_retrans: 2,
            avg_rtt: 10.0,
            avg_exec: 20.0,
            avg_queue: 5.0,
            kb_per_op: 1.0,
            kb_per_sec: 10.0,
            iops: 10.0,
        }]
    }

    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert!(!config.enable_prometheus);
        assert_eq!(config.prometheus_port, 9100);
        assert_eq!(config.prometheus_bind, "127.0.0.1");
        assert_eq!(config.export_interval, Duration::from_secs(10));
    }

    #[test]
    fn test_metrics_manager_creation() {
        let config = MetricsConfig::default();
        let manager = MetricsManager::new(config);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_metrics_export_no_panic() {
        let config = MetricsConfig::default();
        let manager = MetricsManager::new(config).unwrap();
        let mount = create_test_mount();
        let stats = create_test_delta_stats();

        // Should not panic even with no exporters enabled
        manager.export_metrics(&mount, &stats);
    }

    #[cfg(feature = "prometheus")]
    #[test]
    fn test_prometheus_exporter_creation() {
        let exporter = PrometheusExporter::new();
        assert!(exporter.is_ok());
    }
}
