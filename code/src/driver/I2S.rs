#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::vec::Vec;
use bcm2837_lpa::gpio::{Gpio, Function};
use bcm2837_lpa::bsc0::BSC0;

// CS4270 I2C address
const CS4270_I2C_ADDR: u8 = 0x24;

// Initialize I²S GPIO pins
pub fn init_i2s_gpio() {
    let mut gpio = Gpio::new();
    gpio.get_pin(18).into_alt(Function::Alt0); // BCLK
    gpio.get_pin(19).into_alt(Function::Alt0); // LRCLK
    gpio.get_pin(20).into_alt(Function::Alt0); // DATA OUT
    gpio.get_pin(21).into_alt(Function::Alt0); // DATA IN
    gpio.get_pin(23).into_alt(Function::Alt0); // MCLK if needed
}


// Read exactly 4 stereo frames from I²S and return a heap-allocated vector of 4 tuples
pub unsafe fn read_i2c() -> Vec<(i32, i32)> {
    use bcm2837_lpa::i2s::I2S0;

    let i2s = &*I2S0::ptr();
    let mut frames: Vec<(i32, i32)> = Vec::with_capacity(4);

    for _ in 0..4 {
        while i2s.status.read() & (1 << 0) == 0 {} // RX FIFO empty
        let left = i2s.rx_fifo.read() as i32;

        while i2s.status.read() & (1 << 0) == 0 {} // next sample
        let right = i2s.rx_fifo.read() as i32;

        frames.push((left, right));
    }

    frames
}

// Write exactly 4 stereo frames from a vector of 4 tuples
pub unsafe fn write_i2c(frames: &Vec<(i32, i32)>) {
    use bcm2837_lpa::i2s::I2S0;

    let i2s = &*I2S0::ptr();

    for &(left, right) in frames.iter().take(4) {
        while i2s.status.read() & (1 << 1) != 0 {} // TX FIFO full
        i2s.tx_fifo.write(left as u32);

        while i2s.status.read() & (1 << 1) != 0 {}
        i2s.tx_fifo.write(right as u32);
    }
}

