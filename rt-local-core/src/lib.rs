mod base_impl;
pub use crate::base_impl::{spawn_local, yield_now, Task};
pub mod base {
    pub use crate::base_impl::{
        enter, leave, on_idle, on_step, run, RuntimeInjector, RuntimeLoop, RuntimeWaker,
    };
}
pub mod runtime {
    pub mod core;
}
