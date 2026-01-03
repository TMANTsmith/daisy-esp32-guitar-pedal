#![no_std]
#![no_main]

pub mod uart;
use cortex_m::interrupt::Mutex;
use heapless::Vec;
pub mod modules;
use core::cell::RefCell;
use daisy::audio;
use defmt_rtt as _;
use panic_probe as _; // enables the panic handler // optional: RTT transport for defmt
use uart::UartError;

// Optional global, if needed
static AUDIO_INTERFACE: Mutex<RefCell<Option<audio::Interface>>> = Mutex::new(RefCell::new(None));
const SAMPLE: usize = 48_000_usize;

#[link_section = ".sdram"]
static mut DELAY_BUF: [(f32, f32); SAMPLE * 5_usize] = [(0.0_f32, 0.0_f32); SAMPLE * 5_usize];
// for 5 sec delay

#[rtic::app(
    device = daisy::pac,
    peripherals = true,
    dispatchers = [EXTI0]
)]
mod app {
    use crate::uart::Info::Bytes;
    use crate::uart::Info::Str;
    use daisy::hal::dma::Transfer;

    use crate::uart::uart_read;
    use crate::uart::Info;
    static mut ADC_BUF: [u16; 7] = [0_u16; 7];
    use crate::modules::bit_crush::BitCrush;
    use crate::modules::gain::Gain;
    use crate::uart::UartError;
    use daisy::audio::Interface;
    use daisy::hal::nb;
    use daisy::hal::sai::dma;
    use daisy::hal::serial::SerialExt;
    use daisy::hal::time::U32Ext;
    use fugit::ExtU32; // for .millis()
    use fugit::RateExtU32; // for ADC .MHz()

    #[shared]
    struct Shared {
        adc_snap: [u16; 7],
    }

    #[local]
    struct Local {
        adc_dma: Transfer<daisy::hal::dma::dma::Stream3<daisy::pac::device::DMA2>, 
        daisy::hal::adc::Adc<daisy::hal::stm32::ADC1>, 
        daisy::hal::dma::PeripheralToMemory,
        &'static mut [u16], 
        _,
        audio_interface: Interface,
        //TODO just split uart read and write into tx and rx so no clone in needed
        rx: daisy::hal::serial::Rx<daisy::pac::USART1>,
        tx: daisy::hal::serial::Tx<daisy::pac::USART1>,
        gain1: Gain,
        gain2: Gain,
        bit_crush: BitCrush,
        rx_buf: [u8; 64],
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        let rx_buf = [0_u8; 64];
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
        let adc1 = daisy::hal::adc::Adc::adc1(
            dp.ADC1,
            4_u32.MHz(),
            &mut delay,
            ccdr.peripheral.ADC12,
            &ccdr.clocks,
        );
        adc1.configure_scan(Scan::Enabled);

        let streams = StreamsTuple::new(dp.DMA2, ccdr.peripheral.DMA2);

        let mut adc_dma = Transfer::init(
            streams.3,               // DMA channel
            adc,                     // Peripheral (ADC)
            unsafe { &mut ADC_BUF }, // Static buffer
            Circular::Enabled,       // Continuous sampling
            None,
        );

        let mut adc1 = adc1.enable();
        adc1.set_resolution(daisy::hal::adc::Resolution::SixteenBit);

        let channels = (
            pins.GPIO.PIN_15.into_analog(),
            pins.GPIO.PIN_16.into_analog(),
            pins.GPIO.PIN_17.into_analog(),
            pins.GPIO.PIN_18.into_analog(),
            pins.GPIO.PIN_19.into_analog(),
            pins.GPIO.PIN_20.into_analog(),
            pins.GPIO.PIN_21.into_analog(),
        );

        adc_dma.start(|_| adc.read_all_continuous(&channels).unwrap());

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

        // Enable caches
        cp.SCB.enable_icache();
        cp.SCB.enable_dcache(&mut cp.CPUID);

        let mut read_buf: heapless::Vec<u8, 32> = heapless::Vec::new();
        let mut write_buf: heapless::Vec<u8, 32> = heapless::Vec::new();

        let adc_snap = [0_u16; 7];
        let rx_buf = [0_u8; 64];

        (
            Shared { adc_snap },
            Local {
                adc_dma,
                audio_interface,
                tx,
                rx,
                gain1,
                gain2,
                bit_crush,
                rx_buf,
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

    // TODO fix .ok error handling
    #[task(priority = 1, local = [tx])]
    async fn uart_write_byte(cx: uart_write_byte::Context, byte: u8) {
        match cx.local.tx.write(byte) {
            Ok(()) => {}
            Err(nb::Error::WouldBlock) => {
                uart_write_byte::spawn(byte).ok();
            }
            Err(_) => {}
        }
    }

    #[task(priority = 1)]
    async fn uart_write_bytes(_cx: uart_write_bytes::Context, bytes: &'static [u8], is_str: bool) {
        let crc8 = crc::Crc::<u8>::new(&crc::CRC_8_SMBUS);
        let result = crc8.checksum(bytes);

        for byte in bytes {
            uart_write_byte::spawn(*byte).ok();
        }

        uart_write_byte::spawn(result).ok();

        if is_str == true {
            uart_write_byte::spawn(b'\t').ok();
        } else {
            uart_write_byte::spawn(b'\n').ok();
        }
    }

    #[task(priority = 1)]
    async fn uart_write_str(_cx: uart_write_str::Context, string: &'static str) {
        let bytes = string.as_bytes();
        uart_write_bytes::spawn(bytes, true).ok();
    }

    #[task(binds = USART1, priority = 1, local = [rx, rx_buf])]
    fn uart_read_trigger(cx: uart_read_trigger::Context) {
        match uart_read(cx.local.rx, cx.local.rx_buf) {
            Ok(info) => {
                match info {
                    Bytes(b) => {
                        let bytes = b;
                        // process commands here
                    }
                    Str(s) => {
                        let string = s;
                        // process commands here
                    }
                }
            }
            Err(e) => panic!("{}", e),
        };
    }

    //if not make this sofware and make it scedule itself in the furture

    #[task(priority = 1, binds = DMA2_STR3, shared = [adc_snap], local = [adc_dma])]
    fn adc_update(mut cx: adc_update::Context) {
        if cx.local.adc_dma.is_done() {
            cx.local.adc_dma.clear_complete();
            cx.shared.adc_snap.lock(|adc_snap| unsafe {
                adc_snap = &mut ADC_BUF;
            });
        }
    }
}
