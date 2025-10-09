struct Reverb {
    roomsize: f32,
    damp: f32,
    wet: f32,
    dry: f32,
    width: f32,
    mode: bool,
}

impl Reverb {
    fn new(roomsize: f32, damp: f32, wet: f32, dry: f32, width: f32, mode: bool) -> Self {
        Self { roomsize }
        Self { damp }
        Self { wet }
        Self { dry }
        Self { width }
        Self { mode }

    }

    // this will return the freeverb class with the values below
    fn prep() {
        let mut freeverb = Freeverb::new(0.8, 0.3, 0.5, 0.5, 1.0, false);
        freeverb.set_roomsize(self.roomsize);
        freeverb.set_damp(self.damp);
        freeverb.set_wet(self.wet);
        freeverb.set_dry(self.dry);
        freeverb.set_width(self.width);
        freeverb.set_mode(self.mode);
        freeverb
    }
    // run the prep fn and use the output for the 1st value in this which expects a freeverb
    // instance and the buffer which is the output 
    fn process(&self, freeverb: Freeverb, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = freeveb.process(*sample)
        }
    }
}


