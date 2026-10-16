#![no_std]
#![no_main]
extern crate alloc;
mod modules;

use embassy_time::{Instant};
use modules::looper_v2::*;
use alloc::boxed::Box;
use core::num::Wrapping;
use daisy_embassy::{
    DaisyBoard, audio::{Interface, Running, Idle}, hal::{
        self, bind_interrupts, dma, exti::ExtiInput, gpio::Pull, mode::Async, peripherals, usart::{self, Config as UsartConfig, Uart, UartRx, UartTx},
    }, new_daisy_board, sdram::SDRAM_SIZE,
};
use defmt::{debug, info, unwrap};
use defmt_rtt as _;
use embassy_executor::{InterruptExecutor, Spawner};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Delay;
use embedded_alloc::LlffHeap as Heap;
use hal::interrupt::{self, InterruptExt, Priority};
use panic_probe as _;
use pcobs::{deserialize, serialize};
use settings::*;
use static_cell::StaticCell;
use daisy_embassy::hal::gpio::Input;
use daisy_embassy::hal::interrupt::typelevel::EXTI0;
use daisy_embassy::hal::exti;


bind_interrupts!(struct Irqs {
    USART1 => usart::InterruptHandler<peripherals::USART1>;
    DMA1_STREAM4 => dma::InterruptHandler<peripherals::DMA1_CH4>;
    DMA1_STREAM3 => dma::InterruptHandler<peripherals::DMA1_CH3>;
    EXTI0 => exti::InterruptHandler<EXTI0>;
});




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
async fn audio_task(
    mut interface: Interface<'static, Idle>,
    mut looper: Looper,
) {
    //debug!("entered audio");
    use Mute::Mute;
    use self::Mute::Unmute;
    let mut interface = unwrap!(interface.start_interface().await);
    let mut command: CommandUart = CommandUart::Muter(Unmute);
    unwrap!(
        interface
            .start_callback(move |input, output| {
                let mut frames: FrameBlock = [(0.0, 0.0); 32];
                let empty: FrameBlock = [(0.0, 0.0); 32];
                let mut sender: &FrameBlock = &frames;

                    convert_to(input, &mut frames);


                    /*
                    for mut frame in frames {
                        looper.process(&mut frame);
                    }
                    */

                    if let Ok(c) = COMMAND_UART.try_lock() {
                        command = *c;
                    }
                    if command == CommandUart::Muter(Mute) {
                        sender = &empty;
                    }
                    else {
                        sender = &frames
                    }
                    convert_from(sender, output);
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
    let led = board.user_led;



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



    interrupt::TIM15.set_priority(Priority::P3);
    interrupt::TIM17.set_priority(Priority::P5);
    let spawner_high = EXECUTOR_HIGH.start(interrupt::TIM15); 
    let spawner_low = EXECUTOR_LOW.start(interrupt::TIM17); 



    // must be a p#0 so it matches EXTI0
    let button = pins.d15;
    let button = ExtiInput::new(button, p.EXTI0, Pull::None, Irqs);

    let looper = Looper::new(3, Second(30));

    spawner_high.spawn(audio_task(interface, looper).unwrap());
    spawner_low.spawn(button_detector(button).unwrap());
    spawner_low.spawn(uart_receiver(uart_rx, led).unwrap());

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

