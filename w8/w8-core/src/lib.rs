//! # `w8-core`
//!
//! This crate is the **core of W8**. It contains:
//! - [`isa`] — the W8 *Instruction Set Architecture (**ISA**)*.
//! - [`vm`] — the W8 virtual machine itself.
//! - [`loader`] — the loader of files in the W8 Bytecode format (see `docs/File-Format/FILE-FORMAT.md`).
pub mod error;
pub mod isa;
pub mod loader;
pub mod vm;

pub const W8_VERSION: &str = env!("CARGO_PKG_VERSION");
