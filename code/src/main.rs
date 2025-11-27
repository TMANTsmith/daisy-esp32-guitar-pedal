#![no_std]
#![no_main]


use daisy::hal::delay::Delay;
use code::*;
use daisy::hal::*;
use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use daisy::audio;
use hal::time::U32Ext; // <- this provides `.MHz()` for u32
use hal::adc::{Adc, Enabled, Resolution};
use core::fmt::Write;
use daisy::{pac, hal};
use hal::prelude::*;
use hal::gpio;
use daisy::pac::ADC2;
use daisy::pac::ADC1;


mod _boot {
    use core::arch::global_asm;

    global_asm! (
        ".section .text._start"
    );
}

static AUDIO_INTERFACE: Mutex<RefCell<Option<audio::Interface>>> = Mutex::new(RefCell::new(None));

#[no_mangle]
pub extern "C" fn _main() -> ! {
    // Get core and device peripherals, and the board abstraction.
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = daisy::pac::Peripherals::take().unwrap();
    let board = daisy::Board::take().unwrap();

    // Configure board's peripherals.
    let ccdr = daisy::board_freeze_clocks!(board, dp);
    let mut delay = Delay::new(cp.SYST, ccdr.clocks);

     let pins = daisy::board_split_gpios!(board, ccdr, dp);   
    ///ADC setup
    
    let (adc1, adc2) = daisy::hal::adc::adc12(
        dp.ADC1,
        dp.ADC2,
        4_u32.MHz(),
        &mut delay,
        ccdr.peripheral.ADC12,
        &ccdr.clocks,
    );

    let mut adc1: Adc<ADC1, hal::adc::Enabled> =  adc1.enable();
    adc1.set_resolution(adc::Resolution::SixteenBit);

    let mut adc2: Adc<ADC2, hal::adc::Enabled> =  adc2.enable();
    adc2.set_resolution(adc::Resolution::SixteenBit);


    // Create the Adcs struct using HAL GPIO parts
    let mut adc = Adcs::new(
        adc1,
        adc2,
        pins.GPIO.PIN_15, // pc0
        pins.GPIO.PIN_20, // pc1
        pins.GPIO.PIN_21, // pc4
        pins.GPIO.PIN_16, // pa3
        pins.GPIO.PIN_19, // pa6
        pins.GPIO.PIN_18, // pa7
        pins.GPIO.PIN_17, // pb1
    );
 
    // Read a pin
    let value = adc.read_pin_adc1(22);



    ///UART setup

    ///audio processing setup 
    

    loop {
    }
}



#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {


    }
}
