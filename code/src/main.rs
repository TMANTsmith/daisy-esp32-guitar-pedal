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

    use modules::sin::Sine;
    use super::modules::debug::vol::volume;
    use super::Mono;
    use code::modules;
    use cortex_m::prelude::_embedded_hal_adc_OneShot;
    use daisy::audio::Interface;
    use daisy::led::LedUser;
    use modules::process::Effects;
    //use rtic_monotonics::fugit::RateExtU32;
    use daisy::hal::prelude::*;
    use embedded_alloc::LlffHeap as Heap;
    use rtic_monotonics::Monotonic;
    use code::modules::FFT::{Fft, Wave, Waves};
    use crate::make_fft;

    #[global_allocator]
    static HEAP: Heap = Heap::empty();

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        audio_interface: Interface,
        adc1: daisy::hal::adc::Adc<daisy::hal::stm32::ADC1, daisy::hal::adc::Enabled>,
        adc1_channel: daisy::hal::gpio::gpioc::PC4<daisy::hal::gpio::Analog>,
        led: LedUser,
        fft: Fft<4096, 2048>,
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
        let fft = make_fft!(4096, [true, true]);

        let sine_c: Sine = Sine::new(261.63, 0.25); 
        let sine_e: Sine = Sine::new(329.63, 0.25);  
        let sine_g: Sine = Sine::new(392.00, 0.5);  

        defmt::println!("=== Init complete ===");

        let SYST = delay.free();
        Mono::start(SYST, ccdr.clocks.sys_ck().to_Hz()); // default STM32F303 clock-rate is 36MHz
                                                         // use let pin_b = gpio#.###.into_pull_up_input();
                                                         // and pass to struct


        (
            Shared {},
            Local {
                audio_interface,
                adc1,
                adc1_channel,
                led,
                fft,
                sine_c,
                sine_e,
                sine_g,
            },
        )
    }

    // DMA interrupt handler - called when audio buffer is ready
    #[task(binds = DMA1_STR1, local = [audio_interface, adc1, adc1_channel, fft, sine_c, sine_e, sine_g], priority = 2)]
    fn audio_callback(mut cx: audio_callback::Context) {
        // Read ADC value for gain control
        let pot: u32 = cx.local.adc1.read(cx.local.adc1_channel).unwrap();
        let fft = cx.local.fft;
        let sine_c = cx.local.sine_c;
        let sine_e = cx.local.sine_e;
        let sine_g = cx.local.sine_g;
        
        let start = Mono::now();

        if let Ok(r) = fft.compute() {
            let elapsed = Mono::now().checked_duration_since(start).unwrap();
            let millis = elapsed.to_millis();



            let left_largest: [Wave; 3];
            let right_largest: [Wave; 3];

            if let Some(mut left) = r.0 {
                left_largest = left.get_n_largest::<3>();
                let mut left_list = [0_f32; 3];

                for i in 0..left_largest.len() {
                    left_list[i] = left_largest[i].get_hertz()
                }


                defmt::println!("found left: {}", left_list);
            }

            if let Some(mut right) = r.1 {
                right_largest = right.get_n_largest::<3>();
                let mut right_list = [0_f32; 3];

                for i in 0..right_largest.len() {
                    right_list[i] = right_largest[i].get_hertz()
                }

                defmt::println!("found right: {}", right_list);
            }

            defmt::println!("took: {}", millis);
        }
        else {
            //defmt::println!("WAIT");
        }




        // defmt::println!("adc read: {}", pot);

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
                    fft.add(&mut (left, right));

                    

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
