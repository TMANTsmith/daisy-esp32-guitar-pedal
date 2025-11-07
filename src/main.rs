#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod _boot {
    use core::arch::global_asm;

    global_asm! (
        ".section .text._start"
    );
}


//keep _start st the top
#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}

#[panic_handaler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
