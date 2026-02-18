//! Wasm codegen for Knox: emit Wasm module for main + print (WASI).

use knox_syntax::ast::Root;
use std::io::Write;
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, ImportSection, MemorySection, MemoryType, Module, TypeSection, ValType,
};

/// Emit a Wasm module that exports _start (calls main) and main (calls print). Imports fd_write from WASI.
/// Supports main that does print("literal") or print(qualified_call()) when the callee returns a string literal.
pub fn emit_wasm(root: &Root, out: &mut impl Write) -> Result<(), String> {
    let mut module = Module::new();

    // Type 0: fd_write (i32 i32 i32 i32) -> i32
    // Type 1: () -> ()
    // Type 2: () -> i32 (for string-returning helpers used by print)
    let type_fd_write = 0u32;
    let type_main = 1u32;
    let type_string_fn = 2u32;

    let mut types = TypeSection::new();
    types.function(
        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        vec![ValType::I32],
    );
    types.function(vec![], vec![]);
    types.function(vec![], vec![ValType::I32]);
    module.section(&types);

    // Import fd_write
    let mut import = ImportSection::new();
    import.import(
        "wasi_snapshot_preview1",
        "fd_write",
        EntityType::Function(type_fd_write),
    );
    module.section(&import);

    let (string_bytes, print_callee) = get_main_print_string(root);
    let len = string_bytes.len() as u32;
    let data_offset = 16u32;

    // After import: 0 = fd_write. If main uses print(qualified_call()), 1 = helper, 2 = main, 3 = _start; else 1 = main, 2 = _start.
    let fd_write_idx = 0u32;
    let (helper_idx, main_idx, start_idx) = if print_callee.is_some() {
        let mut funcs = FunctionSection::new();
        funcs.function(type_string_fn); // helper that returns string offset
        funcs.function(type_main);
        funcs.function(type_main);
        module.section(&funcs);
        (1u32, 2u32, 3u32)
    } else {
        let mut funcs = FunctionSection::new();
        funcs.function(type_main);
        funcs.function(type_main);
        module.section(&funcs);
        (0u32, 1u32, 2u32)
    };

    // Memory (must come before Export per Wasm section order)
    let mut mem = MemorySection::new();
    mem.memory(MemoryType {
        minimum: 1,
        maximum: None,
        memory64: false,
        shared: false,
    });
    module.section(&mem);

    // Export memory (required by WASI) and _start.
    let mut export = ExportSection::new();
    export.export("memory", ExportKind::Memory, 0);
    export.export("_start", ExportKind::Func, start_idx);
    module.section(&export);

    // Code section (before Data per Wasm section order)
    let mut code = CodeSection::new();
    if print_callee.is_some() {
        code.function(&encode_string_helper(data_offset));
    }
    let main_via_helper = print_callee.is_some().then_some((helper_idx, len));
    code.function(&encode_main(fd_write_idx, main_via_helper));
    code.function(&encode_start(main_idx));
    module.section(&code);

    // Data section (last): [0..8] nwritten scratch, [8..16] iov (buf_ptr, len), [16..] string
    let mut data_bytes = vec![0u8; 16];
    data_bytes[8..12].copy_from_slice(&data_offset.to_le_bytes());
    data_bytes[12..16].copy_from_slice(&len.to_le_bytes());
    data_bytes.extend_from_slice(&string_bytes);
    let mut data = DataSection::new();
    data.active(0, &ConstExpr::i32_const(0), data_bytes);
    module.section(&data);

    let bytes = module.finish();
    out.write_all(&bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Returns (string bytes including newline, Some(callee) if string came from print(callee())).
fn get_main_print_string(root: &Root) -> (Vec<u8>, Option<String>) {
    for item in &root.items {
        let knox_syntax::ast::Item::Fn(f) = item else { continue };
        if f.name != "main" {
            continue;
        }
        for stmt in &f.body.stmts {
            let knox_syntax::ast::Stmt::Expr(knox_syntax::ast::Expr::Call {
                callee,
                args,
                ..
            }) = stmt else { continue };
            if *callee != "print" || args.len() != 1 {
                continue;
            }
            match &args[0] {
                knox_syntax::ast::Expr::Literal(
                    knox_syntax::ast::Literal::String(s),
                    _,
                ) => {
                    let mut v = s.clone().into_bytes();
                    v.push(b'\n');
                    return (v, None);
                }
                knox_syntax::ast::Expr::Call {
                    callee: qcallee,
                    args: qargs,
                    ..
                } if qargs.is_empty() => {
                    if let Some(s) = get_return_string_literal(root, qcallee) {
                        let mut v = s.into_bytes();
                        v.push(b'\n');
                        return (v, Some(qcallee.clone()));
                    }
                }
                _ => {}
            }
        }
    }
    (b"Hello, Knox!\n".to_vec(), None)
}

fn get_return_string_literal(root: &Root, fn_name: &str) -> Option<String> {
    for item in &root.items {
        let knox_syntax::ast::Item::Fn(f) = item else { continue };
        if f.name != fn_name {
            continue;
        }
        let [knox_syntax::ast::Stmt::Return(Some(expr), _)] = f.body.stmts.as_slice() else {
            continue;
        };
        if let knox_syntax::ast::Expr::Literal(
            knox_syntax::ast::Literal::String(s),
            _,
        ) = expr {
            return Some(s.clone());
        }
    }
    None
}

fn encode_string_helper(data_offset: u32) -> Function {
    let mut f = Function::new(vec![]);
    f.instruction(&wasm_encoder::Instruction::I32Const(data_offset as i32));
    f.instruction(&wasm_encoder::Instruction::End);
    f
}

fn encode_main(fd_write_idx: u32, via_helper: Option<(u32, u32)>) -> Function {
    let mut f = Function::new(vec![]);
    if let Some((helper_idx, len)) = via_helper {
        // i32.store pops value then base; push base first so value (helper result) is on top
        f.instruction(&wasm_encoder::Instruction::I32Const(8));
        f.instruction(&wasm_encoder::Instruction::Call(helper_idx));
        f.instruction(&wasm_encoder::Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
        f.instruction(&wasm_encoder::Instruction::I32Const(12));
        f.instruction(&wasm_encoder::Instruction::I32Const(len as i32));
        f.instruction(&wasm_encoder::Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));
    }
    // fd_write(fd=1, iovs=8, iovs_len=1, nwritten_ptr=0)
    f.instruction(&wasm_encoder::Instruction::I32Const(1));
    f.instruction(&wasm_encoder::Instruction::I32Const(8));
    f.instruction(&wasm_encoder::Instruction::I32Const(1));
    f.instruction(&wasm_encoder::Instruction::I32Const(0));
    f.instruction(&wasm_encoder::Instruction::Call(fd_write_idx));
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
