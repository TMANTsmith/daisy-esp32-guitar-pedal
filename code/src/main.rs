#![no_std]
#![no_main]
use defmt_rtt as _;
use panic_probe as _;
use rtic::app;
use rtic_monotonics::systick::prelude::*;

systick_monotonic!(Mono, 1000);

#[inline(always)]
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
    use super::Mono;
    use cortex_m::prelude::_embedded_hal_adc_OneShot;
    use daisy::audio::Interface;
    use daisy::led::LedUser;
    //use rtic_monotonics::fugit::RateExtU32;
    use daisy::hal::prelude::*;
    use embedded_alloc::LlffHeap as Heap;
    use rtic_monotonics::Monotonic;
    extern crate alloc;
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
        // Current phase in radians, 0.0 ..= 2π
        // How much phase to advance per sample
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
            },
        )
    }
    // DMA interrupt handler - called when audio buffer is ready
    #[task(binds = DMA1_STR1, local = [audio_interface, adc1, adc1_channel], priority = 2)]
    fn audio_callback(mut cx: audio_callback::Context) {
        // Read ADC value for gain control
        let pot: u32 = cx.local.adc1.read(cx.local.adc1_channel).unwrap();
        // defmt::println!("adc read: {}", pot);
        // Process audio buffer
        cx.local
            .audio_interface
            .handle_interrupt_dma1_str1(|buffer| {
                for frame in buffer {
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
