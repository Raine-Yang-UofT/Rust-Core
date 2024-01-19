#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

// import crates and make them public from crate root
pub mod serial;
pub mod vga_buffer;
pub mod interrupts;
pub mod gdt;


/*
Testable trait
*/
pub trait Testable {
    fn run(&self) -> ();
}

/*
implement Testable for any function.
The run method contains the standard test message printed by serial
and calls the function
*/
impl<T> Testable for T where T: Fn() {
    fn run(&self) {
        serial_print!("{}\t", core::any::type_name::<T>());
        self();
        serial_println!("[OK]");
    }
}

// running test cases
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }

    // exit qemu after running tests
    exit_qemu(QemuExitCode::Success);
}

/*
test_panic_handler is called when a panic happens
during test mode
*/ 
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}


// exit the kernel once the test completes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
// status code: 33 success, 34 failed
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11
}

/*
Send exit_code to iobase indicated in isa-debug-exit config
to exit qemu
*/
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}


// initialization
pub fn init() {
    gdt::init();    // initialize gdt
    interrupts::init_idt();  // initialize interruptions
    unsafe {interrupts::PICS.lock().initialize()}   // initialize PIC
    x86_64::instructions::interrupts::enable();     // enable interrupt controller for CPU 
}



#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    init();
    test_main();
    loop {}
}

// In test mode: call test_panic_handler when a panic happens (test failed)
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}


pub fn hlt_loop() -> ! {
    loop {
        // hlt: halt the CPU until the next interrupt
        x86_64::instructions::hlt();
    }
}