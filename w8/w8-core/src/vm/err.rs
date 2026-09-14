// w8-core/src/vm/err.rs
use std::fmt::{self, Display, Formatter};

use crate::{isa::operand::OperandKind, vm::jit::memory::MemoryProtection};

#[derive(Debug)]
pub enum VMErrorKind {
    /// Incorrect number of operands.
    ///
    /// ## Example error
    /// ```text
    /// MOVE R1, R2, R3
    ///              ^^
    /// ```
    /// The `MOVE` instruction can use only 2 operands.
    IncorrectNumberOfOperands { expected: u8, got: u8 },

    /// Incorrect operand type.
    ///
    /// ## Example error
    /// ```text
    /// MOVE 0, R0
    ///      ^
    /// ```
    /// In the `MOVE` instruction, the first operand must always be a register.
    IncorrectTypeOfOperand {
        expected: OperandKind,
        got: OperandKind,
    },

    /// A register number out of the 0-254 range (W8 has 255 registers).
    InvalidRegister {
        /// The register number from the instruction.
        got: u8,
    },

    /// Invalid address.
    ///
    /// The error is raised if an instruction (for example, `LOAD8`) tries to get
    /// a value from memory at a nonexistent address.
    InvalidAddress {
        /// The address that was attempted to “look into”.
        got: usize,

        /// The memory length.
        memory_length: usize,
    },

    /// Division by zero.
    DivisionByZero,

    /// The call stack is empty (`RET` without `CALL`).
    EmptyCallStack,

    /// The `VMCALL` dispatcher does not know the service number
    /// (or is absent — the program used `VMCALL` without a host).
    UnknownVMCallService {
        /// The service number from the `VMCALL`.
        service: u64,
    },

    /// [Allocation](crate::vm::jit::memory::JITMemory::malloc()) failed.
    JITAllocationFailed,

    /// JIT memory is out of bounds.
    JITMemoryOutOfBounds,

    /// Failed to change the protection of the JIT memory region.
    JITMemoryProtectionFailed,

    /// Can not [write](crate::vm::jit::memory::JITMemory::write) code in the current page.
    CantWriteCodeInTheCurrentPage(MemoryProtection),
}

impl Display for VMErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncorrectNumberOfOperands { expected, got } => {
                write!(f, "expected {expected} operands, but got {got} operands")
            }
            Self::IncorrectTypeOfOperand { expected, got } => write!(
                f,
                "expected type {}, but got {} type",
                expected.kind(),
                got.kind()
            ),
            Self::InvalidRegister { got } => write!(
                f,
                "invalid register number {got}: the register number must be in the range 0-254"
            ),
            Self::InvalidAddress { got, memory_length } => write!(
                f,
                "memory access out of bounds: address {got} is outside memory (size: {memory_length} bytes)"
            ),
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::EmptyCallStack => write!(f, "empty call stack"),
            Self::UnknownVMCallService { service } => {
                write!(f, "unknown syscall service: {service}")
            }
            Self::JITAllocationFailed => write!(f, "JIT compiler's memory allocation failed"),
            Self::JITMemoryOutOfBounds => write!(f, "JIT compiler's memory out of bounds"),
            Self::JITMemoryProtectionFailed => write!(f, "failed to change JIT memory protection"),
            Self::CantWriteCodeInTheCurrentPage(protection) => write!(
                f,
                "It is not possible to write code on the current page because the current \
                protection is {protection}"
            ),
        }
    }
}

#[derive(Debug)]
pub struct VMError {
    pub kind: VMErrorKind,
}

impl Display for VMError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl VMError {
    pub fn new(kind: VMErrorKind) -> Self {
        Self { kind }
    }
}
