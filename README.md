# nfs-gazer

A command-line tool to monitor NFS client I/O statistics on Linux systems. nfs-gazer provides real-time monitoring of NFS mount points with detailed statistics and flexible output formats.

## Requirements

- Linux operating system (required for `/proc/self/mountstats` access)
- Go 1.24.3 or later
- Active NFS mount points to monitor

## Build Instructions

### Quick Build

```bash
# Clone the repository
git clone <repository-url>
cd nfs-gazer

# Build the binary
go build -o nfs-gazer .
```

### Development Build

```bash
# Install dependencies (if any)
go mod download

# Build with debug information
go build -gcflags="all=-N -l" -o nfs-gazer .

# Or build with optimizations
go build -ldflags="-s -w" -o nfs-gazer .
```

### Cross-compilation

```bash
# Build for different architectures (Linux only)
GOOS=linux GOARCH=amd64 go build -o nfs-gazer-amd64 .
GOOS=linux GOARCH=arm64 go build -o nfs-gazer-arm64 .
```

## Installation

```bash
# Install to $GOPATH/bin
go install .

# Or manually copy the binary
sudo cp nfs-gazer /usr/local/bin/
```

## Usage

### Basic Usage

```bash
# Monitor all NFS mounts in nfsiostat format
./nfs-gazer --nfsiostat

# Monitor a specific mount point
./nfs-gazer -m /mnt/nfs

# Monitor with bandwidth statistics
./nfs-gazer -m /mnt/nfs -bw

# Monitor specific operations only
./nfs-gazer -m /mnt/nfs -ops READ,WRITE
```

### Command-Line Options

| Flag | Long Form | Default | Description |
|------|-----------|---------|-------------|
| `-m` | | | Mount point to monitor (monitors all if not specified) |
| `-ops` | | | Comma-separated list of operations to monitor (e.g., READ,WRITE,LOOKUP) |
| `-i` | | 1s | Update interval (e.g., 1s, 500ms, 2m) |
| `-c` | | 0 | Number of iterations (0 = infinite) |
| `-attr` | | false | Show attribute cache statistics |
| `-bw` | | false | Show bandwidth statistics |
| | `--nfsiostat` | false | Use nfsiostat output format |
| | `--clear` | false | Clear screen between iterations |
| `-f` | | /proc/self/mountstats | Path to mountstats file |

### Advanced Examples

```bash
# Monitor with custom interval and iteration count
./nfs-gazer -m /mnt/nfs -i 2s -c 10

# Show attribute cache stats with bandwidth
./nfs-gazer -m /mnt/nfs -attr -bw

# Monitor specific operations with screen clearing
./nfs-gazer -m /mnt/nfs -ops READ,WRITE,GETATTR --clear

# Use custom mountstats file
./nfs-gazer -f /custom/path/mountstats

# Combine multiple options
./nfs-gazer -m /mnt/nfs -ops READ,WRITE -bw -attr -i 500ms --clear
```

### Output Formats

#### Default Format
Shows detailed statistics for each monitored mount point including:
- Operation counts and rates
- Response times
- Error statistics
- Network statistics (if available)

#### nfsiostat Format (`--nfsiostat`)
Mimics the output format of the standard `nfsiostat` tool for compatibility with existing monitoring scripts.

#### Bandwidth Mode (`-bw`)
Adds bandwidth utilization statistics showing:
- Read/write throughput
- Data transfer rates
- Network utilization

#### Attribute Cache Mode (`-attr`)
Displays attribute cache statistics including:
- Cache hit/miss ratios
- Attribute validation statistics
- Cache effectiveness metrics

## Testing

To run the tests, use the following command:

```bash
go test ./...
```

To generate a test coverage report, use the following command:

```bash
go test -coverprofile=coverage.out ./... && go tool cover -html=coverage.out
```