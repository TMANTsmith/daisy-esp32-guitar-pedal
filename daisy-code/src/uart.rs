use daisy::pac::USART1;
use core::fmt::Write;
use daisy::hal::nb;
use daisy::hal::gpio::gpioa::{PA3, PA6, PA7};
use daisy::hal::gpio::gpiob::PB1;
use daisy::hal::gpio::gpioc::{PC0, PC1, PC4};
use daisy::pac::{ADC1, ADC2};
use daisy::{hal, pac};
use hal::adc::{Adc, Enabled};
use hal::gpio::Analog;
use hal::prelude::*;
use hal::serial::{Rx, Tx};

const CMD_LEN: usize = 64;


// WARN most of this stuff is unused

pub enum UartError<'a> {
    WriteError,
    ReadError,
    WouldBlock,
    BufferOverflow,
    Utf8Error([u8; CMD_LEN]),
    AckMismatch(&'a [u8]),
}

// ---------------- UART ----------------
//use std::fmt::{Error, Write};

// TODO use crc8

// Initialize USART1 with PB6=TX, PB7=RX

pub enum Info <'a>{
    Bytes(&'a [u8]),
    Str(&'a str),
}

pub fn uart_read<'a>(rx: &'a mut Rx<USART1>, buf: &'a mut [u8; 64]) -> Result<Info<'a>, UartError<'a>> {
    let mut idx = 0;

    loop {
        match rx.read() {
            Ok(b) => {
                if b == b'\n' {
                    let data = &buf[..(idx - 1)];
                    let crc8 = crc::Crc::<u8>::new(&crc::CRC_8_SMBUS);

                    if buf[idx] != crc8.checksum(data) {
                        return Err(UartError::AckMismatch(data))
                    }

                    // turns entire buffer into data fix this
                    return Ok(Info::Bytes(data));

                } else if b == b'\t' {
                    let data = &buf[..(idx - 1)];
                    let crc8 = crc::Crc::<u8>::new(&crc::CRC_8_SMBUS);

                    if buf[idx] != crc8.checksum(data) {
                        return Err(UartError::AckMismatch(data))
                    }

                    // turns entire buffer into data fix this
                    return Ok(Info::Str(str::from_utf8(data).unwrap()))
                }

                if idx >= buf.len() {
                    return Err(UartError::BufferOverflow)
                }

                buf[idx] = b;
                idx += 1;
            },
            Err(nb::Error::WouldBlock) => {
                // this SHOULD never happen
                return Err(UartError::WouldBlock)
            },
            Err(_) => return Err(UartError::ReadError),
        }
    }
}

