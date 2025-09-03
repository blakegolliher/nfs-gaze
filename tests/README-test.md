# Testing Documentation for nfs-gaze

This document describes the testing strategy, coverage analysis, and procedures for the nfs-gaze NFS monitoring tool.

## Overview

The nfs-gaze project includes comprehensive unit tests covering core functionality with **69.7%** statement coverage. The test suite is designed to ensure reliability and correctness of the NFS statistics parsing and monitoring features.

## Test Structure

### Test Files

- `types_test.go` - Tests for data structure creation and validation
- `stats_test.go` - Tests for NFS statistics parsing and calculation functions 
- `utils_test.go` - Tests for utility functions like flag parsing and mount monitoring
- `integration_test.go` - Integration tests with realistic data and scenarios
- `edge_case_test.go` - Edge cases and error handling tests

### Test Categories

1. **Unit Tests** - Test individual functions and components in isolation
2. **Integration Tests** - Test complete workflows with realistic data
3. **Edge Case Tests** - Test error conditions and boundary cases
4. **Regression Tests** - Ensure fixes don't break existing functionality

## Coverage Analysis

### Current Coverage: 69.7%

| File | Function | Coverage |
|------|----------|----------|
| main.go | main | 0.0% |
| stats.go | parseEvents | 71.6% |
| stats.go | parseMountstats | 86.9% |
| stats.go | calculateDelta | 100.0% |
| stats.go | displayStatsNfsiostat | 100.0% |
| stats.go | displayStatsSimple | 93.3% |
| utils.go | initFlags | 60.7% |
| utils.go | parseOperationsFilter | 100.0% |
| utils.go | getMountsToMonitor | 72.7% |
| utils.go | printInitialSummary | 94.7% |
| utils.go | monitoringLoop | 0.0% |

### High Coverage Functions (90%+)
- `calculateDelta` - 100% - Delta calculation between NFS operation measurements
- `displayStatsNfsiostat` - 100% - nfsiostat-compatible output formatting
- `parseOperationsFilter` - 100% - Command line operation filtering
- `printInitialSummary` - 94.7% - Initial monitoring session output
- `displayStatsSimple` - 93.3% - Simple output format display

### Medium Coverage Functions (70-89%)
- `parseMountstats` - 86.9% - Parse /proc/self/mountstats file
- `getMountsToMonitor` - 72.7% - Determine which mounts to monitor
- `parseEvents` - 71.6% - Parse NFS event statistics

### Lower Coverage Functions (<70%)
- `initFlags` - 60.7% - Command line flag initialization
- `main` - 0.0% - Main entry point (difficult to test)
- `monitoringLoop` - 0.0% - Long-running monitoring loop (difficult to test)

### Coverage Gaps

The main areas not covered by tests are:
1. **Main function** - Entry point with signal handling and main loop
2. **Monitoring loop** - Long-running loop with signal handling and timing
3. **Error paths** - Some error conditions that are difficult to trigger in tests
4. **Flag parsing edge cases** - Complex flag interaction scenarios

## Running Tests

### Basic Test Execution

```bash
# Run all tests
go test ./...

# Run tests with verbose output
go test ./... -v

# Run specific test file
go test -run TestParseEvents
```

### Coverage Reporting

```bash
# Generate coverage report
go test ./... -coverprofile=coverage.out

# View coverage by function
go tool cover -func=coverage.out

# Generate HTML coverage report
go tool cover -html=coverage.out -o coverage.html

# View coverage in browser
open coverage.html
```

### Coverage Thresholds

The project aims for:
- **Target**: 80% statement coverage
- **Current**: 69.7% statement coverage
- **Core functions**: >90% coverage for critical parsing and calculation functions
- **Utility functions**: >70% coverage for helper functions

## Test Scenarios

### Core Functionality Tests

1. **NFS Statistics Parsing**
   - Valid mountstats file parsing
   - Event statistics parsing with various field counts
   - Operation statistics extraction
   - Malformed data handling

2. **Data Structure Tests**
   - NFSOperation creation and validation
   - NFSEvents structure population  
   - NFSMount complete initialization
   - DeltaStats calculation validation

3. **Calculation Functions**
   - Delta calculation between measurements
   - IOPS, bandwidth, and latency calculations
   - Edge cases (zero operations, negative deltas)
   - Error handling for invalid inputs

### Integration Scenarios

1. **File Processing**
   - Complete mountstats file parsing
   - Multiple NFS mounts handling
   - Server/export path extraction
   - Age and byte statistics parsing

2. **Display Functions**
   - nfsiostat-compatible output format
   - Simple monitoring mode output
   - Attribute cache statistics display
   - Empty data handling

3. **Command Line Interface**
   - Flag parsing with various combinations
   - Positional argument handling
   - Operation filtering
   - Invalid input handling

### Edge Cases

1. **Data Corruption**
   - Insufficient mountstats fields
   - Invalid numeric data
   - Missing file handling
   - Partial data scenarios

2. **Boundary Conditions**
   - Zero operations
   - Large numeric values
   - Empty filter lists
   - Single vs multiple mounts

## Test Data

### Synthetic Test Data

The tests use carefully crafted synthetic data to exercise various code paths:

- **Valid mountstats data** - Complete, realistic NFS mount information
- **Corrupted data** - Various malformation scenarios
- **Edge case data** - Boundary conditions and unusual values
- **Empty data** - Missing or zero-value scenarios

### Test File Management

Tests that require file I/O use temporary files that are automatically cleaned up:

```go
tmpfile, err := ioutil.TempFile("", "test_prefix")
defer os.Remove(tmpfile.Name())
```

## Continuous Integration

### Automated Testing

The test suite is designed to run in CI/CD environments:

```bash
# CI test command
go test ./... -race -coverprofile=coverage.out

# Coverage validation
go tool cover -func=coverage.out | grep "total:" | awk '{print $3}' | sed 's/%//'
```

### Quality Gates

1. **All tests must pass** - Zero test failures allowed
2. **Coverage maintenance** - Coverage should not decrease
3. **Race condition detection** - Tests run with `-race` flag
4. **Build validation** - Code must compile without errors

## Troubleshooting

### Common Issues

1. **Import cycle errors**
   - Keep test files in same package as source
   - Use build tags appropriately (`//go:build linux`)

2. **File permission errors**
   - Tests use temporary files in system temp directory
   - Ensure proper cleanup in defer statements

3. **Race conditions**
   - Avoid shared state between tests
   - Use table-driven tests for parallel execution

### Debug Tests

```bash
# Run single test with debug output
go test -run TestSpecificFunction -v

# Run tests with race detection
go test ./... -race

# Generate test profile
go test ./... -cpuprofile cpu.prof
go tool pprof cpu.prof
```

## Contributing Tests

### Test Guidelines

1. **Test Naming** - Use descriptive names that explain the scenario
2. **Table-Driven Tests** - Use for testing multiple inputs/outputs
3. **Error Testing** - Always test both success and failure cases
4. **Cleanup** - Use defer for resource cleanup
5. **Coverage** - Aim to increase overall coverage with new tests

### Adding New Tests

1. Identify uncovered code paths using coverage report
2. Write tests for new functionality
3. Include both positive and negative test cases
4. Update this documentation if adding new test categories

### Test Review Checklist

- [ ] Tests cover both success and error cases
- [ ] Resource cleanup is handled properly
- [ ] Tests are deterministic and don't depend on external state
- [ ] Test names clearly describe the scenario being tested
- [ ] Coverage is maintained or improved

## Performance Testing

While the current test suite focuses on correctness, performance characteristics can be measured:

```bash
# Benchmark tests (if implemented)
go test -bench=. -benchmem

# Profile memory usage
go test -memprofile mem.prof
go tool pprof mem.prof
```

## Future Improvements

To reach the 80% coverage target:

1. **Mock filesystem interactions** - Test file I/O edge cases
2. **Signal handling tests** - Test graceful shutdown scenarios  
3. **Timing-dependent tests** - Test monitoring loop components
4. **Extended error injection** - Test more error paths
5. **Integration with real mountstats** - Test against actual system data

The test suite provides a solid foundation for ensuring the reliability and correctness of the nfs-gaze NFS monitoring tool while maintaining good development practices.