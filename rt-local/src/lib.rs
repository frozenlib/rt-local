pub use rt_local_core::{base, spawn_local, yield_now, Task};
pub mod runtime {
    pub mod core {

        pub use rt_local_core::runtime::core::*;
        pub use rt_local_macros::{core_main as main, core_test as test};
    }

    #[cfg(feature = "windows")]
    #[doc(inline)]
    pub use rt_local_windows as windows;
}
