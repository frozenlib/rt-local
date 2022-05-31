pub use rt_local_core::{base, spawn_local, wait_for_idle, Task};

/// Runtime implementations.
pub mod runtime;

#[cfg(doctest)]
#[doc = include_str!("../../README.md")]
pub mod test_readme {}
