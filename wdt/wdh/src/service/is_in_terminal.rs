// wdh/src/service/is_in_terminal.rs
//
//! The `IsInTerminal` service: checks whether a standard stream of the
//! host process is attached to a terminal.
//!
//! ABI (see `wdh/VMCall-ABI/English/Service-ABI/IsInTerminal-ABI.md`):
//!
//! ```text
//! VMCALL 2, <address>, 2
//! ```
//!
//! The host writes `1` into the register if the stream is a terminal,
//! `0` otherwise, and continues the execution.
use w8_core::isa::register::Register;

use crate::{error::HostError, service::MAX_REGISTER_INDEX};

use super::stream::Stream;

pub const ARGS_SIZE: u64 = 2;

pub fn parse_args(args: &[u8]) -> Result<(Register, Stream), HostError> {
    if args.len() != ARGS_SIZE as usize {
        return Err(HostError::InvalidArgsSize {
            service: crate::service::SERVICE_IS_IN_TERMINAL,
            expected: ARGS_SIZE,
            got: args.len() as u64,
        });
    }

    let index = args[0];
    if index > MAX_REGISTER_INDEX {
        return Err(HostError::InvalidRegister { got: index });
    }

    Ok((Register(index), Stream::from_byte(args[1])?))
}
