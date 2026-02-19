//! Desugaring passes: accessor generation from @pub(get)/@pub(set), etc.

pub mod accessors;

pub use accessors::collect_struct_layouts_and_accessors;
