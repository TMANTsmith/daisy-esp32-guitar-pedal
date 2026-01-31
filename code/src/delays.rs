use daisy::hal::hal::blocking::delay::{DelayUs, DelayMs};

pub struct BusyDelay {
    freq: u32,
}

impl BusyDelay {
    pub fn new(freq: u32) -> Self {
        Self { freq }
    }
}

// Implement for u8
impl DelayUs<u8> for BusyDelay {
    fn delay_us(&mut self, us: u8) {
        let cycles = (self.freq / 1_000_000) * us as u32;
        cortex_m::asm::delay(cycles);
    }
}

// Implement for u16
impl DelayUs<u16> for BusyDelay {
    fn delay_us(&mut self, us: u16) {
        let cycles = (self.freq / 1_000_000) * us as u32;
        cortex_m::asm::delay(cycles);
    }
}

// Implement for u32
impl DelayUs<u32> for BusyDelay {
    fn delay_us(&mut self, us: u32) {
        let cycles = (self.freq / 1_000_000) * us;
        cortex_m::asm::delay(cycles);
    }
}

// Implement for u8
impl DelayMs<u8> for BusyDelay {
    fn delay_ms(&mut self, ms: u8) {
        self.delay_us(ms as u32 * 1000);
    }
}

// Implement for u16
impl DelayMs<u16> for BusyDelay {
    fn delay_ms(&mut self, ms: u16) {
        self.delay_us(ms as u32 * 1000);
    }
}

// Implement for u32
impl DelayMs<u32> for BusyDelay {
    fn delay_ms(&mut self, ms: u32) {
        self.delay_us(ms * 1000);
    }
}
