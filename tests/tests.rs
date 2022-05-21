use async_std::task::sleep;
use rt_local::backends::*;
use rt_local::*;
use std::future::Future;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

fn run(f: impl Future<Output = ()>) {
    runtime::run(&MainLoop::new(), f)
}

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
    p.assert_list(&["1", "2"]);
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
    p.assert_list(&["2"]);
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
    p.assert_list(&["1", "2"]);
}

#[test]
fn test_yield() {
    let p = AssertPass::new();
    run(async {
        p.pass("1");
        yield_now().await;
        p.pass("2");
    });
    p.assert_list(&["1", "2"]);
}

#[derive(Clone)]
struct AssertPass(Arc<Mutex<Vec<&'static str>>>);

impl AssertPass {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
    fn pass(&self, s: &'static str) {
        self.0.lock().unwrap().push(s);
    }
    fn assert_list(&self, s: &[&'static str]) {
        assert_eq!(&*self.0.lock().unwrap(), s);
    }
}
