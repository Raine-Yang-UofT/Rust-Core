use alloc::alloc::Layout;
use super::Locked;
use core::mem;
use alloc::alloc::GlobalAlloc;

use crate::allocator::LinkedListAllocator;

/*
The block sizes we include for fixed-block allocator
The block sizes need to be power of two as required by alignment
For allocations larger than 2048 bytes, we fall back to linkedlist allocator
*/
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

// find the block with the leaset size required by the layout
fn list_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

// the Node of freelist
struct ListNode {
    next: Option<&'static mut ListNode>
}

// the fixed-size block allocator
pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: LinkedListAllocator
}


impl FixedSizeBlockAllocator {
    // create a new empty FixedSizeBlockAllocator
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        FixedSizeBlockAllocator {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: LinkedListAllocator::new()
        }
    }

    // initialize the allocator with given heap bounds
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.fallback_allocator.init(heap_start, heap_size);
    }

    // allocate with fallback allocator
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        unsafe { self.fallback_allocator.allocate(layout) }
    }

}


unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        match list_index(&layout) {
            // the size fits into one of the blocks
            Some(index) => {
                match allocator.list_heads[index].take() {
                    Some(node) => {
                        // remove and return the first node in linkedlist
                        allocator.list_heads[index] = node.next.take();
                        node as *mut ListNode as *mut u8
                    },
                    // the linkedlist is empty
                    None => {
                        let block_size = BLOCK_SIZES[index];
                        let block_align = block_size;
                        let layout = Layout::from_size_align(block_size, block_align).unwrap();
                        /* 
                        use the fallback allocator for this allocation
                        once this block is deallocated, it becomes an available block on list_heads
                        */
                        allocator.fallback_alloc(layout)
                    }
                }
            },
            // the size the too large for any block: use fallback allocator
            None => allocator.fallback_alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();
        match list_index(&layout) {
            // the deallocated size fits in one of the blocks
            Some(index) => {
                let new_node = ListNode {
                    next: allocator.list_heads[index].take()
                };

                // check the block has required size and alignment to store a ListNode
                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);

                // write the new node to heap and append it to the head of corresponding linkedlist
                let new_node_ptr = ptr as *mut ListNode;
                new_node_ptr.write(new_node);
                allocator.list_heads[index] = Some(&mut *new_node_ptr);
            },
            // the size is not one of the blocks, only deallocate from fallback allocator
            None => {
                allocator.fallback_allocator.deallocate(ptr, layout);
            }
        }
    }
}

