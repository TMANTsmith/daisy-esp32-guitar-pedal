use embedded_hal::blocking::i2c::{Write, Read, WriteRead};
use bcm2837_pac::I2C0;

pub struct MyI2C {
    pub i2c: I2C0,
}

impl Write for MyI2C {
    type Error = ();

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        let i2c = &self.i2c;

        // 1. Set slave address and length
        i2c.a.write(|w| unsafe { w.bits(addr as u32) });
        i2c.dlen.write(|w| unsafe { w.bits(bytes.len() as u32) });

        // 2. Clear status flags
        i2c.s.write(|w| unsafe { w.bits(0x302) });

        // 3. Fill FIFO
        for b in bytes {
            while i2c.s.read().txd().bit_is_clear() {}
            i2c.fifo.write(|w| unsafe { w.bits(*b as u32) });
        }

        // 4. Start transfer (write mode)
        i2c.c.write(|w| {
            w.i2c_enable().set_bit();
            w.st().set_bit();
            w.clear().set_bit();
            w.read_write().clear_bit()
        });

        // 5. Wait for completion
        while i2c.s.read().done().bit_is_clear() {}

        // 6. Check errors
        let s = i2c.s.read();
        if s.err().bit_is_set() || s.clkt().bit_is_set() {
            return Err(());
        }

        // 7. Clear DONE
        i2c.s.write(|w| w.done().set_bit());
        Ok(())
    }
}

impl Read for MyI2C {
    type Error = ();

    fn write(
        &mut self,
        addr: u8,
        bytes: &[u8],       // bytes to write first (e.g., register address)
        buffer: &mut [u8],  // buffer to read into
    ) -> Result<(), Self::Error> {
        let i2c = &self.i2c;

        // Enable controller and clear FIFO
        i2c.c.write(|w| w.i2c_enable().set_bit().clear().set_bit());

        // Set slave address
        i2c.a.write(|w| unsafe { w.bits(addr as u32) });

        // Clear status flags
        i2c.s.write(|w| unsafe { w.bits(0x302) });

        // Write phase: fill FIFO with register address / write bytes
        i2c.dlen.write(|w| unsafe { w.bits(bytes.len() as u32) });
        for b in bytes {
            while i2c.s.read().txd().bit_is_clear() {}
            i2c.fifo.write(|w| unsafe { w.bits(*b as u32) });
        }

        // Start write transfer
        i2c.c.modify(|_, w| w.st().set_bit().read_write().clear_bit());

        // Wait for write to finish
        while i2c.s.read().done().bit_is_clear() {}

        // Check errors
        let s = i2c.s.read();
        if s.err().bit_is_set() || s.clkt().bit_is_set() {
            return Err(());
        }
        i2c.s.write(|w| w.done().set_bit());

        // Read phase: set length to buffer size
        i2c.dlen.write(|w| unsafe { w.bits(buffer.len() as u32) });

        // Clear status flags and FIFO
        i2c.s.write(|w| unsafe { w.bits(0x302) });
        i2c.c.write(|w| w.i2c_enable().set_bit().clear().set_bit());

        // Set READ bit and start transfer
        i2c.c.modify(|_, w| w.read_write().set_bit().st().set_bit());

        // Read bytes from FIFO
        let mut index = 0;
        while index < buffer.len() {
            while i2c.s.read().rxd().bit_is_clear() {}
            buffer[index] = i2c.fifo.read().bits() as u8;
            index += 1;
        }

        // Wait until DONE
        while i2c.s.read().done().bit_is_clear() {}

        // Check errors
        let s = i2c.s.read();
        if s.err().bit_is_set() || s.clkt().bit_is_set() {
            return Err(());
        }

        // Clear DONE
        i2c.s.write(|w| w.done().set_bit());

        Ok(())
    }
}
