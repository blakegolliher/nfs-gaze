# Prometheus Metrics Documentation

## Overview

`nfs-gaze` ships with an optional Prometheus metrics exporter that exposes
NFS I/O statistics, VFS event counters, and per-mount information through
an HTTP `/metrics` endpoint. This lets you scrape detailed per-operation
NFS performance data into your existing Prometheus + Grafana stack
without installing eBPF tools, kernel modules, or running as root.

The exporter is gated behind the `prometheus` cargo feature so the
default build stays lean for users who only want the live terminal
display.

## Quick Start

### Build with Prometheus support

```bash
cargo build --release --features prometheus
```

### Run the exporter

```bash
# Defaults: bind 127.0.0.1, port 9100
./target/release/nfs-gaze --prometheus

# Bind on all interfaces (e.g. for a remote Prometheus scraper)
./target/release/nfs-gaze --prometheus \
    --prometheus-bind 0.0.0.0 \
    --prometheus-port 9100

# Monitor a specific mount and a tighter sample interval
./target/release/nfs-gaze -m /mnt/nfs --prometheus -i 5
```

When the server starts you will see a log line on stderr:

```
Prometheus metrics server listening on http://127.0.0.1:9100/metrics
```

### Available HTTP endpoints

| Path       | Description                                              |
|------------|----------------------------------------------------------|
| `/metrics` | Prometheus text exposition format                        |
| `/health`  | Liveness check, returns `OK\n` with HTTP 200             |
| `/`        | Plain-text index page listing the endpoints              |

Any other path returns `404 Not Found`.

### CLI flags

These flags are only present when the binary was built with the
`prometheus` feature.

| Flag                  | Default     | Description                                |
|-----------------------|-------------|--------------------------------------------|
| `--prometheus`        | `false`     | Enable the Prometheus HTTP exporter        |
| `--prometheus-bind`   | `127.0.0.1` | Address the HTTP server binds to           |
| `--prometheus-port`   | `9100`      | Port the HTTP server listens on            |
| `--metrics-interval`  | `10`        | Metrics export interval in seconds         |

## Metrics Reference

All metrics carry contextual labels — there are **no unlabelled
aggregates**. The label sets used are:

- **Operation labels** (`mount_point`, `server`, `operation`) — applied to
  every per-operation metric.
- **Mount labels** (`mount_point`, `server`) — applied to VFS event
  counters and per-mount metrics.

Where:

- `mount_point` is the local path the NFS share is mounted on (e.g.
  `/mnt/nfs`).
- `server` is the NFS server hostname or address parsed from the
  `device` field.
- `operation` is the NFS RPC name (`READ`, `WRITE`, `GETATTR`,
  `LOOKUP`, …).

### Operation metrics

#### `nfs_operations_total`
- **Type**: Counter
- **Labels**: `mount_point`, `server`, `operation`
- **Description**: Total NFS operations performed since the exporter
  started, broken down by operation type.
- **Use**: IOPS calculations, traffic mix analysis.

```
nfs_operations_total{mount_point="/mnt/nfs",server="nfs-server",operation="READ"} 1547
nfs_operations_total{mount_point="/mnt/nfs",server="nfs-server",operation="WRITE"} 312
```

#### `nfs_operation_rtt_seconds_total`
- **Type**: Counter
- **Labels**: `mount_point`, `server`, `operation`
- **Description**: Cumulative RPC round-trip time in seconds, per
  operation type. Sourced from the per-op `rtt` field of
  `/proc/self/mountstats` (milliseconds, converted to seconds).
- **Use**: Average latency over any window:
  `rate(nfs_operation_rtt_seconds_total[5m]) / rate(nfs_operations_total[5m])`

#### `nfs_operation_exec_seconds_total`
- **Type**: Counter
- **Labels**: `mount_point`, `server`, `operation`
- **Description**: Cumulative operation execution time (queueing +
  RTT) in seconds — the latency the application actually experiences.
- **Use**: `rate(exec)/rate(ops)` for app-observed latency; compare
  with the RTT average to see how much time is spent queued on the
  client versus on the wire/server.

#### `nfs_operation_queue_seconds_total`
- **Type**: Counter
- **Labels**: `mount_point`, `server`, `operation`
- **Description**: Cumulative time operations spent queued on the
  client before transmission, in seconds.
- **Use**: A rising `rate(queue)/rate(ops)` with a flat RTT average
  points at client-side contention (e.g. RPC slot exhaustion) rather
  than server or network latency.

```
nfs_operation_rtt_seconds_total{mount_point="/mnt/nfs",server="nfs-server",operation="READ"}   12.456
nfs_operation_exec_seconds_total{mount_point="/mnt/nfs",server="nfs-server",operation="READ"}  14.021
nfs_operation_queue_seconds_total{mount_point="/mnt/nfs",server="nfs-server",operation="READ"}  1.565
```

> **Why counters and not a histogram?** `/proc/self/mountstats` only
> exposes per-interval *sums* of latency, never individual RPC
> timings, so a histogram built from this source had a `_count` equal
> to the number of sampling intervals (not operations) and quantiles
> that answered no real question. The counter pair yields exact,
> correctly weighted averages over any window. True percentiles are
> not derivable from mountstats; use eBPF-based tooling if you need
> them.

#### `nfs_operation_bytes_total`
- **Type**: Counter
- **Labels**: `mount_point`, `server`, `operation`
- **Description**: Total bytes transferred (sent + received) per
  operation type.
- **Use**: Per-operation throughput, identifying which RPCs dominate
  bandwidth.

#### `nfs_operation_errors_total`
- **Type**: Counter
- **Labels**: `mount_point`, `server`, `operation`
- **Description**: Total NFS operation errors per operation type.
- **Use**: Error-rate alerts, correlating failures with specific RPCs.

#### `nfs_operation_timeouts_total`
- **Type**: Counter
- **Labels**: `mount_point`, `server`, `operation`
- **Description**: Total NFS operation timeouts (retransmissions) per
  operation type.
- **Use**: Detecting network or server responsiveness issues without
  waiting for outright errors.

### VFS event metrics

VFS counters report kernel-side virtual file system events that
nfs-gaze parses from the `events:` line in `/proc/self/mountstats`.
Each event type is exposed as its own counter so you can chart and
alert on them independently. All carry the mount labels
(`mount_point`, `server`).

| Metric                    | Source field        | Description                            |
|---------------------------|---------------------|----------------------------------------|
| `nfs_vfs_open_total`      | `vfs_open`          | File opens against the mount           |
| `nfs_vfs_lookup_total`    | `vfs_lookup`        | Path-component lookups                 |
| `nfs_vfs_access_total`    | `vfs_access`        | Permission checks                      |
| `nfs_vfs_read_page_total` | `vfs_read_page`     | Page-cache read fills                  |
| `nfs_vfs_write_page_total`| `vfs_write_page`    | Page-cache write-backs                 |
| `nfs_vfs_getdents_total`  | `vfs_getdents`      | Directory enumeration calls            |
| `nfs_vfs_setattr_total`   | `vfs_setattr`       | Attribute updates                      |
| `nfs_vfs_flush_total`     | `vfs_flush`         | File flushes (close-to-open semantics) |
| `nfs_vfs_fsync_total`     | `vfs_fsync`         | Explicit fsync calls                   |

A counter is only updated when its underlying delta is non-zero, so
mounts with no activity will not register samples between scrapes.

### Mount information metrics

All carry the mount labels (`mount_point`, `server`).

#### `nfs_mount_age_seconds`
- **Type**: Gauge
- **Description**: Age of the NFS mount in seconds, taken from the
  `age:` field in `/proc/self/mountstats`.
- **Use**: Detecting recent remounts (a sudden drop indicates the
  mount was re-established).

#### `nfs_mount_bytes_read_total`
- **Type**: Counter
- **Description**: Wire-level bytes read from the server
  (`serverreadbytes` from the `bytes:` line): O_DIRECT reads are
  included and page-cache hits are excluded, so this measures real
  network transfer. Grows by the per-interval delta since the
  exporter started.
- **Use**: Read bandwidth: `rate(nfs_mount_bytes_read_total[5m])`.

#### `nfs_mount_bytes_written_total`
- **Type**: Counter
- **Description**: Wire-level bytes written to the server
  (`serverwritebytes`). Same delta-based semantics as the read
  counter.
- **Use**: Write bandwidth: `rate(nfs_mount_bytes_written_total[5m])`.

## Prometheus Configuration

### Basic scrape

```yaml
scrape_configs:
  - job_name: 'nfs-gaze'
    static_configs:
      - targets: ['localhost:9100']
    scrape_interval: 15s
    metrics_path: /metrics
```

A working example lives at `examples/prometheus.yml` in this
repository.

### Multiple hosts

```yaml
scrape_configs:
  - job_name: 'nfs-gaze'
    static_configs:
      - targets:
          - 'host-a:9100'
          - 'host-b:9100'
        labels:
          environment: production
    scrape_interval: 15s
```

Because every metric already carries `mount_point` and `server`
labels, you do **not** need a separate scrape job per mount on the
same host — a single nfs-gaze instance covers every NFS mount in its
namespace.

### Kubernetes service discovery

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
        replacement: ${1}:9100
```

## Example Queries

### Per-operation IOPS

```promql
sum by (operation) (rate(nfs_operations_total[1m]))
```

### Average RTT per operation

```promql
sum by (operation) (rate(nfs_operation_rtt_seconds_total[5m]))
  /
sum by (operation) (rate(nfs_operations_total[5m]))
```

### Application-observed latency (exec) vs. wire latency (RTT)

```promql
# Time per op spent queued on the client (slot pressure indicator):
  sum by (operation) (rate(nfs_operation_queue_seconds_total[5m]))
/ sum by (operation) (rate(nfs_operations_total[5m]))
```

### Error rate (%) per operation

```promql
100
* sum by (operation) (rate(nfs_operation_errors_total[5m]))
/ sum by (operation) (rate(nfs_operations_total[5m]))
```

### Throughput in MiB/s per mount

```promql
sum by (mount_point) (rate(nfs_operation_bytes_total[1m])) / 1024 / 1024
```

### VFS open vs. lookup ratio (cache effectiveness proxy)

```promql
rate(nfs_vfs_lookup_total[5m]) / rate(nfs_vfs_open_total[5m])
```

### Mount uptime in hours

```promql
nfs_mount_age_seconds / 3600
```

## Grafana panel ideas

1. **IOPS by operation** — `sum by (operation) (rate(nfs_operations_total[1m]))`
2. **Average latency by operation** — `sum by (operation) (rate(nfs_operation_rtt_seconds_total[5m])) / sum by (operation) (rate(nfs_operations_total[5m]))` (mountstats exposes latency sums, not per-RPC timings, so averages — not percentiles — are the honest statistic here)
3. **Throughput by mount** — `sum by (mount_point) (rate(nfs_operation_bytes_total[1m]))`, unit `bytes/sec` with SI prefix
4. **Error & timeout rates** — `sum by (operation) (rate(nfs_operation_errors_total[5m]))` and `…_timeouts_total`
5. **Mount age** — `nfs_mount_age_seconds`, displayed as a duration

## Alerting Rules

```yaml
groups:
  - name: nfs_alerts
    rules:
      - alert: HighNFSLatency
        expr: |
          (
              sum by (mount_point, operation) (rate(nfs_operation_rtt_seconds_total[5m]))
            /
              sum by (mount_point, operation) (rate(nfs_operations_total[5m]))
          ) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High NFS average latency on {{ $labels.mount_point }} ({{ $labels.operation }})"
          description: "average RTT is {{ $value }}s (threshold 0.1s)"

      - alert: NFSErrorsRising
        expr: |
          sum by (mount_point, operation) (
            rate(nfs_operation_errors_total[5m])
          ) > 1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "NFS errors on {{ $labels.mount_point }} ({{ $labels.operation }})"
          description: "Error rate {{ $value }}/s"

      - alert: NFSTimeouts
        expr: |
          sum by (mount_point) (
            rate(nfs_operation_timeouts_total[5m])
          ) > 1
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "NFS retransmits on {{ $labels.mount_point }}"
          description: "{{ $value }} timeouts/sec — check network/server"

      - alert: NFSMountRecycled
        expr: nfs_mount_age_seconds < 60
        for: 1m
        labels:
          severity: info
        annotations:
          summary: "NFS mount {{ $labels.mount_point }} was recently re-mounted"
```

## Testing the endpoint

```bash
# Quick sanity check
curl -s http://127.0.0.1:9100/metrics | head -40

# Filter for one metric family
curl -s http://127.0.0.1:9100/metrics | grep '^nfs_operations_total'

# Liveness
curl -i http://127.0.0.1:9100/health
```

If you have `promtool` installed:

```bash
curl -s http://127.0.0.1:9100/metrics | promtool check metrics
```

## Troubleshooting

1. **`Address already in use`** when starting the exporter
   Another process owns the port. Pick a different one:
   `--prometheus-port 9101`.

2. **Prometheus can't reach the exporter from another host**
   The default bind is `127.0.0.1`, which only accepts loopback
   connections. Bind explicitly with
   `--prometheus-bind 0.0.0.0` (or a specific NIC address).

3. **Metrics endpoint is empty / counters never increase**
   - Confirm there are NFS mounts in `/proc/self/mountstats`:
     `grep -c '^device .* nfs' /proc/self/mountstats`
   - Confirm activity is reaching them:
     `mount -t nfs,nfs4 && ls /mnt/nfs`
   - Counters only export non-zero deltas, so a totally idle mount
     will appear blank.

4. **Counter values look like they "reset"**
   The underlying `/proc/self/mountstats` counters are zeroed on
   remount or `umount`/`mount` cycles. When nfs-gaze sees any
   monotonic counter for an operation move backwards between
   samples, it treats that as a reset, **drops the reset sample
   entirely**, and rebases against the post-reset values on the next
   tick. The exporter is therefore safe across remounts (no negative
   `inc_by` calls reach Prometheus), and `rate()` queries recover on
   the next non-zero delta.

## Performance & Security

- The exporter holds metrics in memory and serves them on demand;
  scraping does not touch the disk.
- The default bind address is `127.0.0.1`, so the endpoint is **not**
  exposed off-host unless you opt in with `--prometheus-bind`.
- There is no authentication on the HTTP endpoint. If you bind on a
  routable address, place it behind a reverse proxy or restrict
  access at the network layer.
- Metrics contain only NFS performance data — no file paths beyond
  the mount point and no payload content.
- A reasonable starting point for `scrape_interval` is 15s; intervals
  shorter than ~5s rarely produce meaningfully different rate values.

## Adding new metrics

1. Define the field on `PrometheusExporter` in `src/metrics.rs`.
2. Construct it in `PrometheusExporter::new()` and `register` it.
3. Update it inside the relevant `export_*` method on the
   `MetricsExporter` impl.
4. Add an entry to the **Metrics Reference** section above and a
   regression test in `tests/metrics_test.rs`.

## License

See the project [LICENSE](LICENSE) file.
