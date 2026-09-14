// wdasm/src/codegen/err.rs
//
//! Code generation errors.
use std::fmt::{self, Display, Formatter};

use crate::position::Position;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenErrorKind {
    /// A label is declared more than once.
    DuplicateLabel { name: String },

    /// A reference to a nonexistent label.
    UndefinedLabel { name: String },

    /// A local label (`*name:`) is declared outside any global label scope.
    LocalLabelWithoutScope { name: String },
}

impl Display for CodegenErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateLabel { name } => write!(f, "duplicate label '{name}'"),
            Self::UndefinedLabel { name } => write!(f, "undefined label '{name}'"),
            Self::LocalLabelWithoutScope { name } => {
                write!(f, "local label '{name}' is outside any global label scope")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenError {
    pub kind: CodegenErrorKind,
    pub position: Position,
}

impl CodegenError {
    pub fn new(kind: CodegenErrorKind, position: Position) -> Self {
        Self { kind, position }
    }
}

impl Display for CodegenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}
