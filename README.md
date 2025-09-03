# nfs-gaze

A command-line tool to monitor NFS client I/O statistics on Linux in real-time. Track NFS performance metrics including IOPS, bandwidth, latency, and operation-specific statistics with support for both live monitoring and nfsiostat-compatible output.

## Features

- **Real-time NFS monitoring** - Live statistics with configurable update intervals
- **Multiple output formats** - Simple tabular view or nfsiostat-compatible format
- **Operation filtering** - Monitor specific NFS operations (READ, WRITE, GETATTR, etc.)
- **Performance metrics** - IOPS, bandwidth (MB/s, KB/op), latency (RTT, execution time)
- **Attribute cache stats** - VFS operations and cache invalidation statistics
- **Flexible targeting** - Monitor all NFS mounts or specific mount points
- **Screen clearing** - Clean display updates for continuous monitoring

## Usage

### Basic Monitoring

```bash
# Monitor all NFS mounts (simple format)
./nfs-gaze

# Monitor a specific mount point
./nfs-gaze -m /mnt/nfs

# Monitor with 5-second intervals
./nfs-gaze -i 5s

# Run for 10 iterations then exit
./nfs-gaze -c 10
```

### Output Formats

```bash
# Use nfsiostat-compatible format (detailed)
./nfs-gaze --nfsiostat

# Simple format with bandwidth statistics
./nfs-gaze -m /mnt/nfs -bw

# Show attribute cache statistics (nfsiostat mode)
./nfs-gaze --nfsiostat --attr
```

### Operation Filtering

```bash
# Monitor only READ and WRITE operations
./nfs-gaze -m /mnt/nfs -ops READ,WRITE

# Monitor metadata operations
./nfs-gaze -ops GETATTR,LOOKUP,ACCESS

# Combine with bandwidth stats
./nfs-gaze -ops READ,WRITE -bw
```

### Display Options

```bash
# Clear screen between updates for clean display
./nfs-gaze -m /mnt/nfs --clear

# Monitor with custom mountstats file
./nfs-gaze -f /proc/mountstats

# Positional arguments (legacy compatibility)
./nfs-gaze /mnt/nfs 2 10  # mount point, interval (seconds), count
```

## Command Line Options

| Flag | Description | Default |
|------|-------------|---------|
| `-m <path>` | Mount point to monitor | All NFS mounts |
| `-ops <list>` | Comma-separated operations to monitor | All operations |
| `-i <duration>` | Update interval | 1s |
| `-c <count>` | Number of iterations (0 = infinite) | 0 |
| `--nfsiostat` | Use nfsiostat output format | false |
| `-bw` | Show bandwidth statistics | false |
| `--attr` | Show attribute cache statistics | false |
| `--clear` | Clear screen between iterations | false |
| `-f <path>` | Path to mountstats file | /proc/self/mountstats |

## Output Explanation

### Simple Format
Shows IOPS, average RTT, and execution time for each operation:
```
Operation       IOPS   Avg RTT(ms)   Avg Exec(ms)
READ            125.2      2.145         1.823
WRITE            67.8      3.287         2.951
```

With `-bw` flag, adds bandwidth metrics:
```
Operation       IOPS   Avg RTT(ms)   Avg Exec(ms)      MB/s    KB/op
READ            125.2      2.145         1.823         15.654   128.00
WRITE            67.8      3.287         2.951          8.542   128.00
```

### nfsiostat Format
Provides detailed per-operation statistics including errors and retransmissions:
```
ops/s            rpc bklog
193.000               0.000

read:       ops/s         kB/s        kB/op      retrans  avg RTT (ms)  avg exe (ms)  avg queue (ms)    errors
           125.300     16029.440      128.000     0 (0.0%)        2.145        1.823           0.322     0 (0.0%)
```

## Examples

### Monitor High-Traffic NFS Mount
```bash
# Real-time monitoring with screen clearing
./nfs-gaze -m /data/nfs --clear -i 2s
```

### Troubleshoot NFS Performance
```bash
# Detailed nfsiostat format with attribute cache stats
./nfs-gaze --nfsiostat --attr
```

### Focus on Data Operations
```bash
# Monitor only data I/O with bandwidth
./nfs-gaze -ops READ,WRITE -bw --clear
```

### Batch Monitoring
```bash
# Run 60 iterations (2 minutes at 2s intervals)
./nfs-gaze -i 2s -c 60 > nfs-stats.log
```

## Testing

To run the tests, use the following command:

```bash
go test ./...
```

To generate a test coverage report, use the following command:

```bash
go test -coverprofile=coverage.out ./... && go tool cover -html=coverage.out
```