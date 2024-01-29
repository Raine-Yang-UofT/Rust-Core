use x86_64::{
    structures::paging::{
        PageTable, OffsetPageTable, PhysFrame, Size4KiB, FrameAllocator
    },
    structures::paging::page_table::FrameError,
    VirtAddr,
    PhysAddr,
    registers::control::Cr3
};

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

/*
Initialize a new OffsetPageTable
*/
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    // retrieve a reference to level 4 page table
    let level_4_table = active_level_4_table(physical_memory_offset);
    // create offset page table
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}


// return a mutable reference to the level 4 page table
/*
We retrieve the physical address of level 4 page table from Cr3 register through bootloader
Then, the physical address is mapped to the corresponding virtual address
*/
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)
    -> &'static mut PageTable
{
    let (level_4_table_frame, _) = Cr3::read();  // read the address from cr3 register

    let phys = level_4_table_frame.start_address();  // read the physical address
    let virt = physical_memory_offset + phys.as_u64();    // convert physical address to virtual address
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();     // create a pointer to virtual address

    &mut *page_table_ptr    // unsafe: dereference raw pointer
}


// create a FrameAllocator from memory map passed from bootloader
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize
}

impl BootInfoFrameAllocator {
    // initialize a BootInfoFrame Allocator from memory map
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0
        }
    }

    // return an iterator containing all usable frames
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // convert memory map to iterator
        let regions = self.memory_map.iter();
        // only keep the unused mappings in memory map
        let usable_regions = regions
            .filter(|r| r.region_type == MemoryRegionType::Usable);
        // convert each MemoryRegion to a Range from start address to end address
        let addr_ranges = usable_regions
            .map(|r| r.range.start_addr()..r.range.end_addr());
        // convert each Range to an iterator with step size 4096 (the page size)
        // this operation converts a consecutive memory to discrete pages
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // convert each page in the iterator to physical frame
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // find usable physical frames from memory map and retrieve the frame with index self.next
        /*
        this method requires recreating memory map every time, which is inefficient
        however, we cannot store a type impl Iterator by now
        Perhaps after we implement heap allocation we can use Box?
        */
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}


/*
Note:
This function is written only for learning purpose
x86_64 had provided abstractions for converting virtual address to physical address
OffsetPageTable type is used when we map the complete physical memory to virtual memory
(which is our case)
OffsetPageTable provides mapping for both normal pages and huge pages
*/
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr)
    -> Option<PhysAddr>
{
    translate_addr_inner(addr, physical_memory_offset)
}

fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr)
    -> Option<PhysAddr>
{
    // read physical address of level 4 table from Cr3 register
    let (level_4_table_frame, _) = Cr3::read();

    // calculate the corresponding indexes at each level of page table based on the given virtual address
    /*
    In x86_64 the page table index for each level is derived directly from virtual address
    Virtual address (64 bits):
        0-12    page offset
        12-21   level 1 index 
        21-30   level 2 index
        30-39   level 3 index
        39-48   level 4 index
        48-64   sign-extension (copies of 47th bit)
    */
    let table_indexes = [
        addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index()
    ];

    // start from level 4 page frame
    let mut frame = level_4_table_frame;

    // iterate through 4 levels of page tables to find the physical address
    for &index in &table_indexes {
        // access the virtual address of the next-level page table
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };  // retrieve the next-level page table

        let entry = &table[index];
        // get the frame from the page table with given index
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported")
        };
    }

    // translate the virtual address to physical address
    Some(frame.start_address() + u64::from(addr.page_offset()))
}
