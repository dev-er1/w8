// wdh/src/observer.rs
//
//! Monitoring of the service calls.
//!
//! The observer is a host-side “parallel” trace of the `VMCALL`s:
//! which services are called and with which arguments.

/// The observer of the service calls.
///
/// An observer is notified by the host ([`WDH::dispatch`](crate::WDH::dispatch))
/// before the service is executed.
pub trait Observer {
    /// Called on every `VMCALL`, before the service runs.
    ///
    /// - `service` — the service number from the `VMCALL`;
    /// - `args` — the block of arguments (inside the VM memory).
    fn on_vmcall(&mut self, service: u64, args: &[u8]);
}

/// The no-op observer: does nothing, costs nothing.
pub struct NoOpObserver;

impl Observer for NoOpObserver {
    fn on_vmcall(&mut self, _service: u64, _args: &[u8]) {}
}
