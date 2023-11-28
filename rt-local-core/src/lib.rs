mod base_impl;
pub use crate::base_impl::{spawn_local, wait_for_idle, Task};

/// Components to implement runtime.
pub mod base {
    pub use crate::base_impl::{enter, leave, on_idle, on_step, run, RuntimeInjector, RuntimeLoop};
}
/// Runtime implementations.
pub mod runtime {
    pub mod core;
}
