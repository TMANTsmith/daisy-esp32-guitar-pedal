use libm::floorf;

pub struct BitCrush {
    bits: u8,
    levels: f32,
}

impl BitCrush {
    /// Create a bitcrusher with `bits` in range 1–16
    pub fn new(bits: u8) -> Self {
        let bits = bits.clamp(1, 16);
        let levels = (1u32 << bits) as f32;

        Self { bits, levels }
    }

    /// Change bit depth at runtime safely
    #[inline]
    pub fn set_bits(&mut self, bits: u8) {
        self.bits = bits.clamp(1, 16);
        self.levels = (1u32 << self.bits) as f32;
    }

    /// Process a stereo frame
    #[inline(always)]
    pub fn process(&self, frame: &mut (f32, f32)) {
        frame.0 = crush(frame.0, self.levels);
        frame.1 = crush(frame.1, self.levels);
    }
}

#[inline(always)]
fn crush(x: f32, levels: f32) -> f32 {
    // Clamp input to avoid NaNs
    let x = x.clamp(-1.0, 1.0);

    // -1..1 → 0..1
    let x = (x + 1.0) * 0.5;

    // Quantize
    let x = libm::floorf(x * levels) / levels;

    // 0..1 → -1..1
    x * 2.0 - 1.0
}

