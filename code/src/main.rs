#![no_std]
#![no_main]

// ONLY USE defmt::error! or defmt::println!
use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use daisy::audio;
use daisy::hal::stm32;
use defmt::*;
use defmt_rtt as _;
use panic_probe as _; // enables the panic handler // optional: RTT transport for defmt
use rtic::app;
use rtic_monotonics::systick::prelude::*;

systick_monotonic!(Mono, 1000);

#[export_name = "_defmt_timestamp"]
fn timestamp() -> u64 {
    0
}

#[rtic::app(
    device = daisy::pac,
    peripherals = true,
    dispatchers = [EXTI0],
)]
mod app {
    use super::Mono;
    use cortex_m::asm;
    use cortex_m::prelude::_embedded_hal_adc_OneShot;
    use daisy::led::LedUser;
    use rtic_monotonics::fugit::ExtU32; // for u32.millis()
    use rtic_monotonics::Monotonic;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        // adc1: daisy::hal::adc::Adc<daisy::hal::stm32::ADC1, daisy::hal::adc::Enabled>,
        // adc1_channel: daisy::hal::gpio::gpioc::PC4<daisy::hal::gpio::Analog>,
        led: LedUser,
        // scale_factor: f32,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::println!("Daisy Seed booting...");
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

        /*
        let mut delay = daisy::hal::delay::Delay::new(cp.SYST, ccdr.clocks);
        let mut adc1 = daisy::hal::adc::Adc::adc1(
            dp.ADC1,
            4_u32.MHz(),
            &mut delay,
            ccdr.peripheral.ADC12,
            &ccdr.clocks,
        )
        .enable();
        adc1.set_resolution(daisy::hal::adc::Resolution::SixteenBit);

        let mut adc1_channel = pins.GPIO.PIN_21.into_analog();

        let channels = (
            pins.GPIO.PIN_15.into_analog(),
            pins.GPIO.PIN_16.into_analog(),
            pins.GPIO.PIN_17.into_analog(),
            pins.GPIO.PIN_18.into_analog(),
            pins.GPIO.PIN_19.into_analog(),
            pins.GPIO.PIN_20.into_analog(),
            pins.GPIO.PIN_21.into_analog(),
        );
        */

        // Initialize gains

        // Initialize UART
        // Enable caches
        cp.SCB.enable_icache();
        cp.SCB.enable_dcache(&mut cp.CPUID);

        let led = daisy::board_split_leds!(pins).USER;
        let scale_factor = ccdr.clocks.sys_ck().to_Hz() as f32 / 65_535.0;
        Mono::start(cp.SYST, ccdr.clocks.sys_ck().to_Hz()); // default STM32F303 clock-rate is 36MHz

        // Listen for update events (overflow)

        toggle_led::spawn().unwrap();

        (
            Shared {},
            Local {
                // adc1,
                // adc1_channel,
                led,
                // scale_factor,
            },
        )
    }

    #[task(local = [led], priority = 1)]
    async fn toggle_led(cx: toggle_led::Context) {
        // Try different defmt macros
        defmt::println!("println test");
        defmt::error!("error test");

        loop {
        defmt::println!("blink");
            cx.local.led.toggle();
            Mono::delay(1000.millis()).await;
        }
    }

    /*
    #[idle(local = [adc1, adc1_channel, led, scale_factor])]
    fn idle(mut cx: idle::Context) -> ! {
        defmt::println!("idle entered");


        loop {
            let pot: u32 = cx.local.adc1.read(cx.local.adc1_channel).unwrap();

            let ticks = (pot as f32 * *cx.local.scale_factor) as u32;

            defmt::println!("ADC {}", pot);

            cx.local.led.set_high();
            cortex_m::asm::delay(ticks);
            cx.local.led.set_low();
            cortex_m::asm::delay(ticks);
        }
    }
    */
}
