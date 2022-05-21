use slabmap::SlabMap;
use std::{
    cell::RefCell,
    collections::VecDeque,
    future::Future,
    mem::{replace, swap},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll, Wake, Waker},
};

const ID_NULL: usize = usize::MAX;
const ID_MAIN: usize = usize::MAX - 1;

pub trait RuntimeBackend {
    fn waker(&self) -> Arc<dyn RuntimeWaker>;
}
pub trait RuntimeMainLoop {
    fn waker(&self) -> Arc<dyn RuntimeWaker>;
    fn run(&self, cb: impl RuntimeCallback);
}

pub trait RuntimeWaker: 'static + Send + Sync {
    fn wake(&self);
}
pub trait RuntimeCallback {
    fn on_step(&mut self) -> bool;
    fn on_idle(&mut self) -> bool;
}

pub fn run<T>(main_loop: &impl RuntimeMainLoop, f: impl Future<Output = T>) -> T {
    let runner = Runner::new(main_loop.waker());
    Runtime::enter(&runner.rc);
    runner.rc.push_wake(ID_MAIN);

    let mut cb = RunCallback {
        main: Box::pin(f),
        main_wake: TaskWake::new(ID_MAIN, &runner.rc),
        runner,
        result: None,
    };
    main_loop.run(&mut cb);
    Runtime::leave();
    cb.result.expect("message loop aborted")
}
struct RunCallback<F: Future> {
    main: Pin<Box<F>>,
    main_wake: Arc<TaskWake>,
    runner: Runner,
    result: Option<F::Output>,
}
impl<F: Future> RuntimeCallback for &mut RunCallback<F> {
    fn on_step(&mut self) -> bool {
        while self.runner.ready_requests() {
            for id in self.runner.reqs.wakes.drain(..) {
                if id == ID_MAIN {
                    match self
                        .main
                        .as_mut()
                        .poll(&mut Context::from_waker(&self.main_wake.waker()))
                    {
                        Poll::Ready(value) => {
                            self.result = Some(value);
                            return false;
                        }
                        Poll::Pending => {}
                    }
                } else {
                    run_item(&mut self.runner.rs[id]);
                }
            }
            self.runner.apply_drops();
        }
        true
    }

    fn on_idle(&mut self) -> bool {
        if let Some(on_idle) = Runtime::with(|rt| rt.rc.pop_on_idle()) {
            on_idle.wake();
            true
        } else {
            false
        }
    }
}

thread_local! {
    static RUNNER: RefCell<Option<Runner>> = RefCell::new(None);
}

pub fn enter(backend: impl RuntimeBackend) {
    let runner = Runner::new(backend.waker());
    Runtime::enter(&runner.rc);
    RUNNER.with(|r| *r.borrow_mut() = Some(runner));
}
pub fn leave() {
    RUNNER.with(|r| {
        if r.borrow_mut().take().is_none() {
            panic!("runtime backend is not exists")
        }
    });
    Runtime::leave();
}
pub fn step() {
    RUNNER.with(|r| {
        r.borrow_mut()
            .as_mut()
            .expect("runtime backend is not exists")
            .step()
    });
}

#[must_use]
pub fn spawn_local<Fut: Future + 'static>(fut: Fut) -> Task<Fut::Output> {
    Runtime::with(|rt| {
        let need_wake = rt.rs.is_empty();
        let task = RawTask::new(&rt.rc);
        rt.rs.push(Box::pin(RawRunnable {
            task: task.clone(),
            fut,
        }));
        if need_wake {
            rt.rc.0.waker.wake();
        }
        Task {
            task,
            is_detach: false,
        }
    })
}
pub async fn yield_now() {
    struct YieldNow {
        is_ready: bool,
    }
    impl Future for YieldNow {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.is_ready {
                Poll::Ready(())
            } else {
                self.is_ready = true;
                Runtime::with(|rt| rt.rc.push_on_idle(cx.waker().clone()));
                Poll::Pending
            }
        }
    }

    YieldNow { is_ready: false }.await;
}

#[derive(Clone)]
struct RequestChannel(Arc<RequestsData>);

impl RequestChannel {
    fn new(waker: Arc<dyn RuntimeWaker>) -> Self {
        Self(Arc::new(RequestsData {
            reqs: Mutex::new(RawRequests::new()),
            waker,
        }))
    }
    fn swap(&self, reqs: &mut RawRequests) {
        swap(reqs, &mut *self.0.reqs.lock().unwrap())
    }
    fn push_with(&self, f: impl FnOnce(&mut RawRequests)) {
        let mut d = self.0.reqs.lock().unwrap();
        let call_wake = d.is_empty();
        f(&mut d);
        if call_wake {
            self.0.waker.wake();
        }
    }
    fn push_wake(&self, id: usize) {
        self.push_with(|d| d.wakes.push(id));
    }
    fn push_drop(&self, id: usize) {
        self.push_with(|d| d.drops.push(id));
    }
    fn push_on_idle(&self, waker: Waker) {
        self.push_with(|d| d.on_idle.push_back(waker));
    }
    fn pop_on_idle(&self) -> Option<Waker> {
        self.0.reqs.lock().unwrap().on_idle.pop_front()
    }
}
struct RequestsData {
    waker: Arc<dyn RuntimeWaker>,
    reqs: Mutex<RawRequests>,
}

struct RawRequests {
    wakes: Vec<usize>,
    drops: Vec<usize>,
    on_idle: VecDeque<Waker>,
}

impl RawRequests {
    fn new() -> Self {
        Self {
            wakes: Vec::new(),
            drops: Vec::new(),
            on_idle: VecDeque::new(),
        }
    }
    fn is_empty(&self) -> bool {
        self.wakes.is_empty() && self.drops.is_empty() && self.on_idle.is_empty()
    }
}

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = RefCell::new(None);
}

struct Runtime {
    rc: RequestChannel,
    rs: Vec<Pin<Box<dyn DynRunnable>>>,
}

impl Runtime {
    fn new(rc: RequestChannel) -> Self {
        Self { rc, rs: Vec::new() }
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
    fn with<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        RUNTIME.with(|rt| f(rt.borrow_mut().as_mut().expect("runtime is not running")))
    }
}

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
            TaskState::Finished => panic!("`poll` called twice."),
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

struct RawRunnable<Fut: Future> {
    task: Arc<RawTask<Fut::Output>>,
    fut: Fut,
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
                let fut = Pin::new_unchecked(&mut this.fut);
                if let Poll::Ready(value) = fut.poll(&mut Context::from_waker(waker)) {
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
    reqs: RawRequests,
    rs: SlabMap<Option<Runnable>>,
}

impl Runner {
    fn new(waker: Arc<dyn RuntimeWaker>) -> Self {
        Self {
            rc: RequestChannel::new(waker),
            reqs: RawRequests::new(),
            rs: SlabMap::new(),
        }
    }
    fn ready_requests(&mut self) -> bool {
        self.rc.swap(&mut self.reqs);
        Runtime::with(|rt| {
            for r in rt.rs.drain(..) {
                self.reqs.wakes.push(
                    self.rs
                        .insert_with_key(|id| Some(Runnable::new(r, id, &self.rc))),
                );
            }
        });
        !self.reqs.is_empty()
    }
    fn apply_drops(&mut self) {
        for id in self.reqs.drops.drain(..) {
            self.rs.remove(id);
        }
    }

    fn step(&mut self) {
        while self.ready_requests() {
            for id in self.reqs.wakes.drain(..) {
                run_item(&mut self.rs[id]);
            }
            self.apply_drops();
        }
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
