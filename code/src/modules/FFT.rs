extern crate alloc;
use core::ops::Deref;
use libm::powf;
use libm::sqrtf;
use microfft::real;
use microfft::Complex32;
use defmt::{debug, unwrap};

// TODO: make structs have a generic type <noBuf> or <wait> or <ready> insted of an enum state

use alloc::boxed::Box;

pub struct RunFft;


pub trait GetFft<const N: usize> {
    fn get_complex(input: &mut [f32; N]) -> &mut [Complex32];
    fn get_bin_hz() -> f32;
}

#[derive(defmt::Format, PartialEq, Debug, Clone)]
pub enum FftState<const N: usize> {
    Wait(usize),
    NoBuf,
    Ready(Box<[f32; N]>)
}



#[derive(Debug)]
// H = N / 2
pub struct FftRead<const N: usize, const H: usize> {
    read_buf: Option<Box<[f32; N]>>,
    output_buf: Box<[f32; H]>,
}

impl<const N: usize, const H: usize> FftRead<N, H> {
    pub fn new() -> Self {
        let read_buf = None;
        let output_buf: Box<[f32; H]> = Box::new([0.0; H]);
        Self {
            read_buf,
            output_buf,
        }
    }
    pub fn set_buf(&mut self, input: Box<[f32; N]>) {
        //debug!("read buf set: {}", *input);
        self.read_buf = Some(input);
    }

    pub fn get_waves(&mut self) -> &mut Box<[f32; H]> {
        &mut self.output_buf
    }

    pub fn compute(&mut self) -> Result<Box<[f32; N]>, FftState<N>>
    where
        RunFft: GetFft<N>,
    {


        if let Some(mut buf) = self.read_buf.take() {
            // Hann window


            //debug!("buf used: {}", *buf);
            for i in 0..N {
                let window = 0.5
                    * (1.0 - libm::cosf(2.0 * core::f32::consts::PI * i as f32 / (N - 1) as f32));
                buf[i] *= window;
            }


            let spectrum = <RunFft as GetFft<N>>::get_complex(&mut buf);
            spectrum[0].im = 0.0;
            /*
            for val in spectrum.iter() {
                if val.norm_sqr() != 0.0 {
                    debug!("c: {}", val.norm_sqr());
                }
            }
            */
            for (i, c) in spectrum.iter().enumerate() {
                // this is the sqrt amplitude
                let amp = c.norm_sqr();
                /*
                if amp != 0.0 {
                    debug!("amp: {}", amp);
                }
                */

                self.output_buf[i] = amp;
            }
            //debug!("computed!");
            Ok(buf)
        }
        else {
            Err(FftState::NoBuf)
        }
    }

    pub fn bin_hz(&mut self) -> f32
    where
        RunFft: GetFft<N>,
    {
        <RunFft as GetFft<N>>::get_bin_hz()
    }
}

pub struct FftWrite<const N: usize, const H: usize> {
    write_buf: Option<Box<[f32; N]>>,
    index: usize,
}

impl<const N: usize, const H: usize> FftWrite<N, H> {
    pub fn new() -> Self {
        let write_buf = None;
        let index = 0;

        Self {
            write_buf,
            index,
        }
    }


    pub fn set_buf(&mut self, input: Box<[f32; N]>) {
        self.write_buf = Some(input);
        self.index = 0;
    }

    pub fn get_buf(&mut self) -> Result<Box<[f32; N]>, FftState<N>> {
        if let Some(o) = self.write_buf.take() {
            //debug!("write buf rtn: {}", *o);
            Ok(o)
        }
        else {
            Err(FftState::NoBuf)
        }
    }
    pub fn bin_hz() -> f32
    where
        RunFft: GetFft<N>,
    {
        <RunFft as GetFft<N>>::get_bin_hz()
    }

    pub fn add(&mut self, input: f32) -> Result<(), FftState<N>>{
        if let Some(mut buf) = self.write_buf.take() {
            if self.index == N {
                Err(FftState::Ready(buf))
            }
            else {
                buf[self.index] = input;
                self.index += 1;
                self.write_buf = Some(buf);
                Ok(())
            }
        }
        else {
            Err(FftState::NoBuf) 
        }
    }
}

impl GetFft<2> for RunFft {
    fn get_complex(input: &mut [f32; 2]) -> &mut [Complex32] {
        real::rfft_2(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 2.0
    }
}

impl GetFft<4> for RunFft {
    fn get_complex(input: &mut [f32; 4]) -> &mut [Complex32] {
        real::rfft_4(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 4.0
    }
}
impl GetFft<8> for RunFft {
    fn get_complex(input: &mut [f32; 8]) -> &mut [Complex32] {
        real::rfft_8(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 8.0
    }
}
impl GetFft<16> for RunFft {
    fn get_complex(input: &mut [f32; 16]) -> &mut [Complex32] {
        real::rfft_16(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 16.0
    }
}
impl GetFft<32> for RunFft {
    fn get_complex(input: &mut [f32; 32]) -> &mut [Complex32] {
        real::rfft_32(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 32.0
    }
}
impl GetFft<64> for RunFft {
    fn get_complex(input: &mut [f32; 64]) -> &mut [Complex32] {
        real::rfft_64(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 64.0
    }
}
impl GetFft<128> for RunFft {
    fn get_complex(input: &mut [f32; 128]) -> &mut [Complex32] {
        real::rfft_128(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 128.0
    }
}
impl GetFft<256> for RunFft {
    fn get_complex(input: &mut [f32; 256]) -> &mut [Complex32] {
        real::rfft_256(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 256.0
    }
}
impl GetFft<512> for RunFft {
    fn get_complex(input: &mut [f32; 512]) -> &mut [Complex32] {
        real::rfft_512(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 512.0
    }
}
impl GetFft<1024> for RunFft {
    fn get_complex(input: &mut [f32; 1024]) -> &mut [Complex32] {
        real::rfft_1024(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 1024.0
    }
}
impl GetFft<2048> for RunFft {
    fn get_complex(input: &mut [f32; 2048]) -> &mut [Complex32] {
        real::rfft_2048(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 2048.0
    }
}
impl GetFft<4096> for RunFft {
    fn get_complex(input: &mut [f32; 4096]) -> &mut [Complex32] {
        real::rfft_4096(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 4096.0
    }
}
impl GetFft<8192> for RunFft {
    fn get_complex(input: &mut [f32; 8192]) -> &mut [Complex32] {
        real::rfft_8192(input)
    }
    fn get_bin_hz() -> f32 {
        48000.0 / 8192.0
    }
}

