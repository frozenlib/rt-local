use rt_local_core::base::{on_idle, RuntimeLoop, RuntimeWaker};
use std::{future::Future, marker::PhantomData, ops::ControlFlow, sync::Arc};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, PeekMessageW, PostThreadMessageW, TranslateMessage, MSG,
        PM_REMOVE, WM_NULL, WM_QUIT,
    },
};

/// Executes the specified future and blocks until it completes.
pub fn run<F: Future>(future: F) -> F::Output {
    rt_local_core::base::run(&WindowsMessageLoop::new(), future)
}

pub use rt_local_macros::windows_main as main;
pub use rt_local_macros::windows_test as test;

struct WindowsMessageLoop {
    waker: Arc<Waker>,
    _not_send: PhantomData<*mut ()>,
}

impl WindowsMessageLoop {
    fn new() -> Self {
        unsafe {
            let thread_id = GetCurrentThreadId();
            Self {
                waker: Arc::new(Waker { thread_id }),
                _not_send: PhantomData,
            }
        }
    }
}
impl RuntimeLoop for WindowsMessageLoop {
    fn waker(&self) -> Arc<dyn RuntimeWaker> {
        self.waker.clone()
    }
    fn run<T>(&self, mut on_step: impl FnMut() -> ControlFlow<T>) -> T {
        loop {
            if let ControlFlow::Break(value) = on_step() {
                return value;
            }
            let mut msg = MSG::default();
            unsafe {
                if !PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).as_bool() {
                    if on_idle() {
                        continue;
                    } else {
                        GetMessageW(&mut msg, HWND(0), 0, 0).ok().unwrap();
                    }
                }
                if msg.message == WM_QUIT {
                    panic!("message loop terminated");
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

struct Waker {
    thread_id: u32,
}
impl RuntimeWaker for Waker {
    fn wake(&self) {
        unsafe {
            PostThreadMessageW(self.thread_id, WM_NULL, WPARAM(0), LPARAM(0));
        }
    }
}
