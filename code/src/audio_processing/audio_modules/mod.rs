pub mod gain;
pub mod invert;

// optional: re-export for easier imports
pub use gain::GainEffect;
pub use invert::InvertEffect;

pub struct AudioBuffer {
    pub samples: [i16; 1024],
}

