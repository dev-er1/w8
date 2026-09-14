//! # `libw8`
//!
//! `libw8` is the crate for using W8.
//!
//! Two parts:
//! - [`W8`] — bytecode execution. It takes bytecode via
//!   [`BytecodeSource`] and executes it;
//! - [`W8Assembler`] — compilation of w8 Assembly into instructions
//!   and into bytecode (`.wb`).
use std::{
    io::IsTerminal,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use w8_core::{loader::W8Loader, vm::memory::WVMMemory};
use wdasm::{
    codegen,
    lexer::Lexer,
    parser::Parser,
    position::Position,
    preprocess::{self, PreprocessorError},
    src::SourceCode,
    str_pool::StrPool,
};

// The public API of `libw8`: the error type and instructions — re-exported from `w8-core`.
pub use w8_core::W8_VERSION;
pub use w8_core::error::{WVMError, WVMErrorKind};
pub use w8_core::isa::instruction::Instruction;
pub use w8_core::vm::{ExecuteVariant, VMCallDecision, WVM};

// Compilation errors — re-exported from `wdasm`.
pub use wdasm::error::{W8AsmError, W8AsmErrorKind};

/// The default VM memory size (in bytes).
pub const DEFAULT_MEMORY_SIZE: usize = 64 * 1024;

/// The bytecode source for [`W8::run`].
pub enum BytecodeSource {
    /// A path to a file in the W8 Bytecode (`.wb`) format.
    File(PathBuf),

    /// Raw bytecode bytes (for example, read from stdin).
    Bytes(Vec<u8>),

    /// Already parsed instructions.
    Instructions(Vec<Instruction>),
}

pub struct W8 {
    pub memory_size: usize,
}

impl W8 {
    pub fn new() -> Self {
        Self {
            memory_size: DEFAULT_MEMORY_SIZE,
        }
    }

    pub fn with_memory_size(memory_size: usize) -> Self {
        Self { memory_size }
    }

    /// Loads the bytecode from the given [`BytecodeSource`] into instructions.
    fn load(&self, source: BytecodeSource) -> Result<Vec<Instruction>, WVMError> {
        match source {
            BytecodeSource::File(path) => {
                let bytes = std::fs::read(&path)
                    .map_err(|e| WVMError::new(WVMErrorKind::IoError(e), None, false))?;
                W8Loader::new(bytes)
                    .transpile()
                    .map_err(|e| WVMError::new(WVMErrorKind::LoaderError(e), None, false))
            }
            BytecodeSource::Bytes(bytes) => W8Loader::new(bytes)
                .transpile()
                .map_err(|e| WVMError::new(WVMErrorKind::LoaderError(e), None, false)),
            BytecodeSource::Instructions(instructions) => Ok(instructions),
        }
    }

    /// Executes the bytecode from the given [`BytecodeSource`] without a host.
    ///
    /// Returns the exit code: `Some(code)` — the program terminated via
    /// a `VMCALL` with the `Exit` decision; `None` — the program fell off
    /// the end of the program without an explicit exit.
    pub fn run(
        &self,
        source: BytecodeSource,
        executeby: ExecuteVariant,
    ) -> Result<Option<u64>, WVMError> {
        let instructions = self.load(source)?;
        let mut vm =
            WVM::from_program_and_memory(instructions, WVMMemory::new(self.memory_size), executeby);

        vm.interpretate()
            .map_err(|e| WVMError::new(WVMErrorKind::VMError(e), None, false))?;

        Ok(vm.exit_code)
    }

    /// Executes the bytecode from the given [`BytecodeSource`] with
    /// a caller-provided host dispatcher.
    ///
    /// The `dispatch` is called for every `VMCALL <service>, <address>, <size>`
    /// of the program: it receives `&mut WVM`, the service number and the
    /// arguments block (`address..address + size` — the VM has already checked
    /// that the block lies within the memory). The host returns the
    /// [`VMCallDecision`].
    ///
    /// Returns the exit code as [`Self::run`].
    pub fn run_with<F>(
        &self,
        source: BytecodeSource,
        dispatch: F,
        executeby: ExecuteVariant,
    ) -> Result<Option<u64>, WVMError>
    where
        F: FnMut(&mut WVM, u64, u64, u64) -> VMCallDecision + 'static,
    {
        let instructions = self.load(source)?;
        let mut vm =
            WVM::from_program_and_memory(instructions, WVMMemory::new(self.memory_size), executeby);

        if executeby == ExecuteVariant::ByInterpreter {
            vm.interpretate_with(dispatch)
                .map_err(|e| WVMError::new(WVMErrorKind::VMError(e), None, false))?;
        } else {
            vm.jit_compile_with(DEFAULT_MEMORY_SIZE, dispatch)
                .map_err(|e| WVMError::new(WVMErrorKind::VMError(e), None, false))?;
        }

        Ok(vm.exit_code)
    }
}

impl Default for W8 {
    fn default() -> Self {
        Self::new()
    }
}

/// Compiler from W8 Assembly into W8 Bytecode.
///
/// Builds the full compilation pipeline of the textual assembler:
///
/// ```text
/// text -> preprocess -> lexer -> parser -> codegen [-> encoder -> .wb]
/// ```
///
/// On error, the very first compilation error ([`W8AsmError`]) is
/// returned with a position and a fragment of the source code.
pub struct W8Assembler;

impl W8Assembler {
    /// Compiles W8 Assembly source text into instructions.
    ///
    /// Labels are resolved into instruction indices (see `codegen`).
    ///
    /// The `#include "<path>"` directive is not available here (there is
    /// no directory to resolve the path against) — use
    /// [`Self::assemble_from_path`].
    ///
    /// ## Example
    ///
    /// ```rust
    /// use libw8::W8Assembler;
    ///
    /// let instructions = W8Assembler::assemble("MOVE R0, 42\nNOP").expect("valid program");
    /// assert_eq!(instructions.len(), 2);
    /// ```
    // An error carries a fragment of the source code for pretty-printing
    // (W8AsmError::format) — this is a deliberate size.
    #[allow(clippy::result_large_err)]
    pub fn assemble(source: &str) -> Result<Vec<Instruction>, W8AsmError> {
        Self::compile_with(source, None, None, None).map(|result| result.instructions)
    }

    /// Compiles an W8 Assembly file into instructions.
    ///
    /// Unlike [`Self::assemble`], the file may contain the
    /// `#include "<path>"` directive: the paths are resolved relative to
    /// the directory of the including file, and errors are reported with
    /// the file name and position of the file in which they were found.
    #[allow(clippy::result_large_err)]
    pub fn assemble_from_path(path: impl AsRef<Path>) -> Result<Vec<Instruction>, W8AsmError> {
        let result = Self::compile_from_path(path.as_ref())?;
        Ok(result.instructions)
    }

    /// Compiles W8 Assembly source text into the bytes of a `.wb` file.
    ///
    /// Unlike [`Self::assemble`], this encodes the instructions into
    /// the W8 Bytecode format (see `docs/File-Format/File-Format.md`).
    #[allow(clippy::result_large_err)]
    pub fn assemble_to_bytecode(source: &str) -> Result<Vec<u8>, W8AsmError> {
        let result = Self::compile_with(source, None, None, None)?;

        Ok(codegen::encoder::encode(
            &result.instructions,
            result.min_version,
        ))
    }

    /// Compiles an W8 Assembly file into the bytes of a `.wb` file.
    ///
    /// Supports the `#include "<path>"` directive, like
    /// [`Self::assemble_from_path`].
    #[allow(clippy::result_large_err)]
    pub fn assemble_to_bytecode_from_path(path: impl AsRef<Path>) -> Result<Vec<u8>, W8AsmError> {
        let result = Self::compile_from_path(path.as_ref())?;

        Ok(codegen::encoder::encode(
            &result.instructions,
            result.min_version,
        ))
    }

    /// Compiles a file with `#include` support.
    fn compile_from_path(path: &Path) -> Result<codegen::CodegenResult, W8AsmError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(e) => {
                return Err(W8AsmError::error(
                    Position::new(0, 0),
                    W8AsmErrorKind::PreprocessorError(PreprocessorError::CannotReadFile {
                        path: path.display().to_string(),
                        error: e.to_string(),
                    }),
                    ansi_supported(),
                    Vec::new(),
                ));
            }
        };

        let filename = path.display().to_string();
        let dir = path.parent().map(Path::to_path_buf);
        let mut resolver = |path: &str| std::fs::read_to_string(path);

        Self::compile_with(
            &source,
            Some(&filename),
            dir.as_deref(),
            Some(&mut resolver),
        )
    }

    /// Runs the full compilation pipeline:
    ///
    /// ```text
    /// text -> preprocess -> lexer -> parser -> code generator
    /// ```
    ///
    /// On error, the very first compilation error ([`W8AsmError`]) is
    /// returned with a position and a fragment of the source code.
    #[allow(clippy::result_large_err)]
    fn compile_with(
        source: &str,
        filename: Option<&str>,
        dir: Option<&Path>,
        resolver: Option<preprocess::Resolver<'_>>,
    ) -> Result<codegen::CodegenResult, W8AsmError> {
        // ====== Preprocessing ======

        let preprocessed =
            preprocess::preprocess(source, filename, dir, resolver, ansi_supported())?;
        let segments = preprocessed.segments;
        let source = SourceCode::new(preprocessed.text);
        let mut str_pool = StrPool::from_source(&source);

        // The map of global offsets to file ids (for `.ldefine` scoping).
        let files: Vec<(u32, u32)> = segments
            .iter()
            .map(|segment| (segment.global_start, segment.file))
            .collect();

        // ====== Lexer ======

        let (tokens, lexer_errors) = {
            let mut lexer = Lexer::new(source.clone(), &mut str_pool);
            let tokens = lexer.tokenize().to_vec();
            (tokens, lexer.errors.clone())
        };

        if let Some(err) = lexer_errors.first() {
            return Err(W8AsmError::error(
                err.pos,
                W8AsmErrorKind::LexerError(err.clone()),
                ansi_supported(),
                segments,
            ));
        }

        // ====== Parser ======

        let mut parser = Parser::new(tokens, &str_pool, &files);
        let ast = parser.parse().clone();

        if let Some(err) = parser.errors.first() {
            return Err(W8AsmError::error(
                err.position,
                W8AsmErrorKind::ParserError(err.clone()),
                ansi_supported(),
                segments,
            ));
        }

        // ====== Code generator ======

        codegen::generate(&ast, &str_pool).map_err(|err| {
            W8AsmError::error(
                err.position,
                W8AsmErrorKind::CodegenError(err),
                ansi_supported(),
                segments,
            )
        })
    }
}

fn ansi_supported() -> bool {
    static SUPPORTED: OnceLock<bool> = OnceLock::new();
    *SUPPORTED.get_or_init(|| {
        if !std::io::stdout().is_terminal() && !std::io::stderr().is_terminal() {
            return false;
        }
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        if cfg!(target_os = "windows") {
            return std::env::var("WT_SESSION").is_ok()
                || std::env::var("ConEmuANSI")
                    .map(|v| v == "ON")
                    .unwrap_or(false)
                || std::env::var("TERM_PROGRAM").is_ok();
        }
        match std::env::var("TERM") {
            Ok(term) => {
                let t = term.to_lowercase();
                t != "dumb"
                    && (t.contains("color")
                        || t.contains("xterm")
                        || t.contains("256")
                        || t.contains("linux")
                        || t.contains("ansi")
                        || t.contains("kitty")
                        || t.contains("alacritty"))
            }
            Err(_) => false,
        }
    })
}
