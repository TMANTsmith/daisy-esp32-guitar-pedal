#![no_std]
#![no_main]
extern crate alloc;

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
use alloc::boxed::Box;


use embedded_alloc::LlffHeap as Heap;

const FFT_N: usize = 4096;
const FFT_H: usize = FFT_N / 2;
const FFT_L: usize = 2; 


static BUFA: Signal<CriticalSectionRawMutex, Box<[f32; FFT_N]>> = Signal::new();
static BUFB: Signal<CriticalSectionRawMutex, Box<[f32; FFT_N]>> = Signal::new();

static FFT_READ: StaticCell<FftRead<FFT_N, FFT_H>> = StaticCell::new();
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
async fn fft_compute(fft_read: &'static mut FftRead<FFT_N, FFT_H>) {
    // WAIT B 
    // SIGNAL A
    loop {
        fft_read.set_buf(BUFB.wait().await);
        //debug!("buffer received from audio");

        match fft_read.compute() {
            Ok(buf) => {BUFA.signal(buf); /* debug!("buffer sent from compute"); */},
            Err(FftState::NoBuf) => continue,
            _ => ()
        }
        let result = fft_read.get_waves().get_n_largest::<3>();

        info!("____");
        for wave in result.iter() {
            info!("hertz: {}", wave.get_hertz());
        }
        info!("____");
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
                    //frame.1 = sin.get_next();
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



    let interface = board
        .audio_peripherals
        .prepare_interface(Default::default())
        .await;
    let interface = unwrap!(interface.start_interface().await);

    interrupt::TIM15.set_priority(Priority::P3);
    interrupt::TIM17.set_priority(Priority::P5);
    let spawner_high = EXECUTOR_HIGH.start(interrupt::TIM15); // reader
    let spawner_low = EXECUTOR_LOW.start(interrupt::TIM17); // computer






    let fft_read = FFT_READ.init(FftRead::<FFT_N, FFT_H>::new());
    let fft_write = FFT_WRITE.init(FftWrite::<FFT_N, FFT_H>::new());

    let sin = Sine::new(10_000.0, 0.5);


    spawner_high.spawn(audio_task(interface, fft_write, sin).unwrap());
    spawner_low.spawn(fft_compute(fft_read).unwrap());

    let buf_a = Box::new([0_f32; FFT_N]);
    let buf_b = Box::new([0_f32; FFT_N]);

    BUFA.signal(buf_a);
    BUFB.signal(buf_b);
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

