#![cfg(all(target_os = "windows", feature = "windows"))]
use rt_local::runtime::windows::{run, run_with_options, set_use_tsf_keystroke_mgr, RunOptions};

#[test]
fn run_with_options_without_tsf() {
    let mut executed = false;
    run_with_options(
        async {
            executed = true;
        },
        &RunOptions::default(),
    );
    assert!(executed);
}

#[test]
fn run_without_tsf_keystroke_mgr() {
    set_use_tsf_keystroke_mgr(false);
    let mut executed = false;
    run(async {
        executed = true;
    });
    set_use_tsf_keystroke_mgr(true);
    assert!(executed);
}

mod test_utils;
mod common {
    mod tests;
}
