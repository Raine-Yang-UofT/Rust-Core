use core::{future::Future, pin::Pin};
use core::task::{Context, Poll};
use core::sync::atomic::{AtomicU64, Ordering};
use alloc::boxed::Box;


pub mod simple_executor;    // a dummy executor for testing
pub mod executor;      // the task executor
pub mod keyboard;    // handle keyboard scancodes.

// a unique id for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        // generate a unique id for a new task
        // use atomic operating to ensure thread safety
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

// a newtype for pinned Future trait object with no return value
pub struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()>>>
}

impl Task {
    // create a new task from future
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            id: TaskId::new(),
            future: Box::pin(future)
        }
    }

    // poll the future stored in task
    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}