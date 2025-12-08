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

#[derive(defmt::Format, core::fmt::Debug)]
pub enum UartError{
    WriteError,
    ReadError,
    WouldBlock,
    BufferOverflow,
    InvalidUtf8,
    AckMismatch,
    Timeout,
}

impl UartError {
    pub fn log(self) {
        defmt::error!("UART error: {}", self);
    }
}



// ---------------- UART ----------------
//use std::fmt::{Error, Write};

pub struct UartCmd {
    tx: Tx<pac::USART1>,
    rx: Rx<pac::USART1>,
}

// Initialize USART1 with PB6=TX, PB7=RX

impl UartCmd {

    pub fn new(tx: Tx<pac::USART1>, rx: Rx<pac::USART1>) -> UartCmd {
        UartCmd { tx, rx }
    }

    /// Reads until `\n`, returns (length, buffer)
    pub fn read_cmd_lazy(&mut self) -> Result<(usize, [u8; 64]), UartError> {
        let mut buf = [0u8; 64];
        let mut pos = 0;

        loop {
            match self.rx.read() {
                Ok(byte) => {
                    if pos >= buf.len() {
                        return Err(UartError::BufferOverflow);
                    }

                    if byte == b'\n' {
                        // strip newline: just don't include it in pos
                        return Ok((pos, buf));
                    }

                    buf[pos] = byte;
                    pos += 1;
                }

                Err(nb::Error::WouldBlock) => return Err(UartError::WouldBlock),
                Err(_) => return Err(UartError::ReadError),
            }
        }
    }

    // use this to get string let (len, buf) = uart.read_cmd()?;

    //let s = core::str::from_utf8(&buf[..len])
    //.map_err(|_| UartError::InvalidUtf8)?;

    /// Non-lazy: wait for a full line, echo it back
    pub fn read_cmd(&mut self) -> Result<(usize, [u8; 64]), UartError> {
        let (len, buf) = loop {
            match self.read_cmd_lazy() {
                Ok((len, buf)) if len > 0 => break (len, buf),
                Ok(_) => continue,
                Err(e) => return Err(e),
            }
        };

        Ok((len, buf))
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

        // TODO this is technicly blocking and should be async 
        // or at least give a timeout error
        let (len, response) = loop {
            match self.read_cmd_lazy() {
                Ok(l) => break l,
                Err(UartError::WouldBlock) => continue,
                Err(e) => return Err(e),
            }
        };
        if len < s.len() {
            return Err(UartError::AckMismatch);
        }

        if &response[..s.len()] != s.as_bytes() {
            return Err(UartError::AckMismatch);
        }


        Ok(())
    }
}

