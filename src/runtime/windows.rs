pub use rt_local_runtime_windows::*;

/// Mark the asynchronous function as a the entry point.
///
/// The asynchronous runtime is launched using [`run`].
/// # Examples
///
/// ```
/// #[rt_local::runtime::windows::main]
/// async fn main() {
///     // ...
/// }
/// ```
pub use rt_local_macros::windows_main as main;

/// Mark the function as a test.
///
/// When specified for an asynchronous function, use [`run`] to launch the asynchronous runtime.
/// When specified for a synchronous function, do not launch the asynchronous runtime.
///
/// # Examples
///
/// ```
/// use rt_local::runtime::windows::test;
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
pub use rt_local_macros::windows_test as test;
