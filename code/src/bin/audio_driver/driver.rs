#![no_std]
#![no_main]

// allow this file to be usable as a module in your project (no bin startup here)
// If you want this as your main, add #[no_mangle] pub extern "C" fn main() -> ! { ... }

extern crate alloc;
use alloc::vec::Vec;

use core::panic::PanicInfo;

use esp32s3_pac::Peripherals;
use crate::config::settings; // keep your settings crate as before

// ---------- minimal runtime pieces ----------
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// ---------- errors & result alias ----------
#[derive(Debug)]
pub enum Error {
    I2cConfig,
    I2cTransfer,
    I2sConfig,
    BadArgument,
}

pub type Result<T> = core::result::Result<T, Error>;

// ---------- Audio driver stub (no HAL) ----------
pub struct AudioDriver {
    // No HAL object — store whatever state you need here.
    // For bare-metal, you often store peripheral ownership marker or buffer pointers.
    // Keep it empty for now; add fields as needed (e.g., device_addr, buffers).
}

    // 0x6000_F000 to 0x6000_FFFF
impl AudioDriver {
    /// Initialize I2C peripheral and pins for SDA/SCL
    pub fn init_i2c() -> Result<Self> {
        let peripherals = esp32s3::Peripherals::take().unwrap();
        let i2c0 = &peripherals.I2C0;

        // Master mode, enable peripheral
        i2c0.ctr.write(|w| {
            w.ms_mode().set_bit();
            w.sda_force_out().set_bit();
            w.scl_force_out().set_bit();
            w.en().set_bit();
            w
        });

        // 100kHz clock
        i2c0.scl_low_period.write(|w| unsafe { w.bits(200) });
        i2c0.scl_high_period.write(|w| unsafe { w.bits(200) });

        // Assign GPIOs
        let gpio = &peripherals.GPIO;
        gpio.enable.write(|w| unsafe { w.bits((1 << I2C_SDA) | (1 << I2C_SCL)) });

        let iomux = &peripherals.IO_MUX;
        iomux.gpio[I2C_SDA as usize].func_sel.write(|w| w.func_sel().i2c_sda());
        iomux.gpio[I2C_SCL as usize].func_sel.write(|w| w.func_sel().i2c_scl());

        Ok(Self { })
    }

    /// Write a single byte to a register
    pub fn mwrite(&mut self, _addr: u8, bit: u8, value: bool) -> Result<()> {
        let peripherals = esp32s3::Peripherals::steal();
        let i2c0 = &peripherals.I2C0;

        // --- 1. Start condition and send device address (write) ---
        i2c0.comd.write(|w| unsafe {
            // Combine I2S_write_addr as 7-bit and R/W = 0
            w.bits(((I2S_write_addr << 1) & 0xFE) | 0) 
        });

        // --- 2. Send register address (MAP) ---
        i2c0.data.write(|w| unsafe { w.bits(_addr as u32) });
        i2c0.comd.write(|w| unsafe { w.bits(0x01) }); // command = write
        while i2c0.status.read().bus_busy().bit_is_set() {}

        // --- 3. Read current value from the register ---
        // For CS4270, you may need a repeated START or separate read command
        let mut current: u8 = i2c0.data.read().bits() as u8;

        // --- 4. Modify the bit ---
        if value {
            current |= 1 << bit;
        } else {
            current &= !(1 << bit);
        }

        // --- 5. Send device address again for write ---
        i2c0.comd.write(|w| unsafe {
            w.bits(((I2S_write_addr << 1) & 0xFE) | 0)
        });

        // --- 6. Send register address again ---
        i2c0.data.write(|w| unsafe { w.bits(_addr as u32) });
        i2c0.comd.write(|w| unsafe { w.bits(0x01) });
        while i2c0.status.read().bus_busy().bit_is_set() {}

        // --- 7. Write modified value ---
        i2c0.data.write(|w| unsafe { w.bits(current as u32) });
        i2c0.comd.write(|w| unsafe { w.bits(0x01) });
        while i2c0.status.read().bus_busy().bit_is_set() {}

        Ok(())
    }

    pub fn init_i2s() -> Result<()> {
    }
}

// ---------- helpers: pack/unpack 24-bit <-> f32 ----------
pub fn unpack_24le(in_bytes: &[u8]) -> Vec<f32> {
    let mut out = Vec::with_capacity(in_bytes.len() / 3);
    for chunk in in_bytes.chunks_exact(3) {
        let raw = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        // sign-extend 24->32
        let s = ((raw << 8) as i32) >> 8;
        let f = (s as f32) / (1u32 << 23) as f32;
        out.push(f);
    }
    out
}

pub fn pack_24le(in_samples: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(in_samples.len() * 3);
    for &f in in_samples {
        let s = (f * (1u32 << 23) as f32)
            .clamp(-(1u32 << 23) as f32, (1u32 << 23) as f32 - 1.0) as i32;
        out.push(((s >> 16) & 0xFF) as u8);
        out.push(((s >> 8) & 0xFF) as u8);
        out.push((s & 0xFF) as u8);
    }
    out
}

