#![no_std]
#![no_main]
extern crate alloc;

mod uart;
use uart::Packet;
use core::fmt::write;
use core::num::Wrapping;
use code::modules::FFT::{*, FftState };
use code::modules::sin::Sine;
use code::modules::process::Effects;
use daisy_embassy::{DaisyBoard, hal, new_daisy_board};
use daisy_embassy::audio::{Interface, Running};
use defmt::{debug, info, unwrap};
use embassy_executor::{InterruptExecutor, SendSpawner, Spawner};
use hal::interrupt::{self, InterruptExt, Priority};
use daisy_embassy::sdram::SDRAM_SIZE;
use embassy_time::Delay;
use {defmt_rtt as _, panic_probe as _};
use static_cell::StaticCell;
use critical_section::Mutex;
use embassy_sync::signal::Signal;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use daisy_embassy::hal::time::mhz;
use daisy_embassy::hal::{ Config, bind_interrupts, dma, peripherals, spi };
use daisy_embassy::pins::DaisyPins;
use daisy_embassy::hal::mode::Async;
use alloc::boxed::Box;
use bytemuck::{cast_slice, Pod};
use libm::sqrtf;
use daisy_embassy::hal::gpio::{Level, Output, Speed};
use embassy_time::{Duration, Timer};
use daisy_embassy::hal::usart::Config as UsartConfig;
use daisy_embassy::hal::usart::Uart;
use daisy_embassy::hal::usart;
use daisy_embassy::led::UserLed;
use embassy_futures::join::join;

use embedded_alloc::LlffHeap as Heap;

bind_interrupts!(struct Irqs {
    USART1 => usart::InterruptHandler<peripherals::USART1>;
    DMA1_STREAM3 => dma::InterruptHandler<peripherals::DMA1_CH3>;
    DMA1_STREAM4 => dma::InterruptHandler<peripherals::DMA1_CH4>;
});

const FFT_N: usize = 1024; // input size
const FFT_H: usize = FFT_N / 2; // output size
const FFT_L: usize = 2; 


static BUFA: Signal<CriticalSectionRawMutex, Box<[f32; FFT_N]>> = Signal::new();
static BUFB: Signal<CriticalSectionRawMutex, Box<[f32; FFT_N]>> = Signal::new();
static BUFC: Signal<CriticalSectionRawMutex, Box<[f32; FFT_N]>> = Signal::new();

// audio_task -> BUFB -> compute -> BUFC -> SPI -> BUFA -> audio_task

static FFT_WRITE: StaticCell<FftWrite<FFT_N, FFT_H>> = StaticCell::new();

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();
static EXECUTOR_LOW: InterruptExecutor = InterruptExecutor::new();


#[global_allocator]
static HEAP: Heap = Heap::empty();

#[hal::interrupt]
unsafe fn TIM15() {
    unsafe { EXECUTOR_HIGH.on_interrupt() }
}
#[hal::interrupt]
unsafe fn TIM17() {
    unsafe { EXECUTOR_LOW.on_interrupt() }
}


#[defmt::panic_handler]
fn panic() -> ! {
    core::panic!("panic via defmt::panic!")
}

#[embassy_executor::task]
async fn uart_runner(mut uart: Uart<'static, Async>, mut led: UserLed<'static>) {
    // WAIT C 
    // SIGNAL A 
    let mut packet: Packet<f32, {FFT_H}> = Packet::new(&[0f32; FFT_H]);
    const X25: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);
    const HEADER: [u8; 4] = [0xAA, 0x55, 0xAA, 0x55];

    loop {
        let buffer = BUFC.wait().await;

        let crc = X25.checksum(cast_slice(buffer.as_ref())).to_le_bytes();

        // maybe have the BUF# be a packet so there are no copys
        packet.copy_into(&buffer.as_slice()[..FFT_H]);

        BUFA.signal(buffer);

        let info = packet.info();

        // info!("10kHz {}", info[213]);

        uart.write(packet.as_bytes()).await.unwrap();
    }
}
#[embassy_executor::task]
async fn fft_compute() {

    let mut mags = [0.0f32; FFT_H]; // reusable scratch, stack-allocated, outside the loop

    loop {
        let mut buffer = BUFB.wait().await;
        let result = compute::<FFT_N, FFT_H>(&mut buffer);
        result[0].im = 0.0;

        let mut max_amp: f32 = 0.0;
        let mut max_i = 0;
        for (i, c) in result.iter().enumerate() {
            if c.norm_sqr() > max_amp {
                max_amp = c.norm_sqr();
                max_i = i;
            }
        }
        let freq = max_i as f32 * get_bin_hz::<FFT_N>();

        for i in 0..FFT_H {
            mags[i] = libm::sqrtf(result[i].norm_sqr());
        }

        buffer[..FFT_H].copy_from_slice(&mags);

        BUFC.signal(buffer);
    }
}


#[embassy_executor::task]
async fn audio_task(
    mut interface: Interface<'static, Running>,
    fft_write: &'static mut FftWrite<FFT_N, FFT_H>,
    mut sin: Sine,
) {
    // WAIT A 
    // SIGNAL B
    //debug!("entered audio");
    unwrap!(
        interface
            .start_callback(move |input, output| {
                let mut frames: FrameBlock = [(0.0, 0.0); 32];
                convert_to(input, &mut frames);

                for frame in frames.iter_mut() {
                    frame.1 = sin.get_next();
                    match fft_write.add(frame.1) {
                        Err(FftState::Ready(e)) => {  /* debug!("buffer sent to compute:"); */ BUFB.signal(e); },
                        Err(FftState::NoBuf) => { 
                            if let Some(b) = BUFA.try_take() { 
                                fft_write.set_buf(b); 
                                //debug!("buffer receaved from compute");
                            } 
                        },

                        _ => ()
                    }
                }

                convert_from(&frames, output);

            })
            .await
    );
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    debug!("====program start====");

    let config = daisy_embassy::default_rcc();
    let p = hal::init(config);
    let board: DaisyBoard<'_> = new_daisy_board!(p);



    let mut core = cortex_m::Peripherals::take().unwrap();
    let mut sdram = board.sdram.build(&mut core.MPU, &mut core.SCB);


    let mut delay = Delay;

    let ram_ptr: *mut u32 =  sdram.init(&mut delay) as *mut _;





    // Initialize the global allocator over the SDRAM region, BEFORE any
    // alloc-based type (Vec, Box, HeapRb, etc.) is constructed.
    unsafe {
        HEAP.init(ram_ptr as usize, SDRAM_SIZE);
    }


    let mut spi_config = spi::Config::default();
    spi_config.frequency = mhz(1);

    let pins = board.pins;

    let mut config = UsartConfig::default();
    config.baudrate = 2_000_000;
    let uart = Uart::new(p.USART1, pins.d14, pins.d13, p.DMA1_CH3, p.DMA1_CH4, Irqs, config).unwrap();

    let interface = board
        .audio_peripherals
        .prepare_interface(Default::default())
        .await;


    let interface = unwrap!(interface.start_interface().await);

    interrupt::TIM15.set_priority(Priority::P3);
    interrupt::TIM17.set_priority(Priority::P5);
    let spawner_high = EXECUTOR_HIGH.start(interrupt::TIM15); // reader
    let spawner_low = EXECUTOR_LOW.start(interrupt::TIM17); // computer

    let fft_write = FFT_WRITE.init(FftWrite::<FFT_N, FFT_H>::new());

    let sin = Sine::new(10_000.0, 0.5);

    let led = board.user_led;

    spawner_high.spawn(audio_task(interface, fft_write, sin).unwrap());
    spawner_low.spawn(fft_compute().unwrap());
    spawner_low.spawn(uart_runner(uart, led).unwrap());

    let buf_a = Box::new([0_f32; FFT_N]);
    let buf_b = Box::new([0_f32; FFT_N]);
    let buf_c = Box::new([0_f32; FFT_N]);

    BUFA.signal(buf_a);
    BUFB.signal(buf_b);
    BUFC.signal(buf_c);

    debug!("spawned tasks");
}

pub type Frame = (f32, f32);
pub type FrameBlock = [Frame; 32];

pub fn convert_to(input: &[u32], output: &mut [Frame]) {
    for (chunk, frame) in input.chunks(2).zip(output.iter_mut()) {
        frame.0 = to_f32(chunk[0]);
        frame.1 = to_f32(chunk[1]);
    }
}
pub fn convert_from(input: &[Frame], output: &mut [u32]) {
    for (frame, chunk) in input.iter().zip(output.chunks_mut(2)) {
        chunk[0] = to_u32(frame.0);
        chunk[1] = to_u32(frame.1);
    }
}
fn to_f32(y: u32) -> f32 {
    let y = (Wrapping(y) + Wrapping(0x0080_0000)).0 & 0x00FF_FFFF; // convert to i32
    (y as f32 / 8_388_608.0) - 1.0 // (2^24) / 2
}

#[inline(always)]
fn to_u32(x: f32) -> u32 {
    let x = x * 8_388_607.0;
    let x = x.clamp(-8_388_608.0, 8_388_607.0);
    (x as i32) as u32
}

