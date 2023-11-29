/// Platform-independent runtime.
pub mod blocking;

/// Runtime with Windows message loop.
#[cfg(all(target_os = "windows", feature = "windows"))]
pub mod windows;

#[cfg(feature = "eframe")]
pub mod eframe;
