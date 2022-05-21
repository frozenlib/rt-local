pub mod backends;
mod core;

pub use crate::core::{spawn_local, yield_now, Task};

pub mod runtime {
    pub use crate::core::{
        enter, leave, on_idle, on_step, run, RuntimeBackend, RuntimeLoop, RuntimeWaker,
    };
}
