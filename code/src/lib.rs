#![no_std]
#![no_main]

use core::fmt;
use hal::adc::{Adc, Enabled};
use hal::serial::{Tx, Rx};
use core::fmt::Write;
use daisy::{pac, hal};
use hal::prelude::*;
use hal::gpio::{Analog};
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
    pub pa3: PA3<Analog>,
    pub pb1: PB1<Analog>,
    pub pa7: PA7<Analog>,
    pub pa6: PA6<Analog>, // correct pins as needed
    pub pc1: PC1<Analog>,
    pub pc4: PC4<Analog>,
}


impl Adcs {

    /// Inputs the adcs 0-6 on the daisy seed

    pub fn new(
        adc1: Adc<ADC1, Enabled>,
        adc2: Adc<ADC2, Enabled>,
        pc0: PC0<Analog>,
        pa3: PA3<Analog>,
        pb1: PB1<Analog>,
        pa7: PA7<Analog>,
        pa6: PA6<Analog>,
        pc1: PC1<Analog>,
        pc4: PC4<Analog>,
    ) -> Self {
        Self { adc1, adc2, pc0, pa3, pb1, pa7, pa6, pc1, pc4 }
    }


   /// uses adc1 to read an individual pin 
   ///
   /// TODO set up DMA transfer for a fast scan of all adcs

    pub fn read_all(&mut self, buffer: &mut [u32; 7]) {
        buffer[0] = self.adc1.read(&mut self.pc0).unwrap();
        buffer[1] = self.adc1.read(&mut self.pa3).unwrap();
        buffer[2] = self.adc1.read(&mut self.pb1).unwrap();
        buffer[3] = self.adc1.read(&mut self.pa7).unwrap();
        buffer[4] = self.adc1.read(&mut self.pa6).unwrap();
        buffer[5] = self.adc1.read(&mut self.pc1).unwrap();
        buffer[6] = self.adc1.read(&mut self.pc4).unwrap();
    }


}
// ---------------- Commands ----------------

/// this section turns string commands into numbers to be sent to the esp32 and vice versa
/// uses USART1 on the daisy seed to comunicate witht eh esp32
pub enum UartError {
        InvalidCommandStr(&str),
        InvalidCommandu8(u8),
        WriteError(fmt::Error),
        ReadError(fmt::Error),
    }


pub struct Command<'a> {
    commands: &'a[&str]
}

impl Command {
    pub fn new() -> Self {
        let command: &[&str] = &["ok", "forward", "back", "left", "right", "ping"];
        Self { command }
    }
        
    pub fn from_str(s: &str) -> Result<u8, UartError> {
        if let Some(index) = self.command.iter().position(|&x| x == s) {
            return Ok(index as u8)
        }
        else {
            return Err(UartError::InvalidCommandStr(s))
        }
    }

    pub fn from_u8(v: u8) -> Result<&str, UartError> {
        let b: usize = v as usize;
        if let Some(rtn) = self.command.get(b) {
            return Ok(rtn)
        }
        else {
            return Err(UartError::InvalidCommandu8(v))
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

    pub fn send_cmd(&mut self, cmd: &str) -> Option<UartError> {
        if let Some(val) = Command::from_str(cmd) {
            if let Err(e) = writeln!(self.tx, "{}", val) {
                return Err(UartError::ReadError(e))
            }
            return None
        } else {
            return Some(UartError::InvalidCommand)
        }
    }

    pub fn read_cmd(&mut self) -> Result<Command, UartError> {
        match block!(self.rx.read()) {
            Ok(byte) => Command::from_u8(byte)
                .ok_or(UartError::InvalidCommandu8(byte)),
            Err(e) => Some(UartError::ReadError(e)),
        }
    }
}
