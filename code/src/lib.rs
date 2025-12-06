#![no_std]
#![no_main]

use core::fmt::Debug;
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
pub enum UartError{
    WriteError,
    ReadError,
    WouldBlock,
    BufferOverflow,
    InvalidUtf8,
    AckMismatch,
    Timeout,
}

pub fn log_err<E: Debug + defmt::Format>(err: E) {
    error!("An error occured: {:?}",err);
}


// ---------------- UART ----------------
//use std::fmt::{Error, Write};

pub struct UartCmd {
    tx: Tx<pac::USART1>,
    rx: Rx<pac::USART1>,
}

/// Initialize USART1 with PB6=TX, PB7=RX

impl UartCmd {

    pub fn new(tx: Tx<pac::USART1>, rx: Rx<pac::USART1>) -> UartCmd {
        UartCmd { tx, rx, }
    }
    pub fn read_cmd_lazy(&mut self) -> Result<Option<[u8; 64]>, UartError> {
        let mut buf = [0u8; 64];
        let mut pos = 0;

        loop {
            match self.rx.read() {
                Ok(byte) => {
                    if pos >= buf.len() {
                        return Err(UartError::BufferOverflow);
                    }
                    buf[pos] = byte;
                    pos += 1;

                    if byte == b'\n' {
                        let mut line = [0u8; 64];
                        line[..pos - 1].copy_from_slice(&buf[..pos - 1]); // strip newline
                        return Ok(Some(line));
                    }
                }
                Err(WouldBlock) => return Ok(None),
                Err(_) => return Err(UartError::ReadError),
            }
        }
    }

    /// Non-lazy read: reads a line, then writes it back to acknowledge
    pub fn read_cmd(&mut self) -> Result<[u8; 64], UartError> {
        let line = loop {
            if let Some(l) = self.read_cmd_lazy()? {
                break l;
            }
        };

        // echo it back
        self.write_cmd_lazy(str::from_utf8(&line).map_err(|_| UartError::InvalidUtf8)?)?;

        Ok(line)
    }

    /// Lazy write: writes a line out, does not wait for a response
    pub fn write_cmd_lazy(&mut self, s: &str) -> Result<(), UartError> {
        let bytes = s.as_bytes();
        for &b in bytes {
            // loop until the byte is written
            loop {
                match self.tx.write(b) {
                    Ok(()) => break,
                    Err(nb::Error::WouldBlock) => continue, // try again
                    Err(nb::Error::Other(_)) => return Err(UartError::WriteError),
                }
            }
        }

        // send newline
        loop {
            match self.tx.write(b'\n') {
                Ok(()) => break,
                Err(nb::Error::WouldBlock) => continue,
                Err(nb::Error::Other(_)) => return Err(UartError::WriteError),
            }
        }

        Ok(())
    }

    /// Non-lazy write: writes a line, then waits for echo, returns error if mismatch
    pub fn write_cmd(&mut self, s: &str) -> Result<(), UartError> {
        self.write_cmd_lazy(s)?; // send message

        let response = loop {
            if let Some(resp) = self.read_cmd_lazy()? {
                break resp;
            }
        };

        if &response[..s.len()] != s.as_bytes() {
            return Err(UartError::AckMismatch);
        }

        Ok(())
    }
}

