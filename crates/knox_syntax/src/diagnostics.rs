//! Diagnostics (errors, warnings) with file/line spans.

use crate::span::Location;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub level: Level,
    pub message: String,
    pub location: Option<Location>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, location: Option<Location>) -> Self {
        Self {
            level: Level::Error,
            message: message.into(),
            location,
        }
    }

    pub fn warning(message: impl Into<String>, location: Option<Location>) -> Self {
        Self {
            level: Level::Warning,
            message: message.into(),
            location,
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.level {
            Level::Error => "error",
            Level::Warning => "warning",
        };
        if let Some(loc) = &self.location {
            write!(f, "{} at {:?}: {}", level, loc.span, self.message)
        } else {
            write!(f, "{}: {}", level, self.message)
        }
    }
}

/// Convert byte offset to line/column (1-based) given source.
pub fn offset_to_line_col(source: &str, offset: u32) -> (u32, u32) {
    let offset = offset as usize;
    if offset >= source.len() {
        let lines = source.lines().count() as u32;
        let last_line_len = source.lines().last().map(|l| l.len()).unwrap_or(0) as u32;
        return (lines.max(1), last_line_len + 1);
    }
    let mut line = 1u32;
    let mut col = 1u32;
    for (i, c) in source.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Format a diagnostic with source line (for printing).
pub fn format_diagnostic(source: &str, file_id: crate::span::FileId, diag: &Diagnostic) -> String {
    let (level_str, level) = match diag.level {
        Level::Error => ("error", "error"),
        Level::Warning => ("warning", "warning"),
    };
    let loc = match &diag.location {
        Some(l) => l,
        None => return format!("{}: {}", level_str, diag.message),
    };
    let (line, col) = offset_to_line_col(source, loc.span.start);
    let line_content = source
        .lines()
        .nth((line as usize).saturating_sub(1))
        .unwrap_or("");
    let (_, col_end) = offset_to_line_col(source, loc.span.end);
    let underline = if col_end > col && (col_end as usize) <= line_content.len() + 1 {
        " ".repeat((col as usize).saturating_sub(1)) + &"^".repeat((col_end - col) as usize)
    } else {
        " ".repeat((col as usize).saturating_sub(1)) + "^"
    };
    format!(
        "{}:{}:{}: {}: {}\n  {} | {}\n  {} | {}",
        file_id.0, line, col, level, diag.message, line, line_content, line, underline
    )
}
