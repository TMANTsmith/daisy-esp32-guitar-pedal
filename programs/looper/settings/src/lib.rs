#![no_std]

use core::marker::PhantomData;
use pcobs::{ EncodeError, DecodeError };
extern crate serde;
#[macro_use]
extern crate serde_big_array;
use serde_big_array::BigArray;
use modular_bitfield::{Specifier, bitfield};
use modular_bitfield::specifiers::B6;


// shaired 
pub const BAUDRATE: u32 = 4800;
pub const SAMPLE_RATE: usize = 48000;
pub const FRAME_DELIM: u8 = 0x00;


#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Mute {
    Mute,
    Unmute,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum CommandUart {
    Muter(Mute),
    Looper(State),
}

#[bitfield]
#[derive(Debug, PartialEq, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub struct State {
    pub track_number: B6, // goes up to 4
    pub mode: TrackState,
}


#[derive(Specifier, Debug, PartialEq, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[bits = 2]
pub enum TrackState {
    Idle,
    Recording,
    Clear,
}
    

