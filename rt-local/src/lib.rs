pub use rt_local_core::{base, spawn_local, yield_now, Task};
pub mod runtime {
    pub use rt_local_core::runtime::core;

    #[cfg(feature = "windows")]
    pub use rt_local_windows as windows;
}

pub use rt_local_macros::rt_local_test;
