#![no_std]
#![no_main]


use daisy::{pac, hal};
use hal::adc::{self, Adc};
use hal::delay::Delay;
use hal::gpio::Analog;

/// Enum to represent all ADC-capable pins on the Daisy Seed
pub enum AdcPin {
    PC0(hal::gpio::gpioc::PC0<Analog>), // ADC0
    PA3(hal::gpio::gpioa::PA3<Analog>), // ADC1
    PB1(hal::gpio::gpiob::PB1<Analog>), // ADC2
    PA7(hal::gpio::gpioa::PA7<Analog>), // ADC3
    PA6(hal::gpio::gpioa::PA6<Analog>), // ADC4
    PC1(hal::gpio::gpioc::PC1<Analog>), // ADC5
    PC4(hal::gpio::gpioc::PC4<Analog>), // ADC6
}

/// Struct to handle ADC reads
pub struct Adcs {
    adc1: Adc<pac::ADC1>,
    ccdr: hal::rcc::Ccdr,
    delay: Delay,
}

impl Adcs {
    /// Create a new ADC handler
    pub fn new(dp: pac::Peripherals, 
        ccdr: hal::rcc::Ccdr, 
        cp: cortex_m::Peripherals
    ) -> Self {

        let mut delay = Delay::new(cp.SYST, ccdr.clocks);

        let mut adc1 = adc::Adc::adc1(
            dp.ADC1,
            4.MHz(),
            &mut delay,
            ccdr.peripheral.ADC12,
            &ccdr.clocks,
        )
        .enable();

        adc1.set_resolution(adc::Resolution::SixteenBit);

        Self { adc1, ccdr, delay }
    }

    /// Convert a u8 pin number into an AdcPin enum
    pub fn pin_from_number(
        &self,
        pins: &daisy::Pins,
        pin_num: u8,
    ) -> AdcPin {
        match pin_num {
            15 => AdcPin::PC0(pins.GPIO.PIN_15.into_analog()),
            16 => AdcPin::PA3(pins.GPIO.PIN_16.into_analog()),
            17 => AdcPin::PB1(pins.GPIO.PIN_17.into_analog()),
            18 => AdcPin::PA7(pins.GPIO.PIN_18.into_analog()),
            19 => AdcPin::PA6(pins.GPIO.PIN_19.into_analog()),
            20 => AdcPin::PC1(pins.GPIO.PIN_20.into_analog()),
            21 => AdcPin::PC4(pins.GPIO.PIN_21.into_analog()),
            _ => panic!("Invalid ADC pin number"),
        }
    }

    /// Read a pin by its enum
    pub fn read_pin(&mut self, adc_pin: &mut AdcPin) -> u32 {
        match adc_pin {
            AdcPin::PC0(pin) => self.adc1.read(pin).unwrap(),
            AdcPin::PA3(pin) => self.adc1.read(pin).unwrap(),
            AdcPin::PB1(pin) => self.adc1.read(pin).unwrap(),
            AdcPin::PA7(pin) => self.adc1.read(pin).unwrap(),
            AdcPin::PA6(pin) => self.adc1.read(pin).unwrap(),
            AdcPin::PC1(pin) => self.adc1.read(pin).unwrap(),
            AdcPin::PC4(pin) => self.adc1.read(pin).unwrap(),
        }
    }
}


pub enum Command {
    Forward = 1,
    Back    = 2,
    Left    = 3,
    Right   = 4,
    Ping    = 5,
}

impl Command {
    pub fn from_str(s: &str) -> Option<u8> {
        match s {
            "forward" => Some(Command::Forward as u8),
            "back"    => Some(Command::Back as u8),
            "left"    => Some(Command::Left as u8),
            "right"   => Some(Command::Right as u8),
            "ping"    => Some(Command::Ping as u8),
            _ => None,
        }
    }

    pub fn from_u8(v: u8) -> Option<&'static str> {
        match v {
            x if x == Command::Forward as u8 => Some("forward"),
            x if x == Command::Back    as u8 => Some("back"),
            x if x == Command::Left    as u8 => Some("left"),
            x if x == Command::Right   as u8 => Some("right"),
            x if x == Command::Ping    as u8 => Some("ping"),
            _ => None,
        }
    }
}
// UART interface
pub struct UartCmd {
    uart: Serial<hal::pac::USART1, (gpio::gpiob::PB6<Alternate<7>>, gpio::gpiob::PB7<Alternate<7>>)>,
}

/// Create UART1 on PB6/PB7 and return a UartCmd struct
pub fn uart_init(
    pin12: gpio::gpiob::PB9<gpio::Analog>,
    pin11: gpio::gpiob::PB8<gpio::Analog>,
    clocks: hal::rcc::Clocks,
    apb2: &mut hal::rcc::APB2,
) -> UartCmd {
    // Convert pins to AF7 (USART1)
    let tx = pin12.into_alternate::<7>();
    let rx = pin11.into_alternate::<7>();

    let uart = Serial::usart1(
        tx,
        rx,
        Config::default().baudrate(115_200_i32.bps()),
        clocks,
        apb2,
    );

    UartCmd { uart }
}

impl UartCmd {
    pub fn send_cmd(&mut self, cmd: &str) {
        if let Some(val) = Command::from_str(cmd) {
            let _ = self.uart.write(val);
        }
    }

    pub fn recv_cmd(&mut self) -> Option<&'static str> {
        if let Ok(byte) = self.uart.read() {
            Command::from_u8(byte)
        } else {
            None
        }
    }
}


use cortex_m::interrupt;
use daisy::AudioInterface;
use daisy::AUDIO_INTERFACE;

pub struct Sound {
    audio_interface: AudioInterface,
}

impl Sound {
    /// Create a new Sound struct and store the audio interface in the global.
    pub fn new(audio_interface: AudioInterface) -> Self {
        let audio_interface = audio_interface.spawn().unwrap();

        // Store the audio interface in the global safely.
        interrupt::free(|cs| {
            AUDIO_INTERFACE.borrow(cs).replace(Some(audio_interface));
        });

        // Return the Sound instance
        Self { audio_interface }
    }

    /// Process audio frames using a function or closure.
    /// `processor` takes (left, right) and returns (left, right).
    pub fn process_audio<F>(&mut self, mut processor: F)
    where
        F: FnMut(f32, f32) -> (f32, f32),
    {
        interrupt::free(|cs| {
            if let Some(audio_interface) = AUDIO_INTERFACE.borrow(cs).borrow_mut().as_mut() {
                audio_interface
                    .handle_interrupt_dma1_str1(|audio_buffer| {
                        for frame in audio_buffer {
                            let (left, right) = *frame;
                            *frame = processor(left, right); // Call the passed function
                        }
                    })
                    .unwrap();
            }
        });
    }
}

