# NFS-Gaze Go to Rust Migration Specification

## Project Overview
**Current:** Go-based NFS monitoring tool (~2300 lines)
**Target:** Rust implementation with equivalent functionality
**Platform:** Linux-specific (uses `/proc/self/mountstats`)

## Architecture Analysis

### Core Components
1. **Main Entry Point** (`main.go:13-39`)
   - Signal handling (SIGINT, SIGTERM)
   - Initial mountstats parsing
   - Mount point selection
   - Monitoring loop coordination

2. **Data Types** (`types.go:1-80`)
   - `NFSOperation`: Per-operation statistics
   - `NFSEvents`: VFS event counters
   - `NFSMount`: Mount point information
   - `DeltaStats`: Calculated differences between measurements

3. **Statistics Parser** (`stats.go:1-393`)
   - `/proc/self/mountstats` parsing
   - Event parsing
   - Operation statistics parsing
   - Delta calculations

4. **Utilities** (`utils.go:1-196`)
   - CLI flag handling
   - Mount filtering
   - Operation filtering
   - Monitoring loop

5. **Display** (referenced but not shown)
   - Terminal output formatting
   - Statistics presentation

## Rust Migration Strategy

### Phase 1: Project Setup
```toml
# Cargo.toml
[package]
name = "nfs-gaze"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
signal-hook = "0.3"
anyhow = "1"
thiserror = "1"
chrono = "0.4"
crossterm = "0.28"
serde = { version = "1", features = ["derive"] }

[target.'cfg(target_os = "linux")'.dependencies]
procfs = "0.16"  # Optional: for cleaner /proc access
```

### Phase 2: Type Definitions

#### Go to Rust Type Mappings
| Go Type | Rust Type | Notes |
|---------|-----------|-------|
| `string` | `String` or `&str` | Use `String` for owned data |
| `int64` | `i64` | Direct mapping |
| `map[string]*NFSOperation` | `HashMap<String, NFSOperation>` | Use `std::collections::HashMap` |
| `*NFSEvents` | `Option<NFSEvents>` | Nullable pointer becomes Option |
| `[]string` | `Vec<String>` | Dynamic arrays |
| `error` | `Result<T, Error>` | Error handling via Result |

#### Rust Structs
```rust
// src/types.rs
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NFSOperation {
    pub name: String,
    pub ops: i64,
    pub ntrans: i64,
    pub timeouts: i64,
    pub bytes_sent: i64,
    pub bytes_recv: i64,
    pub queue_time: i64,  // milliseconds
    pub rtt: i64,         // milliseconds
    pub execute_time: i64, // milliseconds
    pub errors: i64,
}

#[derive(Debug, Clone, Default)]
pub struct NFSEvents {
    pub inode_revalidate: i64,
    pub dentry_revalidate: i64,
    pub data_invalidate: i64,
    // ... other fields
}

#[derive(Debug, Clone)]
pub struct NFSMount {
    pub device: String,
    pub mount_point: String,
    pub server: String,
    pub export: String,
    pub age: i64,
    pub operations: HashMap<String, NFSOperation>,
    pub events: Option<NFSEvents>,
    pub bytes_read: i64,
    pub bytes_write: i64,
}

#[derive(Debug)]
pub struct DeltaStats {
    pub operation: String,
    pub delta_ops: i64,
    pub delta_bytes: i64,
    // ... other fields
}
```

### Phase 3: Module Structure
```
nfs-gaze/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point, signal handling
│   ├── types.rs           # Data structures
│   ├── parser.rs          # Mountstats parsing
│   ├── stats.rs           # Statistics calculations
│   ├── display.rs         # Terminal output
│   ├── monitor.rs         # Monitoring loop
│   └── cli.rs             # Command-line interface
└── tests/
    ├── parser_test.rs
    └── stats_test.rs
```

### Phase 4: Key Implementation Patterns

#### Error Handling
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NfsGazeError {
    #[error("Failed to read mountstats: {0}")]
    MountstatsRead(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Mount point not found: {0}")]
    MountNotFound(String),
}

pub type Result<T> = std::result::Result<T, NfsGazeError>;
```

#### Signal Handling
```rust
use signal_hook::{consts::SIGTERM, consts::SIGINT, iterator::Signals};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

let running = Arc::new(AtomicBool::new(true));
let r = running.clone();

std::thread::spawn(move || {
    let mut signals = Signals::new(&[SIGINT, SIGTERM]).unwrap();
    for sig in signals.forever() {
        r.store(false, Ordering::SeqCst);
    }
});
```

#### File Parsing
```rust
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn parse_mountstats(path: &str) -> Result<Vec<NFSMount>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut mounts = Vec::new();

    for line in reader.lines() {
        let line = line?;
        // Parse logic here
    }

    Ok(mounts)
}
```

### Phase 5: CLI with Clap
```rust
use clap::Parser;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "nfs-gaze")]
#[command(about = "NFS I/O Statistics Monitor")]
pub struct Args {
    /// Mount point to monitor
    #[arg(short = 'm', long)]
    pub mount_point: Option<String>,

    /// Comma-separated list of operations to monitor
    #[arg(long = "ops")]
    pub operations: Option<String>,

    /// Update interval in seconds
    #[arg(short = 'i', long, default_value = "1")]
    pub interval: u64,

    /// Number of iterations (0 = infinite)
    #[arg(short = 'c', long, default_value = "0")]
    pub count: usize,

    /// Show attribute cache statistics
    #[arg(long = "attr")]
    pub show_attr: bool,

    /// Show bandwidth statistics
    #[arg(long = "bw")]
    pub show_bandwidth: bool,

    /// Clear screen between iterations
    #[arg(long = "clear")]
    pub clear_screen: bool,

    /// Path to mountstats file
    #[arg(short = 'f', long, default_value = "/proc/self/mountstats")]
    pub mountstats_path: String,
}
```

### Phase 6: Testing Strategy

1. **Unit Tests**: Port existing Go tests
   - Parser tests (`stats_test.go`)
   - Utility tests (`utils_test.go`)
   - Display tests (`display_test.go`)

2. **Integration Tests**:
   - Mock `/proc/self/mountstats` files
   - Test complete monitoring cycles
   - Signal handling verification

3. **Benchmarks**:
   - Parser performance
   - Delta calculation speed
   - Memory usage comparison

### Phase 7: Platform Support

#### Linux-specific Build
```rust
#[cfg(not(target_os = "linux"))]
compile_error!("This application only works on Linux");

#[cfg(target_os = "linux")]
fn main() {
    // Main implementation
}
```

### Migration Steps

1. **Week 1: Foundation**
   - Set up Rust project structure
   - Implement type definitions
   - Create basic CLI with Clap

2. **Week 2: Core Parsing**
   - Port mountstats parser
   - Implement event parsing
   - Add operation statistics parsing

3. **Week 3: Business Logic**
   - Delta calculations
   - Monitoring loop
   - Signal handling

4. **Week 4: Display & Polish**
   - Terminal output formatting
   - Error handling refinement
   - Performance optimization

5. **Week 5: Testing & Documentation**
   - Port all tests
   - Add Rust-specific tests
   - Documentation and examples

### Performance Considerations

1. **Memory Management**
   - Use `Arc<RwLock<>>` for shared state
   - Consider `SmallVec` for small collections
   - Use string interning for operation names

2. **Parsing Optimization**
   - Use `nom` or similar for complex parsing
   - Pre-allocate collections when size is known
   - Consider zero-copy parsing where possible

3. **Display Performance**
   - Use `crossterm` for efficient terminal control
   - Buffer output to reduce syscalls
   - Consider using `tui-rs` for complex displays

### Rust-Specific Improvements

1. **Type Safety**
   - Use enums for operation types
   - Leverage the type system for state machines
   - Use newtypes for different ID types

2. **Concurrency**
   - Use `tokio` for async monitoring
   - Parallel parsing of multiple mount points
   - Lock-free statistics updates with atomics

3. **Error Recovery**
   - Graceful degradation on parse errors
   - Retry logic for transient failures
   - Better error messages with context

### Compatibility Notes

- Maintain CLI compatibility with Go version
- Same output format for easy migration
- Configuration file support (future enhancement)

## Estimated Timeline
- **Total Duration**: 5 weeks
- **Effort**: ~120 hours
- **Complexity**: Medium (straightforward port with improvements)

## Risks and Mitigations
1. **Risk**: `/proc` parsing differences
   - **Mitigation**: Extensive testing with real mountstats files

2. **Risk**: Performance regression
   - **Mitigation**: Benchmarking throughout development

3. **Risk**: Platform-specific edge cases
   - **Mitigation**: Test on multiple Linux distributions

## Success Criteria
- [ ] Feature parity with Go version
- [ ] Equal or better performance
- [ ] Comprehensive test coverage (>80%)
- [ ] Zero memory leaks (verified with valgrind)
- [ ] Documentation complete
- [ ] CI/CD pipeline configured