#![no_std]
#![no_main]

use core::panic::PanicInfo;
use bcm2837_pac::Peripherals;

mod _boot {
    use core::arch::global_asm;

    global_asm! (
        ".section .text._start"
    );
}


//keep _start st the top
#[no_mangle]
pub extern "C" fn _start() -> ! {

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
