pub use rt_local_core::{base, spawn_local, yield_now, Task};

/// Runtime implementations.
pub mod runtime;

#[cfg(doctest)]
#[doc = include_str!("../../README.md")]
pub mod test_readme {}
