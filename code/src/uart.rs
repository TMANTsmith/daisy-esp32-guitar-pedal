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


// usage (&str or [u8])
pub trait Write<T> {
    fn write(&mut self,value: T);
}

impl Write<[u8]> for UartCmd {
    async fn write(&mut self, value: [u8]) -> Result<(), UartError> {
        for &byte in value {
            match self.tx.write(b) {
                Ok(()) => continue,
                Err(nb::Error::WouldBlock) => {
                    yeild_now().await;
                },
                Err(e) => return Err(UartError::WriteError(e))
            }
        }
        self.tx.write(b'\n');
        Ok(())
    }
    
    async fn write(&mut self, value: &str)-> Result<(), UartError> {
        match self.tx.write_str(value) {
            Ok(v) => {
                self.tx.write(b'\n');
                Ok(v)
            },
            Err(nb::Error::WouldBlock) => {
                yeild_now().await;
            },
            Err(e) => Err(UartError::WriteError(e))
        }
    }
}

        
        


impl UartCmd {
    pub fn new(tx: Tx<pac::USART1>, rx: Rx<pac::USART1>) -> UartCmd {
        UartCmd { tx, rx }
    }

    // -------------------------
    // ASYNC READ (newline-terminated)
    // -------------------------
    pub async fn read_buf(
        &mut self,
        buf: &mut [u8],
    ) -> Result<usize, UartError> {
        let mut idx = 0;

        loop {
            match self.rx.read() {
                Ok(b) => {
                    if b == b'\n' {
                        return Ok(idx);
                    }

                    if idx >= buf.len() {
                        return Err(UartError::BufferOverflow);
                    }

                    buf[idx] = b;
                    idx += 1;
                }
                Err(nb::Error::WouldBlock) => {
                    yield_now().await;
                }
                Err(_) => return Err(UartError::Read),
            }
        }
    }
}

