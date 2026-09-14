// wdasm/src/lexer/token.rs
use w8_core::isa::{opcode::OperationCode, register::Register};

use crate::{lexer::directive::Directive, position::Position, str_pool::StrId};

#[derive(Debug, Clone)]
pub enum TokenKind {
    Mnemonic(OperationCode),
    Register(Register),
    Directive(Directive),
    Integer(i64),
    Float(f64),

    /// Version — three integers separated by dots (`0.2.0`).
    Version {
        major: u32,
        minor: u32,
        patch: u32,
    },

    /// Identifier.
    Ident(StrId),
    LocalLabel(StrId),

    Comma,
    Colon,
    Dot,
    OpeningSquareBracket,
    EndingSquareBracket,
    Plus,
    Minus,
    Asterisk,
    Newline,
    End,
}

impl TokenKind {
    /// Needed only for the convenient creation of [`TokenKind`].
    pub fn tokenkind(self) -> Self {
        self
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub position: Position,
    pub kind: TokenKind,
}

impl Token {
    pub fn new(position: Position, kind: TokenKind) -> Self {
        Self { position, kind }
    }
}
