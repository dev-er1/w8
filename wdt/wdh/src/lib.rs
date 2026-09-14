//! # `wdh` — W8 Default Host
//!
//! The default host of the W8 virtual machine.
//!
//! The guest calls the host with `VMCALL <service>, <address>, <size>`:
//! the arguments block lies in the VM memory, [`WDH::dispatch`] reads it
//! and returns what the VM should do — continue execution or terminate
//! with an exit code.
//!
//! The service ABI is described in `wdh/VMCall-ABI/`.
pub mod error;
pub mod observer;
pub mod service;

use w8_core::vm::WVM;

use crate::{error::HostError, observer::Observer};

/// What the host tells the VM to do after a `VMCALL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostDecision {
    /// Continue execution from the next instruction.
    Continue,

    /// Terminate the VM with the given exit code.
    Exit { code: u64 },
}

#[derive(Default)]
pub struct WDH {
    observer: Option<Box<dyn Observer>>,
}

impl WDH {
    pub fn new() -> Self {
        Self::default()
    }

    /// Installs an observer that will be notified on every service call.
    pub fn with_observer(mut self, observer: impl Observer + 'static) -> Self {
        self.observer = Some(Box::new(observer));
        self
    }

    /// Returns a reference to the installed observer, if any.
    pub fn observer(&self) -> Option<&dyn Observer> {
        self.observer.as_deref()
    }

    /// Dispatches the `VMCALL` with the given number and the block of
    /// arguments from the VM memory.
    ///
    /// The VM guarantees that the block is inside the memory bounds.
    /// The services may modify the VM state: for example, `IsInTerminal`
    /// writes its result into a register.
    pub fn dispatch(
        &mut self,
        vm: &mut WVM,
        service: u64,
        args: &[u8],
    ) -> Result<HostDecision, HostError> {
        if let Some(observer) = &mut self.observer {
            observer.on_vmcall(service, args);
        }

        match service {
            service::SERVICE_EXIT => {
                let register = service::exit::parse_args(args)?;
                Ok(HostDecision::Exit {
                    code: vm.registers[register],
                })
            }
            service::SERVICE_WRITE => {
                service::write::write(args)?;
                Ok(HostDecision::Continue)
            }
            service::SERVICE_IS_IN_TERMINAL => {
                let (register, stream) = service::is_in_terminal::parse_args(args)?;
                vm.registers[register] = u64::from(stream.is_terminal());
                Ok(HostDecision::Continue)
            }
            service::SERVICE_TERMINAL_SIZE => {
                let (stream, cols, rows) = service::terminal_size::parse_args(args)?;
                let (width, height) = service::terminal_size::terminal_size(stream);
                vm.registers[cols] = width;
                vm.registers[rows] = height;
                Ok(HostDecision::Continue)
            }
            _ => Err(HostError::UnknownService { got: service }),
        }
    }
}
