struct Looper{
}

impl Looper{

    fn process(&self, buffer: &mut [i16]) {
        for sample in buffer.iter_mut() {
            *sample = (*sample self.gain)
                .clamp(i16::MIN , i16::MAX );
        }
    }
}


