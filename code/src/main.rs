#![no_std]
#![no_main]

pub mod uart;
use heapless::Vec;
use cortex_m::interrupt::Mutex;
pub mod modules;
pub mod adc;
use core::cell::RefCell;
use daisy::audio;
use defmt_rtt as _;
use panic_probe as _; // enables the panic handler // optional: RTT transport for defmt

// Optional global, if needed
static AUDIO_INTERFACE: Mutex<RefCell<Option<audio::Interface>>> = Mutex::new(RefCell::new(None));
const SAMPLE: usize = 48_000_usize;

#[link_section = ".sdram"]
static mut DELAY_BUF: [(f32, f32); SAMPLE * 5_usize] = [(0.0_f32, 0.0_f32); SAMPLE * 5_usize]; 
// for 5 sec delay

static mut ADC_BUF: [u16; 7] = [0; 7];

#[rtic::app(
    device = daisy::pac,
    peripherals = true,
    dispatchers = [EXTI0]
)]
mod app {

    use crate::modules::bit_crush::BitCrush;
    use crate::modules::gain::Gain;
    use daisy::audio::Interface;
    use daisy::hal::serial::SerialExt;
    use daisy::hal::time::U32Ext;
    use fugit::ExtU32; // for .millis()
    use fugit::RateExtU32; // for ADC .MHz()
    use crate::adc::Adcs;
    // pub use crate::uart::UartCmd;


    #[shared]
    struct Shared {
        adc_snap: [u16; 7],
    }

    #[local]
    struct Local {
        adc_dma: ???,
        audio_interface: Interface,
        uart_write: UartCmd,
        uart_read: UartCmd,
        gain1: Gain,
        gain2: Gain,
        bit_crush: BitCrush,
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
        let adc1 = daisy::hal::adc::adc1(
            dp.ADC1,
            4_u32.MHz(),
            &mut delay,
            ccdr.peripheral.ADC12,
            &ccdr.clocks,
        );
        adc.configure_scan(Scan::Enabled);

        let dma2 = dp.DMA2.split();
        let ch = dma2.3;

        let mut adc_dma = Transfer::init(
            ch,                         // DMA channel
            adc,                        // Peripheral (ADC)
            unsafe { &mut ADC_BUF },    // Static buffer
            Circular::Enabled,          // Continuous sampling
            None,
        );


        let mut adc1 = adc1.enable();
        adc1.set_resolution(daisy::hal::adc::Resolution::SixteenBit);


        let channels = (pins.GPIO.PIN_15.into_analog(),
        pins.GPIO.PIN_16.into_analog(),
        pins.GPIO.PIN_17.into_analog(),
        pins.GPIO.PIN_18.into_analog(),
        pins.GPIO.PIN_19.into_analog(),
        pins.GPIO.PIN_20.into_analog(),
        pins.GPIO.PIN_21.into_analog(),
        );


        adc_dma.start(|_| {
        adc.read_all_continuous(&channels).unwrap()
        });




        // Initialize gains
        let gain1 = Gain::new(0.5);
        let gain2 = Gain::new(2.0);
        let bit_crush = BitCrush::new(20u8);

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
        let uart_read = code::UartCmd::new(tx, rx);
        let uart_write = uart_read.clone();

        // Enable caches
        cp.SCB.enable_icache();
        cp.SCB.enable_dcache(&mut cp.CPUID);

        let mut read_buf: heapless::Vec<u8, 32> = heapless::Vec::new();
        let mut write_buf: heapless::Vec<u8, 32> = heapless::Vec::new();

        let adc_snap = [0_u16; 7];

        (
            Shared { adc_snap },
            Local {
                adc_dma,
                audio_interface,
                gain1,
                gain2,
                bit_crush,
                uart_read,
                uart_write,
            },
        )
    }

    // DSP interrupt handler
    #[task(priority = 10, binds = DMA1_STR1, local = [audio_interface, gain1, gain2, bit_crush])]
    fn dsp(cx: dsp::Context) {
        cx.local
            .audio_interface
            .handle_interrupt_dma1_str1(|audio_buffer| {
                for frame in audio_buffer {
                    *frame = (0.0, 0.0);
                    cx.local.gain1.process(frame);
                    cx.local.gain2.process(frame);
                    cx.local.bit_crush.process(frame);

                }
            })
        .expect("audio dsp init error");
    }


       #[task(priority = 1, local = [uart_write])]
       async fn uart_write(cx: uart_write::Context, info: &[u8]) {
       write_buf(&info).unwrap();
       }


       #[task(binds = UART0, priority = 1)]
       fn uart_read_trigger(_: uart_read_trigger::Context) {
       uart_read::spawn().expect("uart spawn error");
       }

       #[task(priority = 1, local = [uart_read], shared = [rx_buf])]
       async fn uart_read(cx: uart_read::Context) {
       let len = cx.local.uart.read_buf(rx_buf).await.unwrap();
    //handle commands here
    }

    //if not make this sofware and make it scedule itself in the furture

    #[task(priority = 1, binds = DMA2_STR3, shared = [adc_snap], local = [adc_dma])]
    fn adc_update(mut cx: adc_update::Context) {
        if cx.local.adc_dma.is_done() {
            cx.local.adc_dma.clear_complete();
            cx.shared.adc_snap.lock(|adc_snap| {
                adc_snap = &ADC_BUF;
            });
        }
    }
}
