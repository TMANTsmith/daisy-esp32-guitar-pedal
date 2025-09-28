
struct Invert;

impl Invert {
    fn process(&self, buffer: &mut [i16]) {
        for sample in buffer.iter_mut() {
            *sample = *sample * -1;
        }
    }
}


