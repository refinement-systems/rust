#![deny(unsafe_op_in_unsafe_fn)]

pub(crate) mod abi;
mod common;
pub use common::*;

// Surfaced as `crate::sys::futex` by the `sys/mod.rs` `pub use pal::*` glob.
pub mod futex;
