// w8-core/src/isa/err.rs
use std::{
    error,
    fmt::{self, Display, Formatter},
};

#[derive(Debug)]
pub enum ISAErrorKind {
    /// An unknown opcode.
    UnknownOperationCode(String),

    /// Unexpected end of data.
    UnexpectedEndOfData { expected: usize, found: usize },

    /// Invalid operand count (> 3).
    InvalidOperandCount { count: u8 },

    /// Unknown operand tag.
    UnknownOperandTag { byte: u8 },

    /// A register number out of the 0-254 range (W8 has 255 registers).
    InvalidRegister { register: u8 },
}

impl Display for ISAErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOperationCode(opcode) => write!(f, "unknown operation code: '{opcode}'"),
            Self::UnexpectedEndOfData { expected, found } => {
                write!(
                    f,
                    "unexpected end of data: expected at least {expected} bytes, but only {found} were provided"
                )
            }
            Self::InvalidOperandCount { count } => {
                write!(f, "invalid operand count: {count} (expected 0-3)")
            }
            Self::UnknownOperandTag { byte } => {
                write!(f, "unknown operand tag byte: {byte}")
            }
            Self::InvalidRegister { register } => write!(
                f,
                "invalid register number {register}: the register number must be in the range 0-254"
            ),
        }
    }
}

#[derive(Debug)]
pub struct ISAError {
    pub kind: ISAErrorKind,
}

impl ISAError {
    pub fn new(kind: ISAErrorKind) -> Self {
        Self { kind }
    }
}

impl Display for ISAError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl error::Error for ISAError {}
