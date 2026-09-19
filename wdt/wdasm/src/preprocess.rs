// wdasm/src/preprocess.rs
//
//! Preprocessing: expansion of the `#include "<path>"` directives.
//!
//! Preprocessing is a text stage that runs before the lexer. The included
//! files are spliced into the source text in place of their `#include`
//! lines.
//!
//! [`Segment`]s map these offsets back to the files, so that errors are
//! rendered with the correct file name, line and column.
use std::{
    fmt::{self, Display, Formatter},
    io,
    path::{Path, PathBuf},
};

use crate::{
    error::{W8AsmError, W8AsmErrorKind},
    position::Position,
    src::SourceCode,
};

/// Errors of the `#include` preprocessing stage.
#[derive(Debug, Clone)]
pub enum PreprocessorError {
    /// The program is compiled from a string, so there is no directory
    /// relative to which the path could be resolved.
    NoBasePath { path: String },

    /// The included file does not exist.
    IncludeNotFound { path: String },

    /// The file includes itself (directly or transitively).
    IncludeCycle { path: String },

    /// The line does not match `#include "<path>"`.
    MalformedInclude,

    /// The file exists but cannot be read.
    CannotReadFile { path: String, error: String },
}

impl Display for PreprocessorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoBasePath { path } => write!(
                f,
                "cannot include `{path}`: compiling from a string, there is no base directory"
            ),
            Self::IncludeNotFound { path } => {
                write!(f, "included file not found: `{path}`")
            }
            Self::IncludeCycle { path } => {
                write!(f, "include cycle detected: `{path}`")
            }
            Self::MalformedInclude => {
                write!(
                    f,
                    "malformed `#include` directive, expected `#include \"<path>\"`"
                )
            }
            Self::CannotReadFile { path, error } => {
                write!(f, "cannot read `{path}`: {error}")
            }
        }
    }
}

/// A piece of the spliced text that belongs to one file.
#[derive(Debug, Clone)]
pub struct Segment {
    pub filename: Option<String>,
    pub src: SourceCode,
    pub global_start: u32,
    pub local_start: u32,
    pub global_len: u32,
    pub file: u32,
}

/// The result of preprocessing.
#[derive(Debug, Clone)]
pub struct Preprocessed {
    /// The whole text after splicing.
    pub text: String,

    /// Segments, in order of their `global_start`, tiling `text`.
    pub segments: Vec<Segment>,
}

/// Reads the file's text by its resolved path.
pub type Resolver<'a> = &'a mut dyn FnMut(&str) -> io::Result<String>;

/// Expands the `#include "<path>"` directives in the source text.
///
/// ## Arguments
///
/// - `filename` — the name of the main file (for error headers);
/// - `dir` — the directory of the main file, relative to which the
///   included paths are resolved. When `None`, every `#include` is an
///   [`PreprocessorError::NoBasePath`] error;
/// - `resolver` — is called with the resolved path and must return the
///   file's text. When `None`, every `#include` is an
///   [`PreprocessorError::NoBasePath`] error (compilation from a raw string);
/// - `have_ansi` — whether errors can use ANSI colors.
///
/// Includes of the same file in different places are allowed; only
/// recursive (cyclic) includes are errors.
pub fn preprocess(
    source: &str,
    filename: Option<&str>,
    dir: Option<&Path>,
    resolver: Option<Resolver<'_>>,
    have_ansi: bool,
) -> Result<Preprocessed, W8AsmError> {
    let mut pp = Preprocessor {
        resolver,
        stack: Vec::new(),
        text: String::with_capacity(source.len()),
        segments: Vec::new(),
        have_ansi,
        next_file: 0,
    };

    pp.expand_file(source, filename.map(String::from), dir, 0)?;

    Ok(Preprocessed {
        text: pp.text,
        segments: pp.segments,
    })
}

struct Preprocessor<'a> {
    resolver: Option<Resolver<'a>>,

    /// Resolved paths of the files that are currently being expanded
    /// (for cycle detection).
    stack: Vec<PathBuf>,
    text: String,
    segments: Vec<Segment>,

    /// The id of the next included file (0 is the main file).
    next_file: u32,
    have_ansi: bool,
}

impl Preprocessor<'_> {
    /// Slices one file into the global buffer, expanding its `#include`s.
    ///
    /// `file` is the id of this file (0 for the main file).
    fn expand_file(
        &mut self,
        source: &str,
        filename: Option<String>,
        dir: Option<&Path>,
        file: u32,
    ) -> Result<(), W8AsmError> {
        let src = SourceCode::new(source.to_string());

        // Byte offset in `source` of the beginning of the current piece
        // that is waiting to be appended.
        let mut cursor = 0usize;

        let mut line_start = 0usize;
        while line_start < source.len() {
            // Find the end of the line (and strip the trailing `\r`).
            let (content_end, line_end, had_newline) = match source[line_start..].find('\n') {
                Some(offset) => {
                    let line_end = line_start + offset;
                    let content_end = if line_end > 0 && source.as_bytes()[line_end - 1] == b'\r' {
                        line_end - 1
                    } else {
                        line_end
                    };
                    (content_end, line_end, true)
                }
                None => (source.len(), source.len(), false),
            };

            let trimmed_start = content_end - source[line_start..content_end].trim_start().len();
            let trimmed = &source[trimmed_start..content_end];

            if trimmed.starts_with("#include") {
                // Append the piece of text before this include line.
                self.push_segment(&source[cursor..line_start], cursor, &filename, &src, file);
                cursor = line_end + usize::from(had_newline);

                // The include line is replaced by the file's text, so the
                // directive sits right after the piece just appended.
                let global_pos = self.text.len() as u32 + (trimmed_start - line_start) as u32;
                self.expand_include(trimmed, global_pos, dir, had_newline)?;
            }

            if !had_newline {
                break;
            }
            line_start = line_end + 1;
        }

        self.push_segment(&source[cursor..], cursor, &filename, &src, file);
        Ok(())
    }

    /// Appends a piece of a file to the spliced text and records a segment.
    fn push_segment(
        &mut self,
        piece: &str,
        local_start: usize,
        filename: &Option<String>,
        src: &SourceCode,
        file: u32,
    ) {
        if piece.is_empty() {
            return;
        }

        let global_start = self.text.len() as u32;
        self.text.push_str(piece);
        self.segments.push(Segment {
            filename: filename.clone(),
            src: src.clone(),
            global_start,
            local_start: local_start as u32,
            global_len: piece.len() as u32,
            file,
        });
    }

    /// Processes one `#include "<path>"` directive line.
    ///
    /// `global_pos` is the byte offset of the `#include` word in the
    /// spliced text; `had_newline` tells whether the include line ended
    /// with a line break (which the file's text must then end with too).
    fn expand_include(
        &mut self,
        trimmed: &str,
        global_pos: u32,
        dir: Option<&Path>,
        had_newline: bool,
    ) -> Result<(), W8AsmError> {
        let rest = &trimmed["#include".len()..];
        let pos = Position::new(global_pos, global_pos + "#include".len() as u32);

        let Some(path) = parse_path(rest) else {
            return Err(self.error(pos, PreprocessorError::MalformedInclude));
        };

        let Some(dir) = dir else {
            return Err(self.error(
                pos,
                PreprocessorError::NoBasePath {
                    path: path.to_string(),
                },
            ));
        };

        let resolved = dir.join(path).components().collect::<PathBuf>();

        if self.stack.contains(&resolved) {
            return Err(self.error(
                pos,
                PreprocessorError::IncludeCycle {
                    path: resolved.display().to_string(),
                },
            ));
        }

        let Some(resolver) = self.resolver.as_deref_mut() else {
            return Err(self.error(
                pos,
                PreprocessorError::NoBasePath {
                    path: path.to_string(),
                },
            ));
        };

        let mut content = match resolver(&resolved.to_string_lossy()) {
            Ok(content) => content,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(self.error(
                    pos,
                    PreprocessorError::IncludeNotFound {
                        path: resolved.display().to_string(),
                    },
                ));
            }
            Err(e) => {
                return Err(self.error(
                    pos,
                    PreprocessorError::CannotReadFile {
                        path: resolved.display().to_string(),
                        error: e.to_string(),
                    },
                ));
            }
        };

        // The include line (with its line break) is replaced by the file's
        // text, so the next line of the parent file is not glued to it.
        if had_newline && !content.ends_with('\n') {
            content.push('\n');
        }

        let child_dir = resolved.parent().map(Path::to_path_buf);

        self.next_file += 1;
        let file = self.next_file;

        self.stack.push(resolved);
        let result = self.expand_file(&content, Some(path.to_string()), child_dir.as_deref(), file);
        self.stack.pop();
        result
    }

    fn error(&self, position: Position, kind: PreprocessorError) -> W8AsmError {
        W8AsmError::error(
            position,
            W8AsmErrorKind::PreprocessorError(kind),
            self.have_ansi,
            self.segments.clone(),
        )
    }
}

fn parse_path(rest: &str) -> Option<&str> {
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('"')?;

    let close = rest.find('"')?;
    let path = &rest[..close];
    if path.is_empty() {
        return None;
    }

    let tail = &rest[close + 1..];
    let tail = tail.trim_start();
    if !tail.is_empty() && !tail.starts_with(';') {
        return None;
    }

    Some(path)
}
