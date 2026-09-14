//! # `wdasm`
//!
//! `wdasm` is a package in the W8 default toolchain (this workspace),
//! for compiling the text W8 Bytecode format (let's call it W8 Assembly)
//! into W8 Bytecode.
//!
//! ## Contents
//! - [`src`] — source code storage.
//! - [`str_pool`] — string pool.
//! - [`position`] — token position structure in the source code.
//! - [`preprocess`] — preprocessing.
//! - [`lexer`] — lexer.
//! - [`parser`] — parser.
//! - [`codegen`] — code generation.
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod position;
pub mod preprocess;
pub mod src;
pub mod str_pool;
