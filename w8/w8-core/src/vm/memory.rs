// w8-core/src/vm/memory.rs
//
//! # W8 memory
//!
//! This module defines the memory of the W8.
//!
//! W8 memory is a contiguous sequence of bytes. Each memory cell has
//! its own address, starting from `0`.
//!
//! Memory is accessed via the instructions of the `LOAD*` and `STORE*`
//! families.
//!
//! ## Memory representation
//!
//! Inside the VM, memory is stored as a byte array:
//!
//! ```text
//! Address:    0    1    2    3    ...
//!           +----+----+----+----+
//! Data      | 12 | FF | 00 | A5 |
//!           +----+----+----+----+
//! ```

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WVMMemory {
    data: Vec<u8>,
}

impl WVMMemory {
    /// Creates memory of the given size.
    ///
    /// All bytes are initialized to zero.
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    // ================== load_* ==================

    /// Loads an 8-bit unsigned value from memory.
    pub fn load_u8(&self, address: usize) -> Option<u8> {
        self.data.get(address).copied()
    }

    /// Loads a 16-bit unsigned value from memory.
    ///
    /// Returns `None` when `address..address + 2` is not entirely
    /// within the memory (including an address that overflows `usize`).
    pub fn load_u16(&self, address: usize) -> Option<u16> {
        Some(u16::from_le_bytes(
            self.data
                .get(address..address.checked_add(2)?)?
                .try_into()
                .unwrap(),
        ))
    }

    pub fn load_u32(&self, address: usize) -> Option<u32> {
        Some(u32::from_le_bytes(
            self.data
                .get(address..address.checked_add(4)?)?
                .try_into()
                .unwrap(),
        ))
    }

    pub fn load_u64(&self, address: usize) -> Option<u64> {
        Some(u64::from_le_bytes(
            self.data
                .get(address..address.checked_add(8)?)?
                .try_into()
                .unwrap(),
        ))
    }

    /// Loads an 8-bit signed value from memory.
    pub fn load_i8(&self, address: usize) -> Option<i8> {
        Some(*self.data.get(address)? as i8)
    }

    pub fn load_i16(&self, address: usize) -> Option<i16> {
        Some(i16::from_le_bytes(
            self.data
                .get(address..address.checked_add(2)?)?
                .try_into()
                .unwrap(),
        ))
    }

    pub fn load_i32(&self, address: usize) -> Option<i32> {
        Some(i32::from_le_bytes(
            self.data
                .get(address..address.checked_add(4)?)?
                .try_into()
                .unwrap(),
        ))
    }

    pub fn load_i64(&self, address: usize) -> Option<i64> {
        Some(i64::from_le_bytes(
            self.data
                .get(address..address.checked_add(8)?)?
                .try_into()
                .unwrap(),
        ))
    }

    /// Loads a 32-bit floating-point value from memory.
    pub fn load_f32(&self, address: usize) -> Option<f32> {
        Some(f32::from_le_bytes(
            self.data
                .get(address..address.checked_add(4)?)?
                .try_into()
                .unwrap(),
        ))
    }

    pub fn load_f64(&self, address: usize) -> Option<f64> {
        Some(f64::from_le_bytes(
            self.data
                .get(address..address.checked_add(8)?)?
                .try_into()
                .unwrap(),
        ))
    }

    // ================== store_* ==================

    /// Writes an 8-bit unsigned value to memory.
    pub fn store_u8(&mut self, address: usize, value: u8) -> Option<()> {
        *self.data.get_mut(address)? = value;
        Some(())
    }

    /// Writes a 16-bit unsigned value to memory.
    ///
    /// Returns `None` when `address..address + 2` is not entirely
    /// within the memory (including an address that overflows `usize`).
    pub fn store_u16(&mut self, address: usize, value: u16) -> Option<()> {
        self.data
            .get_mut(address..address.checked_add(2)?)?
            .copy_from_slice(&value.to_le_bytes());

        Some(())
    }

    pub fn store_u32(&mut self, address: usize, value: u32) -> Option<()> {
        self.data
            .get_mut(address..address.checked_add(4)?)?
            .copy_from_slice(&value.to_le_bytes());

        Some(())
    }

    pub fn store_u64(&mut self, address: usize, value: u64) -> Option<()> {
        self.data
            .get_mut(address..address.checked_add(8)?)?
            .copy_from_slice(&value.to_le_bytes());

        Some(())
    }

    /// Writes an 8-bit signed value to memory.
    pub fn store_i8(&mut self, address: usize, value: i8) -> Option<()> {
        self.store_u8(address, value as u8)
    }

    pub fn store_i16(&mut self, address: usize, value: i16) -> Option<()> {
        self.data
            .get_mut(address..address.checked_add(2)?)?
            .copy_from_slice(&value.to_le_bytes());

        Some(())
    }

    pub fn store_i32(&mut self, address: usize, value: i32) -> Option<()> {
        self.data
            .get_mut(address..address.checked_add(4)?)?
            .copy_from_slice(&value.to_le_bytes());

        Some(())
    }

    pub fn store_i64(&mut self, address: usize, value: i64) -> Option<()> {
        self.data
            .get_mut(address..address.checked_add(8)?)?
            .copy_from_slice(&value.to_le_bytes());

        Some(())
    }

    /// Writes a 32-bit floating-point value to memory.
    pub fn store_f32(&mut self, address: usize, value: f32) -> Option<()> {
        self.data
            .get_mut(address..address.checked_add(4)?)?
            .copy_from_slice(&value.to_le_bytes());

        Some(())
    }

    /// Writes a 64-bit floating-point value to memory.
    pub fn store_f64(&mut self, address: usize, value: f64) -> Option<()> {
        self.data
            .get_mut(address..address.checked_add(8)?)?
            .copy_from_slice(&value.to_le_bytes());

        Some(())
    }
}
