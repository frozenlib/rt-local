use crate::runtime::{on_idle, RuntimeLoop, RuntimeWaker};
use std::sync::{Arc, Condvar, Mutex};

pub struct MainLoop(Arc<Waker>);

struct Waker {
    is_wake: Mutex<bool>,
    cv: Condvar,
}

impl MainLoop {
    pub fn new() -> Self {
        Self(Arc::new(Waker {
            is_wake: Mutex::new(true),
            cv: Condvar::new(),
        }))
    }
}

impl Default for MainLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeLoop for MainLoop {
    fn waker(&self) -> Arc<dyn RuntimeWaker> {
        self.0.clone()
    }
    fn run(&self, mut on_step: impl FnMut() -> bool) {
        let mut is_wake = self.0.is_wake.lock().unwrap();
        loop {
            if *is_wake {
                *is_wake = false;
                drop(is_wake);
                loop {
                    if !on_step() {
                        return;
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
