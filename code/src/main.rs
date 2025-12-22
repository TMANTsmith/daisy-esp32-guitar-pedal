#![no_std]
#![no_main]

use cortex_m::interrupt::Mutex;
pub mod modules;
use core::cell::RefCell;
use daisy::audio;
use defmt_rtt as _;
use panic_probe as _; // enables the panic handler // optional: RTT transport for defmt

// Optional global, if needed
static AUDIO_INTERFACE: Mutex<RefCell<Option<audio::Interface>>> = Mutex::new(RefCell::new(None));
static SAMPLE: usize = 48_000_usize;

#[link_section = ".sdram"]
static mut DELAY_BUF: [(f32, f32); SAMPLE * 5_usize] = [(0.0_f32, 0.0_f32); SAMPLE * 5_usize]; // for 5 sec delay

#[rtic::app(
    device = daisy::pac,
    peripherals = true,
    dispatchers = [EXTI0]
)]
mod app {

    use crate::modules::gain::Gain;
    use code::UartError;
    use daisy::audio::Interface;
    use daisy::hal::dma::{Circular, Stream1, StreamsTuple, Transfer, DMA2};
    use daisy::hal::serial::SerialExt;
    use daisy::hal::time::U32Ext;
    use fugit::ExtU32; // for .millis()
    use fugit::RateExtU32; // for ADC .MHz()

    #[shared]
    struct Shared {
        adc_dma: Transfer<Stream1<DMA2>, Adc<ADC1>, Circular, &'static mut [u16; 128]>,
    }

    #[local]
    struct Local {
        audio_interface: Interface,
        uart: code::UartCmd,
        gain1: Gain,
        gain2: Gain,
        adc: code::Adcs,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        // Use RTIC-provided peripherals
        let mut cp = cx.core;
        let dp = cx.device;

        // Take the board once
        let board = daisy::Board::take().expect("board take error");

        // Initialize clocks and GPIOs once
        let ccdr = daisy::board_freeze_clocks!(board, dp);
        let pins = daisy::board_split_gpios!(board, ccdr, dp);

        // Initialize audio interface once
        let audio_interface = daisy::board_split_audio!(ccdr, pins)
            .spawn()
            .expect("audio interface spawn error");

        // Initialize ADCs
        let mut delay = daisy::hal::delay::Delay::new(cp.SYST, ccdr.clocks);
        let (adc1, adc2) = daisy::hal::adc::adc12(
            dp.ADC1,
            dp.ADC2,
            4_u32.MHz(),
            &mut delay,
            ccdr.peripheral.ADC12,
            &ccdr.clocks,
        );

        let mut adc1 = adc1.enable();
        adc1.set_resolution(daisy::hal::adc::Resolution::SixteenBit);
        let mut adc2 = adc2.enable();
        adc2.set_resolution(daisy::hal::adc::Resolution::SixteenBit);

        // Initialize Adcs struct
        static mut ADC_BUF: [u32; 7] = [0; 7];
        let streams = StreamsTuple::new(dp.DMA2);
        let dma_stream = streams.1;
        let mut adc_dma = Transfer::init_peripheral_to_memory_circular(
            dma_stream,
            adc1,
            // will can be accessed by one thing so its ok
            // it still can cause problems of somthing else tries to access it
            // use peak() to access with read only
            unsafe { &mut ADC_BUF },
            None,
        );

        adc_dma.start(|_adc1| {});

        let adc = code::Adcs::new(
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

        // Initialize gains
        let gain1 = Gain::new(0.5);
        let gain2 = Gain::new(2.0);

        // Initialize UART
        let tx = pins.GPIO.PIN_13.into_alternate::<7>();
        let rx = pins.GPIO.PIN_14.into_alternate::<7>();

        let temp_uart = dp
            .USART1
            .serial(
                (tx, rx),
                19_200_u32.bps(),
                ccdr.peripheral.USART1,
                &ccdr.clocks,
            )
            .expect("usart init error");

        // enable interupts

        let (tx, rx) = temp_uart.split();
        let uart = code::UartCmd::new(tx, rx);

        // Enable caches
        cp.SCB.enable_icache();
        cp.SCB.enable_dcache(&mut cp.CPUID);

        (
            Shared { adc_dma },
            Local {
                audio_interface,
                gain1,
                gain2,
                uart,
                adc,
            },
        )
    }

    // DSP interrupt handler
    #[task(priority = 10, binds = DMA1_STR1, local = [audio_interface, gain1, gain2])]
    fn dsp(cx: dsp::Context) {
        cx.local
            .audio_interface
            .handle_interrupt_dma1_str1(|audio_buffer| {
                for frame in audio_buffer {
                    cx.local.gain1.process(frame);
                    cx.local.gain2.process(frame);
                }
            })
            .expect("audio dsp init error");
    }

    #[task(binds = UART0, priority = 1)]
    fn uart_read_trigger(_: uart_read_trigger::Context) {
        uart_read::spawn().expect("uart spawn error");
    }

    #[task(priority = 1, local = [uart])]
    async fn uart_read(cx: uart_read::Context) {
        match cx.local.uart.read_cmd() {
            Ok(m) => {
                let message = m;
            }
            Err(UartError::WouldBlock) => {
                defmt::warn!("Blocking error trying again...");
                uart_read::spawn().expect("uart spawn error");
            }
            Err(e) => {
                defmt::warn!("Uart Error: {}", e);
            }
        }
    }

    //if not make this sofware and make it scedule itself in the furture
    #[task(priority = 1, binds = DMA2_STR1, shared = [adc_dma], local = [adc])]
    fn adc_update(mut cx: adc_update::Context) {
        cx.shared.adc_dma.lock(|dma| {
            dma.clear_interrupts();
            // use dma.peek() to get values
        });
    }
}
