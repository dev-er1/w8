//! # Loader of files in the W8 Bytecode format
//!
//! This module implements the loader of `.wb` files,
//! with conversion into a [`Vec<Instruction>`].
pub mod err;

use crate::{
    W8_VERSION,
    isa::{instruction::Instruction, opcode::OperationCode},
    loader::err::{LoaderError, LoaderErrorKind},
};

pub struct W8Loader {
    pub src: Vec<u8>,
}

impl W8Loader {
    pub fn new(src: Vec<u8>) -> Self {
        Self { src }
    }

    #[allow(clippy::cmp_owned)]
    pub fn transpile(&self) -> Result<Vec<Instruction>, LoaderError> {
        // ====== File validity checks ======

        // The minimum size: 5 magic bytes + 6 version bytes = 11.
        if self.src.len() < 11 {
            return Err(LoaderError::new(
                LoaderErrorKind::FileIsNotInW8BytecodeFormat {
                    reason: "the file must be at least 11 bytes in size".to_string(),
                },
            ));
        }

        // Magic check.
        if &self.src[..5] != b"NVMBC" {
            return Err(LoaderError::new(
                LoaderErrorKind::FileIsNotInW8BytecodeFormat {
                    reason: "incorrect magic section".to_string(),
                },
            ));
        }

        // Parse the W8 version with which the file was compiled.
        let file_version = [
            u16::from_le_bytes([self.src[5], self.src[6]]),
            u16::from_le_bytes([self.src[7], self.src[8]]),
            u16::from_le_bytes([self.src[9], self.src[10]]),
        ];

        let compiled_version = format!(
            "{}.{}.{}",
            file_version[0], file_version[1], file_version[2]
        );

        let current_version = w8_version_parts();

        if file_version != current_version {
            return Err(LoaderError::new(LoaderErrorKind::UnsupportedVersion {
                file_version: compiled_version,
                vm_version: W8_VERSION.to_string(),
            }));
        }
        // ====== Instruction parsing ======

        let mut instructions = Vec::new();
        let mut offset = 11;

        while offset < self.src.len() {
            // Each instruction requires at least 2 bytes (opcode + operand count).
            if offset + 2 > self.src.len() {
                return Err(LoaderError::new(LoaderErrorKind::UnexpectedEndOfFile {
                    needed: 2,
                    remaining: self.src.len() - offset,
                }));
            }

            let opcode_byte = self.src[offset];
            if opcode_byte > OperationCode::VMCALL as u8 {
                return Err(LoaderError::new(LoaderErrorKind::UnknownOpcode {
                    byte: opcode_byte,
                }));
            }

            let operand_count = self.src[offset + 1];
            if operand_count > 3 {
                return Err(LoaderError::new(LoaderErrorKind::UnknownOpcode {
                    byte: opcode_byte,
                }));
            }

            // Compute the full instruction size.
            let mut instr_size = 2;
            let mut pos = offset + 2;

            for _ in 0..operand_count {
                if pos >= self.src.len() {
                    return Err(LoaderError::new(LoaderErrorKind::UnexpectedEndOfFile {
                        needed: 1,
                        remaining: self.src.len() - pos,
                    }));
                }

                let op_size = match self.src[pos] {
                    0x00 => 2, //< Tag + 1-byte register
                    0x01 => 9, //< Tag + 8-byte immediate
                    tag => {
                        return Err(LoaderError::new(LoaderErrorKind::UnknownOperandTag {
                            byte: tag,
                        }));
                    }
                };

                instr_size += op_size;
                pos += op_size;
            }

            let end = offset + instr_size;
            if end > self.src.len() {
                return Err(LoaderError::new(LoaderErrorKind::UnexpectedEndOfFile {
                    needed: instr_size,
                    remaining: self.src.len() - offset,
                }));
            }

            let instr_bytes = self.src[offset..end].to_vec();
            let instruction = Instruction::try_from(instr_bytes).map_err(|e| {
                LoaderError::new(LoaderErrorKind::FileIsNotInW8BytecodeFormat {
                    reason: format!("failed to parse instruction at offset {offset}: {e}"),
                })
            })?;

            instructions.push(instruction);
            offset = end;
        }

        Ok(instructions)
    }
}

/// The current W8 version as a numeric `(major, minor, patch)` triple.
///
/// Non-numeric parts (for example, a prerelease suffix) are truncated,
/// missing parts are treated as zero.
fn w8_version_parts() -> [u16; 3] {
    let mut parts = W8_VERSION.split('.').map(|part| {
        part.chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0)
    });

    [
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    ]
}
