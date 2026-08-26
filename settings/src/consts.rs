// change these

// shaired 
pub const FFT_BINS: usize = 4096;
pub const HEADER: [u8; 4] = [0xAA, 0x55, 0xAA, 0x55];

pub type BinValue = f32; 



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
