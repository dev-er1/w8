// wdasm/src/lexer/directive.rs
//
//! W8 Assembler directives.
//!
//! A directive is a `.name` word at the start of a statement: it changes
//! how the assembler builds the program instead of adding an instruction.
use w8_core::isa::register::Register;

use crate::str_pool::StrId;

/// Assembler directive.
#[derive(Debug, Clone, Copy)]
pub enum Directive {
    /// Alignment: `.align <bytes>`.
    ///
    /// After the directive, the number of program instructions becomes a
    /// multiple of `bytes`: the missing positions are filled with `NOP`.
    Align {
        /// The multiple by which the instruction stream is aligned.
        ///
        /// The lexer accepts only positive integer values.
        bytes: u64,
    },

    /// Minimum W8 version: `.minversion <major>.<minor>.<patch>`.
    ///
    /// Written into the header of the `.wb` file instead of the current W8 version.
    MinVersion { major: u16, minor: u16, patch: u16 },

    /// Compile-time constant: `.define <name>, <value>`.
    ///
    /// The constant name can be used in instruction operands —
    /// the assembler will substitute the value at compile time. The
    /// directive does not get into the bytecode.
    Define { name: StrId, value: DefineValue },

    /// File-local compile-time constant: `.ldefine <name>, <value>`.
    ///
    /// Same as [`Directive::Define`], but the constant is visible only
    /// within the file where it is declared.
    LDefine {
        name: StrId,
        /// The value: a number or a register alias.
        value: DefineValue,
    },
}

/// A value of the `.define` directive.
#[derive(Debug, Clone, Copy)]
pub enum DefineValue {
    Number(u64),

    /// A register alias (for example, `.define RAX, R0`).
    Register(Register),
}
