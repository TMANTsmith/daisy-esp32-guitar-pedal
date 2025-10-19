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

impl AudioDriver {
    /// Initialize I2C peripheral and pins for simple control (SDA/SCL).
    /// This config writes to IO_MUX and I2C0 registers via the PAC.
    /// It returns an AudioDriver instance on success.
    pub fn init_i2c() -> Result<Self> {
        // Safety: we take peripheral access (must only do once in the program).
        let peripherals = unsafe { Peripherals::steal() };

        // Example pins from your earlier messages — replace if different:
        const I2C_SDA: usize = settings::I2C_SDA as usize; // e.g. 21
        const I2C_SCL: usize = settings::I2C_SCL as usize; // e.g. 22

        // 1) Enable GPIO pins in direction registers (if needed)
        // Note: PAC register access names differ by version;
        // below uses the common pattern gpio.enable_w1ts / gpio.enable_w1tc or gpio.enable.
        //
        // If your PAC exposes `gpio.enable.write(|w| ...)` use that. Some PACs provide `enable_w1ts`.
        let gpio = &peripherals.GPIO;

        // Set pins as GPIO-enabled (so the IO_MUX/func_sel can route peripheral signals)
        // Prefer write with bits if PAC provides it; choose the correct register available in your PAC.
        // Example with out_w1ts/out_w1tc style (if available):
        #[allow(unused_unsafe)]
        unsafe {
            // NOTE: choose the correct register in your PAC
            // For many PACs: gpio.enable_w1ts.write(|w| w.bits((1 << I2C_SDA) | (1 << I2C_SCL)));
            // Fallback: if `enable` is present as a whole-register writer:
            #[cfg(feature = "gpio_enable_w1ts")]
            gpio.enable_w1ts.write(|w| w.bits(((1u32 << I2C_SDA) | (1u32 << I2C_SCL)) as u32));

            // If your PAC uses `enable.write(|w| unsafe { w.bits(...) })`:
            #[cfg(not(feature = "gpio_enable_w1ts"))]
            gpio.enable.write(|w| unsafe { w.bits(((1u32 << I2C_SDA) | (1u32 << I2C_SCL)) as u32) });
        }

        // 2) Configure IO_MUX / GPIO matrix to route SDA/SCL to I2C0
        // The exact IO_MUX fields differ by PAC version; here is a conceptual example:
        let io_mux = &peripherals.GPIO; // sometimes IO_MUX is separate; adapt if your PAC has IO_MUX

        // TODO: replace the following with the actual IO_MUX/PAC fields and function indexes
        // Example conceptual (not literal PAC API):
        // peripherals.IO_MUX.gpio[I2C_SDA].func_sel.write(|w| w.func_sel().i2c_sda());
        // peripherals.IO_MUX.gpio[I2C_SCL].func_sel.write(|w| w.func_sel().i2c_scl());
        //
        // If your PAC exposes func_out_sel_cfg or func_in_sel registers, use those.

        // 3) Configure I2C timings and enable I2C controller (I2C0)
        let i2c0 = &peripherals.I2C0;

        // Example: set timing register and enable. Exact field names differ.
        // Many ESP PACs use registers like `scl_low` / `scl_high` or a `timing` register.
        // Replace with the fields your pac exposes.
        unsafe {
            // PSEUDOCODE: set to 100kHz; you must compute proper divider values for the peripheral
            // i2c0.timing.write(|w| w.bits(computed_timing_value));
        }

        // Set enable bit (replace with actual field name)
        // i2c0.ctrl.modify(|_, w| w.scl_force_out().set_bit()); // example; adapt to real API

        // NOTE: full I2C transfer code (start, write command list, read, stop) requires
        // implementing the I2C command sequence over the I2C peripheral — not trivial.
        // Many people write a small driver that builds I2C commands into the peripheral's CMD FIFO.

        Ok(Self {})
    }

    /// Placeholder: perform I2C write. Implement actual register-based I2C transfer here.
    pub fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<()> {
        // TODO: implement low-level I2C transfer using I2C0 registers (CMD, DATA, STATUS, INT)
        Err(Error::I2cTransfer)
    }

    /// Placeholder: perform I2C read into provided buffer.
    pub fn read(&mut self, _addr: u8, _buf: &mut [u8]) -> Result<()> {
        // TODO: implement low-level I2C read
        Err(Error::I2cTransfer)
    }

    /// Modify one register bit on an I2C device: read, mask, write back
    pub fn mwrite(&mut self, device_addr: u8, reg_addr: u8, bit: u8, value: bool) -> Result<()> {
        let mut current = [0u8; 1];
        self.read(device_addr, &mut current)?;
        if value {
            current[0] |= 1u8 << bit;
        } else {
            current[0] &= !(1u8 << bit);
        }
        self.write(device_addr, &[reg_addr, current[0]])
    }
}

// ---------- I2S initialization (bare-metal, minimal) ----------
pub fn init_i2s_24_44100() -> Result<()> {
    let peripherals = unsafe { Peripherals::steal() };
    let i2s = &peripherals.I2S0;

    // 1) Reset TX/RX and FIFOs
    // Names below are illustrative; use the exact PAC field names in your version
    i2s.conf.write(|w| {
        w.i2s_tx_reset().set_bit();
        w.i2s_rx_reset().set_bit();
        w
    });
    i2s.conf.modify(|_, w| {
        w.i2s_tx_reset().clear_bit();
        w.i2s_rx_reset().clear_bit();
        w
    });

    // 2) Configure clock generator (clkm_conf) for MCLK/BCLK/LRCK
    // The divider math to get 44.1k @ 24-bit stereo: BCLK = 44_100 * 24 * 2 = 2_116_800 Hz
    // MCLK typically = sample_rate * 256 = 11_289_600 Hz
    // You must compute clkm_div_num/clkm_div_a/clkm_div_b for the APB clock used on your board.
    unsafe {
        i2s.clkm_conf.modify(|_, w| {
            // TODO: set clkm_div_num, clkm_div_a, clkm_div_b appropriately for MCLK
            // Example placeholders:
            // w.clka_en().set_bit();
            // w.clkm_div_num().bits(4);
            // w.clkm_div_b().bits(0);
            // w.clkm_div_a().bits(1);
            w
        });
    }

    // 3) Configure sample rate / bits
    i2s.sample_rate_conf.modify(|_, w| unsafe {
        // tx_bck_div_num controls BCLK divider (tune this)
        // tx_bits_mod should be set for 24-bit
        // Replace field names with your PAC's actual names
        // Example placeholders:
        // w.tx_bck_div_num().bits(8);
        // w.tx_bits_mod().bits(24);
        w
    });

    // 4) Configure channel format (stereo) and data formatting (I2S standard, MSB first)
    i2s.conf_chan.modify(|_, w| {
        // Use the correct bitfields from your PAC to select stereo mode and channel layout.
        w
    });

    // 5) Map pins to I2S signals using GPIO matrix (IO_MUX)
    // Example constants - you must replace SIGNAL_* with actual numeric signal indices from TRM or PAC
    // const I2S0O_BCK_SIGNAL: u32 = TODO;
    // const I2S0O_WS_SIGNAL: u32 = TODO;
    // const I2S0O_DATA_OUT_SIGNAL: u32 = TODO;
    // const I2S0I_DATA_IN_SIGNAL: u32 = TODO;
    //
    // let gpio = &peripherals.GPIO;
    // unsafe {
    //     gpio.func_out_sel_cfg[I2C_BLCK as usize].write(|w| w.bits(I2S0O_BCK_SIGNAL));
    //     gpio.func_out_sel_cfg[I2C_LRCK as usize].write(|w| w.bits(I2S0O_WS_SIGNAL));
    //     gpio.func_out_sel_cfg[I2C_DOUT as usize].write(|w| w.bits(I2S0O_DATA_OUT_SIGNAL));
    //     gpio.func_in_sel_cfg[I2C_DIN as usize].write(|w| w.bits(I2S0I_DATA_IN_SIGNAL));
    // }

    // 6) Enable TX engine
    i2s.conf.modify(|_, w| {
        // set tx_start bit
        // w.i2s_tx_start().set_bit();
        w
    });

    Ok(())
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

