#![no_std]
#![no_main]

use embassy_time::{Instant};
use daisy_embassy::{
    DaisyBoard, audio::{Interface, Running, Idle}, hal::{
        self, bind_interrupts, dma, exti::ExtiInput, exti, gpio::Pull, mode::Async, peripherals, usart::{self, Config as UsartConfig, Uart, UartRx, UartTx},
    }, new_daisy_board,
};
use daisy_embassy::led::UserLed;
use defmt::{debug, info, unwrap};
use defmt_rtt as _;
use embassy_executor::{Spawner};
use embassy_time::Delay;
use panic_probe as _;
use daisy_embassy::hal::interrupt::typelevel::EXTI0;


bind_interrupts!(struct Irqs {
    USART1 => usart::InterruptHandler<peripherals::USART1>;
    DMA1_STREAM4 => dma::InterruptHandler<peripherals::DMA1_CH4>;
    DMA1_STREAM3 => dma::InterruptHandler<peripherals::DMA1_CH3>;
    EXTI0 => exti::InterruptHandler<EXTI0>;
});


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




    let pins = board.pins;



    let interface = board
        .audio_peripherals
        .prepare_interface(Default::default())
        .await;






    // must be a p#0 so it matches EXTI0
    let button = pins.d15;
    let button = ExtiInput::new(button, p.EXTI0, Pull::None, Irqs);

    _spawner.spawn(button_detector(led, button).unwrap());

    debug!("spawned tasks");
}



#[embassy_executor::task]
pub async fn button_detector(
    mut led: UserLed<'static>, 
    mut pin: ExtiInput<'static, Async>) {
    let mut is_on = false;
    loop {
        pin.wait_for_rising_edge().await;
        let time = Instant::now();
        embassy_time::Timer::after_millis(10).await;

        pin.wait_for_falling_edge().await;
        let held_for = time.elapsed().as_millis();
        info!("button pressed for {}", &held_for);
        if is_on {
            led.off();
            is_on = false;
        }
        else {
            led.on();
            is_on = true;
        }

    }
}

