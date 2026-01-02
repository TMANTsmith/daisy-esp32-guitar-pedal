use core::fmt::Write;
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


// WARN most of this stuff is unused

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

// TODO use crc8
pub struct UartCmd {
    tx: Tx<pac::USART1>,
    rx: Rx<pac::USART1>,
}

// Initialize USART1 with PB6=TX, PB7=RX

enum Info <'a>{
    Bytes(&'a [u8]),
    Str(&'a str),
}

impl UartCmd {
    pub fn new(tx: Tx<pac::USART1>, rx: Rx<pac::USART1>) -> UartCmd {
        UartCmd { tx, rx }
    }

    async fn write(&mut self, info: Info) -> Result<(), UartError> {
        match info {
            Info::Bytes(b) => self.write_bytes(b),
            Info::Str(s) => self.write_str(s),
        }
    }

    async fn write_bytes(&mut self, value: &[u8]) -> Result<(), UartError> {
        for &byte in value {
            loop {
                match self.tx.write(b) {
                    Ok(()) => continue,
                    Err(nb::Error::WouldBlock) => {
                        yeild_now().await;
                    },
                    Err(e) => return Err(UartError::WriteError(e))
                }
            }
        }
        loop {
            match self.tx.write(b'\n') {
                Ok(()) => break,
                Err(nb::Error::WouldBlock) => yield_now().await,
                Err(e) => return Err(UartError::WriteError(e)),
            }
        }

        Ok(())
    }

    async fn write_str(&mut self, value: &str) -> Result<(), UartError> {
        loop {
            match self.tx.write_str(value) {
                Ok(_) => {
                    self.tx.write(b'\n').map_err(UartError::WriteError)?;
                    return Ok(());
                }
                Err(nb::Error::WouldBlock) => {
                    yield_now().await;
                }
                Err(e) => return Err(UartError::WriteError(e)),
            }
        }
    }

    // -------------------------
    // ASYNC READ (newline-terminated)
    // -------------------------

    pub async fn read_buf(
        &mut self,
        buf: &mut [u8],
    ) -> Result<Info, UartError> {
        let mut idx = 0;

        loop {
            match self.rx.read() {
                Ok(b) => {
                    if b == b'\n' {
                        return Ok(Info::Bytes(buf));
                    } else if b == b'\t' {
                        return Ok(Info::Str(str::from_utf8(buf).unwrap()))
                    }
                    if idx >= buf.len() {
                        return Err(UartError::BufferOverflow);
                    }

                    buf[idx] = b;
                    idx += 1;
                },
                Err(nb::Error::WouldBlock) => {
                    return Err(UartError::WouldBlock)
                },
                Err(_) => return Err(UartError::ReadError),
            }
        }
    }
}

