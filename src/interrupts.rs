use x86_64::structures::idt::{
    InterruptDescriptorTable, 
    InterruptStackFrame, 
    PageFaultErrorCode};
use crate::{println, eprintln, gdt, hlt_loop};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin;


use crate::print;

/*
The default PIC interrupt vector numbers are 0-15, which
have been used by CPU exceptions. We need to map PIC numbers
to 32-47 for PIC
*/
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;


// the index for each hardware interrupt
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET + 1
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}


// singleton initialization of primary and secondary PIC
pub static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(
    unsafe {
        ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)
    }
);


// singleton initialization of IDT
/*
Interruption Description Table (IDT) is used to store pointers
to handler functions for each type of interruption
*/
lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();  // create an IDTc

        /*
        register interrupt handlers to IDT
        */
        // add handler of breakpoint interrupt
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        // add handler of double fault
        unsafe {    
            // use the interruption stack for handling double fault
            idt.double_fault.set_handler_fn(double_fault_handler)
                            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        // add handler of page fault
        idt.page_fault.set_handler_fn(page_fault_handler);
        // add handler of timer interrupt
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        // add handler of keyboard interrupt
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}


pub fn init_idt() {
    IDT.load();
}

/*
List of interrupt handlers
*/

// CPU exceptions

// the handler for breakpoint interruption
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

// the handler for double fault
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame, _error_code: u64) -> !
{
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

// the handler for page fault
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode)
{
    use x86_64::registers::control::Cr2;

    eprintln!("EXCEPTION: PAGE FAULT");
    eprintln!("Accessed Address: {:?}", Cr2::read());
    eprintln!("Error Code: {:?}", error_code);
    eprintln!("{:#?}", stack_frame);
    hlt_loop();
}


// hardware interrupts

// the handler for timer interrupt
extern "x86-interrupt" fn timer_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    // print!(".");

    // send end of interrupt (EOI) signal to interrupt handler
    /*
    The interrupt controller needs an explicit EOI signal from interrupt handler
    Otherwise, it is waiting for the current interrupt to be handled
     */
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame) 
{
    use x86_64::instructions::port::Port;
    use spin::Mutex;
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

    // singleton initialization of converter from scan code to key
    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore));
    }
    
    /*
    Read scancode from the IO port for PS/2 controller
    The keyboard controller would not send another interrupt before 
    we read the scancode
     */
    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    // add scancode to scancode queue
    crate::task::keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}


// test cases
#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}