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

// Initialize USART1 with PB6=TX, PB7=RX

enum Info <'a>{
    Bytes(&'a [u8]),
    Str(&'a str),
}
