mod audio_driver;
mod audio_processing;
mod config;


use crate::audio_driver::driver::{init_i2s, AudioDriver};

fn main() -> anyhow::Result<()> {
    // I2S driver
    let mut i2s = init_i2s()?;

    // I2C driver
    let mut audio_driver = AudioDriver::new()?;

    mwrite(0x02, 7, true); // freeze bit
    mwrite(0x03, 0, true); // popguard on
    // reminder set clocking ratios
    if auto_mute == true {
        mwrite(0x06, 5, true);
    } 
    mwrite(0x02, 7, ,false); // freeze bit (finalizes flash)


    // setting up buffer //
    let mut buffer = [0u8; 1024];
    
    loop {

        i2s.read(&mut buffer)?;

        let mut buffer_unpacked = unpack(&buffer);
    
        run_modules(&mut buffer_unpacked);
        
        buffer_packed = pack(&buffer_unpacked);

        i2s.write(&buffer_packed)?;

    }   
     
}
