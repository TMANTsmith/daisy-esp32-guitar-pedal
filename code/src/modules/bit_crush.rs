struct BitCrush {
    value: f64, // Bit depth
}

impl BitCrush {
    fn new(value: f64) -> Self {
        match value {
            2.0 | 4.0 | 8.0 | 12.0 | 16.0 | 20.0 => (),
            _ => panic!("BitCrush value must be 2,4,8,12,16,20"),
        }
        BitCrush { value }
    }

    fn process(&self, input: &mut (f64, f64)) {
        let levels = 2f64.powf(self.value) - 1.0;
        // Map -1..1 -> 0..1, round, map back
        input.0 = ((input.0 + 1.0) / 2.0 * levels).round() / levels * 2.0 - 1.0;
        input.1 = ((input.1 + 1.0) / 2.0 * levels).round() / levels * 2.0 - 1.0;
    }

    fn process_list(&self, input: &mut [(f64, f64)]) {
        for tuple in input.iter_mut() {
            self.process(tuple);
        }
    }
}
