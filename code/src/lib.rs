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


/// # ---------------- ADC ----------------
/// - This sets up the ADCs 

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

pub enum Adc_val, { 
    adc0: f32,
    adc1: f32,
    adc2: f32,
    adc3: f32,
    adc4: f32,
    adc5: f32,
    adc6: f32,
}

impl Adcs {
    
        /// Inputs the adcs 0-6 on the daisy seed

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


   /// uses adc1 to read an individual pin 
   ///
   /// TODO set up DMA transfer for a fast scan of all adcs

    pub fn read_all(&mut self) -> { 
        pins = [ self.pc0, self.pc1, self.pc4, self.pa3, self.pa6, self.pa7, self.pb1 ];
        self.adc1.configure_scan(&pins, AdcConfig::default());

        let mut buffer: [u16; 7] = [0; 7];

        let mut dma_transfer = Transfer::init(
            &mut adc,
            &mut buffer,
            DmaConfig::default()
        );




    pub fn read_pin_adc1(&mut self, pin: u8) -> f32 {
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

    /// uses adc2 to read an individual pin 
    pub fn read_pin_adc2(&mut self, pin: u8) -> f32 {
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

/// this section turns string commands into numbers to be sent to the esp32 and vice versa
/// uses USART1 on the daisy seed to comunicate witht eh esp32
pub enum Command {
    ok      = 0,
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
            "ok" => Some(Command::ok as u8),
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
            x if x == Command::ok as u8 => Some("ok"),
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

impl UartCmd {

    pub fn new(tx: Tx<pac::USART1>, rx: Rx<pac::USART1>) -> UartCmd {
        UartCmd { tx, rx }
    }

    pub fn send_cmd(&mut self, cmd: &str) {
        if let Some(val) = Command::from_str(cmd) {
            writeln!(self.tx, "{}", val).unwrap();
        }
    }

    pub fn read_cmd(&mut self) -> Option<&'static str> {
        match block!(self.rx.read()){
            Ok(byte) => Command::from_u8(byte),
            Err(_) => None,
        }
    }
}

