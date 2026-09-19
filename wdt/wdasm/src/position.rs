// w8-core/src/position.rs
//
//! Position of a token in the source code.
use std::ops::Range;

/// Stores flat byte offsets from the beginning of the file `[start..end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub start: u32,
    pub end: u32,
}

impl Position {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn as_range(&self) -> Range<usize> {
        (self.start as usize)..(self.end as usize)
    }

    pub fn to(&self, other: Self) -> Self {
        Self {
            start: self.start,
            end: other.end,
        }
    }
}
