use rt_local::{runtime::blocking::run, spawn_local, wait_for_idle, Task};
use std::cell::RefCell;

mod test_utils;
mod common {
    mod tests;
}

thread_local! {
    static COUNTER: RefCell<usize> = const { RefCell::new(0) };
}
fn increment() {
    COUNTER.with(|c| *c.borrow_mut() += 1);
}
#[track_caller]
fn assert_counter(value: usize) {
    assert_eq!(COUNTER.with(|c| *c.borrow()), value);
}

#[test]
fn run_repeat() {
    fn spawn_local_increment() -> Task<()> {
        spawn_local(async {
            increment();
        })
    }

    COUNTER.with(|c| *c.borrow_mut() = 0);
    run(async {
        let _t = spawn_local_increment();
        let _t = spawn_local_increment();
        wait_for_idle().await;
    });
    assert_counter(2);

    run(async {
        let _t = spawn_local_increment();
        let _t = spawn_local_increment();
        let _t = spawn_local_increment();
        wait_for_idle().await;
    });
    assert_counter(5);
}
