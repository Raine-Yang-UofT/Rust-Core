/*
Implementation of a bump allocator

A bump allocator allocates heap linearly, with a pointer pointing to the next page
to be allocated. The pointer moves forward whether a new allocation is assigned and
never moves backward

It has a counter that increases by 1 on each allocation and decreases by 1 on each deallocation
When the counter reaches zero, the whole heap can be reused.

Bump allocator is very efficient compared with other allocators
A severe drawback of bump allocator is that it can only free the memory all at once
*/
use alloc::alloc::{GlobalAlloc, Layout};
use super::{align_up, Locked};
use core::ptr;


pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize
}

impl BumpAllocator {
    // create a new empty allocator
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0
        }
    }

    // note: we create a seperate init function to make the interface identitical wih linked_list_allocator
    // so we can change allocator without code change

    // initialize the bump allocator with given heap bounds
    // the caller needs to ensure the validity of heap_start and heap_size, 
    // and the method should only be called once
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}


// implement GlobalAlloc trait for BumpAllocator
// Note: GlobalAlloc only allows &self instead of &mut self, since 
// global heap allocator is a static variable, which is immutable
unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock();
        let alloc_start = align_up(bump.next, layout.align());  // round up to the next address
        let alloc_end = match alloc_start.checked_add(layout.size()) {  // .checked_add: prevent integer overflow
            Some(end) => end,
            None => return ptr::null_mut()
        };

        if alloc_end > bump.heap_end {  // out of memory
            ptr::null_mut()
        } else {
            // update allocator and return pointer to allocated heap region
            bump.next = alloc_end;
            bump.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut bump = self.lock();

        bump.allocations -= 1;

        // optimization: if the deallocated object is the last allocated one, 
        // we can move back bump.next to reuse space
        if ptr as usize + layout.size() == bump.next {
            bump.next = ptr as usize
        }
        

        // reuse memory only when all heap objects are dropped
        if bump.allocations == 0 {
            bump.next = bump.heap_start;
        }
    }
}