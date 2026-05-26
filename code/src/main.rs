#![no_std]
#![no_main]

extern crate alloc;
use defmt_rtt as _;
use panic_probe as _;
use rtic::app;
use rtic_monotonics::systick::prelude::*;
mod modules;

systick_monotonic!(Mono, 1000);

#[export_name = "_defmt_timestamp"]
fn timestamp() -> u64 {
    0
}

#[rtic::app(
    device = daisy::pac,
    peripherals = true,
    dispatchers = [EXTI0, EXTI1],
)]
mod app {

    use super::modules::debug::vol::volume;
    use super::Mono;
    use code::modules::{self, FFT::*};
    use cortex_m::prelude::_embedded_hal_adc_OneShot;
    use daisy::audio::Interface;
    use daisy::led::LedUser;
    use modules::process::Effects;
    use modules::sin::Sine;
    //use rtic_monotonics::fugit::RateExtU32;
    use crate::make_fft;
    use daisy::hal::prelude::*;
    use embedded_alloc::LlffHeap as Heap;
    use rtic_monotonics::Monotonic;

    #[global_allocator]
    static HEAP: Heap = Heap::empty();

    #[shared]
    struct Shared {
        fft_read: Fft_read<4096, 2048>,
        fft_write: Fft_write<4096, 2048>,
    }

    #[local]
    struct Local {
        audio_interface: Interface,
        adc1: daisy::hal::adc::Adc<daisy::hal::stm32::ADC1, daisy::hal::adc::Enabled>,
        adc1_channel: daisy::hal::gpio::gpioc::PC4<daisy::hal::gpio::Analog>,
        led: LedUser,
        sine_c: Sine,
        sine_e: Sine,
        sine_g: Sine,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::println!("=== INIT START ===");

        let mut cp = cx.core;
        let dp = cx.device;
        let board = daisy::Board::take().expect("board take error");
        let ccdr = daisy::board_freeze_clocks!(board, dp);
        let pins = daisy::board_split_gpios!(board, ccdr, dp);
        let sdram = daisy::board_split_sdram!(cp, dp, ccdr, pins);

        unsafe { HEAP.init(sdram.base_address as usize, sdram.size()) }

        // Create and spawn audio interface
        defmt::println!("Setting up audio interface...");
        let audio_interface = daisy::board_split_audio!(ccdr, pins)
            .spawn() // Call spawn - it starts the codec and DMA
            .expect("audio spawn error");
        defmt::println!("Audio interface spawned!");

        cp.SCB.enable_icache();
        cp.SCB.enable_dcache(&mut cp.CPUID);

        // starts stopwatch
        cp.DWT.enable_cycle_counter();

        defmt::println!("Initializing ADC...");

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
        defmt::println!("ADC ready!");

        let adc1_channel = pins.GPIO.PIN_21.into_analog();
        let led = daisy::board_split_leds!(pins).USER;

        // sets up Fft
        let (fft_read, fft_write) = make_fft!(4096, [true, true]);

        let sine_c: Sine = Sine::new(261.63, 0.33);
        let sine_e: Sine = Sine::new(329.63, 0.33);
        let sine_g: Sine = Sine::new(392.00, 0.33);

        defmt::println!("=== Init complete ===");

        let SYST = delay.free();
        Mono::start(SYST, ccdr.clocks.sys_ck().to_Hz()); // default STM32F303 clock-rate is 36MHz
                                                         // use let pin_b = gpio#.###.into_pull_up_input();
                                                         // and pass to struct

        (
            Shared {
                fft_read,
                fft_write,
            },
            Local {
                audio_interface,
                adc1,
                adc1_channel,
                led,
                sine_c,
                sine_e,
                sine_g,
            },
        )
    }


    #[task(shared = [fft_read], priority = 2)]
    async fn fft_process(mut cx: fft_process::Context) {
        //defmt::println!("fft_process start");
        cx.shared.fft_read.lock(|fft_read| {
            let start = cortex_m::peripheral::DWT::cycle_count();
            if let Ok(tuple) = fft_read.compute() {
                if let Some(mut waves) = tuple.0 {
                    let large_waves = waves.get_n_largest::<3>();
                    /*
                    defmt::println!("left waves start");
                    large_waves
                        .iter()
                        .for_each(|wave| defmt::println!("[ {} ] ", wave.get_hertz()));
                    defmt::println!("left waves end");
                    */
                }

                if let Some(mut waves) = tuple.1 {
                    let large_waves = waves.get_n_largest::<3>();
                    /*
                    defmt::println!("right waves start");
                    large_waves
                        .iter()
                        .for_each(|wave| defmt::println!("[ {} ] ", wave.get_hertz()));
                    defmt::println!("right waves end");
                    */
                }
            }
            let cycles = cortex_m::peripheral::DWT::cycle_count().wrapping_sub(start);
            let micros = cycles / 400; // 400MHz = 400 cycles per µs
            //defmt::println!("compute took: {}us", micros);
        });
        //defmt::println!("fft_process finnished");
    }

    // DMA interrupt handler - called when audio buffer is ready
    #[task(
        binds = DMA1_STR1,
        local = [audio_interface, adc1, adc1_channel, sine_c, sine_e, sine_g],
        shared = [fft_write, fft_read],
        priority = 10
        )]
    fn audio_callback(mut cx: audio_callback::Context) {
        // Read ADC value for gain control
        let pot: u32 = cx.local.adc1.read(cx.local.adc1_channel).unwrap();
        let sine_c = cx.local.sine_c;
        let sine_e = cx.local.sine_e;
        let sine_g = cx.local.sine_g;

        //defmt::println!("adc read: {}", pot);

        // Process audio buffer
        cx.local
            .audio_interface
            .handle_interrupt_dma1_str1(|buffer| {
                for frame in buffer {
                    let (add_left_c, add_right_c) = sine_c.get_next();
                    let (add_left_g, add_right_g) = sine_g.get_next();
                    let (add_left_e, add_right_e) = sine_e.get_next();
                    let left = add_left_e + add_left_g + add_left_c;
                    let right = add_right_e + add_right_g + add_right_c;
                    *frame = (left, right);
                    cx.shared.fft_write.lock(|fft_write| {
                        fft_write.add(&mut (left, right));
                        cx.shared.fft_read.lock(|fft_read| {
                            if fft_read.copy_from_write(fft_write).is_ok() {
                                //defmt::println!("successful fft copy! spawning process");
                                fft_process::spawn().ok();
                            }
                        })
                    });
                }
            })
        .unwrap();
        }

    #[idle(local = [led])]
    fn idle(cx: idle::Context) -> ! {
        defmt::println!("=== IDLE running ===");
        let sys_freq = 400_000_000;
        let delay_cycles = sys_freq / 2; // 500ms

        loop {
            cx.local.led.toggle();
            cortex_m::asm::delay(delay_cycles);
        }
    }
}
