//! Runtime with [eframe]. ([egui] framework)
//!
//! [eframe]: https://crates.io/crates/eframe
//! [egui]: https://crates.io/crates/egui
//!
//! To use a single-threaded asynchronous runtime in egui application, use one of the following methods.
//!
//! - use [`rt_local::runtime::eframe::run_simple_native`] instead of [`eframe::run_simple_native`].
//! - use [`RtLocalRuntime`] with [`eframe::run_native`]. (see [`RtLocalRuntime`] for details.)
//!
//! [`rt_local::runtime::eframe::run_simple_native`]: crate::runtime::eframe::run_simple_native
//! [`eframe::run_simple_native`]: https://docs.rs/eframe/0.24.1/eframe/fn.run_simple_native.html
//! [`eframe::run_native`]: https://docs.rs/eframe/0.24.1/eframe/fn.run_simple_native.html
pub use rt_local_runtime_eframe::*;
