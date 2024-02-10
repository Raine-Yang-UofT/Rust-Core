use super::{align_up, Locked};
use core::{mem, ptr, fmt};
use alloc::alloc::{GlobalAlloc, Layout};

use crate::println;

// a node of linkedlist allocator
struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>
}

impl ListNode {
    // Note: const fn: the function return value is evaluated at compile time
    const fn new(size: usize) -> Self {
        ListNode {size, next: None}
    }

    // get the starting memory address of the region
    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    // get the end memory address of the region
    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

// test method: display the linked list node
impl fmt::Display for ListNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "start: {}, end: {}", self.start_addr(), self.end_addr())
    }
}


// the linkedlist allocator
pub struct LinkedListAllocator {
    head: ListNode
}

impl LinkedListAllocator {
    // create an empty LinkedListAllocator
    pub const fn new() -> Self {
        Self {
            // head is a placeholder and does not store heap memory
            // head.next points to the first node that stores heap memory
            head: ListNode::new(0)
        }
    }

    // initialize the allocator with the given heap bound
    // unsafe: the caller needs to ensure the starting address and size are valid
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    // apppend a free region with given starting address and size to the linkedlist
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // ensure the memory is aligned 
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        // ensure the memory is large enough to hold the linkedlist
        assert!(size >= mem::size_of::<ListNode>());

        // the new node
        let mut new_node = ListNode::new(size); // the node is now on stack
        // find the proper location of node in linkedlist
        
        // add node to the head of list
        let mut current = &mut self.head;

        if current.next.is_none() ||
            addr < current.next.as_ref().unwrap().start_addr() {    // the head is None or new node has smaller start address than first node
            new_node.next = current.next.take();
        } else {
            // traverse the linkedlist
            while let Some(ref mut next) = current.next {
                if next.start_addr() >= addr {  
                    break;
                }
                current = current.next.as_mut().unwrap()
            }
            // assign the node's next node
            // we can only assign its previous node when the node is written to memory (having a 'static lifecycle)
            new_node.next = current.next.take();
        }


        let node_ptr = addr as *mut ListNode;
        node_ptr.write(new_node);   // write the new node in memory
        current.next = Some(&mut *node_ptr);    // connect the node with the previous node

    }

    // merge consecutive free memory regions into larger regions
    fn merge_region(&mut self) {
        let mut current = &mut self.head;

        while let Some(ref mut node) = current.next {
            // get the start and end address of current node (prevent borrowing issue)
            let start_addr = node.start_addr();
            let end_addr = node.end_addr();
            // check whether it is adjacent to the next node
            if let Some(ref mut next) = node.next {
                if end_addr == next.start_addr() {
                    // combine two memory regions
                    node.size = next.end_addr() - start_addr;
                    node.next = next.next.take();
                }
            }

            current = current.next.as_mut().unwrap();
        }
    }

    // find a large enough unused heap region 
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        let mut current = &mut self.head;
        // while let: repeatedly execute the code as long as the pattern matching is successful
        // equivalent pseudocode: while current.next == Some
        while let Some(ref mut region) = current.next {
            // find a region that is large enough
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                // remove and return the assigned node from free list
                let next = region.next.take();
                // return the region together with alloc_start address
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // traverse to the next node in linkedlist
                current = current.next.as_mut().unwrap();
            }
        }

        // there is no large enough memory region in heap
        None
    }

    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;   // check for intenger overflow

        // return error if the end address does not fit into the memory region
        if alloc_end > region.end_addr() {
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        // the remaining region need to be large enough to create a new ListNode
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            return Err(());
        }

        Ok(alloc_start)
    }

    // adjust the given layout so the allocated memory can store a ListNode when being deallocated again
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }


    // test method: print the linked list    
    fn print_linkedlist(&self) {
        let mut current = &self.head;
        while let Some(ref node) = current.next {
            println!("{}", node);
            current = current.next.as_ref().unwrap();
        }
    }

}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        // find a node that contains a large enough region
        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            // append a new node in free list to store remaining memory region in the allocation
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                allocator.add_free_region(alloc_end, excess_size);
            }
            alloc_start as *mut u8
        } else {
            // cannot find a memory region with appropriate size
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = LinkedListAllocator::size_align(layout);
        // add the freed region to free list
        self.lock().add_free_region(ptr as usize, size);;
        // merge unused regions
        self.lock().merge_region();
    }

}
