use futures_core::future::LocalBoxFuture;
use slabmap::SlabMap;
use std::{
    cell::RefCell,
    future::Future,
    mem::swap,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll, Wake, Waker},
};

pub trait MessageLoop {
    type Waker: MessageLoopWaker;
    fn waker(&self) -> Self::Waker;
    fn run(&self, f: impl FnMut() -> bool);
}
pub trait MessageLoopWaker: 'static + Send + Sync {
    fn wake(&self);
}
pub fn run<T>(message_loop: &impl MessageLoop, mut fut: impl Future<Output = T>) -> T {
    Runtime::enter(Box::new(message_loop.waker()));

    let requests = Requests::new();
    let mut fut = unsafe { Pin::new_unchecked(&mut fut) };
    let fut_id = usize::MAX;
    let fut_wake = TaskWake::new(message_loop.waker(), fut_id, &requests);
    requests.push_wake(fut_id);

    let mut reqs = RawRequests::new();
    let mut runners = SlabMap::new();
    let mut result = None;

    message_loop.run(|| loop {
        requests.swap(&mut reqs);
        Runtime::with(|rt| {
            for task in rt.tasks_new.drain(..) {
                reqs.wakes.push(runners.insert_with_key(|id| {
                    TaskRunner::new(TaskWake::new(message_loop.waker(), id, &requests), task)
                }));
            }
        });
        if reqs.is_empty() {
            return true;
        }
        for id in reqs.wakes.drain(..) {
            if id == fut_id {
                match fut
                    .as_mut()
                    .poll(&mut Context::from_waker(&fut_wake.waker()))
                {
                    Poll::Ready(value) => {
                        result = Some(value);
                        return false;
                    }
                    Poll::Pending => {}
                }
            } else {
                runners[id].run();
            }
        }
        for id in reqs.drops.drain(..) {
            runners.remove(id);
        }
    });
    Runtime::leave();
    result.expect("message loop aborted")
}
pub fn spawn_local(fut: impl Future<Output = ()> + 'static) {
    Runtime::with(|rt| {
        let need_wake = rt.tasks_new.is_empty();
        rt.tasks_new.push(Box::pin(fut));
        if need_wake {
            rt.waker.wake();
        }
    });
}

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = RefCell::new(None);
}

#[derive(Clone)]
struct Requests(Arc<Mutex<RawRequests>>);

impl Requests {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(RawRequests::new())))
    }
    fn swap(&self, other: &mut RawRequests) {
        swap(&mut *self.0.lock().unwrap(), other);
    }
    fn push_wake(&self, id: usize) -> bool {
        let mut l = self.0.lock().unwrap();
        let is_empty = l.is_empty();
        l.wakes.push(id);
        is_empty
    }
    fn push_drop(&self, id: usize) -> bool {
        let mut l = self.0.lock().unwrap();
        let is_empty = l.is_empty();
        l.drops.push(id);
        is_empty
    }
}

struct RawRequests {
    wakes: Vec<usize>,
    drops: Vec<usize>,
}

impl RawRequests {
    fn new() -> Self {
        Self {
            wakes: Vec::new(),
            drops: Vec::new(),
        }
    }
    fn is_empty(&self) -> bool {
        self.wakes.is_empty() && self.drops.is_empty()
    }
}

struct Runtime {
    waker: Box<dyn MessageLoopWaker>,
    tasks_new: Vec<LocalBoxFuture<'static, ()>>,
}

impl Runtime {
    fn new(waker: Box<dyn MessageLoopWaker>) -> Self {
        Self {
            waker,
            tasks_new: Vec::new(),
        }
    }
    fn enter(waker: Box<dyn MessageLoopWaker>) {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            if rt.is_some() {
                panic!("message loop is already running");
            }
            *rt = Some(Runtime::new(waker));
        })
    }
    fn leave() {
        RUNTIME.with(|rt| rt.borrow_mut().take());
    }
    fn with<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        RUNTIME.with(|rt| {
            f(rt.borrow_mut()
                .as_mut()
                .expect("message loop is not running"))
        })
    }
}
struct TaskRunner<W: MessageLoopWaker>(Option<RawTaskRunner<W>>);

impl<W: MessageLoopWaker> TaskRunner<W> {
    fn new(wake: Arc<TaskWake<W>>, task: LocalBoxFuture<'static, ()>) -> Self {
        Self(Some(RawTaskRunner { wake, task }))
    }
    fn run(&mut self) {
        if let Some(task) = &mut self.0 {
            if task
                .task
                .as_mut()
                .poll(&mut Context::from_waker(&task.wake.waker()))
                .is_ready()
            {
                self.0.take();
            }
        }
    }
}

struct RawTaskRunner<W: MessageLoopWaker> {
    wake: Arc<TaskWake<W>>,
    task: LocalBoxFuture<'static, ()>,
}

struct TaskWake<W: MessageLoopWaker> {
    waker: W,
    id: usize,
    is_wake: AtomicBool,
    requests: Requests,
}

impl<W: MessageLoopWaker> TaskWake<W> {
    fn new(waker: W, id: usize, requests: &Requests) -> Arc<Self> {
        Arc::new(TaskWake {
            waker,
            id,
            is_wake: AtomicBool::new(true),
            requests: requests.clone(),
        })
    }
    fn waker(self: &Arc<Self>) -> Waker {
        self.is_wake.store(false, Ordering::SeqCst);
        self.clone().into()
    }
}

impl<W: MessageLoopWaker> Wake for TaskWake<W> {
    fn wake(self: Arc<Self>) {
        if !self.is_wake.swap(true, Ordering::SeqCst) && self.requests.push_wake(self.id) {
            self.waker.wake();
        }
    }
}
impl<W: MessageLoopWaker> Drop for TaskWake<W> {
    fn drop(&mut self) {
        if self.requests.push_drop(self.id) {
            self.waker.wake();
        }
    }
}
