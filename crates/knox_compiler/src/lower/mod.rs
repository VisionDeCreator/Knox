//! Lower AST to IR. Consumes typed AST + layouts + accessors, produces a single Program.

mod to_ir;

pub use to_ir::lower_to_ir;
