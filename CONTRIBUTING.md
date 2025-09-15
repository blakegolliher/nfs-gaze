# Contributing to nfs-gaze

Thank you for your interest in contributing to nfs-gaze! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

Please be respectful and constructive in all interactions. We aim to maintain a welcoming environment for all contributors.

## How to Contribute

### Reporting Issues

1. **Check existing issues** first to avoid duplicates
2. **Use a clear title** that describes the problem
3. **Provide details**:
   - NFS server type and version
   - Linux kernel version
   - Go version
   - Steps to reproduce
   - Expected vs actual behavior
   - Error messages or logs

### Suggesting Features

1. **Open an issue** with the "enhancement" label
2. **Describe the use case** - why is this feature needed?
3. **Provide examples** of how it would work
4. **Consider implementation** complexity

### Submitting Code

#### Getting Started

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature-name`
3. Make your changes
4. Add tests if applicable
5. Ensure all tests pass: `go test ./...`
6. Commit with clear messages
7. Push to your fork
8. Open a pull request

#### Code Standards

- **Go conventions**: Follow standard Go formatting (`go fmt`)
- **Error handling**: Always check and handle errors appropriately
- **Comments**: Add comments for non-obvious code
- **Testing**: Add tests for new functionality
- **Performance**: Consider performance implications, especially in the monitoring loop

#### Commit Messages

Use clear, descriptive commit messages:
- Start with a verb (Add, Fix, Update, Remove)
- Keep the first line under 50 characters
- Add detailed description if needed

Examples:
```
Add support for NFSv4.2 operations
Fix RTT calculation for async operations
Update README with performance tuning guide
```

### Testing

#### Running Tests

```bash
# Run all tests
go test ./...

# Run with verbose output
go test -v ./...

# Run with race detection
go test -race ./...

# Generate coverage report
go test -coverprofile=coverage.out ./...
go tool cover -html=coverage.out
```

#### Writing Tests

- Test edge cases and error conditions
- Use table-driven tests where appropriate
- Mock external dependencies (filesystem, network)
- Ensure tests are deterministic

### Documentation

- Update README.md for user-facing changes
- Add code comments for complex logic
- Update command-line help text
- Include examples for new features

## Development Setup

### Prerequisites

- Go 1.21 or later
- Linux environment (or WSL for Windows)
- Access to NFS mounts for testing

### Building

```bash
# Standard build
go build -o nfs-gaze .

# Debug build
go build -gcflags="all=-N -l" -o nfs-gaze .

# Build and install
go install .
```

### Project Structure

```
nfs-gaze/
├── main.go              # Entry point and CLI handling
├── main_unsupported.go  # Non-Linux platform support
├── stats.go             # Core statistics parsing
├── stats_test.go        # Statistics tests
├── types.go             # Data structures
├── utils.go             # Utility functions
├── README.md            # User documentation
├── LICENSE              # MIT license
└── CONTRIBUTING.md      # This file
```

## Areas for Contribution

### High Priority

- [ ] Support for NFSv4.2 operations
- [ ] JSON output format option
- [ ] Prometheus metrics exporter
- [ ] Historical data recording/playback

### Medium Priority

- [ ] Configuration file support
- [ ] Alert thresholds and notifications
- [ ] Performance optimization for large mount counts
- [ ] Additional statistical calculations (percentiles, stddev)

### Low Priority

- [ ] GUI or TUI interface
- [ ] MacOS/FreeBSD support (if feasible)
- [ ] Integration with monitoring systems

## Pull Request Process

1. **Ensure your PR**:
   - Has a clear description of changes
   - References any related issues
   - Includes tests for new functionality
   - Passes all existing tests
   - Updates documentation as needed

2. **Review process**:
   - Maintainers will review within a few days
   - Address feedback constructively
   - Be patient - this is a personal project

3. **After merge**:
   - Delete your feature branch
   - Update your fork's main branch

## Questions?

Feel free to open an issue for questions about contributing. We appreciate your interest in improving nfs-gaze!

## License

By contributing, you agree that your contributions will be licensed under the MIT License.