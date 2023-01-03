use std::{cell::RefCell, time::Duration};

use async_std::task::sleep;
use rt_local::{runtime::core::run, spawn_local, wait_for_idle, Task};

mod test_utils;
mod common {
    mod tests;
}

thread_local! {
    static COUNTER: RefCell<usize> = RefCell::new(0);
}
fn increment() {
    COUNTER.with(|c| *c.borrow_mut() += 1);
}
fn assert_counter(value: usize) {
    assert_eq!(COUNTER.with(|c| *c.borrow()), value);
}

#[test]
fn run_repeat() {
    fn spwan_local_increment() -> Task<()> {
        spawn_local(async {
            increment();
        })
    }

    COUNTER.with(|c| *c.borrow_mut() = 0);
    run(async {
        let _t = spwan_local_increment();
        let _t = spwan_local_increment();
        wait_for_idle().await;
    });
    assert_counter(2);

    run(async {
        let _t = spwan_local_increment();
        let _t = spwan_local_increment();
        let _t = spwan_local_increment();
        wait_for_idle().await;
    });
    assert_counter(5);
}
