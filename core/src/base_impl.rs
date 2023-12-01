use slabmap::SlabMap;
use std::{
    cell::RefCell,
    future::Future,
    mem::{replace, swap},
    ops::ControlFlow,
    pin::{pin, Pin},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll, Wake, Waker},
};

const ID_NULL: usize = usize::MAX;
const ID_MAIN: usize = usize::MAX - 1;

pub trait EventLoop {
    fn waker(&self) -> Waker;
    fn run<T>(&self, poll: impl FnMut() -> ControlFlow<T>) -> T;
}

/// Execute asynchronous runtime that blocks the current thread.
///
/// If not blocking current thread, use [`enter`] and [`leave`] instead.
pub fn run<F: Future>(l: &impl EventLoop, future: F) -> F::Output {
    let mut runner = Runner::new(l.waker());
    Runtime::enter(&runner.rc);
    runner.rc.push_wake(ID_MAIN);

    let mut main = pin!(future);
    let main_wake = TaskWake::new(ID_MAIN, &runner.rc);
    let value = l.run(|| {
        runner.ready_requests();
        for id in runner.wakes.drain(..) {
            if id == ID_MAIN {
                match main
                    .as_mut()
                    .poll(&mut Context::from_waker(&main_wake.waker()))
                {
                    Poll::Ready(value) => return ControlFlow::Break(value),
                    Poll::Pending => {}
                }
            } else {
                run_item(&mut runner.rs[id]);
            }
        }
        runner.apply_drops();
        ControlFlow::Continue(())
    });
    Runtime::leave();
    value
}

thread_local! {
    static RUNNER: RefCell<Option<Runner>> = RefCell::new(None);
}

/// Init asynchronous runtime without blocking the current thread.
///
/// When ending asynchronous runtime, it is necessary to call [`leave`].
pub fn enter(waker: Waker) {
    let runner = Runner::new(waker);
    Runtime::enter(&runner.rc);
    RUNNER.with(|r| *r.borrow_mut() = Some(runner));
}

/// Finish asynchronous runtime initiated by [`enter`].
pub fn leave() {
    let runner = RUNNER.with(|r| r.borrow_mut().take().expect("runtime is not exists"));
    Runtime::leave();
    drop(runner);
}

/// Call [`poll`](std::future::Future::poll) of futures started by [`spawn_local`].
///
/// `Future::poll` is called for all futures that were woken up before the call to this function.
///
/// `Future::poll` is not called for futures that were woken up during the call to this function.
pub fn poll() {
    call_runner(|r| r.poll())
}

fn call_runner<T>(f: impl FnOnce(&mut Runner) -> T) -> T {
    RUNNER
        .with(|r| r.borrow_mut().as_mut().map(f))
        .expect("runtime is not start by `enter()`")
}

/// Awaken one of the waiting futures created by [`wait_for_idle`].
///
/// If there are tasks waken up, does nothing and returns true.
///
/// If there are no tasks woken up, resumes all tasks suspended by wait_for_idle, and returns true if one or more tasks have been resumed.
pub fn idle() -> bool {
    Runtime::with(|rt| rt.wake_idles())
}

/// Spawn a future on the current thread.
///
/// # Panics
///
/// Panics if the runtime is not running.
#[must_use]
#[track_caller]
pub fn spawn_local<F: Future + 'static>(future: F) -> Task<F::Output> {
    Runtime::with(|rt| {
        let need_wake = rt.rs.is_empty();
        let task = RawTask::new(&rt.rc);
        rt.rs.push(Box::pin(RawRunnable {
            task: task.clone(),
            future,
        }));
        if need_wake {
            rt.rc.0.waker.wake_by_ref();
        }
        Task {
            task,
            is_detach: false,
        }
    })
}

/// Wait until there are no more operations to be performed now on the current thread.
///
/// The "operations to be performed now" include not only tasks spawned by [`spawn_local`], but also events handled by the runtime backend.
pub async fn wait_for_idle() {
    struct WaitForIdle {
        is_ready: bool,
    }
    impl Future for WaitForIdle {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.is_ready {
                Poll::Ready(())
            } else {
                self.is_ready = true;
                Runtime::with(|rt| rt.rc.push_idle_waker(cx.waker().clone()));
                Poll::Pending
            }
        }
    }

    WaitForIdle { is_ready: false }.await;
}

#[derive(Clone)]
struct RequestChannel(Arc<RequestsData>);

impl RequestChannel {
    fn new(waker: Waker) -> Self {
        Self(Arc::new(RequestsData {
            reqs: Mutex::new(RawRequests::new()),
            waker,
        }))
    }
    fn get_wakes_drops(&self, wakes: &mut Vec<usize>, drops: &mut Vec<usize>) {
        assert!(wakes.is_empty());
        assert!(drops.is_empty());
        let mut reqs = self.0.reqs.lock().unwrap();
        swap(wakes, &mut reqs.wakes);
        swap(drops, &mut reqs.drops);
    }
    fn get_idles(&self, idles: &mut Vec<Waker>) {
        assert!(idles.is_empty());
        swap(idles, &mut self.0.reqs.lock().unwrap().idles);
    }

    fn push_with(&self, f: impl FnOnce(&mut RawRequests)) {
        let mut d = self.0.reqs.lock().unwrap();
        let call_wake = d.is_empty();
        f(&mut d);
        if call_wake {
            self.0.waker.wake_by_ref();
        }
    }
    fn push_wake(&self, id: usize) {
        self.push_with(|d| d.wakes.push(id));
    }
    fn push_drop(&self, id: usize) {
        self.push_with(|d| d.drops.push(id));
    }
    fn push_idle_waker(&self, waker: Waker) {
        self.push_with(|d| d.idles.push(waker));
    }
}

struct RequestsData {
    waker: Waker,
    reqs: Mutex<RawRequests>,
}

struct RawRequests {
    wakes: Vec<usize>,
    drops: Vec<usize>,
    idles: Vec<Waker>,
}

impl RawRequests {
    fn new() -> Self {
        Self {
            wakes: Vec::new(),
            drops: Vec::new(),
            idles: Vec::new(),
        }
    }
    fn is_empty(&self) -> bool {
        self.wakes.is_empty() && self.drops.is_empty() && self.idles.is_empty()
    }
}

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = RefCell::new(None);
}

struct Runtime {
    rc: RequestChannel,
    rs: Vec<Pin<Box<dyn DynRunnable>>>,
    idles: Vec<Waker>,
}

impl Runtime {
    fn new(rc: RequestChannel) -> Self {
        Self {
            rc,
            rs: Vec::new(),
            idles: Vec::new(),
        }
    }
    fn enter(rc: &RequestChannel) {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            if rt.is_some() {
                panic!("runtime is already running");
            }
            *rt = Some(Runtime::new(rc.clone()));
        })
    }
    fn leave() {
        RUNTIME.with(|rt| rt.borrow_mut().take());
    }
    fn wake_idles(&mut self) -> bool {
        if !self.rs.is_empty() {
            return true;
        }
        self.rc.get_idles(&mut self.idles);
        if self.idles.is_empty() {
            return false;
        }
        for waker in self.idles.drain(..) {
            waker.wake();
        }
        true
    }
    #[track_caller]
    fn with<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        RUNTIME
            .with(|rt| rt.borrow_mut().as_mut().map(f))
            .expect("runtime is not running")
    }
}

/// A spawned task.
///
/// When a [`Task`] is dropped, the asynchronous operation is canceled.
///
/// To drop a task without canceling, it is necessary to call [`Task::detach()`].
pub struct Task<T> {
    task: Arc<RawTask<T>>,
    is_detach: bool,
}

struct RawTask<T> {
    state: Mutex<TaskState<T>>,
    reqs: RequestChannel,
}

enum TaskState<T> {
    Running { id: usize, waker: Option<Waker> },
    Cancelled,
    Completed(T),
    Finished,
}

impl<T> Task<T> {
    /// Drop a task without canceling.
    pub fn detach(mut self) {
        self.is_detach = true;
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if !self.is_detach {
            let mut state = self.task.state.lock().unwrap();
            if let &TaskState::Running { id, .. } = &*state {
                *state = TaskState::Cancelled;
                if id != ID_NULL {
                    self.task.reqs.push_wake(id);
                }
            }
        }
    }
}
impl<T> Future for Task<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.task.state.lock().unwrap();
        match &*state {
            &TaskState::Running { id, .. } => {
                *state = TaskState::Running {
                    id,
                    waker: Some(cx.waker().clone()),
                };
                Poll::Pending
            }
            TaskState::Cancelled => Poll::Pending,
            TaskState::Completed(_) => {
                if let TaskState::Completed(value) = replace(&mut *state, TaskState::Finished) {
                    Poll::Ready(value)
                } else {
                    unreachable!()
                }
            }
            TaskState::Finished => panic!("`poll` called twice"),
        }
    }
}

impl<T> RawTask<T> {
    fn new(rc: &RequestChannel) -> Arc<Self> {
        Arc::new(RawTask {
            state: Mutex::new(TaskState::Running {
                id: ID_NULL,
                waker: None,
            }),
            reqs: rc.clone(),
        })
    }
    fn complete(&self, value: T) {
        if let TaskState::Running {
            waker: Some(waker), ..
        } = replace(
            &mut *self.state.lock().unwrap(),
            TaskState::Completed(value),
        ) {
            waker.wake()
        }
    }
    fn is_cancelled(&self) -> bool {
        matches!(&*self.state.lock().unwrap(), TaskState::Cancelled)
    }
}

trait DynRunnable {
    fn set_id(self: Pin<&Self>, id: usize);
    fn run(self: Pin<&mut Self>, waker: &Waker) -> bool;
}

struct RawRunnable<F: Future> {
    task: Arc<RawTask<F::Output>>,
    future: F,
}
impl<Fut: Future> DynRunnable for RawRunnable<Fut> {
    fn set_id(self: Pin<&Self>, id: usize) {
        if let TaskState::Running { id: id_, .. } = &mut *self.task.state.lock().unwrap() {
            *id_ = id;
        }
    }
    fn run(self: Pin<&mut Self>, waker: &Waker) -> bool {
        if self.task.is_cancelled() {
            false
        } else {
            unsafe {
                let this = self.get_unchecked_mut();
                let f = Pin::new_unchecked(&mut this.future);
                if let Poll::Ready(value) = f.poll(&mut Context::from_waker(waker)) {
                    this.task.complete(value);
                    false
                } else {
                    true
                }
            }
        }
    }
}

struct Runner {
    rc: RequestChannel,
    wakes: Vec<usize>,
    drops: Vec<usize>,
    rs: SlabMap<Option<Runnable>>,
}

impl Runner {
    fn new(waker: Waker) -> Self {
        Self {
            rc: RequestChannel::new(waker),
            wakes: Vec::new(),
            drops: Vec::new(),
            rs: SlabMap::new(),
        }
    }
    fn ready_requests(&mut self) {
        self.rc.get_wakes_drops(&mut self.wakes, &mut self.drops);
        Runtime::with(|rt| {
            for r in rt.rs.drain(..) {
                self.wakes.push(
                    self.rs
                        .insert_with_key(|id| Some(Runnable::new(r, id, &self.rc))),
                );
            }
        });
    }
    fn apply_drops(&mut self) {
        for id in self.drops.drain(..) {
            self.rs.remove(id);
        }
    }

    fn poll(&mut self) {
        self.ready_requests();
        for id in self.wakes.drain(..) {
            run_item(&mut self.rs[id]);
        }
        self.apply_drops();
    }
}

struct Runnable {
    wake: Arc<TaskWake>,
    r: Pin<Box<dyn DynRunnable>>,
}

impl Runnable {
    fn new(r: Pin<Box<dyn DynRunnable>>, id: usize, rc: &RequestChannel) -> Self {
        r.as_ref().set_id(id);
        Self {
            wake: TaskWake::new(id, rc),
            r,
        }
    }
    fn run(&mut self) -> bool {
        self.r.as_mut().run(&self.wake.waker())
    }
}
fn run_item(r: &mut Option<Runnable>) {
    if let Some(runnable) = r {
        if !runnable.run() {
            r.take();
        }
    }
}

struct TaskWake {
    id: usize,
    is_wake: AtomicBool,
    rc: RequestChannel,
}

impl TaskWake {
    fn new(id: usize, rc: &RequestChannel) -> Arc<Self> {
        Arc::new(TaskWake {
            id,
            is_wake: AtomicBool::new(true),
            rc: rc.clone(),
        })
    }
    fn waker(self: &Arc<Self>) -> Waker {
        self.is_wake.store(false, Ordering::SeqCst);
        self.clone().into()
    }
}

impl Wake for TaskWake {
    fn wake(self: Arc<Self>) {
        if !self.is_wake.swap(true, Ordering::SeqCst) {
            self.rc.push_wake(self.id)
        }
    }
}
impl Drop for TaskWake {
    fn drop(&mut self) {
        self.rc.push_drop(self.id);
    }
}
