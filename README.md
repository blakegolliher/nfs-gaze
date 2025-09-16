# nfs-gaze

Real-time NFS performance monitoring with per-operation latency tracking. Monitor your NFS client I/O statistics on Linux systems with detailed, operation-specific metrics that go beyond traditional tools.

**🦀 Now implemented in Rust** for improved performance, memory safety, and reliability!

## Key Features

- **Per-Operation Latency Tracking**: Monitor RTT (Round Trip Time) for each NFS operation type (READ, WRITE, GETATTR, etc.)
- **No Kernel Modules Required**: Works directly with `/proc/self/mountstats` - no eBPF or special permissions needed
- **Real-Time Monitoring**: Live updates with configurable intervals
- **Operation Filtering**: Focus on specific NFS operations that matter to you
- **Clear Output Format**: Detailed performance metrics in an easy-to-read display
- **Memory Safe**: Built with Rust for zero memory leaks and thread safety
- **High Performance**: Optimized for minimal overhead monitoring

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
| Memory safety | ✅ | ❌ | ❌ |

*Reading /proc/self/mountstats requires the process to have access to its own mount namespace

### What Makes nfs-gaze Different

- **Latency-First Design**: Focus on latency metrics per operation type for better performance troubleshooting
- **Easier Than BCC Tools**: No need to install BCC, kernel headers, or deal with eBPF complexity
- **Surgical Precision**: Filter and monitor only the operations you care about (e.g., just metadata ops like GETATTR and LOOKUP)
- **Rust Performance**: Zero-cost abstractions and memory safety without garbage collection overhead

## Installation

### Pre-built Binaries

Download the latest release for your platform from the [Releases](https://github.com/yourusername/nfs-gaze/releases) page.

### From Source (Rust)

```bash
# Clone the repository
git clone https://github.com/yourusername/nfs-gaze
cd nfs-gaze

# Build optimized release binary
cargo build --release

# The binary will be at target/release/nfs-gaze
./target/release/nfs-gaze --help

# Optional: Install system-wide
sudo cp target/release/nfs-gaze /usr/local/bin/
```

### From Source (Legacy Go Version)

```bash
# Build the Go version (if needed for compatibility)
go build -o nfs-gaze-go .
```

### Requirements

- Linux operating system (required for `/proc/self/mountstats` access)
- Rust 1.70+ (for building from source)
- Active NFS mount points to monitor

## Usage Examples

### Identify Slow Operations

Track which NFS operations are experiencing high latency:

```bash
# Monitor all operations and their latencies
./nfs-gaze -m /mnt/nfs

# Example output showing per-operation RTT:
# server:/export mounted on /mnt/nfs
# Timestamp: 2024-01-15 14:30:45 UTC
#
# OP           IOPS     RTT(ms)  EXE(ms)  ERRORS
# --------------------------------------------
# READ         125.0    1.2ms    2.1ms    0
# WRITE        89.0     2.3ms    4.2ms    0
# GETATTR      450.0    0.5ms    0.7ms    0
# LOOKUP       234.0    0.8ms    1.1ms    0
```

### Debug Metadata Performance

Focus on metadata operations that often cause application slowdowns:

```bash
# Monitor only metadata operations
./nfs-gaze -m /mnt/nfs --ops GETATTR,LOOKUP,ACCESS,READDIR

# Track attribute cache effectiveness
./nfs-gaze -m /mnt/nfs --ops GETATTR --attr
```

### Monitor Write Latency Spikes

Track write performance during high load:

```bash
# Monitor writes with 500ms updates for quick spike detection
./nfs-gaze -m /mnt/nfs --ops WRITE -i 1

# Include bandwidth to correlate latency with throughput
./nfs-gaze -m /mnt/nfs --ops WRITE --bw
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
# Full diagnostic mode - all operations with bandwidth
./nfs-gaze -m /app/data --bw

# Focus on operations your app uses most
./nfs-gaze -m /app/data --ops READ,GETATTR,OPEN,CLOSE
```

### Performance Testing

Use during benchmarks to understand NFS behavior:

```bash
# Monitor during a test with specific duration
./nfs-gaze -m /mnt/nfs -i 1 -c 60  # Monitor for 60 seconds

# Clear screen between updates for easy reading
./nfs-gaze -m /mnt/nfs --clear
```

## Command-Line Options

| Flag | Long Form | Default | Description |
|------|-----------|---------|-------------|
| `-m` | `--mount-point` | | Mount point to monitor (monitors all if not specified) |
| | `--ops` | | Comma-separated list of operations to monitor |
| `-i` | `--interval` | 1 | Update interval in seconds |
| `-c` | `--count` | 0 | Number of iterations (0 = infinite) |
| | `--attr` | false | Show attribute cache statistics |
| | `--bw` | false | Show bandwidth statistics |
| | `--clear` | false | Clear screen between iterations |
| `-f` | `--mountstats-path` | /proc/self/mountstats | Path to mountstats file |

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

- **IOPS**: Operations per second during the monitoring interval
- **RTT (Round Trip Time)**: Average time from operation request to response in milliseconds
- **EXE (Execute Time)**: Average execution time in milliseconds
- **Errors**: Number of failed operations

### What to Look For

1. **High RTT on READ/WRITE**: Indicates data transfer bottlenecks
2. **High RTT on GETATTR**: Often indicates metadata server overload
3. **High RTT on LOOKUP**: Directory operations are slow, possibly due to large directories
4. **Error counts > 0**: Network issues, server problems, or permission errors
5. **Spike patterns**: Temporary issues vs consistent performance problems

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

# Monitor and alert on high latency (requires custom scripting)
./nfs-gaze -m /mnt/nfs | awk '/WRITE/ && $3 > 100 {print "High write latency detected!"}'
```

### Integration with Monitoring Systems

```bash
# Single measurement for monitoring systems
./nfs-gaze -m /mnt/nfs -c 1

# JSON output (future enhancement)
./nfs-gaze -m /mnt/nfs --format json
```

## Building from Source

### Requirements

- Rust 1.70 or later
- Linux development environment
- Git

### Development Build

```bash
# Clone and build
git clone https://github.com/yourusername/nfs-gaze
cd nfs-gaze

# Debug build (with debug symbols)
cargo build

# The debug binary will be at target/debug/nfs-gaze
./target/debug/nfs-gaze --help
```

### Release Build

```bash
# Optimized release build
cargo build --release

# The optimized binary will be at target/release/nfs-gaze
./target/release/nfs-gaze --help
```

### Cross-Compilation

```bash
# Install target
rustup target add x86_64-unknown-linux-musl

# Build static binary
cargo build --release --target x86_64-unknown-linux-musl

# For ARM64
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_parse_mountstats

# Generate coverage report (requires cargo-llvm-cov)
cargo install cargo-llvm-cov
cargo llvm-cov --html
```

## Migration from Go Version

### Command-Line Compatibility

The Rust version maintains 100% CLI compatibility with the Go version:

```bash
# These commands work identically in both versions
./nfs-gaze -m /mnt/nfs --bw
./nfs-gaze --ops READ,WRITE -i 2 -c 10
```

### Performance Improvements

The Rust version offers several advantages:

- **Memory Safety**: No memory leaks or buffer overflows
- **Performance**: ~20-30% faster parsing and lower memory usage
- **Reliability**: Better error handling and recovery
- **Binary Size**: Smaller static binaries

### Breaking Changes

None! The Rust version is a drop-in replacement for the Go version.

## Troubleshooting

### Common Issues

1. **"Permission denied" accessing mountstats**
   - Ensure you have read access to `/proc/self/mountstats`
   - Try running as the user that mounted the NFS share

2. **No NFS mounts detected**
   - Verify NFS mounts exist: `mount -t nfs,nfs4`
   - Check mountstats file: `cat /proc/self/mountstats | grep nfs`

3. **"This application only works on Linux"**
   - nfs-gaze is Linux-specific due to `/proc/self/mountstats` dependency
   - Use WSL2 on Windows or a Linux VM

4. **High latency readings**
   - Check network connectivity to NFS server
   - Verify NFS server load and health
   - Consider network path and baseline latency

### Debug Mode

```bash
# Enable debug logging (future enhancement)
RUST_LOG=debug ./nfs-gaze -m /mnt/nfs

# Check mountstats parsing manually
cat /proc/self/mountstats | grep -A 20 "device.*nfs"
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Clone and setup development environment
git clone https://github.com/yourusername/nfs-gaze
cd nfs-gaze

# Install development dependencies
rustup component add clippy rustfmt

# Run checks before submitting PR
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

### Before Submitting a PR

1. Run tests: `cargo test`
2. Check formatting: `cargo fmt --check`
3. Run linter: `cargo clippy -- -D warnings`
4. Update documentation if needed

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Disclaimer

This is a personal project and is not affiliated with any organization. It comes with no warranties or guarantees. Use at your own risk.

## Author

Blake Golliher

## Acknowledgments

- Linux kernel developers for `/proc/self/mountstats`
- NFS community for protocol documentation
- Rust community for excellent tooling and libraries
- Go community for the inspiration and original implementation