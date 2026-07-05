extern crate alloc;
use core::ops::Deref;
use libm::powf;
use libm::sqrtf;
use microfft::real;
use microfft::Complex32;
use defmt::{debug, unwrap};

// TODO: fix Wave and Waves to only store a [f32; N] and calculate the hertz so that less will be in SDRAM so more cahce hits 
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
    output_buf: Box<Waves<H>>,
}

impl<const N: usize, const H: usize> FftRead<N, H> {
    pub fn new() -> Self {
        let read_buf = None;
        let output_buf: Box<Waves<H>> = Box::new(Waves::new([Wave::new(0.0, 0.0, None); H]));
        Self {
            read_buf,
            output_buf,
        }
    }
    pub fn set_buf(&mut self, input: Box<[f32; N]>) {
        //debug!("read buf set: {}", *input);
        self.read_buf = Some(input);
    }

    pub fn get_waves(&mut self) -> &mut Waves<H> {
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
                let hertz = FftRead::<N, H>::bin_hz() * i as f32;
                let amp = c.norm_sqr();
                /*
                if amp != 0.0 {
                    debug!("amp: {}", amp);
                }
                */

                self.output_buf.get(i).set_hertz(hertz);
                self.output_buf.get(i).set_amplitude(amp);
                self.output_buf.get(i).set_confidence(None);
            }
            //debug!("computed!");
            Ok(buf)
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
#[derive(defmt::Format, Debug)]
pub struct Waves<const H: usize> {
    waves: [Wave; H],
    sorted: bool,
    strongest: f32,
}

impl<const H: usize> Waves<H> {
    pub fn new(waves: [Wave; H]) -> Self {
        Self {
            waves,
            sorted: false,
            strongest: 0.0,
        }
    }
    pub fn get(&mut self, index: usize) -> &mut Wave {
        &mut self.waves[index]
    }
    pub fn get_largest(&mut self) {
        for wave in self.waves {
            if wave.get_amplitude() > self.strongest {
                self.strongest = wave.get_amplitude();
            }
        }
    }

    pub fn iter(&self) -> core::slice::Iter<'_, Wave> {
        self.waves.iter()
    }

    pub fn get_n_largest<const N: usize>(&mut self) -> [Wave; N] {
        let mut top: [Wave; N] = [Wave::default(); N];
        let mut top_vals: [f32; N] = [f32::NEG_INFINITY; N];

        let mut prev_2: f32;
        let mut prev: f32;
        let mut curr: f32;
        let mut next: f32;
        let mut next_2: f32;
        if self.waves.len() < 2 {
            panic!("FFT_N must be more than 4");
        }

        // find local peaks only (higher than both neighbors)
        for j in 2..self.waves.len() - 2 {
            prev_2 = self.waves[j - 2].get_amplitude();
            prev = self.waves[j - 1].get_amplitude();
            curr = self.waves[j].get_amplitude();
            next = self.waves[j + 1].get_amplitude();
            next_2 = self.waves[j + 2].get_amplitude();
            if curr >= prev
                && curr >= next
                && curr >= prev_2
                && curr >= next_2
                && self.waves[j].get_hertz() < 20_000.0
                && self.waves[j].get_hertz() > 20.0
                && curr > 0.01
            {
                if curr > top_vals[N - 1] {
                    top_vals[N - 1] = curr;
                    top[N - 1] = self.waves[j];
                    // bubble up
                    for i in (1..N).rev() {
                        if top_vals[i] > top_vals[i - 1] {
                            top_vals.swap(i, i - 1);
                            top.swap(i, i - 1);
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // set confidence relative to strongest
        let strongest = top_vals[0];
        for wave in top.iter_mut() {
            wave.set_confidence(Some(wave.get_amplitude() / strongest));
        }

        top
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, defmt::Format)]
pub struct Wave {
    hertz: f32,
    amplitude: f32,
    confidence: Option<f32>,
}

impl Wave {
    pub fn new(hertz: f32, amplitude: f32, confidence: Option<f32>) -> Self {
        Wave {
            hertz,
            amplitude,
            confidence,
        }
    }
    pub fn set_hertz(&mut self, hertz: f32) {
        self.hertz = hertz
    }
    pub fn set_amplitude(&mut self, amplitude: f32) {
        self.amplitude = amplitude;
    }
    pub fn set_confidence(&mut self, confidence: Option<f32>) {
        self.confidence = confidence;
    }
    pub fn get_hertz(&self) -> f32 {
        self.hertz
    }
    pub fn get_amplitude(&self) -> f32 {
        sqrtf(self.amplitude)
    }
    pub fn get_confidence(&self) -> Option<f32> {
        self.confidence
    }
}
/*
    2
    4
    8
    16
    32
    64
    128
    256
    512
    1024
    2048
    4096
    8192

*/

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
