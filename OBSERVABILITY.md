# Observability Integration for nfs-gaze

This document is the high-level guide to running `nfs-gaze` as a metrics
source for an existing observability stack. For the full metric
reference, query examples, and alert templates, see
[PROMETHEUS.md](PROMETHEUS.md).

## Overview

`nfs-gaze` ships with an optional Prometheus exporter that turns
`/proc/self/mountstats` into a labelled, scrape-friendly view of NFS
client behaviour. The exporter is gated behind the `prometheus` cargo
feature so the default build stays small for users who only want the
live terminal display.

The exporter exposes:

- per-operation counters, latency histograms, byte counts, error and
  timeout counters (labelled by `mount_point`, `server`, and
  `operation`),
- per-VFS-event counters (open, lookup, access, read_page, write_page,
  getdents, setattr, flush, fsync — labelled by `mount_point` and
  `server`),
- per-mount age and lifetime byte counters.

## Building

```bash
# Default build — terminal display only, no exporter
cargo build --release

# With the Prometheus exporter enabled
cargo build --release --features prometheus
```

When the `prometheus` feature is enabled, nfs-gaze pulls in
`prometheus`, `hyper`, `hyper-util`, `http-body-util`, `tower`, and
`tower-http`.

## Running

```bash
# Defaults: bind 127.0.0.1, port 9100, 10s metrics interval
./nfs-gaze --prometheus

# Bind on all interfaces (e.g. for a remote scraper)
./nfs-gaze --prometheus \
    --prometheus-bind 0.0.0.0 \
    --prometheus-port 9100

# Restrict to one mount and tighten the sample interval
./nfs-gaze -m /mnt/nfs --prometheus -i 5
```

The default bind is `127.0.0.1`, so the metrics endpoint is **only
reachable from the host** unless you opt in with
`--prometheus-bind`. There is no built-in authentication; if you
expose the endpoint off-host, put it behind a reverse proxy or
restrict access at the network layer.

### CLI flags (when built with `--features prometheus`)

| Flag                  | Default     | Description                          |
|-----------------------|-------------|--------------------------------------|
| `--prometheus`        | `false`     | Enable the Prometheus HTTP exporter  |
| `--prometheus-bind`   | `127.0.0.1` | Address the HTTP server binds to     |
| `--prometheus-port`   | `9100`      | Port the HTTP server listens on      |
| `--metrics-interval`  | `10`        | Metrics export interval in seconds   |

### HTTP endpoints

- `GET /metrics` — Prometheus text exposition format
- `GET /health`  — Liveness check, returns `OK`
- `GET /`        — Plain-text index page listing the endpoints

## Prometheus configuration

A working scrape config lives at `examples/prometheus.yml`. The minimal
form is:

```yaml
scrape_configs:
  - job_name: 'nfs-gaze'
    static_configs:
      - targets: ['localhost:9100']
    scrape_interval: 15s
```

Because every metric carries `mount_point` and `server` labels, a
single nfs-gaze instance is enough per host — you do not need a
separate scrape job per NFS mount.

## Sample output

```
# HELP nfs_operations_total Total number of NFS operations performed
# TYPE nfs_operations_total counter
nfs_operations_total{mount_point="/mnt/nfs",server="nfs-server",operation="READ"} 1547
nfs_operations_total{mount_point="/mnt/nfs",server="nfs-server",operation="WRITE"} 312

# HELP nfs_operation_duration_seconds Duration of NFS operations in seconds
# TYPE nfs_operation_duration_seconds histogram
nfs_operation_duration_seconds_bucket{mount_point="/mnt/nfs",server="nfs-server",operation="READ",le="0.005"} 120
nfs_operation_duration_seconds_bucket{mount_point="/mnt/nfs",server="nfs-server",operation="READ",le="0.01"}  890
nfs_operation_duration_seconds_sum{mount_point="/mnt/nfs",server="nfs-server",operation="READ"}              12.456
nfs_operation_duration_seconds_count{mount_point="/mnt/nfs",server="nfs-server",operation="READ"}            1547

# HELP nfs_vfs_open_total Total number of VFS open events
# TYPE nfs_vfs_open_total counter
nfs_vfs_open_total{mount_point="/mnt/nfs",server="nfs-server"} 42
```

Full metric definitions, label semantics, query examples, and alert
templates live in [PROMETHEUS.md](PROMETHEUS.md).

## Kubernetes deployment sketch

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
        prometheus.io/port: "9100"
    spec:
      hostNetwork: true
      hostPID: true
      containers:
        - name: nfs-gaze
          image: nfs-gaze:latest
          args:
            - "--prometheus"
            - "--prometheus-bind=0.0.0.0"
            - "--prometheus-port=9100"
          ports:
            - containerPort: 9100
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

The DaemonSet uses `hostNetwork` so each pod sees the host's
`/proc/self/mountstats` and exposes the exporter on the host's port
9100.

## Performance characteristics

- Metrics are held in process memory and rendered on demand; scraping
  does not touch the disk.
- Counters are only updated when their underlying delta is non-zero,
  so idle mounts cost essentially nothing per scrape.
- The underlying `/proc/self/mountstats` counters reset to zero on
  remount or `umount`/`mount` cycles. When nfs-gaze sees any
  monotonic counter for an operation move backwards between samples,
  it treats that as a reset, drops the reset sample entirely, and
  rebases against the post-reset values on the next tick. Published
  Prometheus counters then resume on the next non-zero delta and
  `rate()` queries recover automatically.
- A `--metrics-interval` of 10s pairs well with a Prometheus
  `scrape_interval` of 15s. Intervals shorter than ~5s rarely produce
  meaningfully different rates.

## Troubleshooting

- **Connection refused from another host** — the exporter is bound to
  `127.0.0.1` by default. Use `--prometheus-bind 0.0.0.0` (or a
  specific NIC) to expose it.
- **Empty `/metrics` output** — confirm there are NFS mounts with
  `grep -c '^device .* nfs' /proc/self/mountstats`. Counters are only
  emitted when their delta is non-zero, so totally idle mounts will
  not appear.
- **Port already in use** — choose another with `--prometheus-port`.
- **Prometheus shows resets** — these correspond to NFS remounts.
  nfs-gaze drops the reset sample, Prometheus handles counter
  resets natively, and `rate()` recovers on the next non-zero delta.

## See also

- [PROMETHEUS.md](PROMETHEUS.md) — full metric reference, PromQL
  queries, Grafana panels, and alert templates.
- [README.md](README.md) — general usage of the `nfs-gaze` CLI.
- `examples/prometheus.yml` — example Prometheus scrape configuration.
- `examples/prometheus_demo.rs` — a small Rust example that drives the
  exporter from synthetic data.
