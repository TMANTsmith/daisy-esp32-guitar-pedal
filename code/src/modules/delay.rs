use heapless::Vec;

pub struct Delay<'a> {
    length: f32, // in millisec 
    volume: f32, // between 0-1
    buffer: &'a mut [(f32, f32)],
    read_idx: u32,
    write_idx: u32,
}

impl<'a> Delay<'a> {
    pub fn new(
        len: f32, 
        volume: f32, 
        buffer: &'a mut [(f32, f32)], 
        sample: f32
    ) -> Self {
        let mut write_idx: u32 = 0;
        let mut read_idx: u32 = len / 1000 * sample;
        Delay { length, volume, buffer, write_idx, read_idx }
    }

    pub fn process(&self, input: &mut (f32, f32)) {
        self.buffer[write_idx] = (input.0, input.1);
        self.write_idx += 1;

        if self.write_idx >= self.sample {
            self.write_idx = 0;
        }
        if self.read_idx >= self.sample {
            self.read_idx = 0;
        }        
        let (left, right) = buffer[self.read_idx];
        *input = (input.0 + (left * self.volume), input.1 + right * self.volume);
    }
    pub fn process_list(&self, input: &mut [(f32, f32)]) {
        for touple in input.iter_mut() {
            self.process(touple);
        }
    }
}
    


