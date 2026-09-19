//! # Machine code generator
//!
//! This module contains the machine code generator (hereinafter simply
//! “codegen”) used by the W8 JIT compiler.
//!
//! ## What is the “codegen”
//!
//! The codegen is the part of the W8 JIT compiler responsible for generating
//! native machine code from W8 bytecode.
//!
//! Unlike a generic compiler backend, the codegen is tightly coupled to the
//! W8 virtual machine and its execution model.
//!
//! The generated machine code is then executed directly by the JIT compiler.
//!
//! ## Architecture-specific code generation
//!
//! Machine code is architecture-dependent. The [`arch`] module contains the
//! implementations required to generate code for supported target
//! architectures.
pub mod arch;

use crate::vm::WVM;

/// Native machine code generator for the W8 JIT compiler.
///
/// `JITCodegen` contains the runtime information required by the code
/// generator to produce machine code that can interact with the state of an
/// W8 instance.
///
/// All VM state is reached through raw pointers: no borrows are stored, so
/// nothing here aliases the `&mut WVM` a host call receives. Every pointer
/// must stay valid (the VM must not move, storage must not be freed) from
/// construction until the generated code has finished executing.
pub struct JITCodegen {
    /// A pointer to the first register in the W8 register file.
    registers: *mut u64,

    /// A pointer to the VM call stack (`CALL` pushes here, `RET` pops).
    ///
    /// The generated machine code reaches the stack through `extern "C"`
    /// trampolines that receive this pointer. The `Vec` itself must outlive
    /// the code generator.
    call_stack: *mut Vec<usize>,

    /// A pointer to the whole VM, handed to the `VMCALL` trampoline.
    ///
    /// The host interface takes `&mut WVM`, so only a pointer to the entire
    /// machine lets generated code reach it. Same validity rules as
    /// [`JITCodegen::call_stack`].
    wvm: *mut WVM,
}

/// The outgoing data of a `VMCALL` trampoline call.
///
/// Lives in normally writable memory owned by [`JIT`](crate::vm::jit::JIT)
/// (never in the executable mapping, which is read-execute). The trampoline
/// fills it; [`JIT::run`](crate::vm::jit::JIT::run) reads it after execution.
pub struct VmcallOut {
    /// Outcome code (`jit::status`, e.g. continue, exit, or error).
    pub status: u64,

    /// First detail of an error (service number, or offending address).
    pub info0: u64,

    /// Second detail of an error (memory length for a bad address).
    pub info1: u64,

    /// Base address of the VM memory buffer, refreshed by the trampoline
    /// after every host call (the host may replace the buffer).
    pub membase: u64,
}

impl VmcallOut {
    pub const fn new() -> Self {
        Self {
            status: 0,
            info0: 0,
            info1: 0,
            membase: 0,
        }
    }

    /// Execution finished without a runtime failure (`RET` never fires this
    /// explicitly; `VMCALL`-Continue falls through with it).
    pub const OK: u64 = 0;

    /// `RET` executed with an empty call stack.
    pub const EMPTY_CALL_STACK: u64 = 1;

    /// `VMCALL` host asked to terminate the VM (exit code already stored).
    pub const VMCALL_EXIT: u64 = 2;

    /// `VMCALL` with no dispatcher (`info0` holds the service number).
    pub const UNKNOWN_SERVICE: u64 = 3;

    /// `VMCALL` arguments block outside memory (`info0` and `info1` hold the
    /// address and the memory length).
    pub const INVALID_ADDRESS: u64 = 4;

    /// 64-bit division or remainder with a zero divisor (reported by the
    /// x86 division helpers).
    pub const DIVISION_BY_ZERO: u64 = 5;
}

impl Default for VmcallOut {
    fn default() -> Self {
        Self::new()
    }
}

/// The output of [`JITCodegen::generate`].
pub struct GeneratedCode {
    /// Machine code bytes followed by an optional address table for dynamic
    /// jumps, placed after the final `ret`.
    pub bytes: Vec<u8>,

    /// Whether execution can report a runtime failure into the JIT status
    /// cell (`RET` on an empty call stack writes a nonzero code there before
    /// returning). If true, [`JIT::run`] checks the cell after execution.
    ///
    /// [`JIT::run`]: crate::vm::jit::JIT::run
    pub has_status: bool,
}

/// Absolute addresses the code generator embeds into machine code, beyond
/// the VM state it already knows.
///
/// The JIT memory mapping never moves, and the cells owned by
/// [`JIT`](crate::vm::jit::JIT) live in normally writable memory.
pub struct GenContext {
    /// Base address of the JIT memory region receiving the code.
    pub code_base: u64,

    /// Address of the whole VM, passed to the `VMCALL` trampoline.
    pub wvm: u64,

    /// Address of the [`VmcallOut`] block for the `VMCALL` trampoline.
    pub out: u64,

    /// Address of the memory-base cell (`VmcallOut::membase`) that
    /// `LOAD*` and `STORE*` read instead of a baked-in base.
    pub membase_addr: u64,

    /// Address of the 8-byte scratch cell for the `RET` trampoline.
    pub scratch_addr: u64,
}

impl JITCodegen {
    pub fn new(registers: *mut u64, call_stack: *mut Vec<usize>, wvm: *mut WVM) -> Self {
        Self {
            registers,
            call_stack,
            wvm,
        }
    }

    /// Returns the W8 pointer handed to the `VMCALL` trampoline.
    pub(crate) fn w8_ptr(&self) -> *mut WVM {
        self.wvm
    }

    /// Returns the current base address of the VM memory buffer.
    ///
    /// Read through the machine pointer on every call, so a host that
    /// replaced the buffer is always observed.
    pub(crate) fn memory_base(&self) -> u64 {
        // SAFETY: same validity contract as the `wvm` field.
        unsafe { (*self.wvm).memory.as_slice().as_ptr() as u64 }
    }

    /// Returns a mutable pointer to the register at the given index.
    ///
    /// Each register occupies 8 bytes (`u64`) in the register file, so the
    /// register's address is calculated by applying the register index as an
    /// offset from the first register.
    pub(crate) fn register(&self, index: usize) -> *mut u64 {
        assert!(index < 255, "register index out of bounds");

        unsafe { self.registers.add(index) }
    }
}
