// wdasm/src/str_pool.rs
//
//! String pool.
use std::collections::HashMap;

use crate::src::SourceCode;

/// String identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct StrId(pub u32);

#[derive(Debug, Clone, Default)]
pub struct StrPool {
    storage: Vec<Box<str>>,

    lookup: HashMap<String, StrId>,
}

impl StrPool {
    pub fn from_source(src: &SourceCode) -> Self {
        let code_len = src.source.len();

        let estimated_lines = src.line_starts.len();
        let estimated_words = (code_len / 6).max(estimated_lines);

        let capacity = estimated_words.max(32);

        Self {
            storage: Vec::with_capacity(capacity),
            lookup: HashMap::with_capacity(capacity),
        }
    }

    pub fn with_capacity(items: usize) -> Self {
        Self {
            storage: Vec::with_capacity(items),
            lookup: HashMap::with_capacity(items),
        }
    }

    pub fn intern(&mut self, s: &str) -> StrId {
        // If the string is already there, just return its ID.
        if let Some(&id) = self.lookup.get(s) {
            return id;
        }

        let id = StrId(self.storage.len() as u32);

        let boxed_str = s.to_string().into_boxed_str();
        self.storage.push(boxed_str);

        self.lookup.insert(s.to_string(), id);

        id
    }

    /// Access to the string by ID in O(1) without hashing.
    #[inline]
    pub fn get(&self, id: StrId) -> &str {
        &self.storage[id.0 as usize]
    }
}
