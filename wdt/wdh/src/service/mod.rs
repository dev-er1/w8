//! The service table of the `wdh` host.
//!
//! The ABI of the services is described in `wdh/VMCall-ABI/`.
pub mod encoding;
pub mod exit;
pub mod is_in_terminal;
pub mod stream;
pub mod terminal_size;
pub mod write;

/// ABI: `VMCall-ABI/English/Service-ABI/Exit-ABI.ru.md`.
pub const SERVICE_EXIT: u64 = 0;

/// ABI: `VMCall-ABI/English/Service-ABI/Write-ABI.ru.md`.
pub const SERVICE_WRITE: u64 = 1;

/// ABI: `VMCall-ABI/English/Service-ABI/IsInTerminal-ABI.ru.md`.
pub const SERVICE_IS_IN_TERMINAL: u64 = 2;

/// ABI: `VMCall-ABI/English/Service-ABI/TerminalSize-ABI.ru.md`.
pub const SERVICE_TERMINAL_SIZE: u64 = 3;

pub const MAX_REGISTER_INDEX: u8 = 254;
