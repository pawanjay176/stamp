//! Hardware-independent reader code.
//!
//! This crate is intentionally `no_std` by default. Host-only test helpers may
//! opt into the `std` feature. Learning-critical reader behavior will be added
//! by the project owner phase by phase; this initial crate contains only the
//! contracts needed to keep that code independent from the X4 HAL.

#![cfg_attr(not(any(test, feature = "std")), no_std)]

pub mod display;
pub mod input;
pub mod storage;
