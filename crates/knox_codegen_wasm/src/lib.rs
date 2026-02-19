//! Wasm emitter for Knox. Emits WebAssembly (wasm-wasi) from typed AST or from IR.

use knox_syntax::ast::Root;
use knox_syntax::ir::{IrFunction, IrInstr, Program};
use wasm_encoder::{BlockType, *};

fn memarg(align: u32, offset: u64) -> MemArg {
    MemArg {
        offset,
        align,
        memory_index: 0,
    }
}

/// Emit Wasm from IR. Single path: no pattern matching; works for any valid Program.
/// Uses fd_write for print (itoa for int, no NUL bytes). _start calls Knox main.
pub fn emit_from_ir(program: &Program, debug: bool) -> Vec<u8> {
    if debug {
        eprintln!(
            "[KNOX_DEBUG] codegen emit_from_ir: {} functions, {} struct layouts, {} string data",
            program.functions.len(),
            program.struct_layouts.len(),
            program.string_data.len(),
        );
        for (i, l) in program.struct_layouts.iter().enumerate() {
            eprintln!(
                "[KNOX_DEBUG] struct layout {}: {}::{} size={} fields={:?}",
                i,
                l.module,
                l.struct_name,
                l.total_size,
                l.fields
                    .iter()
                    .map(|(n, _, o)| (n.as_str(), o))
                    .collect::<Vec<_>>(),
            );
        }
    }

    // Use second page so itoa/iov don't overlap with any first-page use by host.
    const RUNTIME_BASE: u32 = 8192;
    const ITOA_OFF: u32 = RUNTIME_BASE;
    const IOV_OFF: u32 = ITOA_OFF + 12;
    const NEWLINE_OFF: u32 = IOV_OFF + 16;
    const NWRITTEN_OFF: u32 = NEWLINE_OFF + 4;
    const BUMP_INITIAL: u32 = NWRITTEN_OFF + 4;

    let mut string_offsets: Vec<u32> = Vec::with_capacity(program.string_data.len());
    let mut off = 0u32;
    for s in &program.string_data {
        string_offsets.push(off);
        off += s.len() as u32;
    }
    let _data_len = off;

    let mut module = Module::new();

    let mut types = TypeSection::new();
    types.function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    types.function([ValType::I32], []); // proc_exit
    types.function([ValType::I32], []); // print_int
    types.function([ValType::I32, ValType::I32], []); // print_str
    types.function([], []); // () -> ()
    types.function([ValType::I32], [ValType::I32]); // getter int
    types.function([ValType::I32, ValType::I32], []); // setter
    types.function([ValType::I32], [ValType::I32, ValType::I32]); // getter string
    module.section(&types);

    let mut imports = ImportSection::new();
    imports.import(
        "wasi_snapshot_preview1",
        "fd_write",
        EntityType::Function(0),
    );
    imports.import(
        "wasi_snapshot_preview1",
        "proc_exit",
        EntityType::Function(1),
    );
    module.section(&imports);

    let mut functions = FunctionSection::new();
    functions.function(2); // print_int
    functions.function(3); // print_str
    for f in &program.functions {
        let ty = ir_func_type_index(f);
        functions.function(ty);
    }
    functions.function(4); // _start
    module.section(&functions);

    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: None,
        memory64: false,
        shared: false,
    });
    module.section(&memories);

    let mut globals = wasm_encoder::GlobalSection::new();
    globals.global(
        wasm_encoder::GlobalType {
            val_type: ValType::I32,
            mutable: true,
        },
        &ConstExpr::i32_const(BUMP_INITIAL as i32),
    );
    module.section(&globals);

    let main_idx = 4u32;
    let start_idx = 4 + program.functions.len() as u32;

    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("_start", ExportKind::Func, start_idx);
    module.section(&exports);

    // Start section so wasmtime run invokes _start at instantiation (WASI stdio connected).
    module.section(&StartSection {
        function_index: start_idx,
    });

    let mut codes = CodeSection::new();

    let mut print_int_fn = Function::new([(1, ValType::I32)]);
    emit_print_int_body(
        &mut print_int_fn,
        ITOA_OFF,
        IOV_OFF,
        NEWLINE_OFF,
        NWRITTEN_OFF,
    );
    codes.function(&print_int_fn);

    let mut print_str_fn = Function::new(vec![]);
    emit_print_str_body(&mut print_str_fn, IOV_OFF, NEWLINE_OFF, NWRITTEN_OFF);
    codes.function(&print_str_fn);

    for (ir_idx, f) in program.functions.iter().enumerate() {
        let mut wf = Function::new(
            f.locals
                .iter()
                .map(|_| (1u32, ValType::I32))
                .collect::<Vec<_>>(),
        );
        emit_ir_function(
            f,
            program,
            &string_offsets,
            main_idx + ir_idx as u32,
            &mut wf,
            debug,
        );
        codes.function(&wf);
    }

    let mut start_fn = Function::new(vec![]);
    start_fn.instruction(&Instruction::I32Const(NEWLINE_OFF as i32));
    start_fn.instruction(&Instruction::I32Const(10));
    start_fn.instruction(&Instruction::I32Store8(memarg(0, 0)));
    start_fn.instruction(&Instruction::Call(main_idx));
    start_fn.instruction(&Instruction::I32Const(0));
    start_fn.instruction(&Instruction::Call(1));
    start_fn.instruction(&Instruction::End);
    codes.function(&start_fn);

    module.section(&codes);

    let mut data_bytes = Vec::new();
    for s in &program.string_data {
        data_bytes.extend_from_slice(s.as_bytes());
    }
    if !data_bytes.is_empty() {
        let mut data = DataSection::new();
        data.active(0, &ConstExpr::i32_const(0), data_bytes);
        module.section(&data);
    }

    module.finish()
}

fn ir_func_type_index(f: &IrFunction) -> u32 {
    let p = &f.params;
    let has_return_int = f.body.iter().any(|i| matches!(i, IrInstr::ReturnInt(_)));
    let has_return_str = f.body.iter().any(|i| matches!(i, IrInstr::ReturnStr(_, _)));
    if p.is_empty() {
        4
    } else if p.len() == 1 && has_return_str {
        7
    } else if p.len() == 1 && has_return_int {
        5
    } else if p.len() == 2 {
        6
    } else {
        4
    }
}

fn emit_print_int_body(
    f: &mut wasm_encoder::Function,
    itoa_off: u32,
    iov_off: u32,
    newline_off: u32,
    nwritten_off: u32,
) {
    let d0 = 48i32;
    // Store order: address then value (Wasm spec pops value then address).
    f.instruction(&Instruction::LocalGet(0));
    f.instruction(&Instruction::I32Const(10));
    f.instruction(&Instruction::I32RemS);
    f.instruction(&Instruction::I32Const(d0));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::LocalSet(1));
    f.instruction(&Instruction::I32Const(itoa_off as i32 + 1));
    f.instruction(&Instruction::LocalGet(1));
    f.instruction(&Instruction::I32Store8(memarg(0, 0)));
    f.instruction(&Instruction::LocalGet(0));
    f.instruction(&Instruction::I32Const(10));
    f.instruction(&Instruction::I32DivS);
    f.instruction(&Instruction::I32Const(d0));
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::LocalSet(1));
    f.instruction(&Instruction::I32Const(itoa_off as i32));
    f.instruction(&Instruction::LocalGet(1));
    f.instruction(&Instruction::I32Store8(memarg(0, 0)));
    f.instruction(&Instruction::LocalGet(0));
    f.instruction(&Instruction::I32Const(10));
    f.instruction(&Instruction::I32LtS);
    f.instruction(&Instruction::If(BlockType::Empty));
    f.instruction(&Instruction::I32Const((itoa_off + 1) as i32));
    f.instruction(&Instruction::I32Load8U(memarg(0, 0)));
    f.instruction(&Instruction::LocalSet(1));
    f.instruction(&Instruction::I32Const(itoa_off as i32));
    f.instruction(&Instruction::LocalGet(1));
    f.instruction(&Instruction::I32Store8(memarg(0, 0)));
    f.instruction(&Instruction::End);
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::LocalGet(0));
    f.instruction(&Instruction::I32Const(10));
    f.instruction(&Instruction::I32GeS);
    f.instruction(&Instruction::I32Add);
    f.instruction(&Instruction::LocalTee(1));
    f.instruction(&Instruction::Drop);
    // First fd_write: digits only (1 iov)
    f.instruction(&Instruction::I32Const(iov_off as i32));
    f.instruction(&Instruction::I32Const(itoa_off as i32));
    f.instruction(&Instruction::I32Store(memarg(2, 0)));
    f.instruction(&Instruction::I32Const(iov_off as i32 + 4));
    f.instruction(&Instruction::LocalGet(1));
    f.instruction(&Instruction::I32Store(memarg(2, 0)));
    f.instruction(&Instruction::I32Const(1)); // fd 1 (stdout)
    f.instruction(&Instruction::I32Const(iov_off as i32));
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::I32Const(nwritten_off as i32));
    f.instruction(&Instruction::Call(0));
    f.instruction(&Instruction::Drop);
    // Second fd_write: newline only (1 iov)
    f.instruction(&Instruction::I32Const(iov_off as i32));
    f.instruction(&Instruction::I32Const(newline_off as i32));
    f.instruction(&Instruction::I32Store(memarg(2, 0)));
    f.instruction(&Instruction::I32Const(iov_off as i32 + 4));
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::I32Store(memarg(2, 0)));
    f.instruction(&Instruction::I32Const(1)); // fd 1 (stdout)
    f.instruction(&Instruction::I32Const(iov_off as i32));
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::I32Const(nwritten_off as i32));
    f.instruction(&Instruction::Call(0));
    f.instruction(&Instruction::Drop);
    f.instruction(&Instruction::End);
}

fn emit_print_str_body(
    f: &mut wasm_encoder::Function,
    iov_off: u32,
    newline_off: u32,
    nwritten_off: u32,
) {
    // First fd_write: string only (1 iov)
    f.instruction(&Instruction::I32Const(iov_off as i32));
    f.instruction(&Instruction::LocalGet(0));
    f.instruction(&Instruction::I32Store(memarg(2, 0)));
    f.instruction(&Instruction::I32Const(iov_off as i32 + 4));
    f.instruction(&Instruction::LocalGet(1));
    f.instruction(&Instruction::I32Store(memarg(2, 0)));
    f.instruction(&Instruction::I32Const(1)); // fd 1 (stdout)
    f.instruction(&Instruction::I32Const(iov_off as i32));
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::I32Const(nwritten_off as i32));
    f.instruction(&Instruction::Call(0));
    f.instruction(&Instruction::Drop);
    // Second fd_write: newline only (1 iov)
    f.instruction(&Instruction::I32Const(iov_off as i32));
    f.instruction(&Instruction::I32Const(newline_off as i32));
    f.instruction(&Instruction::I32Store(memarg(2, 0)));
    f.instruction(&Instruction::I32Const(iov_off as i32 + 4));
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::I32Store(memarg(2, 0)));
    f.instruction(&Instruction::I32Const(1)); // fd 1 (stdout)
    f.instruction(&Instruction::I32Const(iov_off as i32));
    f.instruction(&Instruction::I32Const(1));
    f.instruction(&Instruction::I32Const(nwritten_off as i32));
    f.instruction(&Instruction::Call(0));
    f.instruction(&Instruction::Drop);
    f.instruction(&Instruction::End);
}

fn emit_ir_function(
    f: &IrFunction,
    program: &Program,
    string_offsets: &[u32],
    func_base: u32,
    wf: &mut wasm_encoder::Function,
    debug: bool,
) {
    if debug {
        eprintln!(
            "[KNOX_DEBUG] codegen function: {} (params: {}, locals: {})",
            f.name,
            f.params.len(),
            f.locals.len()
        );
    }
    let mut done = false;
    for instr in &f.body {
        if done {
            break;
        }
        match instr {
            IrInstr::ConstInt(v) => {
                wf.instruction(&Instruction::I32Const(*v as i32));
            }
            IrInstr::ConstString {
                ptr_local,
                len_local,
                data_id,
            } => {
                let ptr = string_offsets.get(*data_id as usize).copied().unwrap_or(0);
                let len = program
                    .string_data
                    .get(*data_id as usize)
                    .map(|s| s.len() as i32)
                    .unwrap_or(0);
                wf.instruction(&Instruction::I32Const(ptr as i32));
                wf.instruction(&Instruction::LocalSet(*ptr_local));
                wf.instruction(&Instruction::I32Const(len));
                wf.instruction(&Instruction::LocalSet(*len_local));
            }
            IrInstr::LocalGet(i) => {
                wf.instruction(&Instruction::LocalGet(*i));
            }
            IrInstr::LocalSet(i) => {
                wf.instruction(&Instruction::LocalSet(*i));
            }
            IrInstr::StructAlloc(layout_id) => {
                let size = program
                    .struct_layouts
                    .get(*layout_id as usize)
                    .map(|l| l.total_size)
                    .unwrap_or(0);
                let size_aligned = (size + 3) & !3;
                wf.instruction(&Instruction::GlobalGet(0));
                wf.instruction(&Instruction::GlobalGet(0));
                wf.instruction(&Instruction::I32Const(size_aligned as i32));
                wf.instruction(&Instruction::I32Add);
                wf.instruction(&Instruction::GlobalSet(0));
            }
            IrInstr::StructSet(ptr_local, field_offset, value_local) => {
                wf.instruction(&Instruction::LocalGet(*ptr_local));
                wf.instruction(&Instruction::I32Const(*field_offset as i32));
                wf.instruction(&Instruction::I32Add);
                wf.instruction(&Instruction::LocalGet(*value_local));
                wf.instruction(&Instruction::I32Store(memarg(2, 0)));
            }
            IrInstr::StructSetStr(ptr_local, field_offset, ptr_val_local, len_val_local) => {
                wf.instruction(&Instruction::LocalGet(*ptr_local));
                wf.instruction(&Instruction::I32Const(*field_offset as i32));
                wf.instruction(&Instruction::I32Add);
                wf.instruction(&Instruction::LocalGet(*ptr_val_local));
                wf.instruction(&Instruction::I32Store(memarg(2, 0)));
                wf.instruction(&Instruction::LocalGet(*ptr_local));
                wf.instruction(&Instruction::I32Const(*field_offset as i32 + 4));
                wf.instruction(&Instruction::I32Add);
                wf.instruction(&Instruction::LocalGet(*len_val_local));
                wf.instruction(&Instruction::I32Store(memarg(2, 0)));
            }
            IrInstr::StructGet(ptr_local, field_offset, dest_local) => {
                wf.instruction(&Instruction::LocalGet(*ptr_local));
                wf.instruction(&Instruction::I32Const(*field_offset as i32));
                wf.instruction(&Instruction::I32Add);
                wf.instruction(&Instruction::I32Load(memarg(2, 0)));
                wf.instruction(&Instruction::LocalSet(*dest_local));
            }
            IrInstr::StructGetStr(ptr_local, field_offset, ptr_dest, len_dest) => {
                wf.instruction(&Instruction::LocalGet(*ptr_local));
                wf.instruction(&Instruction::I32Const(*field_offset as i32));
                wf.instruction(&Instruction::I32Add);
                wf.instruction(&Instruction::I32Load(memarg(2, 0)));
                wf.instruction(&Instruction::LocalSet(*ptr_dest));
                wf.instruction(&Instruction::LocalGet(*ptr_local));
                wf.instruction(&Instruction::I32Const(*field_offset as i32 + 4));
                wf.instruction(&Instruction::I32Add);
                wf.instruction(&Instruction::I32Load(memarg(2, 0)));
                wf.instruction(&Instruction::LocalSet(*len_dest));
            }
            IrInstr::Call(ir_idx) => {
                wf.instruction(&Instruction::Call(func_base + ir_idx));
            }
            IrInstr::CallStr(ir_idx, ptr_dest, len_dest) => {
                wf.instruction(&Instruction::Call(func_base + ir_idx));
                wf.instruction(&Instruction::LocalSet(*len_dest));
                wf.instruction(&Instruction::LocalSet(*ptr_dest));
            }
            IrInstr::PrintInt(local) => {
                wf.instruction(&Instruction::LocalGet(*local));
                wf.instruction(&Instruction::Call(2));
            }
            IrInstr::PrintStr(ptr_local, len_local) => {
                wf.instruction(&Instruction::LocalGet(*ptr_local));
                wf.instruction(&Instruction::LocalGet(*len_local));
                wf.instruction(&Instruction::Call(3));
            }
            IrInstr::Return => {
                wf.instruction(&Instruction::End);
                done = true;
            }
            IrInstr::ReturnInt(local) => {
                wf.instruction(&Instruction::LocalGet(*local));
                wf.instruction(&Instruction::End);
                done = true;
            }
            IrInstr::ReturnStr(ptr_local, len_local) => {
                wf.instruction(&Instruction::LocalGet(*ptr_local));
                wf.instruction(&Instruction::LocalGet(*len_local));
                wf.instruction(&Instruction::End);
                done = true;
            }
        }
    }
    if !done {
        wf.instruction(&Instruction::End);
    }
}

/// (Legacy) Emit a single module's AST to Wasm bytes (wasm-wasi). Only supports main() with a single print(string).
/// Prefer the IR pipeline: lower_to_ir + emit_from_ir.
pub fn emit(ast: &Root) -> Vec<u8> {
    let message = extract_print_message(ast);
    let message = message.as_bytes();
    let mut module = Module::new();
    let mut types = TypeSection::new();
    types.function(
        [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    types.function([ValType::I32], []);
    types.function([], []);
    module.section(&types);
    let mut imports = ImportSection::new();
    imports.import(
        "wasi_snapshot_preview1",
        "fd_write",
        EntityType::Function(0),
    );
    imports.import(
        "wasi_snapshot_preview1",
        "proc_exit",
        EntityType::Function(1),
    );
    module.section(&imports);
    let mut functions = FunctionSection::new();
    functions.function(2);
    functions.function(2);
    module.section(&functions);
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: None,
        memory64: false,
        shared: false,
    });
    module.section(&memories);
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("_start", ExportKind::Func, 3);
    module.section(&exports);
    let data_start = 8u32;
    let nwritten_ptr = (data_start + message.len() as u32 + 3) & !3;
    let mut codes = CodeSection::new();
    let mut main_fn = Function::new(vec![]);
    main_fn.instruction(&Instruction::I32Const(0));
    main_fn.instruction(&Instruction::I32Const(8));
    main_fn.instruction(&Instruction::I32Store(memarg(2, 0)));
    main_fn.instruction(&Instruction::I32Const(4));
    main_fn.instruction(&Instruction::I32Const(message.len() as i32));
    main_fn.instruction(&Instruction::I32Store(memarg(2, 0)));
    for (i, &b) in message.iter().enumerate() {
        main_fn.instruction(&Instruction::I32Const(8 + i as i32));
        main_fn.instruction(&Instruction::I32Const(b as i32));
        main_fn.instruction(&Instruction::I32Store8(memarg(0, 0)));
    }
    main_fn.instruction(&Instruction::I32Const(1));
    main_fn.instruction(&Instruction::I32Const(0));
    main_fn.instruction(&Instruction::I32Const(1));
    main_fn.instruction(&Instruction::I32Const(nwritten_ptr as i32));
    main_fn.instruction(&Instruction::Call(0));
    main_fn.instruction(&Instruction::Drop);
    main_fn.instruction(&Instruction::End);
    codes.function(&main_fn);
    let mut start_fn = Function::new(vec![]);
    start_fn.instruction(&Instruction::Call(2));
    start_fn.instruction(&Instruction::I32Const(0));
    start_fn.instruction(&Instruction::Call(1));
    start_fn.instruction(&Instruction::End);
    codes.function(&start_fn);
    module.section(&codes);
    module.finish()
}

fn extract_print_message(ast: &Root) -> String {
    for item in &ast.items {
        if let knox_syntax::ast::Item::Fn(f) = item {
            if f.name == "main" {
                return extract_first_print_string(&f.body);
            }
        }
    }
    "".to_string()
}

fn extract_first_print_string(block: &knox_syntax::ast::Block) -> String {
    for stmt in &block.stmts {
        if let knox_syntax::ast::Stmt::Expr {
            expr:
                knox_syntax::ast::Expr::Call {
                    name,
                    args,
                    receiver: None,
                    ..
                },
            ..
        } = stmt
        {
            if name == "print" && args.len() == 1 {
                if let knox_syntax::ast::Expr::StringLiteral { value, .. } = &args[0] {
                    return value.clone();
                }
            }
        }
    }
    "".to_string()
}
