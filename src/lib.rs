//! # nfs-gaze
//!
//! A real-time NFS I/O statistics monitor for Linux, built on top of
//! `/proc/self/mountstats`.
//!
//! This crate provides the building blocks used by the `nfs-gaze` CLI:
//!
//! - [`parser`] parses Linux `/proc/self/mountstats` into structured
//!   [`types::NFSMount`] values.
//! - [`stats`] computes per-operation delta statistics between successive
//!   samples (handling counter resets on remount).
//! - [`metrics`] exposes the optional Prometheus exporter behind the
//!   `prometheus` cargo feature.
//! - [`monitor`] drives the main sampling loop and signal handling for the
//!   binary.
//! - [`display`] formats delta statistics into a human-readable terminal
//!   table.
//! - [`cli`] defines the [`clap`] argument parser used by `main.rs`.
//!
//! Most consumers only need the re-exports at the crate root: the error
//! type [`NfsGazeError`], the [`Result`] alias, and the core domain types
//! [`NFSMount`], [`NFSOperation`], [`NFSEvents`], and [`DeltaStats`].
//!
//! ## Feature flags
//!
//! - `prometheus` — enables the Prometheus metrics exporter and HTTP
//!   `/metrics` endpoint. Disabled by default.
//!
//! ## Platform support
//!
//! The parser reads a Linux-specific `/proc` format; the crate compiles on
//! other platforms for development but does not provide useful runtime
//! behavior there.

pub mod cli;
pub mod display;
pub mod metrics;
pub mod monitor;
pub mod parser;
pub mod snapshot;
pub mod stats;
pub mod types;

#[cfg(test)]
pub mod test_utils;

pub use types::{DeltaStats, NFSEvents, NFSMount, NFSOperation, NfsGazeError, Result};

// Re-export the parser entry points so library consumers don't need to know
// the module layout.
pub use parser::{parse_events, parse_mountstats, parse_mountstats_reader, parse_nfs_operation};
