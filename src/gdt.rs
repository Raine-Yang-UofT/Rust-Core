use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use lazy_static::lazy_static;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};
use x86_64::structures::gdt::SegmentSelector;

// use stack 0 at IST to handle double fault
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;


// singleton initialization of TSS
/*
TSS contains pointer to up to 7 interruption stacks,
which can be used to handle stackoverflow
*/
lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        // create the stack for double fault
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] ={
            /*
            NOTE:
            This is only a temporary solution. We haven't implemented memory management
            and we just allocate an array as the stack
             */
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe{&STACK});
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss 
    };
}


// specific which GDT and TSS the CPU should use
struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector
}

// singletone initialization of global descriptor table
lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));  // select the custom TSS
        (gdt, Selectors {code_selector, tss_selector})
    };
}

pub fn init() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, Segment};

    GDT.0.load();   // load GDT
    unsafe {
        CS::set_reg(GDT.1.code_selector);   // load kernal code segment
        load_tss(GDT.1.tss_selector);   // load our custom TSS
    }
}