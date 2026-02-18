//! Wasm codegen for Knox: emit Wasm module for main + print (WASI).

use knox_syntax::ast::Root;
use std::io::Write;
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, ImportSection, MemorySection, MemoryType, Module, TypeSection, ValType,
};

/// Emit a Wasm module that exports _start (calls main) and main (calls print). Imports fd_write from WASI.
pub fn emit_wasm(root: &Root, out: &mut impl Write) -> Result<(), String> {
    let mut module = Module::new();

    // Type 0: fd_write (i32 i32 i32 i32) -> i32
    // Type 1: () -> ()
    let type_fd_write = 0u32;
    let type_main = 1u32;

    let mut types = TypeSection::new();
    types.function(
        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        vec![ValType::I32],
    );
    types.function(vec![], vec![]);
    module.section(&types);

    // Import fd_write
    let mut import = ImportSection::new();
    import.import(
        "wasi_snapshot_preview1",
        "fd_write",
        EntityType::Function(type_fd_write),
    );
    module.section(&import);

    // After import: 0 = fd_write. Our funcs: 1 = main, 2 = _start.
    let main_idx = 1u32;
    let start_idx = 2u32;

    let mut funcs = FunctionSection::new();
    funcs.function(type_main); // main
    funcs.function(type_main); // _start
    module.section(&funcs);

    // Memory (must come before Export per Wasm section order)
    let mut mem = MemorySection::new();
    mem.memory(MemoryType {
        minimum: 1,
        maximum: None,
        memory64: false,
        shared: false,
    });
    module.section(&mem);

    // Export memory (required by WASI) and _start. Do not export "main" so the
    // runtime only runs the start function once (_start calls main internally).
    let mut export = ExportSection::new();
    export.export("memory", ExportKind::Memory, 0);
    export.export("_start", ExportKind::Func, start_idx);
    module.section(&export);

    // No Start section: wasmtime invokes _start once by name for WASI. A Start
    // section would also run _start on load, causing double execution.

    // Code section (before Data per Wasm section order)
    let mut code = CodeSection::new();
    code.function(&encode_main());
    code.function(&encode_start(main_idx));
    module.section(&code);

    // Data section (last)
    let string_bytes = get_main_print_string(root);
    let len = string_bytes.len() as u32;
    let mut data_bytes = vec![0u8; 16];
    data_bytes[8..12].copy_from_slice(&16u32.to_le_bytes());
    data_bytes[12..16].copy_from_slice(&len.to_le_bytes());
    data_bytes.extend_from_slice(&string_bytes);
    let mut data = DataSection::new();
    data.active(0, &ConstExpr::i32_const(0), data_bytes);
    module.section(&data);

    let bytes = module.finish();
    out.write_all(&bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn get_main_print_string(root: &Root) -> Vec<u8> {
    for item in &root.items {
        let knox_syntax::ast::Item::Fn(f) = item;
        if f.name == "main" {
            for stmt in &f.body.stmts {
                if let knox_syntax::ast::Stmt::Expr(knox_syntax::ast::Expr::Call {
                    callee,
                    args,
                    ..
                }) = stmt
                {
                    if callee == "print" && args.len() == 1 {
                        if let knox_syntax::ast::Expr::Literal(
                            knox_syntax::ast::Literal::String(s),
                            _,
                        ) = &args[0]
                        {
                            let mut v = s.clone().into_bytes();
                            v.push(b'\n');
                            return v;
                        }
                    }
                }
            }
        }
    }
    b"Hello, Knox!\n".to_vec()
}

fn encode_main() -> Function {
    let mut f = Function::new(vec![]);
    // fd_write(fd=1, iovs=8, iovs_len=1, nwritten_ptr=0)
    f.instruction(&wasm_encoder::Instruction::I32Const(1));
    f.instruction(&wasm_encoder::Instruction::I32Const(8));
    f.instruction(&wasm_encoder::Instruction::I32Const(1));
    f.instruction(&wasm_encoder::Instruction::I32Const(0));
    f.instruction(&wasm_encoder::Instruction::Call(0)); // import 0 = fd_write
    f.instruction(&wasm_encoder::Instruction::Drop);
    f.instruction(&wasm_encoder::Instruction::End);
    f
}

fn encode_start(main_fn_index: u32) -> Function {
    let mut f = Function::new(vec![]);
    f.instruction(&wasm_encoder::Instruction::Call(main_fn_index));
    f.instruction(&wasm_encoder::Instruction::End);
    f
}
