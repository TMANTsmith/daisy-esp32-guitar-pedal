use crc8_rs::{has_valid_crc8, insert_crc8};
use daisy::hal::gpio::gpioa::{PA3, PA6, PA7};
use daisy::hal::gpio::gpiob::PB1;
use daisy::hal::gpio::gpioc::{PC0, PC1, PC4};
use daisy::hal::nb;
use daisy::pac::{ADC1, ADC2};
use daisy::{hal, pac};
use hal::adc::{Adc, Enabled};
use hal::gpio::Analog;
use hal::prelude::*;
use hal::serial::{Rx, Tx};
use hal::dma::dma::Instance;
use hal::dma::dma::Stream3;
use hal::dma::Transfer;
use hal::dma::PeripheralToMemory;
use hal::adc::AdcDmaMode::Circular;



const CMD_LEN: usize = 32;
// TODO remember to calabrate and power up

#[derive(defmt::Format, core::fmt::Debug)]
pub enum AdcsError {
    ReadError(&'static str),
    WouldBlock,
    InvalidNum(usize),
}

pub struct MyAudio<ADC_DMA> {
    adc_dma: ADC_DMA,
}

impl<ADC_DMA> MyAudio<ADC_DMA> {
    pub fn new(adc_dma: ADC_DMA) -> Self {
        Self { adc_dma }
    }
}

pub struct Adcs {
    pub adc1: Adc<pac::ADC1, Enabled>,
    adc_dma: ADC_DMA, 
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
        adc_dma: Transfer<dma2::C3, Adc<pac::ADC1>, PeripheralToMemory, Circular>,
        pc0: PC0<Analog>,
        pa3: PA3<Analog>,
        pb1: PB1<Analog>,
        pa7: PA7<Analog>,
        pa6: PA6<Analog>,
        pc1: PC1<Analog>,
        pc4: PC4<Analog>,
    ) -> Self {
        Self {
            adc1,
            adc_dma,
            pc0,
            pa3,
            pb1,
            pa7,
            pa6,
            pc1,
            pc4,
        }
    }
}
