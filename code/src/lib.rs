#![no_std]
#![no_main]

use crate::hal::nb;
use crc8_rs::{has_valid_crc8, insert_crc8};
use daisy::hal::gpio::gpioa::{PA3, PA6, PA7};
use daisy::hal::gpio::gpiob::PB1;
use daisy::hal::gpio::gpioc::{PC0, PC1, PC4};
use daisy::pac::{ADC1, ADC2};
use daisy::{hal, pac};
use hal::adc::{Adc, Enabled};
use hal::gpio::Analog;
use hal::prelude::*;
use hal::serial::{Rx, Tx};

const CMD_LEN: usize = 32;

#[derive(defmt::Format, core::fmt::Debug)]
pub enum AdcsError {
    ReadError(&'static str),
    WouldBlock,
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
        Self {
            adc1,
            adc2,
            pc0,
            pa3,
            pb1,
            pa7,
            pa6,
            pc1,
            pc4,
        }
    }

    // uses adc1 to read an individual pin
    //
    // TODO set up DMA transfer for a fast scan of all adcs

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

// ---------------- Commands ----------------

// this section turns string commands into numbers to be sent to the esp32 and vice versa
// uses USART1 on the daisy seed to comunicate witht eh esp32

pub enum UartError {
    WriteError,
    ReadError,
    WouldBlock,
    BufferOverflow([u8; CMD_LEN]),
    Utf8Error([u8; CMD_LEN]),
    AckMismatch([u8; CMD_LEN]),
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
    pub fn read_cmd_lazy(&mut self) -> Result<[u8; CMD_LEN], UartError> {
        let mut out = [0u8; CMD_LEN];
        let mut index = 0;

        loop {
            match self.rx.read() {
                Ok(byte) => {
                    if byte == b'\n' {
                        if has_valid_crc8(out, 0xD5) {
                            return Ok(out);
                        } else {
                            self.write_cmd_lazy("error").ok();
                            return Err(UartError::AckMismatch(out));
                        }
                    } else {
                        out[index] = byte;
                    }
                }

                //this is technicly blockling but if this was not here
                //the reads would be broken up
                Err(nb::Error::WouldBlock) => continue,
                Err(nb::Error::Other(e)) => {
                    defmt::warn!("Uart Error: {}", e);
                    return Err(UartError::ReadError);
                }
            }
            index += 1;
        }
    }

    // use this to get string let (len, buf) = uart.read_cmd()?;

    //let s = core::str::from_utf8(&buf[..len])
    //.map_err(|_| UartError::InvalidUtf8)?;

    /// Non-lazy: wait for a full line, echo it back
    pub fn read_cmd(&mut self) -> Result<[u8; CMD_LEN], UartError> {
        let vec = self.read_cmd_lazy()?;

        //conver vec to &str and output to lazy write
        self.write_cmd_lazy("error")?;

        Ok(vec)
    }

    /// Lazy write: writes a line out, does not wait for a response
    pub fn write_cmd_lazy(&mut self, s: &str) -> Result<(), UartError> {
        let bytes = s.as_bytes();

        // Runtime check: string must not exceed 32 bytes
        if bytes.len() > CMD_LEN {
            return Err(UartError::BufferOverflow([0u8; CMD_LEN]));
        }

        // Copy into a fixed-size 32-byte buffer, padding remaining bytes with 0
        let mut buf = [0u8; 32];
        buf[..bytes.len()].copy_from_slice(bytes);

        // Compute CRC on the 32-byte buffer
        let crc_data = insert_crc8(buf, 0xD5);

        // Send newline first

        // Send the bytes with CRC
        for &b in &crc_data {
            match self.tx.write(b) {
                Ok(()) => continue,
                Err(nb::Error::WouldBlock) => return Err(UartError::WouldBlock),
                Err(nb::Error::Other(e)) => {
                    defmt::warn!("Uart Error: {}", e);
                    return Err(UartError::WriteError);
                }
            }
        }

        match self.tx.write(b'\n') {
            Ok(()) => Ok(()),
            Err(nb::Error::WouldBlock) => Err(UartError::WouldBlock),
            Err(nb::Error::Other(e)) => {
                defmt::warn!("Uart Error: {}", e);
                Err(UartError::WriteError)
            }
        }
    }

    /// Non-lazy write: writes a line, then waits for echo, returns error if mismatch
    pub fn write_cmd(&mut self, s: &str) -> Result<(), UartError> {
        self.write_cmd_lazy(s)?; // send message

        let vec = match self.read_cmd_lazy() {
            Ok(l) => l,
            Err(UartError::WouldBlock) => return Err(UartError::WouldBlock),
            Err(e) => return Err(e),
        };

        let tmp: &str = "error";
        let s_in: &str = bytes_to_str(&vec)?;

        if s_in == tmp {
            Err(UartError::AckMismatch(vec))
        } else {
            Ok(())
        }
    }
}
fn bytes_to_str(arr: &[u8; CMD_LEN]) -> Result<&str, UartError> {
    // Find the first zero byte or use full length
    let len = arr.iter().position(|&b| b == 0).unwrap_or(arr.len());

    // Slice out the meaningful bytes
    let slice = &arr[..len];

    // Convert to &str safely
    core::str::from_utf8(slice).map_err(|_| UartError::Utf8Error(*arr))
}
