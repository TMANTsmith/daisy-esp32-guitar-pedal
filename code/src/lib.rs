#![no_std]
#![no_main]

use crc8_rs::{ has_valid_crc8, insert_crc8 };
use crc8_rs::insert_crc8;
use heapless::Vec;
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

#[derive(defmt::Format, core::fmt::Debug)]
pub enum AdcsError {
    ReadError(&'static str),
    WouldBlock
}
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

    pub fn read_all(&mut self, buffer: &mut [u32; 7]) -> Result<(), AdcsError> {
        buffer[0] = match self.adc1.read(&mut self.pc0) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pc0")),
        };

        buffer[1] = match self.adc1.read(&mut self.pa3) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pa3")),
        };

        buffer[2] = match self.adc1.read(&mut self.pb1) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pb1")),
        };

        buffer[3] = match self.adc1.read(&mut self.pa7) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pa7")),
        };

        buffer[4] = match self.adc1.read(&mut self.pa6) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pa6")),
        };

        buffer[5] = match self.adc1.read(&mut self.pc1) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pc1")),
        };

        buffer[6] = match self.adc1.read(&mut self.pc4) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pc4")),
        };

        Ok(())
    }


}

// lets the read return &str
pub struct UartString {
    buf: Vec<u8, 64>,
}

impl UartString {
    /// Create a new empty UartString
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
        }
    }

    /// Return as &str (valid UTF-8)
    pub fn as_str(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(&self.buf)
    }

    /// Return as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Current length
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Push a byte (returns error if full)
    pub fn push(&mut self, byte: u8) -> Result<(), ()> {
        self.buf.push(byte).map_err(|_| ())
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

// ---------------- Commands ----------------

/// this section turns string commands into numbers to be sent to the esp32 and vice versa
/// uses USART1 on the daisy seed to comunicate witht eh esp32

#[derive(defmt::Format, core::fmt::Debug)]
pub enum UartError{
    WriteError(T),
    ReadError(T),
    WouldBlock,
    BufferOverflow(&Vec<u8, 64>),
    Utf8Error(&Vec<u8, 64>),
    AckMismatch(&Vec<u8, 64>),
}


// ---------------- UART ----------------
//use std::fmt::{Error, Write};

// chang UART do I2C?
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
    /// does not echo back
    pub fn read_cmd_lazy(&mut self) -> Result<UartString, UartError> {
        let mut out = UartString::new();

        loop {
            match self.rx.read() {
                Ok(byte) => {
                    if byte == b'\n' {
                        if has_valid_crc8(out.as_bytes(), 0xD5) {
                            return Ok(out);
                        } else {
                            write_cmd_lazy("error").ok();
                            return Err(UartError::AckMismatch);
                        }
                    }

                    if out.push(byte).is_err() {
                        return Err(UartError::BufferOverflow);
                    }
                }

                Err(nb::Error::WouldBlock) => continue,
                Err(nb::Error::Other(e)) => return Err(UartError::ReadError(e)),
            }
        }
    }

    // use this to get string let (len, buf) = uart.read_cmd()?;

    //let s = core::str::from_utf8(&buf[..len])
    //.map_err(|_| UartError::InvalidUtf8)?;

    /// Non-lazy: wait for a full line, echo it back
    pub fn read_cmd(&mut self) -> Result<UartString, UartError> {
        loop {
            match self.read_cmd_lazy() {
                Ok(vec) if vec.iter().len() > 0 => break,
                Ok(_) => continue,
                Err(e) => return Err(e),
            }
        };
        //conver vec to &str and output to lazy write
        self.write_cmd_lazy("error")?;

        Ok(vec)
    }

    /// Lazy write: writes a line out, does not wait for a response
    pub fn write_cmd_lazy(&mut self, s: &str) -> Result<(), UartError> {
        let bytes = s.as_bytes();
        let crc_data = insert_crc8(bytes , 0xD5);

        loop {
            match self.tx.write(b'\n') {
                Ok(()) => break,
                Err(nb::Error::WouldBlock) => return Err(UartError::WouldBlock),
                Err(nb::Error::Other(e)) => return Err(UartError::WriteError(e)),
            }
        }

        for &b in bytes {
            // loop until the byte is written
            loop {
                match self.tx.write(b) {
                    Ok(()) => break,
                    Err(nb::Error::WouldBlock) => return Err(UartError::WouldBlock), 
                    Err(nb::Error::Other(e)) => return Err(UartError::WriteError(e)),
                }
            }
        }
        Ok(())
    }

    /// Non-lazy write: writes a line, then waits for echo, returns error if mismatch
    pub fn write_cmd(&mut self, s: &str) -> Result<(), UartError> {
        self.write_cmd_lazy(s)?; // send message

        let vec = loop {
            match self.read_cmd_lazy() {
                Ok(l) => break l,
                Err(UartError::WouldBlock) => return Err(UartError::WouldBlock),
                Err(e) => return Err(e),
            }
        };

        let tmp: &str = "error";
        let s_in: &str = core::str::from_utf8(&vec).expect(UartError::Utf8Error(&vec));
        
        if s_in == tmp {
            return UartError::AckMismatch(&vec)
        }
        else { 
            Ok(())
        }
    }
}

