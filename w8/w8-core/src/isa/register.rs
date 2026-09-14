// w8-core/src/isa/register.rs
//
//! # W8 registers
//!
//! This module defines the register type of the virtual machine.
//!
//! A register is an identifier of one of the 255 general-purpose
//! registers. The register indicates which register to access.
use std::fmt::{self, Display, Formatter};

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Register(pub u8);

impl Register {
    /// The maximum register number: W8 has 255 registers, numbered 0-254.
    pub const MAX_INDEX: u8 = 254;

    pub const fn new(number: u8) -> Option<Self> {
        if number <= Self::MAX_INDEX {
            Some(Self(number))
        } else {
            None
        }
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "R{}", self.0)
    }
}
