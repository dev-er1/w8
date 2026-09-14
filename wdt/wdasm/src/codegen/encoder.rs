// wdasm/src/codegen/encoder.rs
//
//! # W8 Bytecode generator
//!
//! Encodes a instruction into the W8 Bytecode binary format — the
//! format in which virtual machine programs are stored and executed
//! (see `docs/File-Format/File-Format.md`).
use w8_core::{
    W8_VERSION,
    isa::{
        instruction::Instruction,
        operand::{Operand, OperandKind},
        register::Register,
    },
};

/// The magic signature of a `.wb` file — `NVMBC`.
const MAGIC: [u8; 5] = *b"NVMBC";

/// Header size. 5 bytes of magic + 6 bytes of version.
const HEADER_SIZE: usize = 11;

/// Encodes a program into the W8 Bytecode format.
///
/// The header records the minimum required W8 version: the one declared
/// by the `.minversion` directive, or — if the directive was not used —
/// the current VM version [`W8_VERSION`] in the `major.minor.patch`
/// format.
pub fn encode(instructions: &[Instruction], min_version: Option<(u16, u16, u16)>) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER_SIZE + instructions.len() * 11);

    bytes.extend_from_slice(&MAGIC);

    match min_version {
        Some((major, minor, patch)) => {
            bytes.extend_from_slice(&major.to_le_bytes());
            bytes.extend_from_slice(&minor.to_le_bytes());
            bytes.extend_from_slice(&patch.to_le_bytes());
        }
        None => push_version(&mut bytes, W8_VERSION),
    }

    for instruction in instructions {
        bytes.push(instruction.opcode as u8);
        bytes.push(instruction.operand_count() as u8);

        for operand in [
            instruction.operand1,
            instruction.operand2,
            instruction.operand3,
        ]
        .into_iter()
        .flatten()
        {
            push_operand(&mut bytes, operand);
        }
    }

    bytes
}

/// Writes the version `major.minor.patch` as three `u16`s in little-endian.
///
/// Non-numeric parts (for example, a prerelease suffix) are truncated,
/// missing parts are treated as zero.
fn push_version(bytes: &mut Vec<u8>, version: &str) {
    let mut parts = version.split('.').map(version_number);

    for _ in 0..3 {
        bytes.extend_from_slice(&parts.next().unwrap_or(0).to_le_bytes());
    }
}

/// Parses the numeric part of a string; returns 0 for a non-number.
fn version_number(part: &str) -> u16 {
    part.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// Writes an operand: a tag byte and the data.
fn push_operand(bytes: &mut Vec<u8>, operand: Operand) {
    match operand.kind {
        OperandKind::Register(Register(number)) => {
            bytes.push(0x00);
            bytes.push(number);
        }
        OperandKind::Immediate(value) => {
            bytes.push(0x01);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
}
