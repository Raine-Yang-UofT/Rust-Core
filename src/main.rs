#![no_std]
#![no_main]

// using custom test framework, requiring implementation of test_runner()
#![feature(custom_test_frameworks)]
#![test_runner(rust_core::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use rust_core::{println, eprintln};
use bootloader::{BootInfo, entry_point};
use x86_64::VirtAddr;

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


/*
The program entry specified by the bootloader
*/
entry_point!(kernal_main);

fn kernal_main(boot_info: &'static BootInfo) -> ! {
    rust_core::init();  // initializing kernal

    //  running test cases with cargo test
    #[cfg(test)]
    test_main();

    use rust_core::allocator;
    use rust_core::memory::{self, BootInfoFrameAllocator};

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    extern crate alloc;
    use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};

    // test box
    {
        let heap_value = Box::new(42);
    }

    println!("---");

    {
        let heap_value = vec![1000, 2000, 3000, 4000, 5000, 6000];
    }

    println!("---");
    
    {
        let heap_value = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    }
    


    println!("It did not crash");
    rust_core::hlt_loop();
}
