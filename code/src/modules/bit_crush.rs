use libm;

pub struct BitCrush {
    value: f32, // Bit depth
}

impl BitCrush {
    pub fn new(value: f32) -> Self {
        match value {
            2.0 | 4.0 | 8.0 | 12.0 | 16.0 | 20.0 => (),
            _ => panic!("BitCrush value must be 2,4,8,12,16,20"),
        }
        BitCrush { value }
    }

    pub fn process(&self, input: &mut (f32, f32)) {
        let levels = libm::powf(2.0, self.value) - 1.0;
        // Map -1..1 -> 0..1, round, map back
        input.0 = libm::roundf((input.0 + 1.0) / 2.0 * levels) / levels * 2.0 - 1.0;
        input.1 = libm::roundf((input.1 + 1.0) / 2.0 * levels) / levels * 2.0 - 1.0;
    }

    pub fn process_list(&self, input: &mut [(f32, f32)]) {
        for tuple in input.iter_mut() {
            self.process(tuple);
        }
    }
}
