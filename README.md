# nfs-gaze

Real-time NFS performance monitoring with per-operation latency tracking. Monitor your NFS client I/O statistics on Linux systems with detailed, operation-specific metrics that go beyond traditional tools.

## Key Features

- **Per-Operation Latency Tracking**: Monitor RTT (Round Trip Time) for each NFS operation type (READ, WRITE, GETATTR, etc.)
- **No Kernel Modules Required**: Works directly with `/proc/self/mountstats` - no eBPF or special permissions needed
- **Real-Time Monitoring**: Live updates with configurable intervals
- **Operation Filtering**: Focus on specific NFS operations that matter to you
- **Clear Output Format**: Detailed performance metrics in an easy-to-read display

## Why nfs-gaze?

### Comparison with Other Tools

| Feature | nfs-gaze | nfsstat | nfsslower (bcc) |
|---------|----------|---------|-----------------|
| Per-operation latency | ✅ | ❌ | ✅ |
| No kernel modules needed | ✅ | ✅ | ❌ |
| No root required* | ✅ | ✅ | ❌ |
| Real-time monitoring | ✅ | ❌ | ✅ |
| Operation filtering | ✅ | ❌ | ✅ |
| Easy setup | ✅ | ✅ | ❌ |
| RTT/latency per op type | ✅ | ❌ | ✅ |

*Reading /proc/self/mountstats requires the process to have access to its own mount namespace

### What Makes nfs-gaze Different

- **Latency-First Design**: Focus on latency metrics per operation type for better performance troubleshooting
- **Easier Than BCC Tools**: No need to install BCC, kernel headers, or deal with eBPF complexity
- **Surgical Precision**: Filter and monitor only the operations you care about (e.g., just metadata ops like GETATTR and LOOKUP)

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/nfs-gaze
cd nfs-gaze

# Build the binary
go build -o nfs-gaze .

# Optional: Install system-wide
sudo cp nfs-gaze /usr/local/bin/
```

### Requirements

- Linux operating system (required for `/proc/self/mountstats` access)
- Go 1.21 or later (for building from source)
- Active NFS mount points to monitor

## Usage Examples

### Identify Slow Operations

Track which NFS operations are experiencing high latency:

```bash
# Monitor all operations and their latencies
./nfs-gaze -m /mnt/nfs

# Example output showing per-operation RTT:
# Mount: /mnt/nfs
# Operation    Ops/s    RTT(ms)    Avg RTT    Total Ops
# READ         125      2.3        2.1        15234
# WRITE        89       4.5        4.2        10892
# GETATTR      450      0.8        0.7        54321
# LOOKUP       234      1.2        1.1        28456
```

### Debug Metadata Performance

Focus on metadata operations that often cause application slowdowns:

```bash
# Monitor only metadata operations
./nfs-gaze -m /mnt/nfs -ops GETATTR,LOOKUP,ACCESS,READDIR

# Track attribute cache effectiveness
./nfs-gaze -m /mnt/nfs -ops GETATTR -attr
```

### Monitor Write Latency Spikes

Track write performance during high load:

```bash
# Monitor writes with 500ms updates for quick spike detection
./nfs-gaze -m /mnt/nfs -ops WRITE -i 500ms

# Include bandwidth to correlate latency with throughput
./nfs-gaze -m /mnt/nfs -ops WRITE -bw
```

### Compare Multiple Mount Points

Monitor different NFS servers or exports simultaneously:

```bash
# Monitor all NFS mounts to compare latencies
./nfs-gaze

# Or specific mounts in separate terminals
./nfs-gaze -m /mnt/nfs-server1
./nfs-gaze -m /mnt/nfs-server2
```

### Troubleshooting Application Slowness

When applications are slow, identify if NFS is the bottleneck:

```bash
# Full diagnostic mode - all operations with bandwidth and attributes
./nfs-gaze -m /app/data -bw -attr

# Focus on operations your app uses most
./nfs-gaze -m /app/data -ops READ,GETATTR,OPEN,CLOSE
```

### Performance Testing

Use during benchmarks to understand NFS behavior:

```bash
# Monitor during a test with specific duration
./nfs-gaze -m /mnt/nfs -i 1s -c 60  # Monitor for 60 seconds

# Clear screen between updates for easy reading
./nfs-gaze -m /mnt/nfs --clear
```

## Command-Line Options

| Flag | Long Form | Default | Description |
|------|-----------|---------|-------------|
| `-m` | | | Mount point to monitor (monitors all if not specified) |
| `-ops` | | | Comma-separated list of operations to monitor |
| `-i` | | 1s | Update interval (e.g., 1s, 500ms, 2m) |
| `-c` | | 0 | Number of iterations (0 = infinite) |
| `-attr` | | false | Show attribute cache statistics |
| `-bw` | | false | Show bandwidth statistics |
| | `--clear` | false | Clear screen between iterations |
| `-f` | | /proc/self/mountstats | Path to mountstats file |

### Supported NFS Operations

Common operations you can monitor:
- **READ/WRITE**: Data operations
- **GETATTR/SETATTR**: Attribute operations
- **LOOKUP**: Directory lookups
- **ACCESS**: Permission checks
- **OPEN/CLOSE**: File handles
- **CREATE/REMOVE**: File lifecycle
- **RENAME**: File moves
- **READDIR/READDIRPLUS**: Directory reading
- **FSSTAT/FSINFO**: Filesystem information
- **COMMIT**: Write commits

## Understanding the Output

### Latency Metrics

- **RTT (Round Trip Time)**: Time from operation request to response in milliseconds
- **Avg RTT**: Average RTT over the monitoring period
- **Delta RTT**: Change in total RTT since last interval

### What to Look For

1. **High RTT on READ/WRITE**: Indicates data transfer bottlenecks
2. **High RTT on GETATTR**: Often indicates metadata server overload
3. **High RTT on LOOKUP**: Directory operations are slow, possibly due to large directories
4. **Spike patterns**: Temporary issues vs consistent performance problems

### Performance Thresholds (Guidelines)

- **Excellent**: < 1ms RTT for metadata ops, < 5ms for data ops
- **Good**: < 5ms RTT for metadata ops, < 20ms for data ops  
- **Acceptable**: < 10ms RTT for metadata ops, < 50ms for data ops
- **Poor**: > 10ms RTT for metadata ops, > 50ms for data ops

## Advanced Usage

### Continuous Monitoring

```bash
# Log to file for analysis
./nfs-gaze -m /mnt/nfs | tee -a nfs-performance.log

# Monitor and alert on high latency
./nfs-gaze -m /mnt/nfs | awk '/WRITE/ && $3 > 100 {print "High write latency: " $3 "ms"}'
```

### Integration with Monitoring Systems

```bash
# Filter specific operations
./nfs-gaze -m /mnt/nfs -ops READ,WRITE

# Feed to monitoring tools
./nfs-gaze -m /mnt/nfs -c 1 | custom-metrics-collector
```

## Building from Source

### Development Build

```bash
# With debug symbols
go build -gcflags="all=-N -l" -o nfs-gaze .

# Optimized build
go build -ldflags="-s -w" -o nfs-gaze .
```

### Cross-Compilation

```bash
# Build for different architectures
GOOS=linux GOARCH=amd64 go build -o nfs-gaze-amd64 .
GOOS=linux GOARCH=arm64 go build -o nfs-gaze-arm64 .
```

## Testing

```bash
# Run tests
go test ./...

# Generate coverage report
make coverage
```

📁 **Testing Documentation**: See the [tests/](tests/) directory for:
- [Testing Guide](tests/TESTING.md) - Complete testing documentation
- [Coverage Report](tests/COVERAGE.md) - Latest test coverage metrics
- [Test README](tests/README.md) - Quick testing reference

## Troubleshooting

### Common Issues

1. **"Permission denied" accessing mountstats**
   - Ensure you have read access to `/proc/self/mountstats`
   - Try running as the user that mounted the NFS share

2. **No NFS mounts detected**
   - Verify NFS mounts exist: `mount -t nfs,nfs4`
   - Check mountstats file: `cat /proc/self/mountstats | grep nfs`

3. **High latency readings**
   - Check network connectivity to NFS server
   - Verify NFS server load
   - Consider network path and latency

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Disclaimer

This is personal project and is not affiliated with any organization. It comes with no warranties or guarantees. Use at your own risk.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Before submitting a PR:
1. Run tests: `make test`
2. Check coverage: `make coverage`
3. See [tests/TESTING.md](tests/TESTING.md) for testing guidelines

## Author

Blake Golliher

## Acknowledgments

- Linux kernel developers for `/proc/self/mountstats`
- NFS community for protocol documentation
- Go community for excellent standard library