//! # W8's JIT compiler
//!
//! This module contains the W8 bytecode JIT compiler.
//!
//! ## What is the “JIT compiler”
//!
//! JIT (Just-In-Time) compiler is a compiler that compiles
//! some code into machine code and executes it “on the fly”.
//!
//! ## Why is JIT compiler needed in W8
//!
//! W8 bytecode can be executed directly by an interpreter, but interpreting
//! every instruction introduces overhead. A JIT compiler can identify frequently
//! executed bytecode and compile it into native machine code.
//!
//! ## Procedure
//!
//! 1. Allocate memory.
//! 2. Compile bytecode into machine code.
//! 3. Write machine code into memory.
//! 5. Change memory protection to RX.
//! 6. Execute machine code in memory.
//!
//! ## JIT components
//!
//! The JIT compiler consists of two main components:
//! - [`codegen`] — translates W8 bytecode into native machine code;
//! - [`memory`] — allocates and manages the memory used to store generated
//!   machine code and controls its memory protection.
//!
//! The [`JIT`] combines these components into a single compilation and
//! execution pipeline.
//!
//! ## Memory protection
//!
//! JIT memory is initially allocated with read and write permissions so that
//! generated machine code can be written into it. Before execution, the
//! memory protection is changed to read and execute permissions.
//!
//! ## W^X
//!
//! The JIT follows the W^X (Write XOR Execute) principle: memory used for
//! generated machine code is never writable and executable at the same time.
//!
//! JIT memory is initially writable (RW) so that machine code can be
//! generated and written into it. Before execution, its protection is changed
//! to readable and executable.
//!
//! ## What is a “trampoline”
//!
//! A trampoline is a small `extern "C"` function compiled into the host
//! binary that generated machine code `call`s to interact with VM state it
//! cannot touch itself (the call stack, the whole `WVM`, the `VMCALL` out
//! block). It receives raw pointers embedded at codegen time, runs the Rust
//! logic, and returns control to the generated code.
pub mod codegen;
pub mod memory;

use crate::{
    isa::instruction::Instruction,
    vm::{
        VMCallDecision, WVM,
        err::{VMError, VMErrorKind},
        jit::{
            codegen::{GenContext, JITCodegen, VmcallOut},
            memory::{JITMemory, MemoryProtection},
        },
    },
};

pub struct JIT {
    codegen: JITCodegen,
    memory: JITMemory,

    /// Outgoing data of `VMCALL` trampoline calls (see [`VmcallOut`]).
    ///
    /// Writable normal memory on purpose: the executable mapping is
    /// read-execute after [`MemoryProtection`] is applied, so everything
    /// the generated code writes must live outside it (W^X).
    out: VmcallOut,

    /// Scratch cell for the `RET` trampoline (see [`GenContext`]).
    scratch: usize,
}

impl JIT {
    pub fn new(
        registers: *mut u64,
        call_stack: *mut Vec<usize>,
        wvm: *mut WVM,
        jit_memory_size: usize,
    ) -> Result<Self, VMError> {
        Ok(Self {
            codegen: JITCodegen::new(registers, call_stack, wvm),
            memory: JITMemory::malloc(jit_memory_size)?,
            out: VmcallOut::new(),
            scratch: 0,
        })
    }

    pub fn run(&mut self, instructions: &[Instruction]) -> Result<(), VMError> {
        // 1. Compile instructions.
        self.out = VmcallOut::new();
        self.scratch = 0;
        self.out.membase = self.codegen.memory_base();
        let ctx = GenContext {
            code_base: self.memory.base_address(),
            wvm: self.codegen.w8_ptr() as u64,
            out: &mut self.out as *mut VmcallOut as u64,
            membase_addr: &mut self.out.membase as *mut u64 as u64,
            scratch_addr: &mut self.scratch as *mut usize as u64,
        };
        let generated = self.codegen.generate(instructions, &ctx)?;

        // 2. Write compiled code to memory.
        self.memory.write(0, &generated.bytes)?;

        // 3. Make memory RX.
        let rx = MemoryProtection::READ | MemoryProtection::EXECUTE;
        self.memory.protect(rx)?;

        // 4. Execute code.
        self.memory.execute()?;

        // 5. Translate the runtime status cell, if the program has
        // failure or exit paths.
        if generated.has_status {
            match self.out.status {
                VmcallOut::OK => Ok(()),
                VmcallOut::EMPTY_CALL_STACK => Err(VMError::new(VMErrorKind::EmptyCallStack)),
                VmcallOut::VMCALL_EXIT => Ok(()),
                VmcallOut::UNKNOWN_SERVICE => {
                    Err(VMError::new(VMErrorKind::UnknownVMCallService {
                        service: self.out.info0,
                    }))
                }
                VmcallOut::INVALID_ADDRESS => Err(VMError::new(VMErrorKind::InvalidAddress {
                    got: self.out.info0 as usize,
                    memory_length: self.out.info1 as usize,
                })),
                _ => unreachable!("unknown JIT runtime status"),
            }
        } else {
            Ok(())
        }
    }
}

impl WVM {
    pub fn jit_compile(&mut self, jit_memory_size: usize) -> Result<(), VMError> {
        let mut jit = JIT::new(
            self.registers.as_mut_ptr(),
            &mut self.call_stack as *mut Vec<usize>,
            std::ptr::addr_of_mut!(*self),
            jit_memory_size,
        )?;
        jit.run(&self.program)
    }

    pub fn jit_compile_with(
        &mut self,
        jit_memory_size: usize,
        dispatch: impl FnMut(&mut WVM, u64, u64, u64) -> VMCallDecision + 'static,
    ) -> Result<(), VMError> {
        self.dispatch = Some(Box::new(dispatch));
        self.jit_compile(jit_memory_size)
    }
}
