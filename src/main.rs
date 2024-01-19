#![no_std]
#![no_main]

// using custom test framework, requiring implementation of test_runner()
#![feature(custom_test_frameworks)]
#![test_runner(rust_core::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use rust_core::println;
use rust_core::eprintln;


/*
panic handler for non-test configuration (cargo run)

Print the panic message and wait indefinitely
*/
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    eprintln!("{}", info);
    rust_core::hlt_loop();
}


/*
panic handler for test configuration (cargo test)

invoke test_panic_handler from lib.rs
*/
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rust_core::test_panic_handler(info)
}


#[no_mangle]    // disable function renaming during compile
/*
The program entry
*/
pub extern "C" fn _start() -> ! {
    rust_core::init();  // initializing kernal

    //  running test cases with cargo test
    #[cfg(test)]
    test_main();

    rust_core::hlt_loop();
}
