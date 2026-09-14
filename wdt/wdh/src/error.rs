// wdh/src/error.rs
use std::{
    fmt::{self, Display, Formatter},
    io,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    /// The service with this number is not in the host's service table.
    UnknownService { got: u64 },

    /// The size of the arguments block does not match the service ABI.
    InvalidArgsSize {
        service: u64,

        /// The size required by the service ABI.
        expected: u64,

        /// The size from the `VMCALL`.
        got: u64,
    },

    /// The register number from the arguments does not exist.
    InvalidRegister {
        /// The register number from the arguments.
        got: u8,
    },

    /// The output of the service failed (the write to the sink or the
    /// flush of the buffer returned an error).
    WriteFailed { kind: io::ErrorKind },

    /// The encoding byte of the arguments is not in the encoding table.
    UnknownEncoding { got: u8 },

    /// A byte of the declared ASCII string is outside the ASCII range.
    InvalidASCII {
        /// The offset of the first offending byte in the string data.
        offset: usize,
    },

    /// The string bytes are not valid UTF-8.
    InvalidUTF8 { offset: usize },

    /// The string bytes are not valid UTF-16.
    InvalidUTF16 { offset: usize },

    /// The stream byte of the arguments is not in the stream table.
    UnknownStream { got: u8 },

    /// The stream cannot be written to (stdin).
    UnwritableStream { got: u8 },
}

impl Display for HostError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownService { got } => {
                write!(f, "unknown service: {got}")
            }
            Self::InvalidArgsSize {
                service,
                expected,
                got,
            } => {
                let s = if expected != &1 {
                    "s".to_string()
                } else {
                    String::new()
                };
                write!(
                    f,
                    "invalid arguments size for service {service}: expected {expected} byte{}, got {got}",
                    s
                )
            }
            Self::InvalidRegister { got } => write!(
                f,
                "invalid register number {got}: the registers are numbered 0..255"
            ),
            Self::WriteFailed { kind } => {
                write!(f, "write failed: {kind:?}")
            }
            Self::UnknownEncoding { got } => {
                write!(f, "unknown encoding byte {got}")
            }
            Self::InvalidASCII { offset } => {
                write!(f, "invalid ASCII string: a byte >= 0x80 at offset {offset}")
            }
            Self::InvalidUTF8 { offset } => {
                write!(
                    f,
                    "invalid UTF-8 string: the sequence at offset {offset} is not valid"
                )
            }
            Self::InvalidUTF16 { offset } => {
                write!(
                    f,
                    "invalid UTF-16 string: the sequence at offset {offset} is not valid"
                )
            }
            Self::UnknownStream { got } => {
                write!(f, "unknown stream byte {got}")
            }
            Self::UnwritableStream { got } => {
                write!(f, "the stream byte {got} cannot be written to")
            }
        }
    }
}

impl std::error::Error for HostError {}
