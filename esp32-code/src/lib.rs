#![no_std]

extern crate alloc;
use alloc::boxed::Box;
use core::marker::PhantomData;
use core::mem::size_of;
use bytemuck::{ Pod, NoUninit, cast_slice };

pub struct Packet<T: Pod, const N: usize> {
    bytes: Box<[u8]>, // layout: [header:4][info:N*4][crc:2]
    phantom: PhantomData<T>
}

/// N should be how many T are in Packet
impl<T: Pod, const N: usize> Packet<T, N> {
    const HEADER: [u8; 4] = [0xAA, 0x55, 0xAA, 0x55];
    const PAYLOAD_AMOUNT: usize = N;
    const PAYLOAD_LEN: usize = N * size_of::<T>();
    const TOTAL_LEN: usize = 4 + Self::PAYLOAD_LEN + 2;
    const X25: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

    /// Build a new packet from FFT magnitude data, computing header + CRC.
    pub fn new(info: &[T; N]) -> Self {
        let mut bytes = alloc::vec![0u8; Self::TOTAL_LEN].into_boxed_slice();
        bytes[0..4].copy_from_slice(&Self::HEADER);
        bytes[4..4 + Self::PAYLOAD_LEN].copy_from_slice(cast_slice(info));
        let crc = Self::X25.checksum(cast_slice(info)).to_le_bytes();
        bytes[4 + Self::PAYLOAD_LEN..].copy_from_slice(&crc);
        Packet { bytes, phantom: PhantomData::<T> }
    }

    pub fn into<U: bytemuck::NoUninit + Pod + Sized>(mut self) -> Packet<U, N> {
        self.bytes.fill(0);
        Packet {
            bytes: self.bytes,
            phantom: PhantomData,
        }
    }

    /// copies info into the info section of header
    pub fn copy_into(&mut self, info: &[T]) {
        self.bytes[4..4 + Self::PAYLOAD_LEN].copy_from_slice(cast_slice(info));

        let crc = Self::X25
            .checksum(&self.bytes[4..4 + Self::PAYLOAD_LEN])
            .to_le_bytes();

        self.bytes[4 + Self::PAYLOAD_LEN..].copy_from_slice(&crc);
    }
    
    pub fn copy_bytes_into(&mut self, info: &[u8]) {
        self.bytes[4..4 + Self::PAYLOAD_LEN].copy_from_slice(info);

        let crc = Self::X25
            .checksum(&self.bytes[4..4 + Self::PAYLOAD_LEN])
            .to_le_bytes();

        self.bytes[4 + Self::PAYLOAD_LEN..].copy_from_slice(&crc);
    }

    /// Reinterpret an existing byte buffer as a Packet (e.g. received data).
    /// Returns None if the length doesn't match what's expected for this N.
    pub fn from_bytes(bytes: Box<[u8]>) -> Option<Self> {
        if bytes.len() != Self::TOTAL_LEN {
            return None;
        }
        Some(Packet { bytes, phantom: PhantomData::<T>})
    }

    pub fn header(&self) -> &[u8; 4] {
        (&self.bytes[0..4]).try_into().unwrap()
    }

    pub fn info(&self) -> &[T; N] {
        let payload = &self.bytes[4..4 + Self::PAYLOAD_LEN];
        bytemuck::cast_slice(payload)
        .try_into()
        .unwrap()
    }

    pub fn info_mut(&mut self) -> &mut [T; N] {
        let payload = &mut self.bytes[4..4 + Self::PAYLOAD_LEN];
        bytemuck::cast_slice_mut(payload)
        .try_into()
        .unwrap()
    }

    pub fn crc(&self) -> &[u8; 2] {
        (&self.bytes[4 + Self::PAYLOAD_LEN..]).try_into().unwrap()
    }

    /// Verify the CRC actually matches the payload.
    pub fn verify(&self) -> bool {
        let payload = &self.bytes[4..4 + Self::PAYLOAD_LEN];

        let expected = Self::X25
            .checksum(payload)
            .to_le_bytes();

        expected == *self.crc()
    }

    /// Raw bytes, ready to send over UART as-is.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
    pub fn payload_bytes(&self) -> &[u8] {
        &self.bytes[4..4 + Self::PAYLOAD_LEN]
    }
}

impl<T: Pod, const N: usize> defmt::Format for Packet<T, N> {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "Packet {{ crc: {=[u8]}, first: {=[u8]}, len: {=usize} }}",
            self.crc()[..],
            self.bytes[..5],
            self.bytes.len()
        );
    }
}

