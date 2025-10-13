use crate::{all_pass::AllPass, comb::Comb};

const FIXED_GAIN: f32 = 0.015;

const SCALE_WET: f32 = 3.0;
const SCALE_DAMPENING: f32 = 0.4;

const SCALE_ROOM: f32 = 0.28;
const OFFSET_ROOM: f32 = 0.7;


const COMB_TUNING_L1: usize = 1116;
const COMB_TUNING_L2: usize = 1188;
const COMB_TUNING_L3: usize = 1277;
const COMB_TUNING_L4: usize = 1356;
const COMB_TUNING_L5: usize = 1422;
const COMB_TUNING_L6: usize = 1491;
const COMB_TUNING_L7: usize = 1557;
const COMB_TUNING_L8: usize = 1617;

const ALLPASS_TUNING_L1: usize = 556;
const ALLPASS_TUNING_L2: usize = 441;
const ALLPASS_TUNING_L3: usize = 341;
const ALLPASS_TUNING_L4: usize = 225;

pub struct Freeverb {
    combs: [Comb; 8],
    allpasses: [AllPass; 4],
    wet_gains: (f32, f32),
    wet: f32,
    width: f32,
    dry: f32,
    input_gain: f32,
    dampening: f32,
    room_size: f32,
    frozen: bool,
}

fn adjust_length(length: usize, sr: usize) -> usize {
    (length as f32 * sr as f32 / 44100.0) as usize
}

impl Freeverb {
    pub fn new(sr: usize) -> Self {
        let mut freeverb = Freeverb {
            combs: [
                    Comb::new(adjust_length(COMB_TUNING_L1, sr)),
                    Comb::new(adjust_length(COMB_TUNING_L2, sr)),
                    Comb::new(adjust_length(COMB_TUNING_L3, sr)),
                    Comb::new(adjust_length(COMB_TUNING_L4, sr)),
                    Comb::new(adjust_length(COMB_TUNING_L5, sr)),
                    Comb::new(adjust_length(COMB_TUNING_L6, sr)),
                    Comb::new(adjust_length(COMB_TUNING_L7, sr)),
                    Comb::new(adjust_length(COMB_TUNING_L8, sr)),
            ],
            allpasses: [
                    AllPass::new(adjust_length(ALLPASS_TUNING_L1, sr)),
                    AllPass::new(adjust_length(ALLPASS_TUNING_L2, sr)),
                    AllPass::new(adjust_length(ALLPASS_TUNING_L3, sr)),
                    AllPass::new(adjust_length(ALLPASS_TUNING_L4, sr)),
            ],
            wet_gains: (0.0, 0.0),
            wet: 0.0,
            dry: 0.0,
            input_gain: 0.0,
            width: 0.0,
            dampening: 0.0,
            room_size: 0.0,
            frozen: false,
        };

        freeverb.set_wet(1.0);
        freeverb.set_width(0.5);
        freeverb.set_dampening(0.5);
        freeverb.set_room_size(0.5);
        freeverb.set_frozen(false);

        freeverb
    }

    // this was also modified for a mono output for the esp32 pedal
    pub fn tick(&mut self, input: f32) -> f32 {
        let input_mixed = input * FIXED_GAIN * self.input_gain;

        let mut out = 0.0;

        for combs in self.combs.iter_mut() {
            out += combs.tick(input_mixed);
        }

        for allpasses in self.allpasses.iter_mut() {
            out = allpasses.tick(out);
        }
        out * self.wet_gains.0 + input * self.dry   
    }

    pub fn set_dampening(&mut self, value: f32) {
        self.dampening = value * SCALE_DAMPENING;
        self.update_combs();
    }

    pub fn set_freeze(&mut self, frozen: bool) {
        self.frozen = frozen;
        self.update_combs();
    }

    pub fn set_wet(&mut self, value: f32) {
        self.wet = value * SCALE_WET;
        self.update_wet_gains();
    }

    pub fn set_width(&mut self, value: f32) {
        self.width = value;
        self.update_wet_gains();
    }

    fn update_wet_gains(&mut self) {
        self.wet_gains = (
            self.wet * (self.width / 2.0 + 0.5),
            self.wet * ((1.0 - self.width) / 2.0),
        )
    }

    fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
        self.input_gain = if frozen { 0.0 } else { 1.0 };
        self.update_combs();
    }

    pub fn set_room_size(&mut self, value: f32) {
        self.room_size = value * SCALE_ROOM + OFFSET_ROOM;
        self.update_combs();
    }

    fn update_combs(&mut self) {
        let (feedback, dampening) = if self.frozen {
            (1.0, 0.0)
        } else {
            (self.room_size, self.dampening)
        };

        for combs in self.combs.iter_mut() {
            //this was changed for mono
            combs.set_feedback(feedback);
            combs.set_dampening(dampening);
        }
    }

    pub fn set_dry(&mut self, value: f32) {
        self.dry = value;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    // this test eas also changed to match the mono output
    fn ticking_does_something() {
        let mut freeverb = super::Freeverb::new(44100);
        assert_eq!(freeverb.tick(1.0), 0.0);
        for _ in 0..super::COMB_TUNING_R8 * 2 {
            freeverb.tick(0.0);
        }
        assert_ne!(freeverb.tick(0.0));
    }
}
