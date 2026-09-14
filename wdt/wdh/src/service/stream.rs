// wdh/src/service/stream.rs
//
//! The standard streams of the host process.
use std::io::{self, IsTerminal};

use crate::error::HostError;

pub const STREAM_STDIN: u8 = 0;

pub const STREAM_STDOUT: u8 = 1;

pub const STREAM_STDERR: u8 = 2;

/// A standard stream of the host process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Stdin,
    Stdout,
    Stderr,
}

impl Stream {
    pub fn from_byte(byte: u8) -> Result<Self, HostError> {
        match byte {
            STREAM_STDIN => Ok(Self::Stdin),
            STREAM_STDOUT => Ok(Self::Stdout),
            STREAM_STDERR => Ok(Self::Stderr),
            _ => Err(HostError::UnknownStream { got: byte }),
        }
    }

    pub fn is_terminal(&self) -> bool {
        match self {
            Self::Stdin => io::stdin().is_terminal(),
            Self::Stdout => io::stdout().is_terminal(),
            Self::Stderr => io::stderr().is_terminal(),
        }
    }
}
