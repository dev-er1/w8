// wdasm/src/error.rs
//
//! Pretty-print of errors.
use std::fmt::{self, Display, Formatter, Write};

use crate::{
    codegen::err::CodegenError,
    lexer::err::LexerError,
    parser::err::ParserError,
    position::Position,
    preprocess::{PreprocessorError, Segment},
    src::SourceCode,
};

#[derive(Debug, Clone)]
pub enum W8AsmErrorKind {
    /// Error of the `#include` preprocessing stage.
    PreprocessorError(PreprocessorError),

    /// Lexical analysis error.
    LexerError(LexerError),

    /// Syntax analysis error.
    ParserError(ParserError),

    /// Code generator error.
    CodegenError(CodegenError),
}

impl Display for W8AsmErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreprocessorError(err) => write!(f, "{err}"),
            Self::LexerError(err) => write!(f, "{err}"),
            Self::ParserError(err) => write!(f, "{err}"),
            Self::CodegenError(err) => write!(f, "{err}"),
        }
    }
}

/// Compilation error together with a fragment of the source code.
#[derive(Debug, Clone)]
pub struct W8AsmError {
    position: Position,
    pub kind: W8AsmErrorKind,

    /// Whether the terminal supports ANSI colors.
    have_ansi: bool,

    /// Segments of the spliced text. They map the error position
    /// to the file in which the error was found.
    segments: Vec<Segment>,
}

impl W8AsmError {
    /// Creates a compilation error.
    pub fn error(
        position: Position,
        kind: W8AsmErrorKind,
        have_ansi: bool,
        segments: Vec<Segment>,
    ) -> Self {
        Self {
            position,
            kind,
            have_ansi,
            segments,
        }
    }

    pub fn report(&self) {
        println!("{}", self.format());
    }

    /// Produces the error text without printing to the console.
    ///
    /// Useful for saving the error to a log or for tests.
    ///
    /// ```text
    /// Error: unexpected character: '!'.
    /// test.wa -> 1:10..1:11
    ///   |
    /// 1 | MOVE R0, !
    ///   |          ^
    /// ```
    pub fn format(&self) -> String {
        let mut out = String::new();

        let error = self.styled("1;31", "Error");
        let message = self.styled("1", &self.kind.to_string());
        writeln!(out, "{error}: {message}.").unwrap();

        // Without segments (for example, when the main file itself could
        // not be read) there is nothing to point to.
        let Some((seg, local_start, local_end)) = self.resolve_position() else {
            return out;
        };

        let (start_line, start_col) = seg.src.lookup_coordinates(local_start);
        let (end_line, end_col) = seg.src.lookup_coordinates(local_end);

        self.write_header(
            &mut out,
            seg.filename.as_deref(),
            start_line,
            start_col,
            end_line,
            end_col,
        );
        self.write_source(&mut out, start_line, start_col, end_line, end_col, &seg.src);

        out
    }

    /// Finds the segment containing the error position and the local
    /// byte offsets in that segment.
    fn resolve_position(&self) -> Option<(&Segment, u32, u32)> {
        let idx = self
            .segments
            .partition_point(|s| s.global_start <= self.position.start);
        let seg = self.segments.get(idx.wrapping_sub(1))?;

        if self.position.start >= seg.global_start + seg.global_len {
            return None;
        }

        let local_start = self.position.start - seg.global_start + seg.local_start;
        let local_end = (self.position.end - seg.global_start + seg.local_start)
            .min(seg.local_start + seg.global_len);
        Some((seg, local_start, local_end))
    }

    /// Error header: `Error: <message>.` and `file -> position`.
    fn write_header(
        &self,
        out: &mut String,
        filename: Option<&str>,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
    ) {
        let position = format!("{start_line}:{start_col}..{end_line}:{end_col}");
        match filename {
            Some(file) => {
                let file = self.styled("36", file);
                let position = self.styled("1", &position);
                writeln!(out, "{file} -> {position}").unwrap();
            }
            None => {
                let position = self.styled("1", &position);
                writeln!(out, "-> {position}").unwrap();
            }
        }
    }

    /// Fragment of the source code with error underlining.
    fn write_source(
        &self,
        out: &mut String,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        src: &SourceCode,
    ) {
        let width = end_line.to_string().len();

        self.write_gutter(out, width);

        if start_line == end_line {
            let line = self.line_text(src, start_line);
            self.write_numbered_line(out, start_line, width, &line);
            let carets = (end_col - start_col).max(1) as usize;
            self.write_carets(out, width, start_col, carets);
            return;
        }

        // Multiline span: first line, middle, last line.
        let first = self.line_text(src, start_line);
        self.write_numbered_line(out, start_line, width, &first);
        let carets = first.len().saturating_sub((start_col - 1) as usize);
        self.write_carets(out, width, start_col, carets.max(1));

        for line_no in start_line + 1..end_line {
            let text = self.line_text(src, line_no);
            self.write_numbered_line(out, line_no, width, &text);
            self.write_carets(out, width, 1, text.len().max(1));
        }

        let last = self.line_text(src, end_line);
        self.write_numbered_line(out, end_line, width, &last);
        let carets = (end_col - 1).max(1) as usize;
        self.write_carets(out, width, 1, carets);
    }

    /// Separator line above the code fragment.
    fn write_gutter(&self, out: &mut String, width: usize) {
        writeln!(out, "{} |", " ".repeat(width)).unwrap();
    }

    /// Source code line with a line number.
    fn write_numbered_line(&self, out: &mut String, line: u32, width: usize, text: &str) {
        let number = self.styled("1", &format!("{line:>width$}"));
        writeln!(out, "{number} | {text}").unwrap();
    }

    /// Error underline line (`^`).
    fn write_carets(&self, out: &mut String, width: usize, column: u32, count: usize) {
        let padding = " ".repeat((column - 1) as usize);
        let carets = self.styled("1;31", &"^".repeat(count));
        writeln!(out, "{} | {padding}{carets}", " ".repeat(width)).unwrap();
    }

    /// Text of the line with the given number without a newline.
    fn line_text(&self, src: &SourceCode, line: u32) -> String {
        let start = src.line_starts[(line - 1) as usize] as usize;
        let end = src
            .line_starts
            .get(line as usize)
            .map_or(src.source.len(), |&offset| offset as usize);

        src.source[start..end]
            .trim_end_matches(['\r', '\n'])
            .to_string()
    }

    /// Wraps the text in ANSI codes if the terminal supports them.
    fn styled(&self, code: &str, text: &str) -> String {
        if self.have_ansi {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}
