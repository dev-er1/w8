// w8-core/src/isa/opcode.rs
//
//! # W8 opcodes
//!
//! This module defines the opcodes of the W8 virtual machine.
//!
//! ## What is an "opcode"
//!
//! An opcode (operation code) is a part of a machine instruction
//! that tells the processor or the virtual machine which
//! action to perform. (taken from <https://ru.wikipedia.org/wiki/Код_операции> and translated into English)
//!
//! Within W8, an opcode is a byte describing the operation that
//! the virtual machine must perform.
//!
//! ## Notation
//!
//! The documentation uses the following notation:
//!
//! - `dst` — the destination operand;
//! - `src1` — the first source operand;
//! - `src2` — the second source operand.
use std::str::FromStr;

use crate::isa::err::{ISAError, ISAErrorKind};

/// Enumeration of opcodes for the VM.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum OperationCode {
    /// Does nothing.
    NOP,

    /// Copies the value from the second operand into the first operand.
    ///
    /// ```text
    /// MOVE <dst>, <src1>
    /// ```
    ///
    /// The value from `src1` is written into `dst`.
    MOVE,

    /// Loads 8 bits from memory into an operand.
    ///
    /// ```text
    /// LOAD8 <dst>, <src1>
    /// ```
    ///
    /// 1 byte read from the address in `src1` is written into `dst`.
    LOAD8,

    /// Loads 16 bits from memory into an operand.
    ///
    /// ```text
    /// LOAD16 <dst>, <src1>
    /// ```
    ///
    /// 2 bytes read from the address in `src1` are written into `dst`.
    LOAD16,

    /// Loads 32 bits from memory into an operand.
    ///
    /// ```text
    /// LOAD32 <dst>, <src1>
    /// ```
    ///
    /// 4 bytes read from the address in `src1` are written into `dst`.
    LOAD32,

    /// Loads 64 bits from memory into an operand.
    ///
    /// ```text
    /// LOAD64 <dst>, <src1>
    /// ```
    ///
    /// 8 bytes read from the address in `src1` are written into `dst`.
    LOAD64,

    /// Stores 8 bits from an operand into memory.
    ///
    /// ```text
    /// STORE8 <dst>, <src1>
    /// ```
    ///
    /// 1 byte from `src1` is written to the address in `dst`.
    STORE8,

    /// Stores 16 bits from an operand into memory.
    ///
    /// ```text
    /// STORE16 <dst>, <src1>
    /// ```
    ///
    /// 2 bytes from `src1` are written to the address in `dst`.
    STORE16,

    /// Stores 32 bits from an operand into memory.
    ///
    /// ```text
    /// STORE32 <dst>, <src1>
    /// ```
    ///
    /// 4 bytes from `src1` are written to the address in `dst`.
    STORE32,

    /// Stores 64 bits from an operand into memory.
    ///
    /// ```text
    /// STORE64 <dst>, <src1>
    /// ```
    ///
    /// 8 bytes from `src1` are written to the address in `dst`.
    STORE64,

    /// Adds two integer values.
    ///
    /// Adds `src1` and `src2`, then writes the result into `dst`.
    ///
    /// ```text
    /// IADD <dst>, <src1>, <src2>
    /// ```
    IADD,

    /// Subtracts two integer values.
    ///
    /// Subtracts `src2` from `src1` and writes the result into `dst`.
    ///
    /// ```text
    /// ISUB <dst>, <src1>, <src2>
    /// ```
    ISUB,

    /// Multiplies two integer values.
    ///
    /// Multiplies `src1` and `src2`, then writes the result into `dst`.
    ///
    /// ```text
    /// IMUL <dst>, <src1>, <src2>
    /// ```
    IMUL,

    /// Signed integer division.
    ///
    /// Divides `src1` by `src2` and writes the result into `dst`.
    ///
    /// ```text
    /// SDIV <dst>, <src1>, <src2>
    /// ```
    SDIV,

    /// Unsigned integer division.
    ///
    /// Divides `src1` by `src2` as unsigned values and writes the result into `dst`.
    ///
    /// ```text
    /// UDIV <dst>, <src1>, <src2>
    /// ```
    UDIV,

    /// Remainder of signed division.
    ///
    /// Computes `src1` % `src2` and writes the result into `dst`.
    ///
    /// ```text
    /// SREM <dst>, <src1>, <src2>
    /// ```
    SREM,

    /// Remainder of unsigned division.
    ///
    /// Computes `src1` % `src2` as unsigned values and writes the result into `dst`.
    ///
    /// ```text
    /// UREM <dst>, <src1>, <src2>
    /// ```
    UREM,

    /// Negates an integer value.
    ///
    /// ```text
    /// INEG <dst>, <src1>
    /// ```
    INEG,

    /// Adds two floating-point numbers.
    ///
    /// Adds `src1` and `src2`, then writes the result into `dst`.
    ///
    /// ```text
    /// FADD <dst>, <src1>, <src2>
    /// ```
    FADD,

    /// Subtracts two floating-point numbers.
    ///
    /// Subtracts `src2` from `src1` and writes the result into `dst`.
    ///
    /// ```text
    /// FSUB <dst>, <src1>, <src2>
    /// ```
    FSUB,

    /// Multiplies two floating-point numbers.
    ///
    /// Multiplies `src1` and `src2`, then writes the result into `dst`.
    ///
    /// ```text
    /// FMUL <dst>, <src1>, <src2>
    /// ```
    FMUL,

    /// Divides two floating-point numbers.
    ///
    /// Divides `src1` by `src2` and writes the result into `dst`.
    ///
    /// ```text
    /// FDIV <dst>, <src1>, <src2>
    /// ```
    FDIV,

    /// Remainder of division of two floating-point numbers.
    ///
    /// Computes the remainder of dividing `src1` by `src2` and writes the result into `dst`.
    ///
    /// ```text
    /// FREM <dst>, <src1>, <src2>
    /// ```
    FREM,

    /// Negates a floating-point number.
    ///
    /// ```text
    /// FNEG <dst>, <src1>
    /// ```
    FNEG,

    /// Bitwise AND.
    ///
    /// ```text
    /// AND <dst>, <src1>, <src2>
    /// ```
    AND,

    /// Bitwise OR.
    ///
    /// ```text
    /// OR <dst>, <src1>, <src2>
    /// ```
    OR,

    /// Bitwise XOR.
    ///
    /// ```text
    /// XOR <dst>, <src1>, <src2>
    /// ```
    XOR,

    /// Bitwise NOT.
    ///
    /// ```text
    /// NOT <dst>, <src1>
    /// ```
    NOT,

    /// Logical shift left.
    ///
    /// ```text
    /// LSL <dst>, <src1>, <src2>
    /// ```
    LSL,

    /// Logical shift right.
    ///
    /// ```text
    /// LSR <dst>, <src1>, <src2>
    /// ```
    LSR,

    /// Arithmetic shift right.
    ///
    /// ```text
    /// SAR <dst>, <src1>, <src2>
    /// ```
    SAR,

    /// Equality check.
    ///
    /// Writes 1 into `dst` if `src1` == `src2`, otherwise 0.
    ///
    /// ```text
    /// IEQ <dst>, <src1>, <src2>
    /// ```
    IEQ,

    /// Inequality check.
    ///
    /// ```text
    /// INE <dst>, <src1>, <src2>
    /// ```
    INE,

    /// Signed less-than.
    ///
    /// ```text
    /// SLT <dst>, <src1>, <src2>
    /// ```
    SLT,

    /// Signed less-than or equal.
    ///
    /// ```text
    /// SLE <dst>, <src1>, <src2>
    /// ```
    SLE,

    /// Signed greater-than.
    ///
    /// ```text
    /// SGT <dst>, <src1>, <src2>
    /// ```
    SGT,

    /// Signed greater-than or equal.
    ///
    /// ```text
    /// SGE <dst>, <src1>, <src2>
    /// ```
    SGE,

    /// Unsigned less-than.
    ///
    /// ```text
    /// ULT <dst>, <src1>, <src2>
    /// ```
    ULT,

    /// Unsigned less-than or equal.
    ///
    /// ```text
    /// ULE <dst>, <src1>, <src2>
    /// ```
    ULE,

    /// Unsigned greater-than.
    ///
    /// ```text
    /// UGT <dst>, <src1>, <src2>
    /// ```
    UGT,

    /// Unsigned greater-than or equal.
    ///
    /// ```text
    /// UGE <dst>, <src1>, <src2>
    /// ```
    UGE,

    /// Equality check.
    ///
    /// ```text
    /// FEQ <dst>, <src1>, <src2>
    /// ```
    FEQ,

    /// Inequality check.
    ///
    /// ```text
    /// FNE <dst>, <src1>, <src2>
    /// ```
    FNE,

    /// Less-than.
    ///
    /// ```text
    /// FLT <dst>, <src1>, <src2>
    /// ```
    FLT,

    /// Less-than or equal.
    ///
    /// ```text
    /// FLE <dst>, <src1>, <src2>
    /// ```
    FLE,

    /// Greater-than.
    ///
    /// ```text
    /// FGT <dst>, <src1>, <src2>
    /// ```
    FGT,

    /// Greater-than or equal.
    ///
    /// ```text
    /// FGE <dst>, <src1>, <src2>
    /// ```
    FGE,

    /// Unconditional jump.
    ///
    /// ```text
    /// JMP <offset>
    /// ```
    JMP,

    /// Jumps if `src1` == 0.
    ///
    /// ```text
    /// JZ <src1>, <offset>
    /// ```
    JZ,

    /// Jumps if `src1` != 0.
    ///
    /// ```text
    /// JNZ <src1>, <offset>
    /// ```
    JNZ,

    /// Calls a subroutine.
    ///
    /// ```text
    /// CALL <offset>
    /// ```
    CALL,

    /// Returns from a subroutine.
    ///
    /// ```text
    /// RET
    /// ```
    RET,

    /// # `VMCALL`
    ///
    /// `VMCALL` is the “channel” between the guest program and the
    /// host. It transfers control to the host.
    ///
    /// ```text
    /// VMCALL <service>, <address>, <size>
    /// ```
    ///
    /// - `service` — the number of the service to call;
    /// - `address` — the address in memory where the block of arguments
    ///   for the service lies;
    /// - `size` — the size of that block, in bytes.
    VMCALL,
}

impl FromStr for OperationCode {
    type Err = ISAError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "nop" => Ok(Self::NOP),
            "move" => Ok(Self::MOVE),

            "load8" => Ok(Self::LOAD8),
            "load16" => Ok(Self::LOAD16),
            "load32" => Ok(Self::LOAD32),
            "load64" => Ok(Self::LOAD64),

            "store8" => Ok(Self::STORE8),
            "store16" => Ok(Self::STORE16),
            "store32" => Ok(Self::STORE32),
            "store64" => Ok(Self::STORE64),

            "iadd" => Ok(Self::IADD),
            "isub" => Ok(Self::ISUB),
            "imul" => Ok(Self::IMUL),
            "sdiv" => Ok(Self::SDIV),
            "udiv" => Ok(Self::UDIV),
            "srem" => Ok(Self::SREM),
            "urem" => Ok(Self::UREM),
            "ineg" => Ok(Self::INEG),

            "fadd" => Ok(Self::FADD),
            "fsub" => Ok(Self::FSUB),
            "fmul" => Ok(Self::FMUL),
            "fdiv" => Ok(Self::FDIV),
            "frem" => Ok(Self::FREM),
            "fneg" => Ok(Self::FNEG),

            "and" => Ok(Self::AND),
            "or" => Ok(Self::OR),
            "xor" => Ok(Self::XOR),
            "not" => Ok(Self::NOT),
            "lsl" => Ok(Self::LSL),
            "lsr" => Ok(Self::LSR),
            "sar" => Ok(Self::SAR),

            "ieq" => Ok(Self::IEQ),
            "ine" => Ok(Self::INE),
            "slt" => Ok(Self::SLT),
            "sle" => Ok(Self::SLE),
            "sgt" => Ok(Self::SGT),
            "sge" => Ok(Self::SGE),
            "ult" => Ok(Self::ULT),
            "ule" => Ok(Self::ULE),
            "ugt" => Ok(Self::UGT),
            "uge" => Ok(Self::UGE),

            "feq" => Ok(Self::FEQ),
            "fne" => Ok(Self::FNE),
            "flt" => Ok(Self::FLT),
            "fle" => Ok(Self::FLE),
            "fgt" => Ok(Self::FGT),
            "fge" => Ok(Self::FGE),

            "jmp" => Ok(Self::JMP),
            "jz" => Ok(Self::JZ),
            "jnz" => Ok(Self::JNZ),
            "call" => Ok(Self::CALL),
            "ret" => Ok(Self::RET),

            "vmcall" => Ok(Self::VMCALL),

            _ => Err(ISAError::new(ISAErrorKind::UnknownOperationCode(
                s.to_string(),
            ))),
        }
    }
}

impl TryFrom<u8> for OperationCode {
    type Error = ISAError;

    #[allow(clippy::missing_transmute_annotations)]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= OperationCode::VMCALL as u8 {
            // SAFETY: all values from 0 to `VMCALL as u8` are valid enum variants.
            Ok(unsafe { std::mem::transmute(value) })
        } else {
            Err(ISAError::new(ISAErrorKind::UnknownOperationCode(
                value.to_string(),
            )))
        }
    }
}
