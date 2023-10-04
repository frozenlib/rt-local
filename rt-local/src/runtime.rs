/// Platform-independent runtime.
pub mod core {

    pub use rt_local_core::runtime::core::*;

    /// Mark the asynchronous function as a entry point.
    ///
    /// The asynchronous runtime is launched using [`run`].
    ///
    /// # Examples
    ///
    /// ```
    /// #[rt_local::runtime::core::main]
    /// async fn main() {
    ///     // ...
    /// }
    /// ```
    pub use rt_local_macros::core_main as main;

    /// Mark the function as a test.
    ///
    /// When specified for an asynchronous function, use [`run`] to launch the asynchronous runtime.
    /// When specified for a synchronous function, do not launch the asynchronous runtime.
    ///
    /// # Examples
    ///
    /// ```
    /// use rt_local::runtime::core::test;
    ///
    /// #[test]
    /// async fn test_async() {
    ///     // ...
    /// }
    ///
    /// #[test]
    /// fn test_sync() {
    ///     // ..
    /// }
    /// ```
    pub use rt_local_macros::core_test as test;
}

/// Runtime with Windows message loop.
#[cfg(all(target_os = "windows", feature = "windows"))]
pub mod windows;
