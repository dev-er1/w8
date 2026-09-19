// w8-core/src/error.rs
//
//! Pretty-printing of errors.
use std::fmt::{self, Display, Formatter};

use crate::{
    isa::{err::ISAError, instruction::Instruction},
    loader::err::LoaderError,
    vm::err::VMError,
};

/// Error kinds across the whole `w8-core` package.
#[derive(Debug)]
pub enum WVMErrorKind {
    ISAError(ISAError),
    VMError(VMError),
    LoaderError(LoaderError),
    IoError(std::io::Error),
}

impl Display for WVMErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ISAError(err) => write!(f, "{err}"),
            Self::VMError(err) => write!(f, "{err}"),
            Self::LoaderError(err) => write!(f, "{err}"),
            Self::IoError(err) => write!(f, "{err}"),
        }
    }
}

#[derive(Debug)]
pub struct WVMError {
    pub kind: WVMErrorKind,

    /// The instruction (optional).
    ///
    /// Needed to show the error.
    ///
    /// ## How the error output looks with this field (example):
    /// ```text
    /// Error: expected type register, but got value type
    ///
    /// --> MOVE 0, R0
    ///     ^^^^^^^^^^
    /// ```
    pub instruction: Option<Instruction>,

    /// Whether ANSI escape sequences are supported.
    /// Needed to print the error with colors when true,
    /// and without colors when false.
    have_ansi: bool,
}

impl WVMError {
    pub fn new(kind: WVMErrorKind, instruction: Option<Instruction>, have_ansi: bool) -> Self {
        Self {
            kind,
            instruction,
            have_ansi,
        }
    }

    pub fn report(&self) {
        if self.have_ansi {
            println!("\x1b[1;31mError\x1b[0m: \x1b[1m{}\x1b[0m.", self.kind);
            println!();
            if let Some(instr) = self.instruction {
                println!("\x1b[36m-->\x1b[0m  {instr}");

                // The number of arrows pointing at the instruction.
                let arrows = "^".repeat(format!("{instr}").len());
                println!("     \x1b[1m{arrows}\x1b[0m");
            }
        } else {
            // The same, but without ANSI escape sequences.
            println!("Error: {}.", self.kind);
            println!();
            if let Some(instr) = self.instruction {
                println!("-->  {instr}");

                let arrows = "^".repeat(format!("{instr}").len());
                println!("     {arrows}");
            }
        }
    }
}
