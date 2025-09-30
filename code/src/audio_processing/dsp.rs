// pulls in all modules
use crate::audioprocessing::audiomodules::prelude::*;


// comment out modules you dont want
// then you can use this fn in main
pub fn run_modules() {
    // like this
    // let example = example;
    // example.proess(&mut buffer)
    let gain = Gain::new(0.5);   // create instance
    gain.process(&mut buffer);   // apply gain
    
    let invert = Invert;
    invert.process(&mut buffer);
}
