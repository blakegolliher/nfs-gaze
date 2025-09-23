# Prometheus Metrics Documentation

## Overview

`nfs-gaze` provides a built-in Prometheus metrics exporter that exposes NFS I/O statistics and mount information through an HTTP endpoint. This allows you to integrate NFS performance monitoring into your existing Prometheus monitoring stack.

## Quick Start

### Running with Prometheus Metrics

```bash
# Build with Prometheus feature enabled
cargo build --release --features prometheus

# Run with default settings (port 9090)
./target/release/nfs-gaze -m /mnt/nfs --prometheus

# Run with custom port
./target/release/nfs-gaze -m /mnt/nfs --prometheus --prometheus-port 9091

# Run with specific update interval
./target/release/nfs-gaze -m /mnt/nfs --prometheus --prometheus-port 9091 -i 5
```

### Available Endpoints

Once running with `--prometheus`, the following HTTP endpoints are available:

- `http://localhost:9090/metrics` - Prometheus metrics in text format
- `http://localhost:9090/health` - Health check endpoint (returns "OK")
- `http://localhost:9090/` - Information page with available endpoints

## Metrics Reference

### Operation Metrics

#### `nfs_operations_total`
- **Type**: Counter
- **Description**: Total number of NFS operations performed
- **Labels**: None (aggregated across all operation types)
- **Use Case**: Track overall NFS activity volume
- **Example Value**: `nfs_operations_total 2906`

#### `nfs_operation_duration_seconds`
- **Type**: Histogram
- **Description**: Duration of NFS operations in seconds (RTT - Round Trip Time)
- **Labels**: None
- **Buckets**: 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0 seconds
- **Use Case**: Analyze operation latency distribution and identify performance issues
- **Example**:
  ```
  nfs_operation_duration_seconds_bucket{le="0.001"} 1250
  nfs_operation_duration_seconds_bucket{le="0.005"} 1489
  nfs_operation_duration_seconds_bucket{le="0.01"} 1502
  ...
  nfs_operation_duration_seconds_sum 12.5
  nfs_operation_duration_seconds_count 1502
  ```

#### `nfs_operation_bytes_total`
- **Type**: Counter
- **Description**: Total bytes transferred in NFS operations (read + write)
- **Labels**: None
- **Use Case**: Monitor data transfer volume
- **Example Value**: `nfs_operation_bytes_total 3041718780`

#### `nfs_operation_errors_total`
- **Type**: Counter
- **Description**: Total number of NFS operation errors
- **Labels**: None
- **Use Case**: Track error rates and identify reliability issues
- **Example Value**: `nfs_operation_errors_total 1`

#### `nfs_operation_timeouts_total`
- **Type**: Counter
- **Description**: Total number of NFS operation timeouts (retransmissions)
- **Labels**: None
- **Use Case**: Identify network or server responsiveness issues
- **Example Value**: `nfs_operation_timeouts_total 0`

### VFS Event Metrics

#### `nfs_vfs_events_total`
- **Type**: Counter
- **Description**: Total number of NFS VFS (Virtual File System) events
- **Labels**: None
- **Use Case**: Track file system level operations
- **Includes**: vfs_open, vfs_lookup, vfs_read_page, vfs_write_page events
- **Example Value**: `nfs_vfs_events_total 9`

### Mount Information Metrics

#### `nfs_mount_age_seconds`
- **Type**: Gauge
- **Description**: Age of the NFS mount in seconds (time since mount)
- **Labels**: None
- **Use Case**: Monitor mount stability and uptime
- **Example Value**: `nfs_mount_age_seconds 40`

#### `nfs_mount_bytes_read_total`
- **Type**: Counter
- **Description**: Total bytes read from the NFS mount since mount time
- **Labels**: None
- **Use Case**: Track read volume over mount lifetime
- **Example Value**: `nfs_mount_bytes_read_total 0`

#### `nfs_mount_bytes_written_total`
- **Type**: Counter
- **Description**: Total bytes written to the NFS mount since mount time
- **Labels**: None
- **Use Case**: Track write volume over mount lifetime
- **Example Value**: `nfs_mount_bytes_written_total 5837422592`

## Prometheus Configuration

### Basic Scrape Configuration

Add this to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'nfs-gaze'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
    metrics_path: /metrics
```

### Multiple NFS Monitors

If monitoring multiple NFS mounts on different ports:

```yaml
scrape_configs:
  - job_name: 'nfs-gaze'
    static_configs:
      - targets:
        - 'server1:9090'  # Monitor for /mnt/nfs1
        - 'server1:9091'  # Monitor for /mnt/nfs2
        - 'server2:9090'  # Monitor on different host
        labels:
          environment: 'production'
    scrape_interval: 15s
```

### With Service Discovery

For Kubernetes environments:

```yaml
scrape_configs:
  - job_name: 'nfs-gaze'
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_label_app]
        action: keep
        regex: nfs-gaze
      - source_labels: [__meta_kubernetes_pod_ip]
        target_label: __address__
        replacement: ${1}:9090
```

## Example Queries

### Basic Queries

```promql
# Current IOPS rate (operations per second)
rate(nfs_operations_total[1m])

# Average operation latency over last 5 minutes
rate(nfs_operation_duration_seconds_sum[5m]) / rate(nfs_operation_duration_seconds_count[5m])

# 95th percentile latency
histogram_quantile(0.95, rate(nfs_operation_duration_seconds_bucket[5m]))

# Error rate percentage
100 * rate(nfs_operation_errors_total[5m]) / rate(nfs_operations_total[5m])

# Throughput in MB/s
rate(nfs_operation_bytes_total[1m]) / 1024 / 1024

# Mount uptime in hours
nfs_mount_age_seconds / 3600
```

### Advanced Queries

```promql
# Detect latency spikes (when p95 > 100ms)
histogram_quantile(0.95, rate(nfs_operation_duration_seconds_bucket[1m])) > 0.1

# Calculate read vs write ratio (requires separate read/write byte counters)
rate(nfs_mount_bytes_read_total[5m]) / (rate(nfs_mount_bytes_read_total[5m]) + rate(nfs_mount_bytes_written_total[5m]))

# Alert on high error rate (> 1%)
100 * rate(nfs_operation_errors_total[5m]) / rate(nfs_operations_total[5m]) > 1
```

## Grafana Dashboard

### Panel Examples

1. **IOPS Panel**
   - Query: `rate(nfs_operations_total[1m])`
   - Visualization: Graph
   - Unit: ops/sec

2. **Latency Distribution**
   - Queries:
     - p50: `histogram_quantile(0.5, rate(nfs_operation_duration_seconds_bucket[5m]))`
     - p95: `histogram_quantile(0.95, rate(nfs_operation_duration_seconds_bucket[5m]))`
     - p99: `histogram_quantile(0.99, rate(nfs_operation_duration_seconds_bucket[5m]))`
   - Visualization: Graph
   - Unit: seconds

3. **Throughput**
   - Query: `rate(nfs_operation_bytes_total[1m])`
   - Visualization: Graph
   - Unit: bytes/sec (with SI prefix)

4. **Error Rate**
   - Query: `rate(nfs_operation_errors_total[5m])`
   - Visualization: Stat
   - Unit: errors/sec

5. **Mount Age**
   - Query: `nfs_mount_age_seconds`
   - Visualization: Stat
   - Unit: seconds (duration)

## Alerting Rules

Example Prometheus alerting rules:

```yaml
groups:
  - name: nfs_alerts
    rules:
      - alert: HighNFSLatency
        expr: histogram_quantile(0.95, rate(nfs_operation_duration_seconds_bucket[5m])) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High NFS operation latency detected"
          description: "95th percentile NFS latency is {{ $value }}s (threshold: 0.1s)"

      - alert: NFSErrors
        expr: rate(nfs_operation_errors_total[5m]) > 10
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High NFS error rate"
          description: "NFS error rate is {{ $value }} errors/sec"

      - alert: NFSTimeouts
        expr: rate(nfs_operation_timeouts_total[5m]) > 1
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "NFS operations timing out"
          description: "NFS timeout rate is {{ $value }} timeouts/sec"
```

## Testing the Metrics Endpoint

### Manual Testing

```bash
# Check if metrics endpoint is working
curl http://localhost:9090/metrics

# Check health endpoint
curl http://localhost:9090/health

# Get metrics in a loop for testing
while true; do
  curl -s http://localhost:9090/metrics | grep nfs_operations_total
  sleep 5
done
```

### Using promtool

```bash
# Validate metrics format
curl -s http://localhost:9090/metrics | promtool check metrics

# Test queries locally
promtool query instant http://localhost:9090 'rate(nfs_operations_total[1m])'
```

## Troubleshooting

### Common Issues

1. **Port Already in Use**
   - Error: `Address already in use`
   - Solution: Use a different port with `--prometheus-port`

2. **No Metrics Appearing**
   - Check that NFS operations are occurring
   - Verify the mount point is correct with `-m /path/to/mount`
   - Ensure the tool is running with `--prometheus` flag

3. **Metrics Not Updating**
   - Check the update interval (`-i` flag)
   - Verify NFS mount is active: `mount | grep nfs`
   - Check `/proc/self/mountstats` is accessible

### Debug Mode

Run with verbose output to see metric updates:

```bash
# This will show the NFS stats on console while also exposing metrics
./nfs-gaze -m /mnt/nfs --prometheus --prometheus-port 9090 -i 2
```

## Performance Considerations

- The metrics endpoint is lightweight and adds minimal overhead
- Each scrape reads current in-memory metrics (no disk I/O)
- Recommended scrape interval: 15-30 seconds
- Lower intervals (< 5s) may not show meaningful changes due to metric granularity

## Security Notes

- The metrics endpoint binds to `0.0.0.0` by default (all interfaces)
- No authentication is implemented - use network security if needed
- Consider running behind a reverse proxy for production deployments
- Metrics do not contain sensitive data (only performance statistics)

## Integration with Other Tools

### Prometheus Pushgateway

For short-lived monitoring sessions:

```bash
# Run nfs-gaze and push metrics to pushgateway
nfs-gaze -m /mnt/nfs --prometheus --prometheus-port 9090 &
sleep 10
curl -s http://localhost:9090/metrics | curl --data-binary @- http://pushgateway:9091/metrics/job/nfs-gaze
```

### Node Exporter Integration

While node_exporter provides basic NFS metrics, nfs-gaze offers:
- More detailed operation-level statistics
- Real-time latency histograms
- Per-operation error tracking
- VFS event monitoring

Both can be used together for comprehensive monitoring.

## Future Enhancements

Potential improvements for the Prometheus integration:

1. **Per-operation labels** - Separate metrics for READ, WRITE, GETATTR, etc.
2. **Mount point labels** - Support multiple mounts with labels
3. **Custom histogram buckets** - Configurable latency buckets
4. **Metric cardinality controls** - Options to limit label combinations
5. **Recording rules** - Pre-computed metrics for common queries

## Contributing

To add new metrics:

1. Define the metric in `src/metrics.rs` PrometheusExporter struct
2. Register it in the `new()` method
3. Update it in the `export_nfs_operation_metrics()` method
4. Add documentation to this file

## License

See the main project LICENSE file.