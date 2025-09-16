# Test Coverage Report

## Summary

This directory contains test coverage reports for nfs-gaze.

## Test Results

All tests passing with comprehensive coverage across:

- **Parser module**: NFS mountstats parsing with various edge cases
- **Stats module**: Delta calculation and operation filtering
- **Monitor module**: Mount point monitoring and signal handling
- **Display module**: Terminal output formatting
- **CLI module**: Command-line argument parsing

## Files Covered

- `src/parser.rs` - Complete mountstats parsing logic
- `src/stats.rs` - Statistics calculation and filtering
- `src/monitor.rs` - Main monitoring loop and mount management
- `src/display.rs` - Terminal display formatting
- `src/cli.rs` - Command-line interface
- `src/types.rs` - Type definitions and error handling

## Test Coverage

The Rust implementation includes 31 test functions covering:

- Parse validation with valid/invalid inputs
- Error handling and edge cases
- Mount point filtering and selection
- Statistics calculations and formatting
- CLI argument parsing
- Signal handling setup

Run `cargo test` to execute all tests.

