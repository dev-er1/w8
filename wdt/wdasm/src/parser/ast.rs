// wdasm/src/parser/ast.rs
//
//! AST definition.
use w8_core::isa::{opcode::OperationCode, register::Register};

use crate::{lexer::directive::Directive, position::Position, str_pool::StrId};

/// Abstract syntax tree of a program in W8 Assembly.
#[derive(Debug, Clone)]
pub struct AST {
    pub program: Vec<Statement>,
}

impl AST {
    pub fn new() -> Self {
        Self {
            program: Vec::new(),
        }
    }

    pub fn with_program(program: Vec<Statement>) -> Self {
        Self { program }
    }
}

impl Default for AST {
    fn default() -> Self {
        Self::new()
    }
}

/// One AST expression, a label, an instruction or a directive.
#[derive(Debug, Clone)]
pub enum Statement {
    /// Label declaration: `name:` or `*name:`.
    Label {
        position: Position,
        name: StrId,

        /// Whether the label is local — scoped to the
        /// nearest preceding global label.
        local: bool,
    },

    /// Instruction.
    Instruction {
        position: Position,
        instruction: Instr,
    },

    /// Assembler directive.
    Directive {
        position: Position,
        directive: Directive,
    },
}

/// Instruction operand inside the AST.
///
/// Besides bytecode operands (register, immediate) an operand can
/// reference a label. The specific label offset is computed by the
/// code generator after parsing.
#[derive(Debug, Clone, Copy)]
pub enum Operand {
    Register(Register),
    Immediate(u64),
    Label(StrId),
    LocalLabel(StrId),
}

/// W8 instruction in the AST.
#[derive(Debug, Clone, Copy)]
pub struct Instr {
    pub opcode: OperationCode,
    pub operand1: Option<Operand>,
    pub operand2: Option<Operand>,
    pub operand3: Option<Operand>,
}

impl Instr {
    /// Returns the number of operands.
    pub fn operand_count(&self) -> usize {
        [self.operand1, self.operand2, self.operand3]
            .into_iter()
            .flatten()
            .count()
    }
}
