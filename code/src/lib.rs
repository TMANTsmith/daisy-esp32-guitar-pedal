#![no_std]
#![no_main]


use hal::adc::{Adc, Enabled, Resolution};
use hal::serial::{Tx, Rx};
use nb::block;
use core::fmt::Write;
use daisy::{pac, hal};
use hal::delay::Delay;
use hal::prelude::*;
use hal::gpio::{gpioa, gpiob, gpioc, Analog, Output, PushPull, Alternate};
use hal::serial::Serial;
use cortex_m::interrupt;
use cortex_m::interrupt::Mutex;
use core::cell::RefCell;
use daisy::pins::Gpio;
use daisy::hal::gpio::gpioa::{PA3, PA6, PA7};
use daisy::hal::gpio::gpiob::PB1;
use daisy::hal::gpio::gpioc::{PC0, PC1, PC4};
use daisy::pac::{ADC1, ADC2};

// ---------------- ADC ----------------

pub struct Adcs {
    pub adc1: Adc<pac::ADC1, Enabled>,
    pub adc2: Adc<pac::ADC2, Enabled>,
    pub pc0: PC0<Analog>,
    pub pc1: PC1<Analog>,
    pub pc4: PC4<Analog>,
    pub pa3: PA3<Analog>,
    pub pa6: PA6<Analog>, // correct pins as needed
    pub pa7: PA7<Analog>,
    pub pb1: PB1<Analog>,
}

impl Adcs {
    
        pub fn new(
            adc1: Adc<ADC1, Enabled>,
            adc2: Adc<ADC2, Enabled>,
            pc0: PC0<Analog>,
            pc1: PC1<Analog>,
            pc4: PC4<Analog>,
            pa3: PA3<Analog>,
            pa6: PA6<Analog>,
            pa7: PA7<Analog>,
            pb1: PB1<Analog>,
    ) -> Self {
        Self { adc1, adc2, pc0, pc1, pc4, pa3, pa6, pa7, pb1 }
    }


    
    pub fn read_pin_adc1(&mut self, pin: u8) -> u32 {
        match pin {
            22 => self.adc1.read(&mut self.pc0).unwrap(),
            23 => self.adc1.read(&mut self.pa3).unwrap(),
            24 => self.adc1.read(&mut self.pb1).unwrap(),
            25 => self.adc1.read(&mut self.pa7).unwrap(),
            26 => self.adc1.read(&mut self.pa6).unwrap(),
            27 => self.adc1.read(&mut self.pc1).unwrap(),
            28 => self.adc1.read(&mut self.pc4).unwrap(),
            _ => panic!("invalid pin"),
        }
    }

    pub fn read_pin_adc2(&mut self, pin: u8) -> u32 {
        match pin {
            22 => self.adc2.read(&mut self.pc0).unwrap(),
            23 => self.adc2.read(&mut self.pa3).unwrap(),
            24 => self.adc2.read(&mut self.pb1).unwrap(),
            25 => self.adc2.read(&mut self.pa7).unwrap(),
            26 => self.adc2.read(&mut self.pa6).unwrap(),
            27 => self.adc2.read(&mut self.pc1).unwrap(),
            28 => self.adc2.read(&mut self.pc4).unwrap(),
            _ => panic!("invalid pin"),
        }
    }
}
// ---------------- Commands ----------------

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
            "back" => Some(Command::Back as u8),
            "left" => Some(Command::Left as u8),
            "right" => Some(Command::Right as u8),
            "ping" => Some(Command::Ping as u8),
            _ => None,
        }
    }

    pub fn from_u8(v: u8) -> Option<&'static str> {
        match v {
            x if x == Command::Forward as u8 => Some("forward"),
            x if x == Command::Back as u8 => Some("back"),
            x if x == Command::Left as u8 => Some("left"),
            x if x == Command::Right as u8 => Some("right"),
            x if x == Command::Ping as u8 => Some("ping"),
            _ => None,
        }
    }
}

// ---------------- UART ----------------

pub struct UartCmd {
    tx: Tx<pac::USART1>,
    rx: Rx<pac::USART1>,
}

/// Initialize USART1 with PB6=TX, PB7=RX
pub fn uart_init(
    dp: pac::Peripherals,
    ccdr: hal::rcc::Ccdr,
    tx: gpiob::PB6<Output<PushPull>>,
    rx: gpiob::PB7<Output<PushPull>>,
) -> UartCmd {
    // Convert pins to alternate function 7
    let tx = tx.into_alternate::<7>();
    let rx = rx.into_alternate::<7>();

    // Create USART1
    let usart = dp
        .USART1
        .serial((tx, rx), 19_200.bps(), ccdr.peripheral.USART1, &ccdr.clocks)
        .unwrap();

    let (tx, rx) = usart.split();

    UartCmd { tx, rx }
}

impl UartCmd {
    pub fn send_cmd(&mut self, cmd: &str) {
        if let Some(val) = Command::from_str(cmd) {
            writeln!(self.tx, "{}", val).unwrap();
        }
    }

    pub fn recv_cmd(&mut self) -> Option<&'static str> {
        match block!(self.rx.read()){
            Ok(byte) => Command::from_u8(byte),
            Err(_) => None,
        }
    }
}

// ---------------- Audio ----------------

use daisy::audio::interface::Interface as AudioInterface;

static AUDIO_INTERFACE: Mutex<RefCell<Option<AudioInterface>>> =
    Mutex::new(RefCell::new(None));

pub struct Sound;

impl Sound {
    pub fn init(interface: AudioInterface) {
        let spawned = interface.spawn().unwrap();

        interrupt::free(|cs| AUDIO_INTERFACE.borrow(cs).replace(Some(spawned)));
    }

    pub fn process<F>(mut processor: F)
    where
        F: FnMut(f32, f32) -> (f32, f32),
    {
        interrupt::free(|cs| {
            if let Some(audio) = AUDIO_INTERFACE.borrow(cs).borrow_mut().as_mut() {
                audio
                    .handle_interrupt_dma1_str1(|buffer| {
                        for frame in buffer {
                            let (l, r) = *frame;
                            *frame = processor(l, r);
                        }
                    })
                    .unwrap();
            }
        });
    }
}

