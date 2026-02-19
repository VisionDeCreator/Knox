//! Minimal IR for Knox: used after lowering and consumed by Wasm codegen.
//! No SSA; simple instruction list per function.

use crate::ast::Type;

/// Whole program: set of functions.
#[derive(Clone, Debug, Default)]
pub struct Program {
    pub functions: Vec<IrFunction>,
    /// Struct layouts in order; layout_id = index.
    pub struct_layouts: Vec<StructLayoutIr>,
    /// String literals for data segment; data_id = index.
    pub string_data: Vec<String>,
}

/// One struct layout: field offsets and total size (for StructAlloc).
#[derive(Clone, Debug)]
pub struct StructLayoutIr {
    pub module: String,
    pub struct_name: String,
    /// (field_name, type, byte_offset)
    pub fields: Vec<(String, Type, u32)>,
    pub total_size: u32,
}

/// Single function in IR.
#[derive(Clone, Debug)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<Type>,
    pub locals: Vec<Type>,
    pub body: Vec<IrInstr>,
}

/// Minimal instruction set for MVP.
#[derive(Clone, Debug)]
pub enum IrInstr {
    ConstInt(i64),
    /// String literal: load ptr and len into the two locals; data_id indexes into Program.string_data.
    ConstString {
        ptr_local: u32,
        len_local: u32,
        data_id: u32,
    },
    LocalGet(u32),
    LocalSet(u32),
    StructAlloc(u32),                 // layout_id
    StructSet(u32, u32, u32),         // ptr_local, field_offset, value_local
    StructSetStr(u32, u32, u32, u32), // ptr_local, field_offset, ptr_val_local, len_val_local
    StructGet(u32, u32, u32),         // ptr_local, field_offset, dest_local (int/bool)
    StructGetStr(u32, u32, u32, u32), // ptr_local, field_offset, ptr_dest, len_dest
    Call(u32),                        // function index (result on stack; use LocalSet after)
    CallStr(u32, u32, u32),           // function index, ptr_dest, len_dest (string return)
    PrintInt(u32),
    PrintStr(u32, u32), // ptr_local, len_local
    Return,
    ReturnInt(u32),
    ReturnStr(u32, u32),
}
