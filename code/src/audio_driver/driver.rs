use crate::config::settings;
use esp_idf_hal::prelude::*;  // Brings in useful helpers like Hz units
use esp_idf_hal::i2c::*;      // I2C driver and config
use esp_idf_hal::peripherals::Peripherals; // Access ESP32 peripherals
use esp_idf_hal::i2s::*;

// Define a struct to hold the state of the audio driver
pub struct AudioDriver {
    i2c: Master<'static>, // 'Master' is the I2C master peripheral
                          // 'static lifetime means it lives for the program's lifetime
                          // This stores the I2C object so it can be used in methods
}

// Implement methods for the AudioDriver struct
impl AudioDriver {
    // Constructor for AudioDriver
    // Returns Result<Self> because initializing I2C can fail
    // 'Self' here refers to the struct AudioDriver itself
    pub fn new() -> anyhow::Result<Self> {
        // Take ownership of ESP32 peripherals
        let peripherals = Peripherals::take().unwrap();

        // Configure GPIO pins for SDA and SCL from settings
        let sda = peripherals.pins.gpio(settings::I2C_SDA);
        let scl = peripherals.pins.gpio(settings::I2C_SCL);

        // Configure I2C parameters: set baud rate from settings
        let config = MasterConfig::new().baudrate(settings::I2C_FREQ_HZ.hz().into());

        // Initialize the I2C peripheral
        // The '?' operator propagates errors as an anyhow::Error
        let i2c = Master::new(peripherals.i2c0, sda, scl, &config)?;

        // Return an instance of AudioDriver wrapped in Ok()
        // Ok() means this function succeeded and provides the value
        Ok(Self { i2c })
    }

    // Method to write bytes to a device at a given I2C address
    // Returns Result<()> to indicate success or failure
    pub fn write(&mut self, addr: u8, data: &[u8]) -> anyhow::Result<()> {
        self.i2c.write(addr, data)?; // Send bytes via I2C; propagate error if it fails
        Ok(()) // Return Ok with unit type () meaning "success with no value"
    }

    // Method to read bytes from a device at a given I2C address
    // Returns Result<()> for error handling
    pub fn read(&mut self, addr: u8, buffer: &mut [u8]) -> anyhow::Result<()> {
        self.i2c.read(addr, buffer)?; // Read bytes into buffer; propagate error if it fails
        Ok(()) // Indicate success
    }
    pub fn mwrite(&mut self, addr: u8, bit : u8, value : bool) -> anyhow::Result<()> {
        let mut current: [u8; 1] = [0];
        audio_driver.read(DEVICE_ADDRESS, &mut current)?;

        if value == True {
            current[0] |= 1 << bit;
        } else {
            current[0] &= !(1 << bit);
        }
        audio_driver.write(DEVICE_ADDRESS, &[addr, current[0]])?;
        Ok(())
    }

        

// ---------- I2S stuff  ---------- //

pub fn init_i2s() -> I2sDriver<'static> {
    let config = I2sDriverConfig::new()
        .sample_rate(44100)
        .data_bits(DataBits::Bits24)
        .channel_format(ChannelFormat::Mono)
        .communication_format(CommunicationFormat::I2S)
        .dma_buf_count(2)
        .dma_buf_len(1024);

    I2sDriver::new(
        I2sNum::I2S0,
        BckPin::new(I2S_BLCK),
        WsPin::new(I2S_LRCK),
        DataOutPin::new(I2S_DOUT),
        Some(DataInPin::new(I2S_DIN)),
        config
    ).unwrap()
}

pub fn unpack(in_bytes: &[u8]) -> Vec<f32> {
    let mut out_samples = Vec::with_capacity(in_bytes.len() / 3);
    for chunk in in_bytes.chunks_exact(3) {
        let raw = ((chunk[0] as u32) << 16)
                | ((chunk[1] as u32) << 8)
                | (chunk[2] as u32);
        let sample_i32 = (raw << 8) as i32 >> 8;
        let f = sample_i32 as f32 / (1 << 23) as f32;
        out_samples.push(f);
    }
    out_samples
}

fn pack(in_samples: &[f32]) -> Vec<u8> {
    let mut out_bytes = Vec::with_capacity(in_samples.len() * 3);
    for &f in in_samples {
        // scale back to 24-bit integer range
        let s = (f * (1 << 23) as f32).clamp(-(1 << 23) as f32, (1 << 23) as f32 - 1.0) as i32;
        out_bytes.push(((s >> 16) & 0xFF) as u8);
        out_bytes.push(((s >> 8) & 0xFF) as u8);
        out_bytes.push((s & 0xFF) as u8);
    }
    out_bytes
}


