// w8-core/src/isa/instruction.rs
//
//! # Representation of an W8 instruction
//!
//! This module defines the structure of a single instruction.
//!
//! An instruction is the smallest unit of program execution.
//! Each instruction consists of:
//! - one opcode;
//! - from zero to three operands.
use std::fmt::{self, Display, Formatter};

use crate::{
    isa::{
        err::{ISAError, ISAErrorKind},
        opcode::OperationCode,
        operand::{Operand, OperandKind},
        register::Register,
    },
    vm::err::{VMError, VMErrorKind},
};

#[derive(Debug, Clone, Copy)]
pub struct Instruction {
    pub opcode: OperationCode,
    pub operand1: Option<Operand>,
    pub operand2: Option<Operand>,
    pub operand3: Option<Operand>,
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match (self.operand1, self.operand2, self.operand3) {
            (Some(op1), Some(op2), Some(op3)) => write!(f, "{:?} {op1}, {op2}, {op3}", self.opcode),
            (Some(op1), Some(op2), None) => write!(f, "{:?} {op1}, {op2}", self.opcode),
            (Some(op1), None, None) => write!(f, "{:?} {op1}", self.opcode),
            (None, None, None) => write!(f, "{:?}", self.opcode),
            _ => unreachable!(),
        }
    }
}

impl Instruction {
    // ==== Helper functions for the VM ====

    pub fn operand_count(&self) -> usize {
        [self.operand1, self.operand2, self.operand3]
            .into_iter()
            .flatten()
            .count()
    }

    pub fn expect1(&self) -> Result<Operand, VMError> {
        if self.operand_count() != 1 {
            return Err(VMError::new(VMErrorKind::IncorrectNumberOfOperands {
                expected: 1,
                got: self.operand_count() as u8,
            }));
        }

        Ok(self.operand1.unwrap())
    }

    pub fn expect2(&self) -> Result<(Operand, Operand), VMError> {
        if self.operand_count() != 2 {
            return Err(VMError::new(VMErrorKind::IncorrectNumberOfOperands {
                expected: 2,
                got: self.operand_count() as u8,
            }));
        }

        Ok((self.operand1.unwrap(), self.operand2.unwrap()))
    }

    pub fn expect3(&self) -> Result<(Operand, Operand, Operand), VMError> {
        if self.operand_count() != 3 {
            return Err(VMError::new(VMErrorKind::IncorrectNumberOfOperands {
                expected: 3,
                got: self.operand_count() as u8,
            }));
        }

        Ok((
            self.operand1.unwrap(),
            self.operand2.unwrap(),
            self.operand3.unwrap(),
        ))
    }
}

impl TryFrom<Vec<u8>> for Instruction {
    type Error = ISAError;

    #[allow(clippy::needless_range_loop)]
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() < 2 {
            return Err(ISAError::new(ISAErrorKind::UnexpectedEndOfData {
                expected: 2,
                found: value.len(),
            }));
        }

        let opcode = OperationCode::try_from(value[0])?;
        let operand_count = value[1];

        if operand_count > 3 {
            return Err(ISAError::new(ISAErrorKind::InvalidOperandCount {
                count: operand_count,
            }));
        }

        let mut offset = 2usize;
        let mut operands = [None, None, None];

        for i in 0..operand_count as usize {
            if offset >= value.len() {
                return Err(ISAError::new(ISAErrorKind::UnexpectedEndOfData {
                    expected: offset + 1,
                    found: value.len(),
                }));
            }

            let tag = value[offset];
            offset += 1;

            let operand = match tag {
                0x00 => {
                    if offset >= value.len() {
                        return Err(ISAError::new(ISAErrorKind::UnexpectedEndOfData {
                            expected: offset + 1,
                            found: value.len(),
                        }));
                    }
                    let reg = value[offset];
                    offset += 1;
                    let Some(register) = Register::new(reg) else {
                        return Err(ISAError::new(ISAErrorKind::InvalidRegister {
                            register: reg,
                        }));
                    };
                    Operand {
                        kind: OperandKind::Register(register),
                    }
                }
                0x01 => {
                    if offset + 8 > value.len() {
                        return Err(ISAError::new(ISAErrorKind::UnexpectedEndOfData {
                            expected: offset + 8,
                            found: value.len(),
                        }));
                    }
                    let imm = u64::from_le_bytes([
                        value[offset],
                        value[offset + 1],
                        value[offset + 2],
                        value[offset + 3],
                        value[offset + 4],
                        value[offset + 5],
                        value[offset + 6],
                        value[offset + 7],
                    ]);
                    offset += 8;
                    Operand {
                        kind: OperandKind::Immediate(imm),
                    }
                }
                _ => {
                    return Err(ISAError::new(ISAErrorKind::UnknownOperandTag { byte: tag }));
                }
            };

            operands[i] = Some(operand);
        }

        Ok(Instruction {
            opcode,
            operand1: operands[0],
            operand2: operands[1],
            operand3: operands[2],
        })
    }
}
