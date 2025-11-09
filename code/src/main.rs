#![no_std]
#![no_main]

use core::panic::PanicInfo;
use bcm2837_lpa::mailbox::core_mailbox_write;
use bcm2837_lpa::mmio::asm::sev;
use core::sync::atomic::{AtomicU32, Ordering};

#[no_mangle]

static mut buffer: [(i32, i32); SIZE] = [(0, 0); SIZE];
static ready: AtomicU8 = AtomicU8::new(0); // 0 = not ready, 1 = ready

mod _boot {
    use core::arch::global_asm;

    global_asm! (
        ".section .text._start"
    );
}

extern "C" {
    static __core1_start: u8;
    static __core2_start: u8;
    static __core3_start: u8;
}

pub unsafe fn start_cores() {
    core_mailbox_write(1, &__core1_start as *const u8 as u32);
    core_mailbox_write(2, &__core2_start as *const u8 as u32);
    core_mailbox_write(3, &__core3_start as *const u8 as u32);
    sev(); // Wake up all sleeping cores
}

#[bcm2837_lpa::entry]
fn core0_main() -> ! {
    unsafe { start_cores(); }

    // initalize heap, I2C, I2S, etc here
    loop {
    // do wifi stuff? 
    }
}

#[no_mangle]
pub extern "C" fn _core1_entry() -> ! {
    loop {

        // read form I2S and put in buffer
        while ready.load(Ordering::Acquire) == 0 {
            core::arch::asm!("wfe");
        }
        buffer = read_i2c();
        ready.store(1, Ordering::Release);
    }
}

#[no_mangle]
pub extern "C" fn _core2_entry() -> ! {
    loop {

        while ready.load(Ordering::Acquire) != 1 {
            core::arch::asm!("wfe");
        }

        // do processing here
        let gain = Gain::new(2);
        Gain.process_list(Self, &mut buffer);

    }    
}

#[no_mangle]
pub extern "C" fn _core3_entry() -> ! {
    loop {

        // write output buffer to I2S
        while Buffer_wrote.load(Ordering::Acquire) == 2{
            core::arch::asm!("wfe");
        }
        read_i2s(buffer);
        ready.store(0, Ordering::Release);
    }
}



    // sets up I2C0
    let peripherals = bcm2837_pac::Peripherals::take().unwrap();
    let mut i2c0 = MyI2C { i2c: peripherals.I2C0 };
   
    i2c0.read(decive_addr, &[reg]. &mut buffer).unwrap(); 
    i2c0.write(device_addr, &[reg, byte]).unwrap(); 
    loop {

    let mut buffer: [(f64, f64); 4] = [
            (0.5, 0.5),
            (1.0, 1.0),
            (0.25, 0.75),
            (0.0, 1.0),
        ];
    
    
    let gain = Gain::new(2);
    Gain::process_list(&Gain, &mut buffer);

    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
