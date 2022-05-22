#![cfg(target_os = "windows")]

use derive_ex::derive_ex;
use rt_local::core::{on_idle, RuntimeLoop, RuntimeWaker};
use std::{marker::PhantomData, ops::ControlFlow, sync::Arc};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, PeekMessageW, PostThreadMessageW, TranslateMessage, MSG,
        PM_REMOVE, WM_NULL, WM_QUIT,
    },
};

#[derive_ex(Default)]
#[default(Self::new())]
pub struct WindowsMessageLoop {
    waker: Arc<Waker>,
    _not_send: PhantomData<*mut ()>,
}

impl WindowsMessageLoop {
    pub fn new() -> Self {
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
    fn run<T>(&self, mut on_step: impl FnMut() -> ControlFlow<T>) -> Option<T> {
        loop {
            if let ControlFlow::Break(value) = on_step() {
                return Some(value);
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
                    return None;
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
