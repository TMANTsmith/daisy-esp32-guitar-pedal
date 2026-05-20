
use libm::powf;
extern crate alloc;
use alloc::boxed::Box;

const fn pow2(n: usize) -> usize {
    1 << n
}

#[derive(Debug)]
pub struct FTT<const N: usize> {
    buffer: [Option<Box<[f32; pow2(N)]>>; 2],
    index: usize,
}

impl<const N: usize> FTT<N> {
    pub fn new(sides: [bool; 2], length: usize) -> Self {

        let buffer: [Option<Box<[f32; pow2(N)]>>; 2] = [None, None];
        let index = 0;
                
        if sides[0] == true {
            buffer[0] = Box::new([f32; pow2(N)]);
        }

        if sides[1] == true {
            buffer[1] = Box::new([f32; pow2(N)]);
        }
        Self { buffer, index }
    }

    pub fn add(&self, &mut input: (f32, f32)) {
        let (left, right) = input;

        if self.buffer[0] != None {
            self.buffer[0][index] = left;
        }

        if self.buffer[1] != None {
            self.buffer[1][index] = right;
        }
    }
}
