use slabmap::SlabMap;
use std::{
    cell::RefCell,
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

pub trait MessageLoop {
    fn waker(&self) -> Arc<dyn MessageLoopWaker>;
    fn run(&self, f: impl FnMut() -> bool);
}
pub trait MessageLoopWaker: 'static + Send + Sync {
    fn wake(&self);
}
pub fn run<T>(message_loop: &impl MessageLoop, mut fut: impl Future<Output = T>) -> T {
    let requests = Requests::new(message_loop.waker());
    Runtime::enter(&requests);
    let mut fut = unsafe { Pin::new_unchecked(&mut fut) };
    let fut_id = ID_MAIN;
    let fut_wake = TaskWake::new(fut_id, &requests);
    requests.push_wake(fut_id);

    let mut reqs = RawRequests::new();
    let mut rs = SlabMap::new();
    let mut result = None;

    message_loop.run(|| loop {
        requests.swap(&mut reqs);
        Runtime::with(|rt| {
            for r in rt.rs.drain(..) {
                reqs.wakes
                    .push(rs.insert_with_key(|id| Some(Runnable::new(r, id, &requests))));
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
            } else if let Some(r) = &mut rs[id] {
                if !r.run() {
                    rs[id].take();
                }
            }
        }
        for id in reqs.drops.drain(..) {
            rs.remove(id);
        }
    });
    Runtime::leave();
    result.expect("message loop aborted")
}
#[must_use]
pub fn spawn_local<Fut: Future + 'static>(fut: Fut) -> Task<Fut::Output> {
    Runtime::with(|rt| {
        let need_wake = rt.rs.is_empty();
        let task = RawTask::new(&rt.reqs);
        rt.rs.push(Box::pin(RawRunnable {
            task: task.clone(),
            fut,
        }));
        if need_wake {
            rt.reqs.0.waker.wake();
        }
        Task {
            task,
            is_detach: false,
        }
    })
}

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = RefCell::new(None);
}

#[derive(Clone)]
struct Requests(Arc<RequestsData>);

impl Requests {
    fn new(waker: Arc<dyn MessageLoopWaker>) -> Self {
        Self(Arc::new(RequestsData {
            reqs: Mutex::new(RawRequests::new()),
            waker,
        }))
    }
    fn swap(&self, reqs: &mut RawRequests) {
        swap(reqs, &mut *self.0.reqs.lock().unwrap())
    }
    fn push_wake(&self, id: usize) {
        let mut l = self.0.reqs.lock().unwrap();
        let is_wake = l.is_empty();
        l.wakes.push(id);
        if is_wake {
            self.0.waker.wake();
        }
    }
    fn push_drop(&self, id: usize) {
        let mut l = self.0.reqs.lock().unwrap();
        let is_wake = l.is_empty();
        l.drops.push(id);
        if is_wake {
            self.0.waker.wake();
        }
    }
}
struct RequestsData {
    waker: Arc<dyn MessageLoopWaker>,
    reqs: Mutex<RawRequests>,
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
    reqs: Requests,
    rs: Vec<Pin<Box<dyn DynRunnable>>>,
}

impl Runtime {
    fn new(reqs: Requests) -> Self {
        Self {
            reqs,
            rs: Vec::new(),
        }
    }
    fn enter(requests: &Requests) {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            if rt.is_some() {
                panic!("message loop is already running");
            }
            *rt = Some(Runtime::new(requests.clone()));
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

pub struct Task<T> {
    task: Arc<RawTask<T>>,
    is_detach: bool,
}

struct RawTask<T> {
    state: Mutex<TaskState<T>>,
    reqs: Requests,
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
    fn new(requests: &Requests) -> Arc<Self> {
        Arc::new(RawTask {
            state: Mutex::new(TaskState::Running {
                id: ID_NULL,
                waker: None,
            }),
            reqs: requests.clone(),
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

struct Runnable {
    wake: Arc<TaskWake>,
    r: Pin<Box<dyn DynRunnable>>,
}

impl Runnable {
    fn new(r: Pin<Box<dyn DynRunnable>>, id: usize, requests: &Requests) -> Self {
        r.as_ref().set_id(id);
        Self {
            wake: TaskWake::new(id, requests),
            r,
        }
    }
    fn run(&mut self) -> bool {
        self.r.as_mut().run(&self.wake.waker())
    }
}

struct TaskWake {
    id: usize,
    is_wake: AtomicBool,
    requests: Requests,
}

impl TaskWake {
    fn new(id: usize, requests: &Requests) -> Arc<Self> {
        Arc::new(TaskWake {
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

impl Wake for TaskWake {
    fn wake(self: Arc<Self>) {
        if !self.is_wake.swap(true, Ordering::SeqCst) {
            self.requests.push_wake(self.id)
        }
    }
}
impl Drop for TaskWake {
    fn drop(&mut self) {
        self.requests.push_drop(self.id);
    }
}
