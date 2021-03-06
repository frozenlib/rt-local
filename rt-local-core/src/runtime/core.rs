use crate::base::{on_idle, RuntimeLoop, RuntimeWaker};
use std::{
    future::Future,
    ops::ControlFlow,
    sync::{Arc, Condvar, Mutex},
};

/// Executes the specified future and blocks until it completes.
pub fn run<F: Future>(future: F) -> F::Output {
    crate::base::run(&NoFrameworkRuntimeLoop::new(), future)
}

struct NoFrameworkRuntimeLoop(Arc<Waker>);

struct Waker {
    is_wake: Mutex<bool>,
    cv: Condvar,
}

impl NoFrameworkRuntimeLoop {
    pub fn new() -> Self {
        Self(Arc::new(Waker {
            is_wake: Mutex::new(true),
            cv: Condvar::new(),
        }))
    }
}

impl Default for NoFrameworkRuntimeLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeLoop for NoFrameworkRuntimeLoop {
    fn waker(&self) -> Arc<dyn RuntimeWaker> {
        self.0.clone()
    }
    fn run<T>(&self, mut on_step: impl FnMut() -> ControlFlow<T>) -> T {
        let mut is_wake = self.0.is_wake.lock().unwrap();
        loop {
            if *is_wake {
                *is_wake = false;
                drop(is_wake);
                loop {
                    if let ControlFlow::Break(value) = on_step() {
                        return value;
                    }
                    if !on_idle() {
                        break;
                    }
                }
                is_wake = self.0.is_wake.lock().unwrap()
            } else {
                is_wake = self.0.cv.wait(is_wake).unwrap();
            }
        }
    }
}

impl RuntimeWaker for Waker {
    fn wake(&self) {
        let mut is_wake = self.is_wake.lock().unwrap();
        if !*is_wake {
            *is_wake = true;
            self.cv.notify_all();
        }
    }
}
