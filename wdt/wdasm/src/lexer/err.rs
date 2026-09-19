// wdasm/src/lexer/err.rs
use std::fmt::{self, Display, Formatter};

use crate::position::Position;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexerErrorKind {
    /// A character was encountered that is not a part of any token.
    UnexpectedCharacter(char),

    /// Invalid integer literal.
    InvalidInteger(String),

    /// Invalid floating-point number literal.
    InvalidFloat(String),

    /// Invalid register: the number is out of the 0-254 range.
    InvalidRegister(String),

    /// Invalid version literal: a part is empty or does not fit into `u32`.
    InvalidVersion(String),
}

impl Display for LexerErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter(c) => write!(f, "unexpected character: '{c}'"),
            Self::InvalidInteger(text) => {
                write!(f, "invalid integer: '{text}'")
            }
            Self::InvalidFloat(text) => {
                write!(f, "invalid floating-point: '{text}'")
            }
            Self::InvalidRegister(text) => write!(
                f,
                "invalid register '{text}': register number must be in the range 0-254"
            ),
            Self::InvalidVersion(text) => {
                write!(f, "invalid version literal: '{text}'")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    pub kind: LexerErrorKind,
    pub pos: Position,
}

impl LexerError {
    pub fn new(kind: LexerErrorKind, pos: Position) -> Self {
        Self { kind, pos }
    }
}

impl Display for LexerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}
