//! # W8 virtual machine
//!
//! This module defines the **W8** virtual machine and its main components.
//!
//! ## Module contents
//!
//! - [`memory`] — the virtual machine's memory;
//! - [`register_file`] — the register bank;
//! - [`err`] — VM errors;
//! - [`interpreter`] — the instruction interpreter.
pub mod err;
pub mod interpreter;
pub mod memory;
pub mod register_file;

#[cfg(target_arch = "x86_64")]
pub mod jit;

use crate::{
    isa::instruction::Instruction,
    vm::{err::VMError, memory::WVMMemory, register_file::RegisterFile},
};

#[derive(Default, PartialEq, Clone, Copy)]
pub enum ExecuteVariant {
    #[cfg(target_arch = "x86_64")]
    ByJIT,

    #[default]
    ByInterpreter,
    // TODO: make `Hybrid` variant.
}

/// What the host tells the VM to do after a `VMCALL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMCallDecision {
    Continue,
    Exit { code: u64 },
}

/// The host dispatcher of `VMCALL`.
///
/// Receives the `&mut WVM` (so the host can read the registers and the
/// arguments block from memory), the service number and the block of
/// arguments (`address..address + size` in the VM memory — the bounds
/// are checked by the VM before the call). Returns the [`VMCallDecision`].
pub type Dispatcher = Box<dyn FnMut(&mut WVM, u64, u64, u64) -> VMCallDecision>;

/// # W8 virtual machine
///
/// Represents the full state of the virtual machine.
///
/// Contains:
/// - the program executed by the VM;
/// - the memory;
/// - the register file;
/// - the call stack;
/// - the `VMCALL` dispatcher and the resulting exit code.
pub struct WVM {
    pub program: Vec<Instruction>,
    pub memory: WVMMemory,
    pub registers: RegisterFile,
    pub call_stack: Vec<usize>,
    pub executeby: ExecuteVariant,

    /// The host dispatcher of `VMCALL`; `None` — a `VMCALL` in the program
    /// is an error ([`err::VMErrorKind::UnknownVMCallService`]).
    pub dispatch: Option<Dispatcher>,
    pub exit_code: Option<u64>,
}

impl WVM {
    pub fn new(memory_size: usize, executeby: ExecuteVariant) -> Self {
        Self {
            program: Vec::new(),
            memory: WVMMemory::new(memory_size),
            registers: RegisterFile::new(),
            call_stack: Vec::new(),
            executeby,
            dispatch: None,
            exit_code: None,
        }
    }

    pub fn from_program_and_memory(
        program: Vec<Instruction>,
        memory: WVMMemory,
        executeby: ExecuteVariant,
    ) -> Self {
        Self {
            program,
            memory,
            registers: RegisterFile::new(),
            call_stack: Vec::new(),
            executeby,
            dispatch: None,
            exit_code: None,
        }
    }

    pub fn run(&mut self, jit_memory_size: Option<usize>) -> Result<(), VMError> {
        match self.executeby {
            ExecuteVariant::ByInterpreter => self.interpretate(),
            ExecuteVariant::ByJIT => {
                if let Some(jit_memory) = jit_memory_size {
                    self.jit_compile(jit_memory)
                } else {
                    self.jit_compile(64 * 1024)
                }
            }
        }
    }
}
