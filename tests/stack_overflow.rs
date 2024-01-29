#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use rust_core::{exit_qemu, QemuExitCode, serial_print, serial_println};


// create a new IDT for testing
lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault.set_handler_fn(test_double_fault_handler)
            .set_stack_index(rust_core::gdt::DOUBLE_FAULT_IST_INDEX);
        }   // use the interrupt handler for handling double fault

        idt
    };
}

// handle double fault, quit qemu with success code
extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64
) -> ! {
    serial_println!("[OK]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}

// load the test IDT
pub fn init_test_idt() {
    TEST_IDT.load();
}


#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_print!("stack_overflow::stack_overflow \t");
    rust_core::gdt::init(); // initiaze GDT
    init_test_idt();
    stack_overflow();   // calling stack overflow
    panic!("Execution continued after stack overflow")
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rust_core::test_panic_handler(info)
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow();
    volatile::Volatile::new(0).read();  // prevent compiler optimization
}

