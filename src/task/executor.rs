use super::{Task, TaskId};
use alloc::{collections::BTreeMap, sync::Arc};
use alloc::task::Wake;
use core::task::{Waker, Context, Poll};
use crossbeam_queue::ArrayQueue;

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>
}


struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>
}

impl TaskWaker {
    // the waker pushes task back to queue once it is ready to be polled again
    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task queue full");
    }

    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue
        }))
    }
}

impl Wake for TaskWaker {

    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}


impl Executor {
    // create a new executor with maximum 100 tasks in queue
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(), // use a B-tree to store tasks
            task_queue: Arc::new(ArrayQueue::new(100)),  // task queue stores task ids
            waker_cache: BTreeMap::new()    // store wakers in a tree for reuse
        }
    }

    // spawn a new task
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        // check whether a task with same id exists in queue
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
        // push task into queue
        self.task_queue.push(task_id).expect("queue full");
    }

    fn run_ready_tasks(&mut self) {
        // use destruction to avoid borrowing issues
        // when we want to mutable borrow each attribute seperately
        let Self {
            tasks,
            task_queue,
            waker_cache
        } = self;

        while let Ok(task_id) = task_queue.pop() {
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue    // task no longer exist
            };

            // create a waker that pushes task to task queue once finished
            // we use waker cache to store and reuse wakers
            // note: by using Arc, task_queue.clone() only copies a reference
            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));
            let mut context = Context::from_waker(waker);
            
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }


    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    // pause the CPU if the task queue is empty
    // the CPU is halt until the next interrupt
    fn sleep_if_idle(&self) {
        if self.task_queue.is_empty() {
            use x86_64::instructions::interrupts::{self, enable_and_hlt};

            // temporarily disable interrupt to prevent an interrupt from occuring after 
            // if condition and before hlt
            interrupts::disable();
            if self.task_queue.is_empty() {
                enable_and_hlt();
            } else {
                interrupts::enable();
            }
        }
    }
}