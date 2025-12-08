#![no_std]
#![no_main]

pub mod modules;
use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use daisy::audio;
use modules::*;

static AUDIO_INTERFACE: Mutex<RefCell<Option<audio::Interface>>> = Mutex::new(RefCell::new(None));


#[rtic::app(device = stm32h7xx_hal::pac, peripherals = true)]
mod app {
    use systick_monotonic::*;

    use daisy::audio::Interface;

    #[monotonic(binds = SysTick, default = true)]
    type Mono = Systick<1000>; // 1 kHz / 1 ms granularity

    #[shared]
    struct Shared {
        adc_buffer: [u32; 7],
    }

    // add audio module structs here

    #[local]
    struct Local {
        audio_interface: Interface,
        uart: code::UartCmd,
        gain1: crate::modules::gain::Gain,
        gain2: crate::modules::gain::Gain,
        adc: code::Adcs,

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
        let mut delay = stm32h7xx_hal::delay::Delay::new(cp.SYST, ccdr.clocks);

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

        let mut adc1 =  adc1.enable();
        adc1.set_resolution(daisy::hal::adc::Resolution::SixteenBit);

        let mut adc2 =  adc2.enable();
        adc2.set_resolution(daisy::hal::adc::Resolution::SixteenBit);


        // Create the Adcs struct using HAL GPIO parts


        // pre allocated memory
        let mut adc_buffer: [u32; 7] = [0; 7];

        let mut adc = code::Adcs::new(
            adc1,
            adc2,
            pins.GPIO.PIN_15.into_analog(), 
            pins.GPIO.PIN_16.into_analog(), 
            pins.GPIO.PIN_17.into_analog(), 
            pins.GPIO.PIN_18.into_analog(), 
            pins.GPIO.PIN_19.into_analog(), 
            pins.GPIO.PIN_20.into_analog(), 
            pins.GPIO.PIN_21.into_analog(), 
        );

        // Read a pin
        // let value = adc.read_pin_adc1(22);

        let gain1 = crate::modules::gain::Gain::new(0.5);
        let gain2 = crate::modules::gain::Gain::new(2.0);


        // init uart here
        let tx = pins.GPIO.PIN_13.into_alternate::<7>();
        let rx = pins.GPIO.PIN_14.into_alternate::<7>();

        use daisy::hal::serial::SerialExt;

        let usart = dp
            .USART1
            .serial((tx, rx), 19_200_i32.bps(), ccdr.peripheral.USART1, &ccdr.clocks)
            .unwrap();

        let (tx, rx) = usart.split();


        let mut uart = code::UartCmd::new(tx, rx);

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

        (Shared {
            adc_buffer
        }, 
         Local { audio_interface, 
             gain1, 
             gain2, 
             uart, 
             adc 
         }, 
             init::Monotonics(mono))
    }

    // Audio is tranfered from the input and to the input periodically thorugh DMA.
    // Every time Daisy is done transferring data, it will ask for more by triggering
    // the DMA 1 Stream 1 interrupt.
    #[task(priority = 10, binds = DMA1_STR1, local = [audio_interface, gain1, gain2])]
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

    #[task(priority = 1, binds = USART1, local = [uart])]
    fn uart_read_control(cx: uart_read_control::Context) {
        let uart = cx.local.uart;
        match uart.read_cmd() {
            Ok(_) => {
                uart.write_cmd("ok").ok();
            },
            Err(e) => {
                e.log();
            },
        }
    }

    #[task(priority = 1, binds = DMA2_STR1, shared = [adc_buffer], local = [adc])]
    fn adc_update(cx: adc_update::Context) {
        let adc = cx.local.adc;
            cx.shared.lock(|adc_buffer| {
                //make this work
                adc.read_all(&mut adc_buffer)
            });
            cx.schedule.adc_update(cx.scheduled + 500_000_000.cycles()).unwrap();
    }
}
