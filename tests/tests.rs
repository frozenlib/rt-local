use std::time::Duration;

use async_std::task::sleep;
use msgloop::*;

#[test]
fn test_run() {
    let mut executed = false;
    run(&EmptyMessageLoop::new(), async {
        executed = true;
    });
    assert!(executed);
}

#[test]
fn test_sleep() {
    let mut executed = false;
    run(&EmptyMessageLoop::new(), async {
        sleep(Duration::from_secs(1)).await;
        executed = true;
    });
    assert!(executed);
}
