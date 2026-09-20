#![no_std]

use core::marker::PhantomData;
use pcobs::{ EncodeError, DecodeError };

extern crate serde;
#[macro_use]
extern crate serde_big_array;
use serde_big_array::BigArray;


// shaired 
pub const BIN_VALUE_SIZE: usize = core::mem::size_of::<BinValue>();
pub const FFT_INPUT: usize = 4096;
pub const FFT_BINS: usize = FFT_INPUT / 2;
pub const BAUDRATE: u32 = 2_000_000;

pub type BinValue = i16; 

pub const FFT_BYTES_TAKEN: usize = FFT_BINS * BIN_VALUE_SIZE;


pub const COBS_BUF: usize = BIN_VALUE_SIZE * FFT_BINS + (BIN_VALUE_SIZE* FFT_BINS + 253) / 254 + 10;

pub const FRAME_DELIM: u8 = 0x00;


// daisy settings

pub const FFT_N: usize = FFT_BINS * 2; // input size


// ESP32 settings

pub const SAMPLE_RATE: usize = 48000;
pub const INPUT_IS_DB: bool = false; // false => device sends linear magnitude, convert client-side
pub const DB_MIN: i32= -100;
pub const DB_MAX: i32 = 50;
pub const MIN_DISPLAY_FREQ: usize = 20; // Hz, log axis can't start at 0
pub const SMOOTHING: f32 = 0.5; // temporal smoothing, 0 = none, closer to 1 = smoother/slower
pub const PEAK_DECAY_DB_PER_SEC: f32 = 14.0;
pub const PEAK_MIN_DB: i32 = -82; // ignore bins quieter than this when picking labeled peaks
pub const PEAK_COUNT: usize = 4;
pub const BIN_IS_FLOAT: bool = BIN_VALUE_SIZE == 4; // true only when BinValue = f32
pub const BIN_SCALE: f32 = if BIN_IS_FLOAT { 1.0 } else if BIN_VALUE_SIZE == 2 { i16::MAX as f32 } else { i8::MAX as f32 };

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Mute {
    Mute,
    Unmute,
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CommandUart {
    Mute(Mute),
}
    


#[derive(serde::Serialize, serde::Deserialize)]
pub struct FFTUart {
    #[serde(with = "BigArray")]
    pub arr: [u8; FFT_BYTES_TAKEN],
}
impl FFTUart {
    pub fn new_bytes(arr: [u8; FFT_BYTES_TAKEN]) -> Self{
        Self { arr }
    }
    pub fn into_bytes(self) -> [u8; FFT_BYTES_TAKEN] {
        self.arr
    }
    pub fn new(arr: [BinValue; FFT_BINS]) -> Self {
        let arr = bytemuck::cast(arr);
        Self { arr }
    }
    pub fn into(self) -> [BinValue; FFT_BINS] {
        bytemuck::cast(self.arr)
    }
}


pub trait FromF32 {
    fn slice_from_f32<const N: usize>(input: &mut [f32; N], output: &mut [Self; N]) where Self: Sized;  
    fn from_f32(x: f32) -> Self;
    fn slice_to_f32<const N: usize>(input: &mut [Self; N], output: &mut [f32; N]) where Self: Sized;  
    fn to_f32(self) -> f32;
}

impl FromF32 for i8 {
    fn slice_from_f32<const N: usize>(input: &mut [f32; N], mut output: &mut [Self; N]) {
        for (i, v) in input.iter().enumerate() {
            output[i] = Self::from_f32(*v);
        }
    }
    fn from_f32(x: f32) -> Self {
        (x.clamp(-1.0, 1.0) * i8::MAX as f32) as i8
    }
    fn slice_to_f32<const N: usize>(input: &mut [Self; N], mut output: &mut [f32; N]) {
        for (i, v) in input.iter().enumerate() {
            output[i] = Self::to_f32(*v);
        }
    }
    fn to_f32(self) -> f32 {
        self as f32 / i8::MAX as f32
    }
}

impl FromF32 for i16 {
    fn slice_from_f32<const N: usize>(input: &mut [f32; N], mut output: &mut [Self; N]) {
        for (i, v) in input.iter().enumerate() {
            output[i] = Self::from_f32(*v);
        }
    }
    fn from_f32(x: f32) -> Self {
        (x.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
    }
    fn slice_to_f32<const N: usize>(input: &mut [Self; N], mut output: &mut [f32; N]) {
        for (i, v) in input.iter().enumerate() {
            output[i] = Self::to_f32(*v);
        }
    }
    fn to_f32(self) -> f32 {
        self as f32 / i16::MAX as f32
    }
}

impl FromF32 for f32 {
    fn slice_from_f32<const N: usize>(input: &mut [f32; N], output: &mut [Self; N]) {
        core::mem::swap(input, output);
    }
    fn from_f32(x: f32) -> Self {
        x // pass through, no scaling
    }
    fn slice_to_f32<const N: usize>(input: &mut [Self; N], output: &mut [f32; N]) {
        core::mem::swap(input, output);
    }
    fn to_f32(self) -> f32 {
        self // pass through
    }
}
