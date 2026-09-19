//! # Parser
//!
//! A parser is a program that turns an array of tokens into
//! an ***abstract syntax tree*** (***AST***).
//!
//! ## Module contents
//! - [`ast`] — the AST;
//! - [`err`] — parser errors.
//!
//! ## Grammar
//!
//! A program consists of lines. Each line is a label, an instruction,
//! a directive, or a combination of the two:
//!
//! ```text
//! program     := statement*
//! statement   := label? (instruction | directive)? eol
//! label       := IDENT ':' | '*' IDENT ':'
//! instruction := MNEMONIC operand (',' operand)*
//! directive   := '.' IDENT [operand | version]
//! operand     := REGISTER | INTEGER | FLOAT | IDENT | '*' IDENT
//! version     := INTEGER '.' INTEGER '.' INTEGER
//! ```
//!
//! A label and an instruction (or a directive) may be on the same line:
//! `main: MOVE R0, 1`, `main: .align 8`. A local label (`*loop:`) is
//! scoped to the nearest preceding global label: a reference to it
//! (`JMP *loop`) is resolved only within that scope.
//!
//! The `.define <name>, <value>` directive declares a compile-time
//! constant: the name must be declared before its use, and every
//! reference in an instruction operand is replaced with the value
//! (a number or a register alias). `.ldefine <name>, <value>` does the
//! same, but the constant is visible only within the file where it is
//! declared — it shadows a `.define` of the same name and does not leak
//! through `#include`.
pub mod ast;
pub mod err;

use std::collections::HashMap;

use w8_core::{W8_VERSION, isa::opcode::OperationCode};

use crate::{
    lexer::{
        directive::{DefineValue, Directive},
        token::{Token, TokenKind},
    },
    position::Position,
    str_pool::{StrId, StrPool},
};

use self::{
    ast::{AST, Instr, Operand, Statement},
    err::{ParserError, ParserErrorKind},
};

/// Parser for W8 Assembly.
///
/// Takes a token stream, builds an [`AST`] from it, and accumulates
/// the found errors in [`errors`](Self::errors).
pub struct Parser<'a> {
    /// The token stream.
    tokens: Vec<Token>,

    /// Index of the current token.
    index: usize,

    /// String pool, from which identifiers are looked up
    /// (needed to recognize directive names).
    str_pool: &'a StrPool,

    /// The built abstract syntax tree.
    ast: AST,

    /// Compile-time constants from `.define <name>, <value>`:
    /// a name is replaced with its value in instruction operands.
    defines: HashMap<StrId, DefineValue>,

    /// File-local compile-time constants from `.ldefine <name>, <value>`,
    /// keyed by `(file, name)`. A local constant shadows a `.define` of
    /// the same name within its file.
    local_defines: HashMap<(u32, StrId), DefineValue>,

    /// File ids by the `global_start` of the segments produced by
    /// preprocessing; empty when the source is a single text (all
    /// tokens are then in file 0).
    files: Vec<(u32, u32)>,

    /// Errors found during parsing.
    pub errors: Vec<ParserError>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, str_pool: &'a StrPool, files: &'a [(u32, u32)]) -> Self {
        Self {
            tokens,
            index: 0,
            str_pool,
            ast: AST::new(),
            defines: HashMap::new(),
            local_defines: HashMap::new(),
            files: files.to_vec(),
            errors: Vec::new(),
        }
    }

    /// Parses the token stream.
    ///
    /// The result is collected into `ast`, errors — into
    /// [`errors`](Self::errors). After an error, parsing continues from the
    /// next line.
    pub fn parse(&mut self) -> &AST {
        loop {
            match self.peek_kind() {
                Some(TokenKind::Newline) => {
                    self.bump();
                }
                Some(TokenKind::End) | None => break,
                Some(TokenKind::Ident(_)) => self.parse_label(),
                Some(TokenKind::LocalLabel(_)) => self.parse_local_label(),
                Some(TokenKind::Mnemonic(_)) => self.parse_instruction(),
                Some(TokenKind::Dot) | Some(TokenKind::Directive(_)) => self.parse_directive(),
                _ => {
                    self.push_error_at_current(ParserErrorKind::UnexpectedToken {
                        expected: "a label, an instruction, or a directive",
                        got: self.current_kind_debug(),
                    });
                    self.skip_to_newline();
                }
            }
        }

        &self.ast
    }

    // ====== Parsing lines ======

    /// Parses a label: `name:` with an optional instruction on the same
    /// line.
    fn parse_label(&mut self) {
        let name = match self.bump_kind() {
            TokenKind::Ident(id) => id,
            _ => unreachable!("called only when the current token is an identifier"),
        };

        if !matches!(self.peek_kind(), Some(TokenKind::Colon)) {
            self.push_error_at_current(ParserErrorKind::ExpectedLabelColon);
            self.skip_to_newline();
            return;
        }

        let colon = self.bump();
        let position = self.tokens[self.index - 2].position.to(colon.position);
        self.ast.program.push(Statement::Label {
            name,
            local: false,
            position,
        });

        match self.peek_kind() {
            Some(TokenKind::Mnemonic(_)) => self.parse_instruction(),
            Some(TokenKind::Dot) | Some(TokenKind::Directive(_)) => self.parse_directive(),
            _ => {}
        }
    }

    /// Parses a local label: `*name:` with an optional instruction on the
    /// same line.
    fn parse_local_label(&mut self) {
        let name = match self.bump_kind() {
            TokenKind::LocalLabel(id) => id,
            _ => unreachable!("called only when the current token is a local label"),
        };

        if !matches!(self.peek_kind(), Some(TokenKind::Colon)) {
            self.push_error_at_current(ParserErrorKind::ExpectedLabelColon);
            self.skip_to_newline();
            return;
        }

        let colon = self.bump();
        let position = self.tokens[self.index - 2].position.to(colon.position);
        self.ast.program.push(Statement::Label {
            name,
            local: true,
            position,
        });

        match self.peek_kind() {
            Some(TokenKind::Mnemonic(_)) => self.parse_instruction(),
            Some(TokenKind::Dot) | Some(TokenKind::Directive(_)) => self.parse_directive(),
            _ => {}
        }
    }

    /// Parses an instruction: `MNEMONIC op1, op2, op3`.
    fn parse_instruction(&mut self) {
        let start_pos = self.current_token().position;
        let opcode = match self.bump_kind() {
            TokenKind::Mnemonic(opcode) => opcode,
            _ => unreachable!("called only when the current token is a mnemonic"),
        };

        let expected = operand_count(opcode);
        let needs_register_dst = requires_register_dst(opcode);

        let mut operands: [Option<Operand>; 3] = [None; 3];
        let mut count = 0usize;

        while count < expected as usize {
            if count > 0 {
                match self.peek_kind() {
                    Some(TokenKind::Comma) => {
                        self.bump();
                    }
                    Some(kind) if is_operand_start(kind) => {
                        // The comma is missing, but an operand begins — parsing
                        // continues so that the rest of the line's errors are found too.
                        self.push_error_at_current(ParserErrorKind::ExpectedComma);
                    }
                    _ => break,
                }
            }

            let Some(kind) = self.peek_kind() else {
                break;
            };

            let Some(operand) = self.operand_from_kind(kind, self.current_token().position) else {
                break;
            };
            self.bump();
            operands[count] = Some(operand);
            count += 1;
        }

        // After a full set of operands, the line must end.
        if count == expected as usize
            && !matches!(
                self.peek_kind(),
                Some(TokenKind::Newline) | Some(TokenKind::End) | None
            )
        {
            // An extra operand is the same IncorrectNumberOfOperands,
            // so it is counted via the number of operands up to the end of the line.
            if self.peek_is_operand() {
                let extra = self.count_operands_until_newline();
                self.push_error(
                    start_pos,
                    ParserErrorKind::IncorrectNumberOfOperands {
                        expected: expected as u8,
                        got: (count + extra) as u8,
                    },
                );
            } else {
                self.push_error_at_current(ParserErrorKind::UnexpectedToken {
                    expected: "end of statement",
                    got: self.current_kind_debug(),
                });
            }
            self.skip_to_newline();
            return;
        }

        if count != expected as usize {
            self.push_error(
                start_pos,
                ParserErrorKind::IncorrectNumberOfOperands {
                    expected: expected as u8,
                    got: count as u8,
                },
            );
            self.skip_to_newline();
            return;
        }

        if needs_register_dst {
            let Some(dst) = operands[0] else {
                unreachable!("count == expected >= 1, so the destination is present");
            };

            if !matches!(dst, Operand::Register(_)) {
                self.push_error(
                    start_pos,
                    ParserErrorKind::ExpectedRegisterOperand {
                        got: format!("{dst:?}"),
                    },
                );
            }
        }

        self.ast.program.push(Statement::Instruction {
            position: start_pos,
            instruction: Instr {
                opcode,
                operand1: operands[0],
                operand2: operands[1],
                operand3: operands[2],
            },
        });
    }

    /// Parses a directive: `.name [operand]`.
    ///
    /// The lexer lexes a well-formed directive into a single
    /// [`TokenKind::Directive`] token; here it is validated and turned
    /// into an AST [`Directive`]. A malformed directive (an unknown name,
    /// a missing or a wrong operand) arrives as a [`TokenKind::Dot`]
    /// followed by ordinary tokens — in that case only the error is
    /// reported.
    fn parse_directive(&mut self) {
        let start = self.current_token().position;

        let directive = match self.bump_kind() {
            TokenKind::Directive(directive) => directive,
            TokenKind::Dot => {
                self.report_malformed_directive(start);
                return;
            }
            _ => unreachable!("called only when the current token starts a directive"),
        };

        let directive = match directive {
            Directive::Align { bytes } => {
                if bytes == 0 {
                    self.push_error(
                        start,
                        ParserErrorKind::InvalidAlignBytes {
                            value: bytes as i64,
                        },
                    );
                    self.skip_to_newline();
                    return;
                }

                Directive::Align { bytes }
            }
            Directive::MinVersion {
                major,
                minor,
                patch,
            } => {
                let version_text = format!("{major}.{minor}.{patch}");

                let current = current_w8_version();
                if (major, minor, patch) > current {
                    self.push_error(
                        start,
                        ParserErrorKind::MinVersionTooNew {
                            version: version_text,
                            current: format!("{}.{}.{}", current.0, current.1, current.2),
                        },
                    );
                    self.skip_to_newline();
                    return;
                }

                Directive::MinVersion {
                    major,
                    minor,
                    patch,
                }
            }
            // The compile-time constant is remembered for the substitution
            // in instruction operands; the last `.define` of a name wins.
            Directive::Define { name, value } => {
                self.defines.insert(name, value);
                Directive::Define { name, value }
            }
            // The file-local constant is remembered per file (see
            // `file_of`); the last `.ldefine` of a name wins.
            Directive::LDefine { name, value } => {
                let file = self.file_of(start.start);
                self.local_defines.insert((file, name), value);
                Directive::LDefine { name, value }
            }
        };

        // After the directive, the line must end.
        if !matches!(
            self.peek_kind(),
            Some(TokenKind::Newline) | Some(TokenKind::End) | None
        ) {
            self.push_error_at_current(ParserErrorKind::UnexpectedToken {
                expected: "end of statement",
                got: self.current_kind_debug(),
            });
            self.skip_to_newline();
            return;
        }

        let position = start.to(self.tokens[self.index - 1].position);
        self.ast.program.push(Statement::Directive {
            position,
            directive,
        });
    }

    /// Reports the error for a directive that the lexer did not turn into
    /// a single [`TokenKind::Directive`] token: either the name is unknown,
    /// or the operand is missing or of the wrong kind.
    ///
    /// `start` is the position of the leading dot.
    fn report_malformed_directive(&mut self, start: Position) {
        let name = match self.bump_kind() {
            TokenKind::Ident(id) => id,
            _ => {
                self.push_error_at_current(ParserErrorKind::ExpectedDirectiveName);
                self.skip_to_newline();
                return;
            }
        };

        let name_text = self.str_pool.get(name).to_string();
        match name_text.to_ascii_lowercase().as_str() {
            "align" => match self.peek_kind() {
                // A negative integer: the lexer only turns a non-negative
                // one into a `Directive` token.
                Some(TokenKind::Integer(bytes)) => {
                    self.push_error_at_current(ParserErrorKind::InvalidAlignBytes {
                        value: *bytes,
                    });
                }
                Some(TokenKind::Newline) | Some(TokenKind::End) | None => {
                    self.push_error_at_current(ParserErrorKind::ExpectedDirectiveOperand);
                }
                Some(kind) => {
                    self.push_error_at_current(ParserErrorKind::UnexpectedDirectiveOperand {
                        got: format!("{kind:?}"),
                    });
                }
            },
            "minversion" => match self.peek_kind() {
                // A version with a part out of `u16` range: the lexer only
                // turns a fitting one into a `Directive` token.
                Some(TokenKind::Version {
                    major,
                    minor,
                    patch,
                }) => {
                    self.push_error_at_current(ParserErrorKind::MinVersionOutOfRange {
                        version: format!("{major}.{minor}.{patch}"),
                    });
                }
                Some(TokenKind::Newline) | Some(TokenKind::End) | None => {
                    self.push_error_at_current(ParserErrorKind::ExpectedDirectiveOperand);
                }
                Some(kind) => {
                    self.push_error_at_current(ParserErrorKind::UnexpectedDirectiveOperand {
                        got: format!("{kind:?}"),
                    });
                }
            },
            "define" | "ldefine" => match self.peek_kind() {
                Some(TokenKind::Ident(_)) => {
                    self.bump();
                    if !matches!(self.peek_kind(), Some(TokenKind::Comma)) {
                        self.push_error_at_current(ParserErrorKind::ExpectedComma);
                        self.skip_to_newline();
                        return;
                    }
                    self.bump();
                    match self.peek_kind() {
                        Some(TokenKind::Newline) | Some(TokenKind::End) | None => {
                            self.push_error_at_current(ParserErrorKind::ExpectedDirectiveOperand);
                        }
                        Some(kind) => {
                            self.push_error_at_current(
                                ParserErrorKind::UnexpectedDirectiveOperand {
                                    got: format!("{kind:?}"),
                                },
                            );
                        }
                    }
                }
                Some(TokenKind::Newline) | Some(TokenKind::End) | None => {
                    self.push_error_at_current(ParserErrorKind::ExpectedDirectiveOperand);
                }
                Some(kind) => {
                    self.push_error_at_current(ParserErrorKind::UnexpectedDirectiveOperand {
                        got: format!("{kind:?}"),
                    });
                }
            },
            _ => {
                self.push_error(start, ParserErrorKind::UnknownDirective { name: name_text });
            }
        }

        self.skip_to_newline();
    }

    // ====== Helper functions ======

    /// Skips tokens up to the end of the line (including the line separator).
    fn skip_to_newline(&mut self) {
        loop {
            match self.peek_kind() {
                Some(TokenKind::Newline) => {
                    self.bump();
                    break;
                }
                Some(TokenKind::End) | None => break,
                _ => {
                    self.bump();
                }
            }
        }
    }

    /// Adds an error to [`errors`](Self::errors)
    /// with the position of the current token.
    fn push_error_at_current(&mut self, kind: ParserErrorKind) {
        let position = self.current_token().position;
        self.push_error(position, kind);
    }

    /// Adds an error to [`errors`](Self::errors).
    fn push_error(&mut self, position: Position, kind: ParserErrorKind) {
        self.errors.push(ParserError::new(kind, position));
    }

    /// Returns the debug representation of the current token.
    fn current_kind_debug(&self) -> String {
        match self.peek_kind() {
            Some(kind) => format!("{kind:?}"),
            None => String::from("end of input"),
        }
    }

    /// Whether the current token starts an operand.
    fn peek_is_operand(&self) -> bool {
        match self.peek_kind() {
            Some(kind) => is_operand_start(kind),
            None => false,
        }
    }

    /// Counts consecutive operands up to the end of the line (for extra operands).
    fn count_operands_until_newline(&self) -> usize {
        let mut extra = 0usize;
        let mut index = self.index;

        while let Some(token) = self.tokens.get(index) {
            match &token.kind {
                TokenKind::Newline | TokenKind::End => break,
                kind if is_operand_start(kind) => extra += 1,
                _ => break,
            }
            index += 1;
        }

        extra
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.peek().map(|token| &token.kind)
    }

    fn current_token(&self) -> &Token {
        self.peek()
            .expect("the parser never reads beyond the final End token")
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens[self.index].clone();
        self.index += 1;
        token
    }

    fn bump_kind(&mut self) -> TokenKind {
        self.bump().kind
    }

    /// Converts a token kind into an AST operand if the token can be an
    /// operand. Integer literals are “wrapped” into `u64` per two's complement
    /// rules, floating-point literals — into their bit
    /// representation (`f64::to_bits`). An identifier of a `.define` or
    /// `.ldefine` constant becomes its value (a number or a register); any
    /// other identifier — a reference to a label. A local constant shadows
    /// a global one of the same name within its file.
    fn operand_from_kind(&self, kind: &TokenKind, at: Position) -> Option<Operand> {
        match kind {
            TokenKind::Register(register) => Some(Operand::Register(*register)),
            TokenKind::Integer(value) => Some(Operand::Immediate(*value as u64)),
            TokenKind::Float(value) => Some(Operand::Immediate((*value).to_bits())),
            TokenKind::Ident(id) => {
                let file = self.file_of(at.start);
                match self
                    .local_defines
                    .get(&(file, *id))
                    .or_else(|| self.defines.get(id))
                {
                    Some(DefineValue::Number(value)) => Some(Operand::Immediate(*value)),
                    Some(DefineValue::Register(register)) => Some(Operand::Register(*register)),
                    None => Some(Operand::Label(*id)),
                }
            }
            TokenKind::LocalLabel(id) => Some(Operand::LocalLabel(*id)),
            _ => None,
        }
    }

    /// The file a global text offset belongs to (see [`Self::files`]);
    /// 0 when the map is empty.
    fn file_of(&self, offset: u32) -> u32 {
        let count = self.files.partition_point(|(start, _)| *start <= offset);
        self.files
            .get(count.wrapping_sub(1))
            .map_or(0, |(_, file)| *file)
    }
}

/// The current W8 version as a numeric `(major, minor, patch)` triple.
///
/// Non-numeric parts (for example, a prerelease suffix) are truncated,
/// missing parts are treated as zero — the same rules as the encoder's.
fn current_w8_version() -> (u16, u16, u16) {
    let mut parts = W8_VERSION.split('.').map(version_number);

    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// Parses the numeric part of a string; returns 0 for a non-number.
fn version_number(part: &str) -> u16 {
    part.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// Whether the token starts an operand.
fn is_operand_start(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Register(_)
            | TokenKind::Integer(_)
            | TokenKind::Float(_)
            | TokenKind::Ident(_)
            | TokenKind::LocalLabel(_)
    )
}

/// The expected number of operands for an opcode.
fn operand_count(opcode: OperationCode) -> u8 {
    use OperationCode::*;

    match opcode {
        NOP | RET => 0,
        JMP | CALL => 1,
        JZ | JNZ | MOVE | LOAD8 | LOAD16 | LOAD32 | LOAD64 | STORE8 | STORE16 | STORE32
        | STORE64 | INEG | FNEG | NOT => 2,
        VMCALL => 3,
        IADD | ISUB | IMUL | SDIV | UDIV | SREM | UREM | FADD | FSUB | FMUL | FDIV | FREM | AND
        | OR | XOR | LSL | LSR | SAR | IEQ | INE | SLT | SLE | SGT | SGE | ULT | ULE | UGT
        | UGE | FEQ | FNE | FLT | FLE | FGT | FGE => 3,
    }
}

/// Whether the destination (the first operand) must be a register.
///
/// Matches the [iserial](crate::lexer) signatures and the executor's
/// dispatch table: the destinations of `MOVE`, `LOAD*`, unary and binary
/// operations — registers only.
fn requires_register_dst(opcode: OperationCode) -> bool {
    use OperationCode::*;

    matches!(
        opcode,
        MOVE | LOAD8
            | LOAD16
            | LOAD32
            | LOAD64
            | INEG
            | FNEG
            | NOT
            | IADD
            | ISUB
            | IMUL
            | SDIV
            | UDIV
            | SREM
            | UREM
            | FADD
            | FSUB
            | FMUL
            | FDIV
            | FREM
            | AND
            | OR
            | XOR
            | LSL
            | LSR
            | SAR
            | IEQ
            | INE
            | SLT
            | SLE
            | SGT
            | SGE
            | ULT
            | ULE
            | UGT
            | UGE
            | FEQ
            | FNE
            | FLT
            | FLE
            | FGT
            | FGE
    )
}
