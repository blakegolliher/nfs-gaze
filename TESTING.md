# NFS-Gaze Testing Documentation

## Overview

This document outlines the testing strategy, test coverage, and instructions for running tests for the nfs-gaze project. The project maintains comprehensive unit tests to ensure code reliability and correctness.

## Test Coverage

Current test coverage: **67.1%**

The test suite covers the following components:

### Core Components Tested

1. **Stats Parsing (`stats.go` / `stats_test.go`)**
   - NFS events parsing with various field configurations
   - NFS operation statistics parsing
   - Mountstats file parsing from `/proc/self/mountstats`
   - Delta calculation between measurements
   - Edge cases: nil values, empty data, malformed input

2. **Utility Functions (`utils.go` / `utils_test.go`)**
   - Command-line flag initialization and parsing
   - Operations filter parsing
   - Mount point selection logic
   - Initial summary printing

3. **Display Functions (`display_test.go`)**
   - nfsiostat format output
   - Simple format output with/without bandwidth
   - Empty stats handling
   - Attribute cache statistics display

### Test Structure

Tests are organized using Go's table-driven test pattern with subtests for better organization and clarity. Each test file corresponds to its source file:

```
stats.go        → stats_test.go
utils.go        → utils_test.go
types.go        → (covered through other tests)
main.go         → (integration testing, not unit tested)
```

## Running Tests

### Basic Test Execution

Run all tests:
```bash
go test ./...
```

Run tests with verbose output:
```bash
go test -v ./...
```

### Coverage Analysis

Generate coverage report:
```bash
go test -coverprofile=coverage.out ./...
```

View coverage percentage:
```bash
go test -cover ./...
```

Generate HTML coverage report:
```bash
go test -coverprofile=coverage.out ./...
go tool cover -html=coverage.out -o coverage.html
# Open coverage.html in a browser
```

View coverage in terminal:
```bash
go test -coverprofile=coverage.out ./...
go tool cover -func=coverage.out
```

### Run Specific Tests

Run a specific test function:
```bash
go test -run TestParseEvents
```

Run tests matching a pattern:
```bash
go test -run "TestParse.*"
```

### Benchmarking

Although no benchmarks are currently defined, you can add them and run:
```bash
go test -bench=.
```

## Test Design Principles

### 1. Testability Improvements

The codebase has been refactored for better testability:
- Added `parseMountstatsReader()` function that accepts an `io.Reader` instead of a file path
- This allows testing with mock data without filesystem dependencies
- Platform-specific build tags have been handled with separate test files

### 2. Table-Driven Tests

Tests use table-driven patterns for comprehensive coverage:
```go
tests := []struct {
    name    string
    input   string
    want    expected
    wantErr bool
}{
    // test cases
}
```

### 3. Edge Case Coverage

Tests include various edge cases:
- Empty input
- Malformed data
- Missing fields
- Nil pointers
- Boundary conditions

### 4. Cross-Platform Testing

The project includes platform-specific code (`//go:build linux`). For testing on non-Linux platforms:
- `stats_nolinux.go` and `utils_nolinux.go` provide implementations without build constraints
- This allows tests to run on any platform while maintaining Linux-specific production code

## Areas for Future Testing Improvement

1. **Integration Tests**: Add tests that simulate the full monitoring loop with mock mountstats data
2. **Main Function Testing**: Consider refactoring main.go to make it more testable
3. **Error Path Coverage**: Increase coverage of error handling paths
4. **Performance Benchmarks**: Add benchmarks for parsing large mountstats files
5. **Concurrent Access**: Test behavior under concurrent mount stats reading
6. **Signal Handling**: Test graceful shutdown behavior

## Continuous Integration

For CI/CD pipelines, use the following commands:

```bash
# Run tests with coverage and fail if coverage drops below threshold
go test -coverprofile=coverage.out ./...
coverage=$(go tool cover -func=coverage.out | grep total | awk '{print $3}' | sed 's/%//')
if (( $(echo "$coverage < 65.0" | bc -l) )); then
    echo "Coverage $coverage% is below threshold 65%"
    exit 1
fi
```

## Contributing

When adding new features or fixing bugs:

1. Write tests first (TDD approach recommended)
2. Ensure all existing tests pass
3. Maintain or improve code coverage
4. Follow the existing test patterns and naming conventions
5. Test names should be descriptive: `TestFunctionName_ScenarioDescription`

## Troubleshooting

### Tests Failing on Linux

If tests fail on Linux systems, ensure:
- You have proper permissions to read `/proc/self/mountstats`
- NFS mounts are present on the system for integration testing

### Build Constraints Issues

If you encounter build constraint issues:
- Production code uses `//go:build linux` for Linux-specific features
- Test helpers use `//go:build !linux` for cross-platform testing
- Ensure your IDE recognizes build tags correctly

### Coverage Not Generated

If coverage reports are empty:
- Verify that the test files are in the same package as the source files
- Check that test functions follow the `Test*` naming convention
- Ensure no build errors are preventing test compilation