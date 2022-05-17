use crate::{MessageLoop, MessageLoopWaker};
use std::sync::{Arc, Condvar, Mutex};

pub struct EmptyMessageLoop(Arc<Waker>);

struct Waker {
    is_wake: Mutex<bool>,
    cv: Condvar,
}

impl EmptyMessageLoop {
    pub fn new() -> Self {
        Self(Arc::new(Waker {
            is_wake: Mutex::new(true),
            cv: Condvar::new(),
        }))
    }
}

impl Default for EmptyMessageLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageLoop for EmptyMessageLoop {
    fn waker(&self) -> Arc<dyn MessageLoopWaker> {
        self.0.clone()
    }

    fn run(&self, mut f: impl FnMut() -> bool) {
        let mut is_wake = self.0.is_wake.lock().unwrap();
        loop {
            if *is_wake {
                *is_wake = false;
                drop(is_wake);
                if !f() {
                    return;
                }
                is_wake = self.0.is_wake.lock().unwrap()
            } else {
                is_wake = self.0.cv.wait(is_wake).unwrap();
            }
        }
    }
}

impl MessageLoopWaker for Waker {
    fn wake(&self) {
        let mut is_wake = self.is_wake.lock().unwrap();
        if !*is_wake {
            *is_wake = true;
            self.cv.notify_all();
        }
    }
}
