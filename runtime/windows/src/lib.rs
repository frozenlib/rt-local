#![cfg(target_os = "windows")]

use rt_local_core::base::{idle, EventLoop};
use std::{
    cell::Cell, future::Future, marker::PhantomData, ops::ControlFlow, sync::Arc, task::Wake,
};
use windows::{
    core::Interface,
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        System::Threading::GetCurrentThreadId,
        UI::{
            TextServices::{ITfKeystrokeMgr, ITfMessagePump, ITfThreadMgr},
            WindowsAndMessaging::{
                DispatchMessageW, GetMessageW, PeekMessageW, PostThreadMessageW, TranslateMessage,
                MSG, PM_REMOVE, WM_KEYDOWN, WM_KEYUP, WM_NULL, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
            },
        },
    },
};

thread_local! {
    static USE_TSF_KEYSTROKE_MGR: Cell<bool> = const { Cell::new(true) };
}

/// Options for [`run_with_options`].
#[derive(Clone, Default)]
pub struct RunOptions {
    /// Thread manager used for Text Services Framework integration.
    ///
    /// The caller is responsible for COM initialization and for activating and deactivating the
    /// thread manager.
    pub tsf_thread_mgr: Option<ITfThreadMgr>,
}

/// Executes the specified future and blocks until it completes.
pub fn run<T>(future: impl Future<Output = T>) -> T {
    run_with_options(future, &RunOptions::default())
}

/// Executes the specified future with options and blocks until it completes.
pub fn run_with_options<T>(future: impl Future<Output = T>, options: &RunOptions) -> T {
    rt_local_core::base::run(&WindowsEventLoop::new(options), future)
}

/// Sets whether [`ITfKeystrokeMgr`] is used on this thread.
pub fn set_use_tsf_keystroke_mgr(value: bool) {
    USE_TSF_KEYSTROKE_MGR.set(value);
}

struct WindowsEventLoop {
    waker: Arc<Waker>,
    tsf: Option<Tsf>,
    _not_send: PhantomData<*mut ()>,
}

impl WindowsEventLoop {
    fn new(options: &RunOptions) -> Self {
        unsafe {
            let thread_id = GetCurrentThreadId();
            Self {
                waker: Arc::new(Waker { thread_id }),
                tsf: options
                    .tsf_thread_mgr
                    .as_ref()
                    .map(Tsf::new)
                    .transpose()
                    .unwrap(),
                _not_send: PhantomData,
            }
        }
    }

    unsafe fn peek_message(&self, msg: &mut MSG) -> bool {
        if let Some(tsf) = &self.tsf {
            let mut result = false.into();
            tsf.message_pump
                .PeekMessageW(msg, HWND::default(), 0, 0, PM_REMOVE.0, &mut result)
                .unwrap();
            result.as_bool()
        } else {
            PeekMessageW(msg, None, 0, 0, PM_REMOVE).as_bool()
        }
    }

    unsafe fn get_message(&self, msg: &mut MSG) {
        if let Some(tsf) = &self.tsf {
            let mut result = false.into();
            tsf.message_pump
                .GetMessageW(msg, HWND::default(), 0, 0, &mut result)
                .unwrap();
        } else {
            GetMessageW(msg, None, 0, 0).ok().unwrap();
        }
    }

    unsafe fn handle_tsf_keystroke(&self, msg: &MSG) -> bool {
        if !USE_TSF_KEYSTROKE_MGR.get() {
            return false;
        }

        let Some(tsf) = &self.tsf else {
            return false;
        };

        match msg.message {
            WM_KEYDOWN | WM_SYSKEYDOWN
                if tsf
                    .keystroke_mgr
                    .TestKeyDown(msg.wParam, msg.lParam)
                    .unwrap()
                    .as_bool() =>
            {
                tsf.keystroke_mgr
                    .KeyDown(msg.wParam, msg.lParam)
                    .unwrap()
                    .as_bool()
            }
            WM_KEYUP | WM_SYSKEYUP
                if tsf
                    .keystroke_mgr
                    .TestKeyUp(msg.wParam, msg.lParam)
                    .unwrap()
                    .as_bool() =>
            {
                tsf.keystroke_mgr
                    .KeyUp(msg.wParam, msg.lParam)
                    .unwrap()
                    .as_bool()
            }
            _ => false,
        }
    }
}
impl EventLoop for WindowsEventLoop {
    fn waker(&self) -> std::task::Waker {
        self.waker.clone().into()
    }
    fn run<T>(&self, mut poll: impl FnMut() -> ControlFlow<T>) -> T {
        loop {
            if let ControlFlow::Break(value) = poll() {
                return value;
            }
            let mut msg = MSG::default();
            unsafe {
                if !self.peek_message(&mut msg) {
                    if idle() {
                        continue;
                    } else {
                        self.get_message(&mut msg);
                    }
                }
                if msg.message == WM_QUIT {
                    panic!("message loop terminated");
                }
                if !self.handle_tsf_keystroke(&msg) {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }
}

struct Tsf {
    message_pump: ITfMessagePump,
    keystroke_mgr: ITfKeystrokeMgr,
}
impl Tsf {
    fn new(tsf_thread_mgr: &ITfThreadMgr) -> windows::core::Result<Self> {
        Ok(Self {
            message_pump: tsf_thread_mgr.cast()?,
            keystroke_mgr: tsf_thread_mgr.cast()?,
        })
    }
}

struct Waker {
    thread_id: u32,
}
impl Wake for Waker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_NULL, WPARAM(0), LPARAM(0));
        }
    }
}
