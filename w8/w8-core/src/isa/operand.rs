// w8-core/src/isa/operand.rs
//
//! # W8 operands
//!
//! This module defines the types that describe instruction operands.
//!
//! An operand is an instruction argument. Depending on the instruction,
//! an operand can be a register or an immediate value.
use std::fmt::{self, Display, Formatter};

use crate::{
    isa::register::Register,
    vm::err::{VMError, VMErrorKind},
};

#[derive(Debug, Clone, Copy)]
pub enum OperandKind {
    /// A virtual machine register.
    Register(Register),
    Immediate(u64),
}

impl Display for OperandKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Immediate(v) => write!(f, "{v}"),
            Self::Register(r) => write!(f, "{r}"),
        }
    }
}

impl OperandKind {
    /// Converts an `OperandKind` into a string with the operand type.
    ///
    /// Needed for [`vm::err`](crate::vm::err).
    pub fn kind(&self) -> &str {
        match self {
            Self::Immediate(_) => "value",
            Self::Register(_) => "register",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Operand {
    pub kind: OperandKind,
}

impl Display for Operand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl Operand {
    pub fn expect_register(&self) -> Result<Register, VMError> {
        match self.kind {
            OperandKind::Register(r) => Ok(r),
            got => Err(VMError::new(VMErrorKind::IncorrectTypeOfOperand {
                expected: OperandKind::Register(Register(0)),
                got,
            })),
        }
    }

    pub fn expect_immediate(&self) -> Result<u64, VMError> {
        match self.kind {
            OperandKind::Immediate(r) => Ok(r),
            got => Err(VMError::new(VMErrorKind::IncorrectTypeOfOperand {
                expected: OperandKind::Immediate(0),
                got,
            })),
        }
    }
}
