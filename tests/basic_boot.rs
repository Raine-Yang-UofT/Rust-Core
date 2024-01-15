#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rust_core::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use rust_core::println;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rust_core::test_panic_handler(info)
}


// test cases
#[test_case]
fn test_println() {
    println!("test_println output");
}