# Observability Integration for nfs-gaze

This document describes the Prometheus and OpenTelemetry integration capabilities in nfs-gaze.

## Overview

nfs-gaze supports exporting NFS monitoring metrics to both Prometheus and OpenTelemetry systems through optional feature flags. This allows integration with modern monitoring and observability stacks.

## Features

### Available Integrations

1. **Prometheus Metrics Export** - HTTP endpoint for Prometheus scraping
2. **OpenTelemetry Metrics** - Push metrics to OTEL collectors
3. **Hybrid Support** - Enable both simultaneously for maximum flexibility

### Metrics Exported

#### NFS Operation Metrics
- `nfs_operations_total` (counter) - Total NFS operations by type
- `nfs_operation_duration_seconds` (histogram) - Operation latency distribution
- `nfs_operation_bytes_total` (counter) - Bytes transferred per operation type
- `nfs_operation_errors_total` (counter) - Error counts by operation type
- `nfs_operation_timeouts_total` (counter) - Timeout counts

#### VFS Event Metrics
- `nfs_vfs_events_total` (counter) - VFS-level events (open, lookup, read, write)

#### Mount Point Metrics
- `nfs_mount_age_seconds` (gauge) - How long mount has been active
- `nfs_mount_bytes_read_total` (counter) - Total bytes read from mount
- `nfs_mount_bytes_written_total` (counter) - Total bytes written to mount

#### Labels
All metrics include contextual labels:
- `mount_point` - The NFS mount path (e.g., `/mnt/nfs`)
- `server` - The NFS server hostname
- `operation` - The NFS operation type (READ, WRITE, etc.)

## Building with Observability Features

### Build Options

```bash
# Default build (no observability)
cargo build

# Prometheus only
cargo build --features prometheus

# OpenTelemetry only
cargo build --features opentelemetry

# Both integrations
cargo build --features observability
```

### Dependencies

When built with observability features, nfs-gaze includes:

- **Prometheus**: `prometheus`, `hyper`, `tower`, `tower-http`
- **OpenTelemetry**: `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-prometheus`

## Usage Examples

### Prometheus Integration

```bash
# Enable Prometheus metrics export
./nfs-gaze --prometheus --prometheus-port 9090

# Monitor specific mount with metrics
./nfs-gaze -m /mnt/nfs --prometheus --prometheus-port 8080

# Scrape metrics
curl http://localhost:9090/metrics
```

### OpenTelemetry Integration

```bash
# Enable OTEL metrics export
./nfs-gaze --opentelemetry --otel-endpoint http://jaeger:4317

# Combined monitoring with OTEL
./nfs-gaze -m /mnt/nfs --opentelemetry --otel-endpoint http://collector:4317
```

### Combined Usage

```bash
# Enable both Prometheus and OpenTelemetry
./nfs-gaze --prometheus --opentelemetry \\
           --prometheus-port 9090 \\
           --otel-endpoint http://otel-collector:4317
```

## Command-Line Options

### Prometheus Options
- `--prometheus` - Enable Prometheus metrics export
- `--prometheus-port <PORT>` - HTTP server port (default: 9090)

### OpenTelemetry Options
- `--opentelemetry` - Enable OpenTelemetry metrics export
- `--otel-endpoint <URL>` - OTEL collector endpoint

### General Options
- `--metrics-interval <SECONDS>` - Export interval (default: 10s)

## Integration Examples

### Prometheus Setup

1. **Build nfs-gaze with Prometheus support**:
```bash
cargo build --features prometheus --release
```

2. **Start nfs-gaze with Prometheus enabled**:
```bash
./target/release/nfs-gaze --prometheus --prometheus-port 9090
```

3. **Configure Prometheus** (see `examples/prometheus.yml` for complete config):
```yaml
scrape_configs:
  - job_name: 'nfs-gaze'
    scrape_interval: 5s
    static_configs:
      - targets: ['localhost:9090']
```

4. **Start Prometheus**:
```bash
prometheus --config.file=examples/prometheus.yml
```

5. **Access metrics**: Visit http://localhost:9090 for Prometheus web UI

### OpenTelemetry Setup

1. **Build nfs-gaze with OpenTelemetry support**:
```bash
cargo build --features opentelemetry --release
```

2. **Start OTEL Collector** (see `examples/otel-collector.yml` for complete config):
```bash
otelcol --config-file=examples/otel-collector.yml
```

3. **Start nfs-gaze with OpenTelemetry enabled**:
```bash
./target/release/nfs-gaze --opentelemetry --otel-endpoint http://localhost:4317
```

4. **Access exported metrics**:
   - Prometheus format: http://localhost:8889/metrics (from OTEL collector)
   - Jaeger UI: http://localhost:16686 (if Jaeger exporter configured)

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: nfs-gaze
spec:
  selector:
    matchLabels:
      name: nfs-gaze
  template:
    metadata:
      labels:
        name: nfs-gaze
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
    spec:
      hostNetwork: true
      hostPID: true
      containers:
      - name: nfs-gaze
        image: nfs-gaze:latest
        args:
          - "--prometheus"
          - "--prometheus-port=9090"
        ports:
        - containerPort: 9090
          name: metrics
        volumeMounts:
        - name: proc
          mountPath: /proc
          readOnly: true
      volumes:
      - name: proc
        hostPath:
          path: /proc
```

## Sample Metrics Output

### Prometheus Format
```
# HELP nfs_operations_total Total number of NFS operations performed
# TYPE nfs_operations_total counter
nfs_operations_total{mount_point="/mnt/nfs",server="nfs-server",operation="READ"} 1547

# HELP nfs_operation_duration_seconds Duration of NFS operations in seconds
# TYPE nfs_operation_duration_seconds histogram
nfs_operation_duration_seconds_bucket{mount_point="/mnt/nfs",server="nfs-server",operation="READ",le="0.005"} 120
nfs_operation_duration_seconds_bucket{mount_point="/mnt/nfs",server="nfs-server",operation="READ",le="0.01"} 890
nfs_operation_duration_seconds_sum{mount_point="/mnt/nfs",server="nfs-server",operation="READ"} 12.456
nfs_operation_duration_seconds_count{mount_point="/mnt/nfs",server="nfs-server",operation="READ"} 1547

# HELP nfs_operation_bytes_total Total bytes transferred in NFS operations
# TYPE nfs_operation_bytes_total counter
nfs_operation_bytes_total{mount_point="/mnt/nfs",server="nfs-server",operation="READ"} 15728640
```

## Performance Considerations

### Overhead
- Metrics collection adds ~2-5% CPU overhead
- Memory usage increases by ~10-20MB for metric storage
- Network overhead depends on export frequency and metric cardinality

### Optimization
- Adjust `--metrics-interval` based on monitoring requirements
- Filter operations with `--ops` to reduce metric cardinality
- Use operation filtering for high-traffic environments

### Scaling
- Single nfs-gaze instance can handle 100+ mount points
- Prometheus scraping scales to 1000+ targets per server
- OpenTelemetry batching reduces network overhead

## Troubleshooting

### Common Issues

1. **Port Already in Use**:
```bash
# Check if port is available
lsof -i :9090

# Use different port
./nfs-gaze --prometheus --prometheus-port 9091
```

2. **OTEL Connection Failed**:
```bash
# Verify collector endpoint
curl http://otel-collector:4317/v1/metrics

# Check connectivity
./nfs-gaze --opentelemetry --otel-endpoint http://otel-collector:4317 -v
```

3. **Missing Metrics**:
```bash
# Verify NFS mounts are active
mount -t nfs,nfs4

# Check metrics export interval
./nfs-gaze --prometheus --metrics-interval 5
```

### Debug Mode
```bash
# Enable verbose logging
RUST_LOG=debug ./nfs-gaze --prometheus --opentelemetry

# Validate metrics endpoint
curl -v http://localhost:9090/metrics
```

## Architecture Notes

### Design Principles
- **Zero Dependencies**: Observability features are optional
- **Minimal Overhead**: Efficient metric collection and export
- **Cloud Native**: Compatible with modern observability stacks

### Implementation Details
- Metrics collected during normal monitoring loop
- Export happens asynchronously to avoid blocking
- Memory-efficient metric storage with automatic cleanup
- Thread-safe metric updates for concurrent access

## Future Enhancements

### Planned Features
- **StatsD Integration** - Support for StatsD protocol
- **InfluxDB Export** - Direct InfluxDB line protocol support
- **Custom Dashboards** - Pre-built Grafana dashboards
- **Alerting Rules** - Prometheus alerting rule templates

### Community Contributions
- Metric collection improvements
- Additional export formats
- Dashboard and visualization templates
- Performance optimizations

## References

- [Prometheus Documentation](https://prometheus.io/docs/)
- [OpenTelemetry Specification](https://opentelemetry.io/docs/)
- [Grafana Dashboards](https://grafana.com/dashboards/)
- [NFS Performance Monitoring Best Practices]()