//! # W8 ISA
//!
//! The ISA (Instruction Set Architecture) of W8 is the specification
//! of the instruction set of the W8 virtual machine.
//!
//! ## Module contents
//!
//! - [`instruction`] — the representation of an W8 instruction;
//! - [`opcode`] — operation codes (opcodes);
//! - [`operand`] — the representation of instruction operands;
//! - [`register`] — virtual machine register identifiers;
//! - [`err`] — errors related to the ISA.
pub mod err;
pub mod instruction;
pub mod opcode;
pub mod operand;
pub mod register;
