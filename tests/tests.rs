use async_std::task::sleep;
use rt_local::backends::*;
use rt_local::*;
use std::collections::HashSet;
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
        yield_now().await;
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
            yield_now().await;
            p2.pass("2-b");
        });
        yield_now().await;
        p1.pass("1-b");
        t.await;
    });
    p1.assert_ex(&[&["1-a"], &["2-a"], &["1-b", "2-b"]]);
}

#[derive(Clone)]
struct AssertPass {
    p: Arc<Mutex<Vec<&'static str>>>,
    print: bool,
}

impl AssertPass {
    fn new() -> Self {
        Self::new_with(false)
    }
    fn new_with(print: bool) -> Self {
        Self {
            p: Arc::new(Mutex::new(Vec::new())),
            print,
        }
    }

    fn pass(&self, s: &'static str) {
        self.p.lock().unwrap().push(s);
        if self.print {
            println!("{}", s);
        }
    }
    fn assert(&self, s: &[&'static str]) {
        assert_eq!(&*self.p.lock().unwrap(), s);
    }
    fn assert_ex(&self, s: &[&[&'static str]]) {
        let mut i = 0;
        let mut e = HashSet::<&str>::new();
        for a in &*self.p.lock().unwrap() {
            while e.is_empty() {
                if i == s.len() {
                    panic!("expect finish but `{}`", a);
                }
                e.extend(s[i]);
                i += 1;
            }
            if e.contains(a) {
                e.remove(a);
            } else if e.len() == 1 {
                panic!("expect `{}` but `{}`", e.iter().next().unwrap(), a);
            } else {
                panic!("expect one of `{:?}` but `{}`", e, a);
            }
        }
        loop {
            if !e.is_empty() {
                if e.len() == 1 {
                    panic!("expect finish but `{}`", e.iter().next().unwrap());
                } else {
                    panic!("expect finish but one of `{:?}`", e);
                }
            }
            if i == s.len() {
                break;
            }
            e.extend(s[i]);
            i += 1;
        }
    }
}
