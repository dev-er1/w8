//! This module contains an implementation of machine code generation
//! for different CPU architectures from W8 bytecode.
//!
//! | CPU architecture | Implementation file |
//! |:----------------:|---------------------|
//! | x64              | [`x64`]             |
//! | x86              | [`x86`]             |
#[cfg(target_arch = "x86_64")]
pub mod x64;
#[cfg(target_arch = "x86")]
pub mod x86;
