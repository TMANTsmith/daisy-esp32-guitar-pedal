extern crate alloc;
use core::ops::Deref;
use libm::powf;
use libm::sqrtf;
use microfft::real;
use microfft::Complex32;

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

#[derive(Debug, Clone)]
pub struct Buffers<const N: usize>(Option<Box<[f32; N]>>, Option<Box<[f32; N]>>);
impl<const N: usize> Buffers<N> {
    pub fn get_left(&mut self) -> &mut Option<Box<[f32; N]>> {
        &mut self.0
    }
    pub fn get_right(&mut self) -> &mut Option<Box<[f32; N]>> {
        &mut self.1
    }

    pub fn set_left(&mut self, index: usize, value: f32) -> bool {
        if let Some(list) = &mut self.0 {
            list[index] = value;
            true
        } else {
            false
        }
    }

    pub fn set_right(&mut self, index: usize, value: f32) -> bool {
        if let Some(list) = &mut self.1 {
            list[index] = value;
            true
        } else {
            false
        }
    }
    pub fn copy_from_slice(&mut self, input: &Buffers<N>) {
        if let (Some(buf1), Some(buf2)) = (&mut self.0, &input.0) {
            buf1.copy_from_slice(buf2.as_slice());
        }
        if let (Some(buf1), Some(buf2)) = (&mut self.1, &input.1) {
            buf1.copy_from_slice(buf2.as_slice());
        }
    }
}

#[derive(Debug)]
// H = N / 2
pub struct Fft_read<const N: usize, const H: usize> {
    read_buf: Buffers<N>,
    output_buf: Box<[Wave; N]>,
}

impl<const N: usize, const H: usize> Fft_read<N, H> {
    pub fn new(sides: [bool; 2]) -> Self {
        let mut read_buf: Buffers<{ N }> = Buffers::<{ N }>(None, None);
        let mut output_buf: Box<[Wave; N]> = Box::new(
            [Wave {
                hertz: 0.0,
                amplitude: 0.0,
                confidence: None,
            }; N],
        );

        if sides[0] {
            read_buf.0 = Some(Box::new([0_f32; N]));
        }

        if sides[1] {
            read_buf.1 = Some(Box::new([0_f32; N]));
        }
        Self {
            read_buf,
            output_buf,
        }
    }

    pub fn compute(&mut self) -> Result<(Option<Waves>, Option<Waves>), FftError>
    where
        RunFft: GetFft<N>,
    {
        let mut right = false;
        let mut left = false;

        if let Some(buf) = self.read_buf.get_left().as_deref_mut() {
            // Hann window
            for i in 0..N {
                let window = 0.5
                    * (1.0 - libm::cosf(2.0 * core::f32::consts::PI * i as f32 / (N - 1) as f32));
                buf[i] *= window;
            }

            left = true;
            let spectrum = <RunFft as GetFft<N>>::get_complex(buf);
            spectrum[0].im = 0.0;
            for (i, c) in spectrum.iter().enumerate() {
                // this is the sqrt amplitude
                let hertz = Fft_read::<N, H>::bin_hz() * i as f32;
                let amp = c.norm_sqr();

                self.output_buf[i].set_hertz(hertz);
                self.output_buf[i].set_amplitude(amp);
                self.output_buf[i].set_confidence(None);
            }
        }

        if let Some(buf) = self.read_buf.get_right().as_deref_mut() {
            // Hann window
            for i in 0..N {
                let window = 0.5
                    * (1.0 - libm::cosf(2.0 * core::f32::consts::PI * i as f32 / (N - 1) as f32));
                buf[i] *= window;
            }

            right = true;
            let spectrum = <RunFft as GetFft<N>>::get_complex(buf);
            spectrum[0].im = 0.0;
            for (i, c) in spectrum.iter().enumerate() {
                // this is the sqrt amplitude
                let hertz = Fft_read::<N, H>::bin_hz() * i as f32;
                let amp = c.norm_sqr();
                let confidence: Option<f32> = None;

                self.output_buf[i + H].set_hertz(hertz);
                self.output_buf[i + H].set_amplitude(amp);
                self.output_buf[i + H].set_confidence(None);
            }
        }

        let (left_out, right_out) = self.output_buf.split_at(H);

        let mut rtn: (Option<Waves>, Option<Waves>) = (None, None);

        if left {
            rtn.0 = Some(Waves {
                waves: left_out,
                sorted: false,
                strongest: 0.0,
            });
        }
        if right {
            rtn.1 = Some(Waves {
                waves: right_out,
                sorted: false,
                strongest: 0.0,
            });
        }

        Ok(rtn)
    }
    pub fn bin_hz() -> f32
    where
        RunFft: GetFft<N>,
    {
        <RunFft as GetFft<N>>::get_bin_hz()
    }
    pub fn copy_from_write(&mut self, input: &mut Fft_write<N, H>) -> Result<(), FftError> {
        if input.get_timer() != 0 {
            return Err(FftError::Wait(input.get_timer()))
        }
        self.read_buf.copy_from_slice(input.get_write_buf());

        input.reset_timer();
        Ok(())
    }
}

pub struct Fft_write<const N: usize, const H: usize> {
    write_buf: Buffers<N>,
    index: usize,
    timer: usize,
}

impl<const N: usize, const H: usize> Fft_write<N, H> {
    pub fn new(sides: [bool; 2]) -> Self {
        let mut write_buf: Buffers<{ N }> = Buffers::<{ N }>(None, None);
        let mut index = 0;
        let mut timer = N;

        if sides[0] {
            write_buf.0 = Some(Box::new([0_f32; N]));
        }

        if sides[1] {
            write_buf.1 = Some(Box::new([0_f32; N]));
        }
        Self {
            write_buf,
            index,
            timer,
        }
    }

    fn reset_timer(&mut self) {
        self.timer = N;
    }

    fn get_write_buf(&mut self) -> &Buffers<N> {
            &self.write_buf
    }

    pub fn get_timer(&self) -> usize {
        self.timer
    }

    pub fn bin_hz() -> f32
    where
        RunFft: GetFft<N>,
    {
        <RunFft as GetFft<N>>::get_bin_hz()
    }

    pub fn add(&mut self, input: &mut (f32, f32)) {
        let (left, right) = input;

        self.write_buf.set_left(self.index, *left);
        self.write_buf.set_right(self.index, *right);
        self.index = (self.index + 1) % N;

        if self.timer > 0 {
            self.timer -= 1;
        }
    }
}
#[derive(defmt::Format)]
pub struct Waves<'a> {
    waves: &'a [Wave],
    sorted: bool,
    strongest: f32,
}

impl<'a> Waves<'a> {
    pub fn new(waves: &'a [Wave]) -> Self {
        Self {
            waves,
            sorted: false,
            strongest: 0.0,
        }
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
