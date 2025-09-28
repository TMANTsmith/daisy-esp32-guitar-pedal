
struct Invert;

impl Invert {
    fn process(&self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = *sample * -1;
        }
    }
}


