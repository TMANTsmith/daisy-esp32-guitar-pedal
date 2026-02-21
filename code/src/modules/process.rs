// TODO add Delay

mod gain;
use gain::Gain;


mod fuzz;
use fuzz::Fuzz;


mod clipping;
use clipping::Clipping;


mod bit_crush;
use bit_crush::BitCrush;


pub trait Effects {
    fn process(&self, imput: &mut (f32, f32)) -> (f32, f32);
}

pub impl Effects for Gain {
    fn process(&mut self, input: &mut (f32, f32)) {
        *input = ((input.0 * self.value as f32), (input.1 * self.value as f32))
    }
}

pub impl Effects for Fuzz {
    fn process(&self, input: &mut (f32, f32)) {
        if input.0 > self.value {
            input.0 = self.value;
        } else if input.0 < -self.value {
            input.0 = -self.value;
        }

        if input.1 > self.value {
            input.1 = self.value;
        } else if input.1 < -self.value {
            input.1 = -self.value;
        }

        input.0 = input.0 * (1.0 / self.value);
        input.1 = input.1 * (1.0 / self.value);
    }
}
pub impl Effects for Clipping {
    fn process(&self, input: &mut (f32, f32)) {
        input.0 = libm::tanhf((input.0 + self.bais) * (self.value + 1.0) * (self.value + 1.0));
        input.1 = libm::tanhf((input.1 + self.bais) * (self.value + 1.0) * (self.value + 1.0));
    }
}

pub impl Effects for BitCrush {
    fn process(&self, frame: &mut (f32, f32)) {
        frame.0 = crush(frame.0, self.levels);
        frame.1 = crush(frame.1, self.levels);
    }
}
