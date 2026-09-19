// wdasm/src/parser/err.rs
//
//! Syntax analysis errors.
use std::fmt::{self, Display, Formatter};

use crate::position::Position;

/// Kinds of errors that the parser can detect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserErrorKind {
    /// The colon is missing after the label name: `name:`.
    ExpectedLabelColon,

    /// The comma between operands is missing.
    ExpectedComma,

    /// An unexpected token was encountered.
    UnexpectedToken {
        /// What was expected (for example, `"end of statement"`).
        expected: &'static str,
        /// What was encountered (`Debug` representation of the token).
        got: String,
    },

    /// The instruction has an incorrect number of operands.
    IncorrectNumberOfOperands {
        /// How many operands the opcode expects.
        expected: u8,
        /// How many operands were encountered.
        got: u8,
    },

    /// An operand that must be a register (destination) is not a register.
    ExpectedRegisterOperand {
        /// What was encountered.
        got: String,
    },

    /// A directive name was expected after the dot.
    ExpectedDirectiveName,

    /// An unknown directive was encountered.
    UnknownDirective {
        /// Directive name.
        name: String,
    },

    /// The directive did not receive an operand.
    ExpectedDirectiveOperand,

    /// The directive has an operand of the wrong kind.
    UnexpectedDirectiveOperand {
        /// What was encountered.
        got: String,
    },

    /// The value of the `.align` directive is not a positive integer.
    InvalidAlignBytes {
        /// The encountered value.
        value: i64,
    },

    /// A version part of the `.minversion` directive is out of the `u16` range.
    MinVersionOutOfRange {
        /// The declared version.
        version: String,
    },

    /// The version of the `.minversion` directive is newer than the current W8 version.
    MinVersionTooNew {
        /// The declared version.
        version: String,
        /// The current W8 version.
        current: String,
    },
}

impl Display for ParserErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedLabelColon => write!(f, "expected ':' after the label name"),
            Self::ExpectedComma => write!(f, "expected ',' between operands"),
            Self::UnexpectedToken { expected, got } => {
                write!(f, "expected {expected}, got {got}")
            }
            Self::IncorrectNumberOfOperands { expected, got } => {
                write!(
                    f,
                    "incorrect number of operands: expected {expected}, got {got}"
                )
            }
            Self::ExpectedRegisterOperand { got } => {
                write!(f, "the operand must be a register, got {got}")
            }
            Self::ExpectedDirectiveName => write!(f, "expected a directive name after '.'"),
            Self::UnknownDirective { name } => write!(f, "unknown directive '.{name}'"),
            Self::ExpectedDirectiveOperand => write!(f, "expected an operand after the directive"),
            Self::UnexpectedDirectiveOperand { got } => {
                write!(f, "unexpected operand for the directive, got {got}")
            }
            Self::InvalidAlignBytes { value } => {
                write!(
                    f,
                    "the '.align' value must be a positive integer, got {value}"
                )
            }
            Self::MinVersionOutOfRange { version } => {
                write!(
                    f,
                    "the '.minversion' parts must fit into u16, got {version}"
                )
            }
            Self::MinVersionTooNew { version, current } => {
                write!(
                    f,
                    "the '.minversion' version {version} is newer than the current W8 version {current}"
                )
            }
        }
    }
}

/// Syntax analysis error together with a position in the source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserError {
    /// Error kind.
    pub kind: ParserErrorKind,

    /// Error position in the source code.
    pub position: Position,
}

impl ParserError {
    /// Creates a syntax analysis error.
    pub fn new(kind: ParserErrorKind, position: Position) -> Self {
        Self { kind, position }
    }
}

impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}
