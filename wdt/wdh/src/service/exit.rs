// wdh/src/service/exit.rs
//
//! The `Exit` service: terminates the VM with an exit code.
//!
//! ABI (see `wdh/VMCall-ABI/English/Service-ABI/Exit-ABI.md`):
//!
//! ```text
//! VMCALL 0, <address>, 1
//! ```
//!
//! The block of arguments is exactly **1 byte**: the number of the
//! register that holds the exit code. The exit code is read from that
//! register at the moment of the call.
use w8_core::isa::register::Register;

use crate::{
    error::HostError,
    service::{MAX_REGISTER_INDEX, SERVICE_EXIT},
};

pub const ARGS_SIZE: u64 = 1;

pub fn parse_args(args: &[u8]) -> Result<Register, HostError> {
    if args.len() != ARGS_SIZE as usize {
        return Err(HostError::InvalidArgsSize {
            service: SERVICE_EXIT,
            expected: ARGS_SIZE,
            got: args.len() as u64,
        });
    }

    let index = args[0];
    if index > MAX_REGISTER_INDEX {
        return Err(HostError::InvalidRegister { got: index });
    }

    Ok(Register(index))
}
