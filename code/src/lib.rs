#![no_std]
#![no_main]

use defmt::error;
use crate::hal::nb;
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

#[derive(Debug)]
pub enum UartError<'a>{
        InvalidCommandStr(&'a str),
        InvalidCommandu8(u8),
        WriteError(fmt::Error),
        ReadError(fmt::Error),
        WouldBlock
    }

pub fn log_err<E: Debug>(err: E) {
    error!("An error occured: {:?}",err);
}


pub struct Command {
    commands: &'static [&'static str],
}

impl Command {
    pub fn new() -> Self {
        let commands: &[&str] = &["ok", "forward", "back", "left", "right", "ping"];
        Self { commands }
    }
        
    pub fn from_str<'a>(&self, s: &'a str) -> Result<u8, UartError<'a>> {
        let index = self.commands
            .iter()
            .position(|&x| x == s)
            .ok_or(UartError::InvalidCommandStr(s))?;
        Ok(index as u8)
    }

    pub fn from_u8(&self, v: u8) -> Result<&str, UartError> {
        let b: usize = v as usize;
        let rtn = self.commands.get(b)
        .ok_or(UartError::InvalidCommandu8(v))?;
        Ok(rtn)
    }
}

// ---------------- UART ----------------

pub struct UartCmd {
    tx: Tx<pac::USART1>,
    rx: Rx<pac::USART1>,
    command: Command,
}

/// Initialize USART1 with PB6=TX, PB7=RX

impl UartCmd {

    pub fn new(tx: Tx<pac::USART1>, rx: Rx<pac::USART1>, command: Command) -> UartCmd {
        UartCmd { tx, rx, command }
    }

    pub fn send_cmd<'a>(&mut self, cmd: &'a str) -> Result<(), UartError<'a>> {
        let val = match self.command.from_str(cmd) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        if let Err(e) = writeln!(self.tx, "{}", val) {
            return Err(UartError::WriteError(e));
        }
        return Ok(())
    }

    pub fn read_cmd(&mut self) -> Result<&str, UartError> {
        let byte = match self.rx.read() {
            Ok(byte) => byte,
            Err(nb::Error::Other(e)) => return Err(UartError::ReadError(fmt::Error)),
            Err(nb::Error::WouldBlock) => return Err(UartError::WouldBlock),
        };

        if let Ok(s) = self.command.from_u8(byte) {
            return Ok(s)
        } else {
            return Err(UartError::InvalidCommandu8(byte))
        }
    }
}
