mod core_impl;
pub mod runtime;

pub use crate::core_impl::{spawn_local, yield_now, Task};

pub mod core {
    pub use crate::core_impl::{
        enter, leave, on_idle, on_step, run, RuntimeInjector, RuntimeLoop, RuntimeWaker,
    };
}
