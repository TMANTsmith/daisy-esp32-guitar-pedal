#![no_std]
#![no_main]
extern crate alloc;

use code::modules::FFT::{Fft, Waves};
use daisy_embassy::{DaisyBoard, hal, new_daisy_board};
use daisy_embassy::audio::{Interface, Running};
use defmt::{debug, unwrap};
use embassy_executor::{InterruptExecutor, Spawner};
use embassy_stm32::interrupt::{self, InterruptExt, Priority};
use embassy_time::Timer;
use ringbuf::{traits::*, HeapRb};
use daisy_embassy::sdram::SDRAM_SIZE;
use embassy_time::Delay;
use {defmt_rtt as _, panic_probe as _};

use embedded_alloc::LlffHeap as Heap;

const FFT_N: usize = 4096;
const FFT_H: usize = FFT_N / 2;
const FFT_L: usize = 5; // number of peaks to report

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();
static EXECUTOR_LOW: InterruptExecutor = InterruptExecutor::new();

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_stm32::interrupt]
unsafe fn UART4() {
    unsafe { EXECUTOR_HIGH.on_interrupt() }
}
#[embassy_stm32::interrupt]
unsafe fn UART5() {
    unsafe { EXECUTOR_LOW.on_interrupt() }
}

#[defmt::panic_handler]
fn panic() -> ! {
    core::panic!("panic via defmt::panic!")
}

#[embassy_executor::task]
async fn fft_compute(
    mut fft: Fft<FFT_N, FFT_H>,
    frequency: u64,
    mut consumer: ringbuf::HeapCons<(f32, f32)>,
) {

    debug!("fft_compute entered");
    let mut last_report = embassy_time::Instant::now();
    let mut samples_drained = 0;

    loop {
        // drain whatever's available right now
        while let Some(sample) = consumer.try_pop() {
            debug!("Sample added {=f32}", sample.0);
            samples_drained += 1;
        }

        if last_report.elapsed() >= embassy_time::Duration::from_secs(frequency) {
            debug!("samples drained {=usize}", samples_drained);

            debug!("before print");
            //fft.print();
            debug!("before compute");
            fft.compute();
            debug!("computed");
            let result = fft.get_result();
            if let Some(line) = result.0 {
                let line_largest = line.get_n_largest::<FFT_L>();
                debug!("_____");
                for val in line_largest {
                    debug!("hertz: {=f32}", val.get_hertz());
                }
                debug!("_____");
            }
            if let Some(line) = result.1 {
                let line_largest = line.get_n_largest::<FFT_L>();
                for val in line_largest {
                    debug!("hertz: {=f32}", val.get_hertz());
                }
            }
            last_report = embassy_time::Instant::now();
        }

        // brief yield so this isn't a busy-spin hogging the executor
        Timer::after_millis(1).await;
    }
}

#[embassy_executor::task]
async fn audio_task(
    mut interface: Interface<'static, Running>,
    mut producer: ringbuf::HeapProd<(f32, f32)>,
) {
    debug!("entered audio");
    unwrap!(
        interface
            .start_callback(move |input, output| {
                let mut frames: FrameBlock = [(0.0, 0.0); 32];
                convert_to(input, &mut frames);
                for frame in frames {
                    let _ = producer.try_push(frame); // non-blocking, never locks
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

    interrupt::UART4.set_priority(Priority::P2);
    interrupt::UART5.set_priority(Priority::P5);
    let spawner_high = EXECUTOR_HIGH.start(interrupt::UART4);
    let spawner_low = EXECUTOR_LOW.start(interrupt::UART5);

    let fft = Fft::<FFT_N, FFT_H>::new((false, true));

    let rb = HeapRb::<(f32, f32)>::new(1024); // sized with headroom
    let (producer, consumer) = rb.split();


    unwrap!(spawner_low.spawn(fft_compute(fft, 1_u64, consumer)));
    unwrap!(spawner_high.spawn(audio_task(interface, producer)));
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
fn to_f32(sample: u32) -> f32 {
    (sample as i32) as f32 / 8388608.0
}
fn to_u32(sample: f32) -> u32 {
    (sample * 8388608.0) as i32 as u32
}
