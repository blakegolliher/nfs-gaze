# Building nfs-gaze

## Prerequisites

- **Go**: Version 1.24.3 or later
- **Linux**: This tool is Linux-specific and requires access to `/proc/self/mountstats`

## Quick Build

```bash
go build -o nfs-gaze
```

This creates the `nfs-gaze` executable in the current directory.

## Development Build

For development with verbose output:

```bash
go build -v -o nfs-gaze
```

## Production Build

For optimized production builds:

```bash
go build -ldflags="-w -s" -o nfs-gaze
```

The `-ldflags="-w -s"` flags strip debug information and symbol tables to reduce binary size.

## Cross-Compilation

Although nfs-gaze is Linux-specific, you can cross-compile from other platforms:

```bash
GOOS=linux GOARCH=amd64 go build -o nfs-gaze
```

## Installing Dependencies

The project uses Go modules. Dependencies are automatically downloaded during build:

```bash
go mod download
go mod tidy
```

## Testing

Run all tests:

```bash
go test ./...
```

Run tests with coverage:

```bash
go test -coverprofile=coverage.out ./...
go tool cover -html=coverage.out
```

## Clean Build

To ensure a clean build environment:

```bash
go clean -cache -modcache -testcache
go build -o nfs-gaze
```

## Build Verification

After building, verify the binary:

```bash
./nfs-gaze --help
file nfs-gaze
ldd nfs-gaze  # Check dynamic dependencies
```