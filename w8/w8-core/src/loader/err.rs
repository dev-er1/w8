// w8-core/src/loader/err.rs
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum LoaderErrorKind {
    /// The file is not in W8 Bytecode format.
    FileIsNotInW8BytecodeFormat {
        /// The full reason of the error.
        reason: String,
    },

    /// The file's version does not match the current VM version.
    UnsupportedVersion {
        /// The version the file was compiled with.
        file_version: String,

        /// The current VM version.
        vm_version: String,
    },

    /// An unknown opcode.
    ///
    /// ## Error example
    /// ```text
    /// [0x00] [0x01] [0x37]
    ///               ^^^^^^
    /// ```
    /// The byte `0x37` does not match any known opcode.
    UnknownOpcode { byte: u8 },

    /// An unknown operand tag.
    ///
    /// ## Error example
    /// ```text
    /// [0x05] [0x01] [0xFF] [0x2A]
    ///               ^^^^^^
    /// ```
    /// The byte `0xFF` is not a valid operand tag (`0x00` and `0x01` are allowed).
    UnknownOperandTag { byte: u8 },

    /// Unexpected end of file.
    ///
    /// ## Error example
    /// ```text
    /// [0x07] [0x02]
    ///        ^^^^^^
    /// ```
    /// There are not enough bytes for a full instruction: the `IADD` opcode expects
    /// 3 operands, but the file ended earlier.
    UnexpectedEndOfFile {
        /// How many more bytes are needed.
        needed: usize,

        /// How many bytes remain.
        remaining: usize,
    },
}

impl Display for LoaderErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileIsNotInW8BytecodeFormat { reason } => {
                write!(f, "the file is not in W8 Bytecode format: {reason}")
            }
            Self::UnsupportedVersion {
                file_version,
                vm_version,
            } => write!(
                f,
                "the file was compiled with W8 version {file_version}, but the current VM version is {vm_version}"
            ),
            Self::UnknownOpcode { byte } => write!(f, "unknown opcode byte: {byte}"),
            Self::UnknownOperandTag { byte } => write!(f, "unknown operand tag: {byte}"),
            Self::UnexpectedEndOfFile { needed, remaining } => write!(
                f,
                "unexpected end of file: needed {needed} more bytes, but only {remaining} remain"
            ),
        }
    }
}

#[derive(Debug)]
pub struct LoaderError {
    pub kind: LoaderErrorKind,
}

impl LoaderError {
    pub fn new(kind: LoaderErrorKind) -> Self {
        Self { kind }
    }
}

impl Display for LoaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}
