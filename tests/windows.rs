#![cfg(all(target_os = "windows", feature = "windows"))]
use rt_local::runtime::windows::run;

mod test_utils;
mod common {
    mod tests;
}
