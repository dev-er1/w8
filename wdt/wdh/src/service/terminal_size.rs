// wdh/src/service/terminal_size.rs
//
//! The `TerminalSize` service: returns the size of a terminal in
//! characters.
//!
//! ABI (see `wdh/VMCall-ABI/English/Service-ABI/TerminalSize-ABI.md`):
//!
//! ```text
//! VMCALL 3, <address>, 3
//! ```
//!
//! The block of arguments is **3 bytes**: the stream selector and the
//! numbers of two registers.
//!
//! The host writes the terminal size in characters into the registers:
//! the width into `cols`, the height into `rows`. If the stream has no
//! terminal, both registers are set to `0`. The execution continues.
use std::io;

use w8_core::isa::register::Register;

use crate::{error::HostError, service::MAX_REGISTER_INDEX};

use super::stream::Stream;

/// The size of the arguments block of the `TerminalSize` service (in bytes).
pub const ARGS_SIZE: u64 = 3;

pub fn parse_args(args: &[u8]) -> Result<(Stream, Register, Register), HostError> {
    if args.len() != ARGS_SIZE as usize {
        return Err(HostError::InvalidArgsSize {
            service: crate::service::SERVICE_TERMINAL_SIZE,
            expected: ARGS_SIZE,
            got: args.len() as u64,
        });
    }

    let stream = Stream::from_byte(args[0])?;

    let cols = args[1];
    if cols > MAX_REGISTER_INDEX {
        return Err(HostError::InvalidRegister { got: cols });
    }

    let rows = args[2];
    if rows > MAX_REGISTER_INDEX {
        return Err(HostError::InvalidRegister { got: rows });
    }

    Ok((stream, Register(cols), Register(rows)))
}

pub fn terminal_size(stream: Stream) -> (u64, u64) {
    let size = match stream {
        Stream::Stdin => ::terminal_size::terminal_size_of(io::stdin()),
        Stream::Stdout => ::terminal_size::terminal_size_of(io::stdout()),
        Stream::Stderr => ::terminal_size::terminal_size_of(io::stderr()),
    };

    match size {
        Some((::terminal_size::Width(w), ::terminal_size::Height(h))) => {
            (u64::from(w), u64::from(h))
        }
        None => (0, 0),
    }
}
