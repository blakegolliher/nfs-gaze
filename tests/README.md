# NFS-Gaze Testing

This directory contains all testing documentation and coverage reports for the nfs-gaze project.

## Contents

- **[TESTING.md](TESTING.md)** - Complete testing documentation including how to run tests, test structure, and guidelines
- **[COVERAGE.md](COVERAGE.md)** - Latest test coverage report with detailed function-level coverage
- **coverage_to_md.go** - Tool to generate markdown coverage reports
- **coverage.out** - Raw coverage data (generated when running tests)

## Quick Start

### Run Tests

From the project root:

```bash
# Run all tests
go test ./...

# Run tests with verbose output
go test -v ./...

# Run tests with coverage
make coverage
```

### View Coverage

The latest coverage report is available in [COVERAGE.md](COVERAGE.md).

To regenerate the coverage report:

```bash
# From project root
make coverage

# Or manually
go test -coverprofile=tests/coverage.out ./...
go run tests/coverage_to_md.go > tests/COVERAGE.md
```

### Test Files Location

Following Go conventions, test files (`*_test.go`) are located alongside the source code in the main project directory:

- `stats_test.go` - Tests for stats parsing functions (ParseEvents, ParseNFSOperation, ParseMountstatsReader, CalculateDelta)
- `utils_test.go` - Tests for utility functions (InitFlags, ParseOperationsFilter, GetMountsToMonitor, PrintInitialSummary)
- `display_test.go` - Tests for display functions (DisplayStatsSimple)

## Coverage Goals

- **Target**: 70% overall coverage
- **Critical Functions**: 90% coverage for core parsing logic
- **Utilities**: 80% coverage for utility functions

## Continuous Integration

For CI/CD pipelines, use:

```bash
make test     # Run tests
make coverage # Generate coverage report
```

## More Information

See [TESTING.md](TESTING.md) for detailed testing documentation.