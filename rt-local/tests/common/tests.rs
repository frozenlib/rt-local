use crate::run;
use crate::test_utils::*;
use async_std::task::sleep;
use rt_local_core::*;
use std::time::Duration;

#[test]
fn test_run() {
    let mut executed = false;
    run(async {
        executed = true;
    });
    assert!(executed);
}

#[test]
fn test_sleep() {
    let mut executed = false;
    run(async {
        sleep(Duration::from_secs(1)).await;
        executed = true;
    });
    assert!(executed);
}

#[test]
fn test_spawn_local() {
    let p = AssertPass::new();
    run(async {
        let p1 = p.clone();
        spawn_local(async move {
            sleep(Duration::from_secs(1)).await;
            p1.pass("1");
        })
        .await;
        p.pass("2");
    });
    p.assert(&["1", "2"]);
}

#[test]
fn test_cancel() {
    let p = AssertPass::new();
    run(async {
        let p1 = p.clone();
        let _ = spawn_local(async move {
            p1.pass("1");
        });
        sleep(Duration::from_secs(1)).await;
        p.pass("2");
    });
    p.assert(&["2"]);
}

#[test]
fn test_detach() {
    let p = AssertPass::new();
    run(async {
        let p1 = p.clone();
        spawn_local(async move {
            p1.pass("1");
        })
        .detach();
        sleep(Duration::from_secs(1)).await;
        p.pass("2");
    });
    p.assert(&["1", "2"]);
}

#[test]
fn test_yield_now() {
    let p = AssertPass::new();
    run(async {
        p.pass("1");
        wait_for_idle().await;
        p.pass("2");
    });
    p.assert(&["1", "2"]);
}

#[test]
fn test_yield_now_many() {
    let p1 = AssertPass::new_with(true);
    run(async {
        p1.pass("1-a");
        let p2 = p1.clone();
        let t = spawn_local(async move {
            p2.pass("2-a");
            wait_for_idle().await;
            p2.pass("2-b");
        });
        wait_for_idle().await;
        p1.pass("1-b");
        t.await;
    });
    p1.assert_ex(&[&["1-a"], &["2-a"], &["1-b", "2-b"]]);
}
