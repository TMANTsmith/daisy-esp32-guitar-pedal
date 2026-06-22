extern crate alloc;
use circular_buffer::CircularBuffer;
use core::ops::Deref;
use microfft::real;
use microfft::Complex32;
use defmt::{debug, unwrap};


use alloc::boxed::Box;

pub struct RunFft;

pub trait GetFft<const N: usize> {
    fn get_complex(input: &mut [f32; N]) -> &mut [Complex32];
    fn get_bin_hz() -> f32;
}

#[derive(defmt::Format)]
pub enum FftError {
    Wait(usize),
    InvalidSize,
}

//macro coded by claude
#[macro_export]
macro_rules! make_fft {
    ($N:expr, $flags:expr) => {{
        const H: usize = $N / 2;
        (
            Fft_read::<{ $N }, H>::new($flags),
            Fft_write::<{ $N }, H>::new($flags),
        )
    }};
}


// H = N / 2
pub struct Fft<const N: usize, const H: usize> {
    circular_left: Option<Box<CircularBuffer::<N, f32>>>,
    circular_right: Option<Box<CircularBuffer::<N, f32>>>,
    output_left: Option<Box<Waves::<H>>>,
    output_right: Option<Box<Waves::<H>>>,
}

impl<const N: usize, const H: usize> Fft<N, H> {
    pub fn new(sides: (bool, bool)) -> Self {
        let mut circular_left = None;
        let mut circular_right = None;
        let mut output_left = None;
        let mut output_right = None;

        if sides.0 {
            circular_left = Some(Box::new(CircularBuffer::<N, f32>::new()));
            output_left = Some(Box::new(Waves::<H>::new([Wave::default(); H])));
        }
        if sides.1 {
            circular_right = Some(Box::new(CircularBuffer::<N, f32>::new()));
            output_right = Some(Box::new(Waves::<H>::new([Wave::default(); H])));
        }
        Self {
            circular_right,
            circular_left,
            output_left,
            output_right,
        }
    }
    
    pub fn print(&self) {
        if let Some(x) = &self.circular_left {
            for v in x.iter() {
                debug!("v={=f32}", v);
            }
        }
        if let Some(x) = &self.circular_right {
            for v in x.iter() {
                debug!("v={=f32}", v);
            }
        }
    }
    pub fn add(&mut self, input: (f32, f32)) {
        if let Some(buf) = &mut self.circular_left {
            buf.push_back(input.0);
        }
        if let Some(buf) = &mut self.circular_right {
            buf.push_back(input.0);
        }
    }

    pub fn get_result(&mut self) -> (&mut Option<Box<Waves::<H>>>, &mut Option<Box<Waves::<H>>>) {
        (&mut self.output_left, &mut self.output_right)
    }

    pub fn compute(&mut self) 
    where
        RunFft: GetFft<N>,
    {
        debug!("bin_hz {=f32}", Fft::<N,H>::bin_hz());

        if let Some(circle) = &mut self.circular_left {
            let buf: &mut [f32; N] = circle.make_contiguous().try_into().expect("length mismatch");

            // Hann window
            for i in 0..N {
                let window = 0.5
                    * (1.0 - libm::cosf(2.0 * core::f32::consts::PI * i as f32 / (N - 1) as f32));
                buf[i] *= window;
            }

            let spectrum = <RunFft as GetFft<N>>::get_complex(buf);
            spectrum[0].im = 0.0;
            for (i, c) in spectrum.iter().enumerate() {
                // this is the sqrt amplitude
                let hertz = Fft::<N, H>::bin_hz() * i as f32;
                let amp = c.norm_sqr();

                if let Some(out) = &mut self.output_left {
                    out.get(i).set_hertz(hertz);
                    out.get(i).set_amplitude(amp);
                    out.get(i).set_confidence(None);
                }
            }
            if let Some(out) = &mut self.output_left {
                debug!("first 10 left");
                for j in 0..10 {
                    debug!("bin {=usize} amp {=f32}", j, out.get(j).get_amplitude());
                }
            }
        }

        if let Some(circle) = &mut self.circular_right {
            let buf: &mut [f32; N] = circle.make_contiguous().try_into().expect("length mismatch");

            // Hann window
            for i in 0..N {
                let window = 0.5
                    * (1.0 - libm::cosf(2.0 * core::f32::consts::PI * i as f32 / (N - 1) as f32));
                buf[i] *= window;
            }

            let spectrum = <RunFft as GetFft<N>>::get_complex(buf);
            spectrum[0].im = 0.0;
            for (i, c) in spectrum.iter().enumerate() {
                // this is the sqrt amplitude
                let hertz = Fft::<N, H>::bin_hz() * i as f32;
                let amp = c.norm_sqr();
                
                if let Some(out) = &mut self.output_right {
                    out.get(i).set_hertz(hertz);
                    out.get(i).set_amplitude(amp);
                    out.get(i).set_confidence(None);
                }
            }
            if let Some(out) = &mut self.output_right {
                debug!("first 10 right");
                for j in 0..10 {
                    debug!("bin {=usize} amp {=f32}", j, out.get(j).get_amplitude());
                }
            }
        }
    }
    pub fn bin_hz() -> f32
    where
        RunFft: GetFft<N>,
    {
        <RunFft as GetFft<N>>::get_bin_hz()
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
    pub fn set(&mut self, index: usize, value: Wave) -> Result<(), ()> {
        if index < self.waves.len() {
            self.waves[index] = value;
            Ok(())
        }
        else {
            Err(())
        }
    }
    pub fn get(&mut self, index: usize) -> &mut Wave {
        &mut self.waves[index]
    }
    pub fn get_largest(&mut self) {
        for wave in &self.waves {
            if wave.get_amplitude_raw() > self.strongest {
                self.strongest = wave.get_amplitude_raw();
            }
        }
    }

    pub fn iter(&self) -> core::slice::Iter<'_, Wave> {
        self.waves.iter()
    }

    pub fn get_n_largest<const N: usize>(&mut self) -> [Wave; N] {
        //TODO optimize to make function
        // pub fn get_n_largest(&self) -> [&Wave]
        let mut top: [Wave; N] = [Wave::default(); N];
        let mut top_vals: [f32; N] = [f32::NEG_INFINITY; N];

        let mut prev_2: f32;
        let mut prev: f32;
        let mut curr: f32;
        let mut next: f32;
        let mut next_2: f32;

        // find local peaks only (higher than both neighbors)
        for j in 2..self.waves.len() - 2 {
            prev_2 = self.waves[j - 2].get_amplitude_raw();
            prev = self.waves[j - 1].get_amplitude_raw();
            curr = self.waves[j].get_amplitude_raw();
            next = self.waves[j + 1].get_amplitude_raw();
            next_2 = self.waves[j + 2].get_amplitude_raw();
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
            wave.set_confidence(Some(wave.get_amplitude_raw() / strongest));
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
        libm::sqrtf(self.amplitude)
    }
    pub fn get_amplitude_raw(&self) -> f32 {
        self.amplitude
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
