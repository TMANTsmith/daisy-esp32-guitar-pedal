#![no_std]
#![no_main]


use cortex_m_rt::entry;
mod modules;
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

#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};


static AUDIO_INTERFACE: Mutex<RefCell<Option<audio::Interface>>> = Mutex::new(RefCell::new(None));


#[rtic::app(device = stm32h7xx_hal::pac, peripherals = true)]
mod app {
    use systick_monotonic::*;

    use daisy::audio::Interface;

    #[monotonic(binds = SysTick, default = true)]
    type Mono = Systick<1000>; // 1 kHz / 1 ms granularity

    #[shared]
    struct Shared {}

    // add audio module structs here

    #[local]
    struct Local {
        audio_interface: Interface,
        gain1: Gain,
        gain2: Gain,
        usart: UartCmd,
        adc: Adcs,

    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // init audio modules here
        // Get core and device peripherals, and the board abstraction.
        let cp = cortex_m::Peripherals::take().unwrap();
        let dp = daisy::pac::Peripherals::take().unwrap();
        let board = daisy::Board::take().unwrap();

        // Configure board's peripherals.
        let ccdr = daisy::board_freeze_clocks!(board, dp);
        let mut delay = Delay::new(cp.SYST, ccdr.clocks);

        let pins = daisy::board_split_gpios!(board, ccdr, dp);

        let audio_interface = daisy::board_split_audio!(ccdr, pins);

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
        // let value = adc.read_pin_adc1(22);

        gain1 = Gain::new(0.5);
        gain2 = Gain::new(2.0);


        // init uart here
        let tx = pins.GPIO.PIN_13.into_alternate::<7>();
        let rx = pins.GPIO.PIN_14.into_alternate::<7>();

        let usart = dp
            .USART1
            .serial((tx, rx), 19_200.bps(), ccdr.peripheral.USART1, &ccdr.clocks)
            .unwrap();

        let (tx, rx) = usart.split();

        let mut uart = UartCmd::new(tx, rx);

        // Get device peripherals.
        let mut cp = cx.core;
        let dp = cx.device;

        // Using caches should provide a major performance boost.
        cp.SCB.enable_icache();
        // NOTE: Data caching requires cache management around all use of DMA.
        // This crate already handles that for audio processing.
        cp.SCB.enable_dcache(&mut cp.CPUID);

        // Initialize the board abstraction.
        let board = daisy::Board::take().unwrap();

        // Configure board's peripherals.
        let ccdr = daisy::board_freeze_clocks!(board, dp);
        let pins = daisy::board_split_gpios!(board, ccdr, dp);
        let audio_interface = daisy::board_split_audio!(ccdr, pins);

        // Start audio processing and put its abstraction into a global.
        let audio_interface = audio_interface.spawn().unwrap();

        // Initialize monotonic timer.
        let mono = Systick::new(cp.SYST, ccdr.clocks.sys_ck().to_Hz());

        (Shared {}, Local { audio_interface, gain1, gain2, uart, adc }, init::Monotonics(mono))
    }

    // Audio is tranfered from the input and to the input periodically thorugh DMA.
    // Every time Daisy is done transferring data, it will ask for more by triggering
    // the DMA 1 Stream 1 interrupt.
    #[task(binds = DMA1_STR1, local = [audio_interface, gain1, gain2])]
    fn dsp(cx: dsp::Context) {
        let audio_interface = cx.local.audio_interface;

        audio_interface
            .handle_interrupt_dma1_str1(|audio_buffer| {
                for frame in audio_buffer {

                    //add audio modules here
                    cx.local.gain1.process(frame);
                    cx.local.gain2.process(frame);

                }
            })
            .unwrap();
    }
    
    #[task(binds = USART1, local = [uart])]
    fn uart_read_control(cx: uart_read_control::Context) {
        uart = cx.local.uart;
        if let Ok(command) = uart.read_cmd();
        uart.write_cmd("ok");
    }
}

