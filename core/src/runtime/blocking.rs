use crate::base::{idle, EventLoop};
use std::{
    future::Future,
    ops::ControlFlow,
    sync::{Arc, Condvar, Mutex},
    task::Wake,
};

/// Executes the specified future and blocks until it completes.
pub fn run<T>(future: impl Future<Output = T>) -> T {
    crate::base::run(&BlockingEventLoop::new(), future)
}

struct BlockingEventLoop(Arc<Waker>);

struct Waker {
    is_wake: Mutex<bool>,
    cv: Condvar,
}

impl BlockingEventLoop {
    pub fn new() -> Self {
        Self(Arc::new(Waker {
            is_wake: Mutex::new(true),
            cv: Condvar::new(),
        }))
    }
}

impl Default for BlockingEventLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl EventLoop for BlockingEventLoop {
    fn waker(&self) -> std::task::Waker {
        self.0.clone().into()
    }
    fn run<T>(&self, mut poll: impl FnMut() -> ControlFlow<T>) -> T {
        let mut is_wake = self.0.is_wake.lock().unwrap();
        loop {
            is_wake = if *is_wake {
                *is_wake = false;
                drop(is_wake);
                while {
                    if let ControlFlow::Break(value) = poll() {
                        return value;
                    }
                    idle()
                } {}
                self.0.is_wake.lock().unwrap()
            } else {
                self.0.cv.wait(is_wake).unwrap()
            }
        }
    }
}

impl Wake for Waker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        let mut is_wake = self.is_wake.lock().unwrap();
        if !*is_wake {
            *is_wake = true;
            self.cv.notify_all();
        }
    }
}
