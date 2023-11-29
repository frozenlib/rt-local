pub use rt_local_core::runtime::blocking::*;

/// Mark the asynchronous function as a entry point.
///
/// The asynchronous runtime is launched using [`run`].
///
/// # Examples
///
/// ```
/// #[rt_local::runtime::blocking::main]
/// async fn main() {
///     // ...
/// }
/// ```
pub use rt_local_macros::blocking_main as main;

/// Mark the function as a test.
///
/// When specified for an asynchronous function, use [`run`] to launch the asynchronous runtime.
/// When specified for a synchronous function, do not launch the asynchronous runtime.
///
/// # Examples
///
/// ```
/// use rt_local::runtime::blocking::test;
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
pub use rt_local_macros::blocking_test as test;
