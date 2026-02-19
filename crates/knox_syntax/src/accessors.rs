//! Shared types for accessor desugaring: struct layout and accessor specs.
//! Used by the compiler desugar pass and by codegen to emit getters/setters generically.

use crate::ast::Type;

/// Byte size of a type for struct layout (Wasm ABI: string = ptr+len = 8, int = 4, etc.).
pub fn field_byte_size(ty: &Type) -> u32 {
    match ty {
        Type::String => 8,
        Type::Int => 4,
        Type::Bool => 4,
        Type::Unit => 0,
        Type::Path(_) => 4,
        Type::Ref(_, _) => 4,
    }
}

/// Layout of one struct: ordered list of (field name, type, byte offset from struct start).
#[derive(Clone, Debug)]
pub struct StructLayout {
    pub module: String,
    pub struct_name: String,
    /// (field_name, type, byte_offset)
    pub fields: Vec<(String, Type, u32)>,
    pub total_size: u32,
}

/// Descriptor for one generated accessor (getter or setter).
#[derive(Clone, Debug)]
pub struct AccessorSpec {
    pub module: String,
    pub struct_name: String,
    pub field_name: String,
    pub get: bool,
    pub set: bool,
    pub ty: Type,
    pub byte_offset: u32,
}

/// Setter name in snake_case: set_<field>.
pub fn setter_name(field_name: &str) -> String {
    format!("set_{}", field_name)
}
