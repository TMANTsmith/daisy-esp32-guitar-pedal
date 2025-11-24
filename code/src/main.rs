#![no_std]
#![no_main]


// Daisy
use daisy::prelude::*;
use daisy::audio::{Audio, StereoFrame};

// stm32h7xx-hal
use stm32h7xx_hal::{
    pac,
    prelude::*,
    adc::{Adc, AdcConfig},
    serial::{Serial, Config, Tx, Rx},
    gpio::{Analog, Alternate},
};

// Others
use num_enum::TryFromPrimitive;
use core::convert::TryFrom;
use core::panic::PanicInfo;



mod _boot {
    use core::arch::global_asm;

    global_asm! (
        ".section .text._start"
    );
}



#[no_mangle]
pub extern "C" fn _main() -> ! {
    // Get core and device peripherals, and the board abstraction.
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = daisy::pac::Peripherals::take().unwrap();
    let board = daisy::Board::take().unwrap();

    // Configure board's peripherals.
    let ccdr = daisy::board_freeze_clocks!(board, dp);
    let pins = daisy::board_split_gpios!(board, ccdr, dp);
    let mut delay = hal::delay::Delay::new(cp.SYST, ccdr.clocks);

    let audio_interface = daisy::board_split_audio!(ccdr, pins);
    
    ///ADC setup
    let acd_0 = Adcs::new(&delay, &dp, &ccdr);

    // Example: read ADC6, which is PC4
    let mut adc_pin = adcs.pin_from_number(&pins, 15);

    let value: u32 = adcs.read_pin(&mut adc_pin);

    ///UART setup
    let uart = uart_init(pins.GPIO.PIN_12, pins.GPIO.PIN_11, &ccdr.clocks, ccdr.peripheral.USART1);

    uart.send_cmd("ping");

    ///audio processing setup 
    let gain = Gain::new(1.5);
    fn prosesser(input: (f32, f32)) -> (f32, f32) {
        gain.process(input)
    }
    
    let sound = Sound::new(&audio_interface);

    loop {
        sound.process(processer);
        // do processing on buffer here
    }
}



#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {


    }
}
