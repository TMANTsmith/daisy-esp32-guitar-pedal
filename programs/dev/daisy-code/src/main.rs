#![no_std]
#![no_main]
extern crate alloc;



mod modules;

use alloc::boxed::Box;
use core::num::Wrapping;
use daisy_embassy::{
    audio::{Interface, Running},
    hal::{
        self, bind_interrupts, dma,
        mode::Async,
        peripherals,
        usart::{self, Config as UsartConfig, Uart, UartRx, UartTx},
    },
    new_daisy_board,
    sdram::SDRAM_SIZE,
    DaisyBoard,
};
use defmt::{debug, info, unwrap};
use defmt_rtt as _;
use embassy_executor::{InterruptExecutor, Spawner};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Delay;
use embedded_alloc::LlffHeap as Heap;
use hal::interrupt::{self, InterruptExt, Priority};
use modules::FFT::*;
use panic_probe as _;
use pcobs::{deserialize, serialize};
use settings::*;
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    USART1 => usart::InterruptHandler<peripherals::USART1>;
    DMA1_STREAM3 => dma::InterruptHandler<peripherals::DMA1_CH3>;
    DMA1_STREAM4 => dma::InterruptHandler<peripherals::DMA1_CH4>;
});



static COMMAND: Signal<CriticalSectionRawMutex, CommandUart> = Signal::new();
static BUFA: Signal<CriticalSectionRawMutex, Box<[f32; FFT_INPUT]>> = Signal::new();
static BUFB: Signal<CriticalSectionRawMutex, Box<[f32; FFT_INPUT]>> = Signal::new();
static BUFC: Signal<CriticalSectionRawMutex, Box<[f32; FFT_INPUT]>> = Signal::new();

// audio_task -> BUFB -> compute -> BUFC -> SPI -> BUFA -> audio_task

static BUFFER_FILLER: StaticCell<BufferFiller<FFT_INPUT>> = StaticCell::new();

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
async fn uart_receiver(mut uart: UartRx<'static, Async>) {
    let mut buf = Box::new([0; 64]);
    let mut counter = 0usize;
    loop {
        let n = match uart.read_until_idle(&mut buf[counter..]).await {
            Err(e) => { info!("uart read error: {}", e); continue; }
            Ok(n) => n,
        };

        let end = counter + n; 
        let mut start = 0;

        while let Some(rel_pos) = buf[start..end].iter().position(|&b| b == FRAME_DELIM) {
            let delim = start + rel_pos;
            let frame = &mut buf[start..delim]; 

            match deserialize::<CommandUart>(frame, frame.len()) {
                Ok(command) => {
                    COMMAND.signal(command);
                }
                Err(e) => info!("bad cobs frame {}", defmt::Debug2Format(&e)),
            }

            start = delim + 1;
        }

        if start == end {
            // consumed everything, reset to front of buffer
            counter = 0;
        } else if start > 0 {
            // shift leftover partial frame to the front of buf
            buf.copy_within(start..end, 0);
            counter = end - start;
        } else {
            // no delimiter found at all — still mid-frame, keep accumulating
            counter = end;
            if counter == buf.len() {
                info!("frame too large, dropping");
                counter = 0;
            }
        }
    }
}



#[embassy_executor::task]
async fn uart_sender(mut uart: UartTx<'static, Async>){
    // WAIT C 
    // SIGNAL A 

    // try to remove box here and optimize for more memory


    let mut cobs = Box::new([0_u8; COBS_BUF]);
    let mut convertion: [BinValue; FFT_BINS] = [BinValue::from(0u8); FFT_BINS];

    loop {
        let mut bufc = BUFC.wait().await;
        let buffer: &mut [f32; FFT_BINS] = (&mut bufc[..FFT_BINS]).try_into().unwrap();
        FromF32::slice_from_f32(buffer, &mut convertion);
        let msg = FFTUart::new(convertion);
        let len = serialize(&msg, cobs.as_mut_slice());
        BUFA.signal(bufc);

        match len {
            Err(err) =>
            {
                info!("uart error {}", defmt::Debug2Format(&err));
            },
            Ok(len) =>
            {
                cobs[len] = FRAME_DELIM;
                uart.write(&cobs[..=len]).await.unwrap();
            }
        }
    }
}
#[embassy_executor::task]
async fn fft_compute() {

    let mut mags = [0.0f32; FFT_BINS]; // reusable scratch, stack-allocated, outside the loop

    loop {
        let mut buffer = BUFB.wait().await;
        let result = compute::<FFT_INPUT, FFT_BINS>(&mut buffer);
        result[0].im = 0.0;

        for i in 0..FFT_BINS{
            mags[i] = libm::sqrtf(result[i].norm_sqr());
        }

        buffer[..FFT_BINS].copy_from_slice(&mags);

        BUFC.signal(buffer);
    }
}


#[embassy_executor::task]
async fn audio_task(
    mut interface: Interface<'static, Running>,
    buffer_filler: &'static mut BufferFiller<FFT_INPUT>,
) {
    // WAIT A 
    // SIGNAL B
    //debug!("entered audio");
    use Mute::Mute;
    use self::Mute::Unmute;
    let mut command: CommandUart = CommandUart::Mute(Unmute);
    unwrap!(
        interface
            .start_callback(move |input, output| {
                let mut frames: FrameBlock = [(0.0, 0.0); 32];
                convert_to(input, &mut frames);

                if let Some(c) = COMMAND.try_take() {
                    command = c;
                }

                for frame in frames.iter_mut() {
                    match buffer_filler.add(frame.1) {
                        Err(BufState::Ready(e)) => {  /* debug!("buffer sent to compute:"); */ BUFB.signal(e); },
                        Err(BufState::NoBuf) => { 
                            if let Some(b) = BUFA.try_take() { 
                                buffer_filler.set_buf(b); 
                                //debug!("buffer receaved from compute");
                            } 
                        },

                        _ => ()
                    }
                }

                if command == CommandUart::Mute(Mute) {
                    frames.fill((0.0, 0.0));
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



    let pins = board.pins;

    let mut config = UsartConfig::default();
    config.baudrate = BAUDRATE;
    let uart = Uart::new(p.USART1, pins.d14, pins.d13, p.DMA1_CH3, p.DMA1_CH4, Irqs, config).unwrap();
    let (uart_tx, uart_rx) = uart.split(); 

    let interface = board
        .audio_peripherals
        .prepare_interface(Default::default())
        .await;


    let interface = unwrap!(interface.start_interface().await);

    interrupt::TIM15.set_priority(Priority::P3);
    interrupt::TIM17.set_priority(Priority::P5);
    let spawner_high = EXECUTOR_HIGH.start(interrupt::TIM15); // reader
    let spawner_low = EXECUTOR_LOW.start(interrupt::TIM17); // computer

    let buffer_filler = BUFFER_FILLER.init(BufferFiller::<FFT_INPUT>::new());



    spawner_high.spawn(audio_task(interface, buffer_filler).unwrap());
    spawner_low.spawn(fft_compute().unwrap());
    spawner_low.spawn(uart_sender(uart_tx).unwrap());
    spawner_low.spawn(uart_receiver(uart_rx).unwrap());

    let buf_a = Box::new([0_f32; FFT_INPUT]);
    let buf_b = Box::new([0_f32; FFT_INPUT]);
    let buf_c = Box::new([0_f32; FFT_INPUT]);

    BUFA.signal(buf_a);
    BUFB.signal(buf_b);
    BUFC.signal(buf_c);

    debug!("spawned tasks");
}

pub type Frame = (f32, f32);
pub type FrameBlock = [Frame; 32];

#[inline(always)]
pub fn convert_to(input: &[u32], output: &mut [Frame]) {
    for (chunk, frame) in input.chunks(2).zip(output.iter_mut()) {
        frame.0 = to_f32(chunk[0]);
        frame.1 = to_f32(chunk[1]);
    }
}

#[inline(always)]
pub fn convert_from(input: &[Frame], output: &mut [u32]) {
    for (frame, chunk) in input.iter().zip(output.chunks_mut(2)) {
        chunk[0] = to_u32(frame.0);
        chunk[1] = to_u32(frame.1);
    }
}

#[inline(always)]
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

