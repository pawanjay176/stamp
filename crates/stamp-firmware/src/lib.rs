//! Portable marker for the firmware composition crate.
//!
//! The opt-in `device` binary has runtime/linker scaffolding. Its startup
//! behavior remains an owner exercise and has not been tested on hardware.

#![no_std]

/// Current learning milestone; this is not a device-readiness flag.
pub const PHASE: &str = "phase-0-scaffold";
