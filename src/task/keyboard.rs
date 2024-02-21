use conquer_once::spin::OnceCell;   // similar to lazy static, but prevent initialization in the interrupt handler
use crossbeam_queue::ArrayQueue;
use core::{pin::Pin, task::{Poll, Context}};
use futures_util::stream::{Stream, StreamExt};
use futures_util::task::AtomicWaker;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

use crate::print;
use crate::println;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

static WAKER: AtomicWaker = AtomicWaker::new();

// add a scancode to queue
// pub(crate): function only available for lib.rs
pub(crate) fn add_scancode(scancode: u8) {
    // get a reference of scancode queue
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        // push scancode to queue
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();   // notify the waker
        }
    } else {
        println!("WARNING: scancode queue uninitialized");
    }
}


pub struct ScancodeStream {
    _private: ()    // prevent construction of the struct from outside (not calling new)
}

impl ScancodeStream {
    pub fn new() -> Self {
        // initialize singleton scancode queue
        SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    /*
    Stream trait is similar to Future, but it returns multiple Ready(Some(Item))
    when called repeatedly and returns Ready(None) if no remaining items.

    Similar to an iterator
     */
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE.try_get().expect("scancode queue not initialized");
        
        // returns immediately if the scancode queue is not empty
        if let Ok(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }
        
        // register waker if the queue is potentially empty
        WAKER.register(&cx.waker());    // register waker
        match queue.pop() {
            Ok(scancode) => {   // scancode exist in queue
                WAKER.take();
                Poll::Ready(Some(scancode))
            }   
            Err(crossbeam_queue::PopError) => Poll::Pending     // no scancode received
        }
    }
}


pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {     // asynchronously read the next key in scancode stream
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                    DecodedKey::Unicode(character) => print!("{}", character)
                }
            }
        }
    }
}
