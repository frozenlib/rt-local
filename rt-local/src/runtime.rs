/// Platform-independent runtime.
pub mod core {

    pub use rt_local_core::runtime::core::*;
    pub use rt_local_macros::{core_main as main, core_test as test};
}

/// Runtime with Windows message loop.
#[cfg(all(windows, feature = "windows"))]
pub mod windows;
