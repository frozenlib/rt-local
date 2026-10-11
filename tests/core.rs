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

#[test]
fn mutable_borrow_across_await() {
    run(async {
        spawn_local(async {
            let mut value = 0;
            let borrowed = &mut value;
            let mut first_poll = true;
            // Suspend once so the runtime polls a future containing a live mutable borrow again.
            std::future::poll_fn(|cx| {
                if std::mem::replace(&mut first_poll, false) {
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                } else {
                    std::task::Poll::Ready(())
                }
            })
            .await;
            *borrowed += 1;
            assert_eq!(*borrowed, 1);
        })
        .await;
    });
}
