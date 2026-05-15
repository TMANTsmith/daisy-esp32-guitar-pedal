//https://docs.rs/rustfft/latest/rustfft/algorithm/struct.Radix4.html

use rustfft::algorithm::Radix4;
use rustfft::{Fft, FftDirection};
use rustfft::num_complex::Complex;

#[derive(Debug)]
pub struct FTT {
    buffer: [Option<Vec<f32>>; 2],
}

impl FTT {
    fn new(buffer: [Option<Vec<f32>>; 2]) -> Self {
        Self { buffer }
    }

    fn process(input: (f32, f32)) {
        }
}



