//! Source location spans for diagnostics.

use std::fmt;

/// A span in source (byte offset start and end).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// File id for multi-file support (MVP: often 0 for single file).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

impl FileId {
    pub const fn new(id: u32) -> Self {
        FileId(id)
    }
}

/// Location: file + span. Used in diagnostics.
#[derive(Clone, Copy, Debug)]
pub struct Location {
    pub file: FileId,
    pub span: Span,
}

impl Location {
    pub fn new(file: FileId, span: Span) -> Self {
        Self { file, span }
    }
}
