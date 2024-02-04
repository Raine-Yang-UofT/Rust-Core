use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB
    },
    VirtAddr
};
use linked_list_allocator::LockedHeap;

// custom allocators
pub mod bump_allocator;
pub mod linked_list;

use bump_allocator::BumpAllocator;
use linked_list::LinkedListAllocator;

// the virtual memory allocated for the heap
pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024;    // the current heap size is 100 KB


// Locked is initially created to implement allocators, but it can have other uses as well
// we can use Mutex in lock to convert &self to &mut self
// We cannot implement traits for spin::Mutex directly due to orphand rule, so we need to implement a NewType
pub struct Locked<T> {
    inner: spin::Mutex<T>
}

impl<T> Locked<T> {
    pub const fn new(inner: T) -> Self {
        Locked {
            inner: spin::Mutex::new(inner)
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<T> {
        self.inner.lock()
    }
}


// Align the given address upwards to alignment "align"
// alignmenmt: place data at addresses that are multiples of a specific size
fn align_up(addr: usize, align: usize) -> usize {
    /*
    let remainder = addr % align;
    if remainder == 0 {
        addr
    } else {
        addr - remainder + align
    }
    */
    // a much faster implementation with same effect as commented code above
    (addr + align - 1) & !(align - 1)
}


// initialize heap with given page table mapper and heap memory allocator
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>
) -> Result<(), MapToError<Size4KiB>> {
    // get the range of pages to allocate
    let page_range = {
        // convert heap start and (inclusive) end constants to virtual addresses
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        // get the pages that contain the virtual addresses
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        // create a range of pages between start and end address
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        // allocate a physical frame
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        // set the (virtual) page to be PRESENT and WRITABLE
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            // create the page table mapping from the page to the physical frame
            // flush() add the page to translation lookaside buffer
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
        };
    }

    // assign the newly allocated memory to heap allocator
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}



// linked_list_allocator crate: 
// static ALLOCATOR: LockedHeap = LockedHeap::empty();

// bump allocator:
//static ALLOCATOR: Locked<BumpAllocator> = Locked::new(BumpAllocator::new());

// linkedlist allocator:
#[global_allocator]
static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());