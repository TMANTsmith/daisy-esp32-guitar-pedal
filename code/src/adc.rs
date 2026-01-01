use daisy::hal::nb;
use crc8_rs::{has_valid_crc8, insert_crc8};
use daisy::hal::gpio::gpioa::{PA3, PA6, PA7};
use daisy::hal::gpio::gpiob::PB1;
use daisy::hal::gpio::gpioc::{PC0, PC1, PC4};
use daisy::pac::{ADC1, ADC2};
use daisy::{hal, pac};
use hal::adc::{Adc, Enabled};
use hal::gpio::Analog;
use hal::prelude::*;
use hal::serial::{Rx, Tx};

const CMD_LEN: usize = 32;

#[derive(defmt::Format, core::fmt::Debug)]
pub enum AdcsError {
    ReadError(&'static str),
    WouldBlock,
}
pub struct Adcs {
    pub buffer: [u32; 7],
    pub adc1: Adc<pac::ADC1, Enabled>,
    pub adc2: Adc<pac::ADC2, Enabled>,
    pub pc0: PC0<Analog>,
    pub pa3: PA3<Analog>,
    pub pb1: PB1<Analog>,
    pub pa7: PA7<Analog>,
    pub pa6: PA6<Analog>, // correct pins as needed
    pub pc1: PC1<Analog>,
    pub pc4: PC4<Analog>,
}

impl Adcs {
    pub fn new(
        adc1: Adc<ADC1, Enabled>,
        adc2: Adc<ADC2, Enabled>,
        pc0: PC0<Analog>,
        pa3: PA3<Analog>,
        pb1: PB1<Analog>,
        pa7: PA7<Analog>,
        pa6: PA6<Analog>,
        pc1: PC1<Analog>,
        pc4: PC4<Analog>,
    ) -> Self {
        let buffer = [0u32; 7];
        Self {
            buffer,
            adc1,
            adc2,
            pc0,
            pa3,
            pb1,
            pa7,
            pa6,
            pc1,
            pc4,
        }
    }

    // uses adc1 to read an individual pin
    //
    // TODO set up DMA transfer for a fast scan of all adcs

    pub fn read_all(&mut self)-> Result<(), AdcsError> {
        self.buffer[0] = match self.adc1.read(&mut self.pc0) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pc0")),
        };

        self.buffer[1] = match self.adc1.read(&mut self.pa3) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pa3")),
        };

        self.buffer[2] = match self.adc1.read(&mut self.pb1) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pb1")),
        };

        self.buffer[3] = match self.adc1.read(&mut self.pa7) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pa7")),
        };

        self.buffer[4] = match self.adc1.read(&mut self.pa6) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pa6")),
        };

        self.buffer[5] = match self.adc1.read(&mut self.pc1) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pc1")),
        };

        self.buffer[6] = match self.adc1.read(&mut self.pc4) {
            Ok(val) => val,
            Err(nb::Error::WouldBlock) => return Err(AdcsError::WouldBlock),
            Err(_) => return Err(AdcsError::ReadError("pc4")),
        };

        Ok(())
    }
}

