mod base_impl;
pub use crate::base_impl::{spawn_local, wait_for_idle, Task};

/// Components to implement runtime.
pub mod base {
    pub use crate::base_impl::{enter, idle, leave, poll, run, EventLoop};
}
/// Runtime implementations.
pub mod runtime {
    pub mod core;
}
