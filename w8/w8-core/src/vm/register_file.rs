// w8-core/src/vm/register_file.rs
//
//! # Register file
//!
//! This module defines the register file of the W8 virtual machine.
//!
//! Each register holds a single value of type [`u64`].
use std::{
    array,
    ops::{Index, IndexMut},
};

use crate::isa::register::Register;

pub struct RegisterFile {
    registers: [u64; 255],
}

impl RegisterFile {
    pub fn new() -> Self {
        Self {
            registers: array::from_fn(|_| u64::default()),
        }
    }

    pub fn from_registers(registers: [u64; 255]) -> Self {
        Self { registers }
    }

    /// Returns a mutable pointer to the first register.
    pub fn as_mut_ptr(&mut self) -> *mut u64 {
        self.registers.as_mut_ptr()
    }
}

// Trait implementations for `RegisterFile`

impl Default for RegisterFile {
    fn default() -> Self {
        Self::new()
    }
}

// The `Index` implementation allows accessing the registers
// via the indexing operator:
//
// ```
// let value = registers[Register(0)];
// ```
impl Index<Register> for RegisterFile {
    type Output = u64;

    fn index(&self, index: Register) -> &Self::Output {
        &self.registers[index.0 as usize]
    }
}

// The `IndexMut` implementation allows modifying register values
// via the indexing operator:
//
// ```
// registers[Register(0)] = Value::default();
// ```
impl IndexMut<Register> for RegisterFile {
    fn index_mut(&mut self, index: Register) -> &mut Self::Output {
        &mut self.registers[index.0 as usize]
    }
}
