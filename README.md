# nfs-gaze

A command-line tool to monitor NFS client I/O statistics on Linux in real-time. Track NFS performance metrics including IOPS, bandwidth, latency, and operation-specific statistics.

## Usage

### Basic Monitoring

```bash
# Monitor all NFS mounts (simple format)
./nfs-gaze

# Monitor a specific mount point
./nfs-gaze -m /mnt/nfs

# Monitor only READ and WRITE operations
./nfs-gaze -m /mnt/nfs -ops READ,WRITE

# Monitor metadata operations
./nfs-gaze -ops GETATTR,LOOKUP,ACCESS

# Combine with bandwidth stats
./nfs-gaze -ops READ,WRITE -bw

```

### Output Formats

```bash
# Simple format with bandwidth statistics
./nfs-gaze -m /mnt/nfs -bw

# Show attribute cache statistics (nfsiostat mode)
./nfs-gaze --nfsiostat --attr
```

### Display Options

```bash
# Clear screen between updates for clean display
./nfs-gaze -m /mnt/nfs --clear

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
## Testing

To run the tests, use the following command:

```bash
go test ./...
```

To generate a test coverage report, use the following command:

```bash
go test -coverprofile=coverage.out ./... && go tool cover -html=coverage.out
```
