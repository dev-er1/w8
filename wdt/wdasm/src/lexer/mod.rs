//! # Lexer (lexical analysis)
//!
//! A lexer is a program that turns code in any language
//! into a stream of tokens.
//!
//! ## Module contents
//! - [`token`] — the token enumeration and the token structure.
//! - [`err`] — the error enumeration and the structure of a single error.
//! - [`directive`] — the directives that the lexer recognizes.
pub mod directive;
pub mod err;
pub mod token;

use crate::{
    lexer::{
        directive::{DefineValue, Directive},
        err::{LexerError, LexerErrorKind},
        token::{Token, TokenKind},
    },
    position::Position,
    src::SourceCode,
    str_pool::StrPool,
};
use w8_core::isa::{opcode::OperationCode, register::Register};

/// Lexer of the source code.
///
/// Parses the source code one character at a time, collecting the found
/// tokens into [`tokens`](Self::tokens) and errors into [`errors`](Self::errors).
pub struct Lexer<'a> {
    pub src: SourceCode,
    pub str_pool: &'a mut StrPool,
    pub tokens: Vec<Token>,
    pub errors: Vec<LexerError>,

    /// Current byte position in the source code.
    index: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: SourceCode, str_pool: &'a mut StrPool) -> Self {
        Self {
            src,
            str_pool,
            tokens: Vec::new(),
            errors: Vec::new(),
            index: 0,
        }
    }

    pub fn tokenize(&mut self) -> &[Token] {
        loop {
            // Skip whitespace and comments.
            self.skip_trivia();

            if self.index >= self.src.source.len() {
                break;
            }

            self.next_token();
        }

        let pos = Position::new(self.index as u32, self.index as u32);
        self.tokens.push(Token::new(pos, TokenKind::End));

        &self.tokens
    }

    /// Parses a single token starting from the current position.
    fn next_token(&mut self) {
        let start = self.index;
        let first = self.bump().expect("called only if the symbol is present");

        let kind = match first {
            b'\n' => TokenKind::Newline,
            b',' => TokenKind::Comma,
            b':' => TokenKind::Colon,
            b'[' => TokenKind::OpeningSquareBracket,
            b']' => TokenKind::EndingSquareBracket,

            // A star before a letter starts a local label name (*loop).
            b'*' if self.peek().is_some_and(is_ident_start) => return self.lex_local_label(start),
            b'*' => TokenKind::Asterisk,

            // A dot before a digit starts a fractional number.
            b'.' if self.is_digit_peek() => return self.lex_number(start, true),
            b'.' => return self.lex_directive(start),

            // A sign before a digit starts a number with the given sign.
            b'+' | b'-' if self.is_digit_peek() => return self.lex_number(start, false),
            b'+' => TokenKind::Plus,
            b'-' => TokenKind::Minus,

            b'0'..=b'9' => return self.lex_number(start, false),

            b'a'..=b'z' | b'A'..=b'Z' | b'_' => return self.lex_word(start),

            other => {
                self.push_error(start, LexerErrorKind::UnexpectedCharacter(other as char));
                return;
            }
        };

        let pos = Position::new(start as u32, self.index as u32);
        self.tokens.push(Token::new(pos, kind));
    }

    /// Lexes a local label name: `*name`.
    ///
    /// The star has already been consumed into `start`. The name consists
    /// of the same characters as a word (`[A-Za-z0-9_]`) and is interned
    /// into the string pool without the star — the locality is carried by
    /// the token kind.
    fn lex_local_label(&mut self, start: usize) {
        let name_start = self.index;

        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            self.bump();
        }

        let name = self
            .str_pool
            .intern(&self.src.source[name_start..self.index]);
        let pos = Position::new(start as u32, self.index as u32);
        self.tokens
            .push(Token::new(pos, TokenKind::LocalLabel(name)));
    }

    /// Lexes a directive: `.name [operand]`.
    ///
    /// The dot has already been consumed into `start`. If the name is a
    /// known directive and its operand is well-formed, the whole directive
    /// becomes a single [`TokenKind::Directive`] token. Otherwise a
    /// [`TokenKind::Dot`] is emitted and `index` is rewound to the name,
    /// so the name and the operand are lexed as ordinary tokens and the
    /// parser reports the error itself.
    fn lex_directive(&mut self, start: usize) {
        let name_start = self.index;

        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            self.bump();
        }

        // A dot without a name (". MOVE R0"): just the dot token.
        if self.index == name_start {
            let pos = Position::new(start as u32, name_start as u32);
            self.tokens.push(Token::new(pos, TokenKind::Dot));
            return;
        }

        let name = &self.src.source[name_start..self.index];
        let directive = match name.to_ascii_lowercase().as_str() {
            "align" => self.lex_directive_align(),
            "minversion" => self.lex_directive_minversion(),
            "define" => self.lex_directive_define(false),
            "ldefine" => self.lex_directive_define(true),
            _ => None,
        };

        let Some(directive) = directive else {
            self.index = name_start;
            let pos = Position::new(start as u32, name_start as u32);
            self.tokens.push(Token::new(pos, TokenKind::Dot));
            return;
        };

        let pos = Position::new(start as u32, self.index as u32);
        self.tokens
            .push(Token::new(pos, TokenKind::Directive(directive)));
    }

    fn lex_directive_align(&mut self) -> Option<Directive> {
        self.skip_trivia();

        let (number_start, leading_dot) = self.lex_number_start()?;
        self.lex_number(number_start, leading_dot);

        match self.tokens.pop() {
            Some(Token {
                kind: TokenKind::Integer(bytes),
                ..
            }) if bytes >= 0 => Some(Directive::Align {
                bytes: bytes as u64,
            }),
            _ => None,
        }
    }

    fn lex_directive_minversion(&mut self) -> Option<Directive> {
        self.skip_trivia();

        let (number_start, leading_dot) = self.lex_number_start()?;
        self.lex_number(number_start, leading_dot);

        match self.tokens.pop() {
            Some(Token {
                kind:
                    TokenKind::Version {
                        major,
                        minor,
                        patch,
                    },
                ..
            }) if major <= u16::MAX as u32
                && minor <= u16::MAX as u32
                && patch <= u16::MAX as u32 =>
            {
                Some(Directive::MinVersion {
                    major: major as u16,
                    minor: minor as u16,
                    patch: patch as u16,
                })
            }
            _ => None,
        }
    }

    fn lex_directive_define(&mut self, local: bool) -> Option<Directive> {
        self.skip_trivia();

        if !self
            .peek()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        {
            return None;
        }
        let name_start = self.index;
        self.lex_word(name_start);

        let name = match self.tokens.pop() {
            Some(Token {
                kind: TokenKind::Ident(name),
                ..
            }) => name,
            _ => return None,
        };

        self.skip_trivia();
        if self.peek() != Some(b',') {
            return None;
        }
        self.bump();

        self.skip_trivia();

        let value = if self.is_number_start() {
            let (number_start, leading_dot) = self.lex_number_start()?;
            self.lex_number(number_start, leading_dot);

            match self.tokens.pop() {
                Some(Token {
                    kind: TokenKind::Integer(value),
                    ..
                }) => DefineValue::Number(value as u64),
                Some(Token {
                    kind: TokenKind::Float(value),
                    ..
                }) => DefineValue::Number(value.to_bits()),
                _ => return None,
            }
        } else if self
            .peek()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        {
            let word_start = self.index;
            self.lex_word(word_start);

            match self.tokens.pop() {
                Some(Token {
                    kind: TokenKind::Register(register),
                    ..
                }) => DefineValue::Register(register),
                _ => return None,
            }
        } else {
            return None;
        };

        let directive = if local {
            Directive::LDefine { name, value }
        } else {
            Directive::Define { name, value }
        };
        Some(directive)
    }

    fn lex_number_start(&mut self) -> Option<(usize, bool)> {
        if !self.is_number_start() {
            return None;
        }

        let start = self.index;
        let leading_dot = self.peek() == Some(b'.');

        if leading_dot || matches!(self.peek(), Some(b'+') | Some(b'-')) {
            self.bump();
        }

        Some((start, leading_dot))
    }

    fn is_number_start(&self) -> bool {
        match self.peek() {
            Some(b'0'..=b'9') => true,
            Some(b'.' | b'+' | b'-') => self.peek_n(1).is_some_and(|b| b.is_ascii_digit()),
            _ => false,
        }
    }

    /// Skips whitespace (except line breaks) and comments.
    ///
    /// A comment starts with the `;` character and lasts until the end of the line.
    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r') => {
                    self.bump();
                }
                Some(b';') => {
                    // Skip everything up to the line break; the line break itself
                    // is left alone: it will become a Newline token.
                    while let Some(byte) = self.peek() {
                        if byte == b'\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    /// Parses a word: a register, a mnemonic, or an identifier.
    ///
    /// A word starts with a letter or an underscore. The first character
    /// has already been consumed as `start`.
    ///
    /// The word is classified in the following order:
    /// 1. `R0`..`R254` — a register;
    /// 2. a known mnemonic — [`TokenKind::Mnemonic`];
    /// 3. everything else — an identifier.
    fn lex_word(&mut self, start: usize) {
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            self.bump();
        }

        let text = &self.src.source[start..self.index];

        let kind = if is_register_name(text) {
            let number = &text[1..];
            match number.parse::<u16>() {
                Ok(n) if n <= 254 => TokenKind::Register(Register(n as u8)),
                _ => {
                    self.push_error(start, LexerErrorKind::InvalidRegister(text.to_string()));
                    return;
                }
            }
        } else if let Ok(opcode) = text.parse::<OperationCode>() {
            TokenKind::Mnemonic(opcode)
        } else {
            TokenKind::Ident(self.str_pool.intern(text))
        };

        let pos = Position::new(start as u32, self.index as u32);
        self.tokens.push(Token::new(pos, kind));
    }

    fn lex_number(&mut self, start: usize, leading_dot: bool) {
        let mut is_float = leading_dot;

        // Integer part.
        while self.is_digit_peek() {
            self.bump();
        }

        // Fractional part.
        if self.peek() == Some(b'.') && self.peek_n(1).is_some_and(|b| b.is_ascii_digit()) {
            is_float = true;
            self.bump();
            while self.is_digit_peek() {
                self.bump();
            }
        }

        // Version: a second dot with a digit.
        if self.peek() == Some(b'.') && self.peek_n(1).is_some_and(|b| b.is_ascii_digit()) {
            self.bump();
            while self.is_digit_peek() {
                self.bump();
            }

            let text = &self.src.source[start..self.index];

            // A version part must be a non-empty run of digits that fits
            // into `u32`; a malformed literal (an empty part, an overflow)
            // is reported instead of being silently clamped to zero.
            let mut parts = text.split('.').map(|part| {
                if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                part.parse::<u32>().ok()
            });

            let (Some(major), Some(minor), Some(patch)) = (
                parts.next().flatten(),
                parts.next().flatten(),
                parts.next().flatten(),
            ) else {
                self.push_error(start, LexerErrorKind::InvalidVersion(text.to_string()));
                return;
            };

            let pos = Position::new(start as u32, self.index as u32);
            self.tokens.push(Token::new(
                pos,
                TokenKind::Version {
                    major,
                    minor,
                    patch,
                },
            ));
            return;
        }

        // Exponent: “e” or “E” with an optional sign.
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            let mut offset = 1;
            if matches!(self.peek_n(offset), Some(b'+') | Some(b'-')) {
                offset += 1;
            }
            if self.peek_n(offset).is_some_and(|b| b.is_ascii_digit()) {
                is_float = true;
                self.bump();
                if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                    self.bump();
                }
                while self.is_digit_peek() {
                    self.bump();
                }
            }
        }

        let text = &self.src.source[start..self.index];

        let kind = if is_float {
            match text.parse::<f64>() {
                Ok(value) if value.is_finite() => TokenKind::Float(value),
                _ => {
                    self.push_error(start, LexerErrorKind::InvalidFloat(text.to_string()));
                    return;
                }
            }
        } else {
            match text.parse::<i64>() {
                Ok(value) => TokenKind::Integer(value),
                Err(_) => {
                    self.push_error(start, LexerErrorKind::InvalidInteger(text.to_string()));
                    return;
                }
            }
        };

        let pos = Position::new(start as u32, self.index as u32);
        self.tokens.push(Token::new(pos, kind));
    }

    fn push_error(&mut self, start: usize, kind: LexerErrorKind) {
        let pos = Position::new(start as u32, self.index as u32);
        self.errors.push(LexerError::new(kind, pos));
    }

    /// Returns the byte at the current position without moving it.
    fn peek(&self) -> Option<u8> {
        self.src.source.as_bytes().get(self.index).copied()
    }

    /// Returns the byte `offset` bytes ahead of the current position.
    fn peek_n(&self, offset: usize) -> Option<u8> {
        self.src.source.as_bytes().get(self.index + offset).copied()
    }

    /// Returns the byte at the current position and moves the position forward.
    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.index += 1;
        Some(byte)
    }

    /// Whether the current byte is a digit.
    fn is_digit_peek(&self) -> bool {
        matches!(self.peek(), Some(b'0'..=b'9'))
    }
}

/// Whether the byte can start an identifier (a letter or an underscore).
fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

/// Whether the word is a register name: `R` followed by digits.
///
/// For example, `R0` — yes, `R255` — in form yes, but with an invalid
/// number, `r` and `r0x` — no.
fn is_register_name(text: &str) -> bool {
    text.len() > 1
        && matches!(text.as_bytes()[0], b'r' | b'R')
        && text[1..].bytes().all(|b| b.is_ascii_digit())
}
