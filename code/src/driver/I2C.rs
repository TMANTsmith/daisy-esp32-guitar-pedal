use bcm2837_lpa::gpio::{Gpio, Function};

pub fn mwrite_i2c(device_address: u8, register: u8, bit: u8, set: bool) {
    let bsc0 = unsafe { &*bcm2837_lpa::bsc0::BSC0::ptr() };

    // --- Write the register address first ---
    bsc0.s.write(0x302); // clear DONE/ERR
    bsc0.a.write(device_address as u32);
    bsc0.dlen.write(1);   // 1 byte = register address
    bsc0.fifo.write(register as u32);
    bsc0.c.write(1 << 15 | 1 << 7); // I2CEN | ST
    while bsc0.s.read() & (1 << 1) == 0 {}
    bsc0.s.write(1 << 1);

    // --- Read the current register value ---
    bsc0.dlen.write(1);
    bsc0.c.write(1 << 15 | 1 << 7 | 1 << 0); // I2CEN | ST | READ
    while bsc0.s.read() & (1 << 1) == 0 {}
    let mut reg_val = bsc0.fifo.read() as u8;
    bsc0.s.write(1 << 1); // clear DONE

    // --- Modify the bit ---
    if set {
        reg_val |= 1 << bit;
    } else {
        reg_val &= !(1 << bit);
    }

    // --- Write the register + new value back ---
    bsc0.s.write(0x302); // clear DONE/ERR
    bsc0.a.write(device_address as u32);
    bsc0.dlen.write(2); // register + value
    bsc0.fifo.write(register as u32);
    bsc0.fifo.write(reg_val as u32);
    bsc0.c.write(1 << 15 | 1 << 7); // start transfer
    while bsc0.s.read() & (1 << 1) == 0 {}
    bsc0.s.write(1 << 1);
}

