//! Wasm codegen for Knox: emit Wasm module for main + print (WASI).

use knox_syntax::ast::*;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, ImportSection, Instruction, MemorySection, MemoryType, Module, TypeSection,
    ValType,
};

/// Result of building aux (i32)->(i32) helper functions: list of (name, type_idx, body) and fn_indices.
type AuxFunctionsResult = Result<(Vec<(String, u32, Function)>, HashMap<String, u32>), String>;

/// Emit a Wasm module that exports _start (calls main) and main. Imports fd_write from WASI.
/// Supports simple main (single print) and full main (lets, assignment, arithmetic, print).
pub fn emit_wasm(root: &Root, out: &mut impl Write) -> Result<(), String> {
    let main_fn = root
        .items
        .iter()
        .find_map(|i| match i {
            Item::Fn(f) if f.name == "main" => Some(f),
            _ => None,
        })
        .ok_or("No main function")?;

    let use_simple = is_simple_main(main_fn);
    let (main_func, data_bytes, aux_functions, _fn_indices) = if use_simple {
        let (string_bytes, print_callee) = get_main_print_string(root);
        let len = string_bytes.len() as u32;
        let data_offset = 16u32;
        let main_via_helper = print_callee.is_some().then_some((1u32, len));
        let main_f = encode_main_simple(0u32, main_via_helper);
        let mut data = vec![0u8; 16];
        data[8..12].copy_from_slice(&data_offset.to_le_bytes());
        data[12..16].copy_from_slice(&len.to_le_bytes());
        data.extend_from_slice(&string_bytes);
        (main_f, data, vec![], HashMap::new())
    } else {
        let (aux_functions, fn_indices) = build_aux_functions(root)?;
        let (main_f, data) = build_main_body(main_fn, &fn_indices)?;
        (main_f, data, aux_functions, fn_indices)
    };
    let mut module = Module::new();
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
    let has_aux = !aux_functions.is_empty();
    if has_aux {
        types.function(vec![ValType::I32], vec![ValType::I32]);
    }
    module.section(&types);

    let mut import = ImportSection::new();
    import.import(
        "wasi_snapshot_preview1",
        "fd_write",
        EntityType::Function(type_fd_write),
    );
    module.section(&import);

    let (_helper_idx, main_idx, start_idx) =
        if use_simple && get_main_print_string(root).1.is_some() {
            let mut funcs = FunctionSection::new();
            funcs.function(type_string_fn);
            funcs.function(type_main);
            funcs.function(type_main);
            module.section(&funcs);
            (1u32, 2u32, 3u32)
        } else {
            let mut funcs = FunctionSection::new();
            for (_, type_idx, _) in &aux_functions {
                funcs.function(*type_idx);
            }
            funcs.function(type_main);
            funcs.function(type_main);
            module.section(&funcs);
            let n_aux = aux_functions.len() as u32;
            (0u32, n_aux + 1u32, n_aux + 2u32)
        };

    let mut mem = MemorySection::new();
    mem.memory(MemoryType {
        minimum: 1,
        maximum: None,
        memory64: false,
        shared: false,
    });
    module.section(&mem);

    let mut export = ExportSection::new();
    export.export("memory", ExportKind::Memory, 0);
    export.export("_start", ExportKind::Func, start_idx);
    module.section(&export);

    let mut code = CodeSection::new();
    if use_simple && get_main_print_string(root).1.is_some() {
        let (_string_bytes, _) = get_main_print_string(root);
        code.function(&encode_string_helper(16));
    }
    for (_, _, body) in &aux_functions {
        code.function(body);
    }
    code.function(&main_func);
    code.function(&encode_start(main_idx));
    module.section(&code);

    let mut data_sec = DataSection::new();
    data_sec.active(0, &ConstExpr::i32_const(0), data_bytes);
    module.section(&data_sec);

    let bytes = module.finish();
    out.write_all(&bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn is_simple_main(f: &FnDecl) -> bool {
    if f.body.stmts.len() != 1 {
        return false;
    }
    if let Stmt::Expr(Expr::Call { callee, args, .. }) = &f.body.stmts[0] {
        if callee != "print" || args.len() != 1 {
            return false;
        }
        match &args[0] {
            Expr::Literal(Literal::String(_), _) => true,
            Expr::Call { args: a, .. } if a.is_empty() => true,
            _ => false,
        }
    } else {
        false
    }
}

/// Build (i32)->(i32) helper functions for &mut int (copy-in/copy-out). Returns (name, type_idx, Function) and fn_indices for call emission.
fn build_aux_functions(root: &Root) -> AuxFunctionsResult {
    let type_i32_to_i32 = 3u32;
    let mut out: Vec<(String, u32, Function)> = Vec::new();
    let mut fn_indices: HashMap<String, u32> = HashMap::new();
    for item in &root.items {
        let Item::Fn(f) = item else { continue };
        if f.name == "main" {
            continue;
        }
        // MVP: single param &mut int, returns ()
        if f.params.len() != 1 {
            continue;
        }
        let p = &f.params[0];
        let Type::Ref(inner, is_mut) = &p.ty else {
            continue;
        };
        if !is_mut || !matches!(inner.as_ref(), Type::Int) {
            continue;
        }
        let body = build_aux_function_body(f)?;
        let idx = 1u32 + out.len() as u32;
        fn_indices.insert(f.name.clone(), idx);
        out.push((f.name.clone(), type_i32_to_i32, body));
    }
    Ok((out, fn_indices))
}

fn build_aux_function_body(f: &FnDecl) -> Result<Function, String> {
    let mut local_indices: HashMap<String, u32> = HashMap::new();
    for (i, p) in f.params.iter().enumerate() {
        local_indices.insert(p.name.clone(), i as u32);
    }
    let num_locals = local_indices.len() as u32;
    let ref_params: HashSet<String> = f
        .params
        .iter()
        .filter(|p| matches!(&p.ty, Type::Ref(_, _)))
        .map(|p| p.name.clone())
        .collect();
    let mut instructions: Vec<Instruction> = Vec::new();
    let mut data = vec![0u8; 16];
    let mut data_len = 16u32;
    for stmt in &f.body.stmts {
        emit_stmt(
            stmt,
            &local_indices,
            num_locals,
            &mut instructions,
            &mut data,
            &mut data_len,
            &ref_params,
            &HashMap::new(),
        )?;
    }
    let mut func = Function::new(vec![(num_locals, ValType::I32)]);
    for inst in &instructions {
        func.instruction(inst);
    }
    func.instruction(&Instruction::End);
    Ok(func)
}

fn build_main_body(
    main_fn: &FnDecl,
    fn_indices: &HashMap<String, u32>,
) -> Result<(Function, Vec<u8>), String> {
    let mut local_indices: HashMap<String, u32> = HashMap::new();
    let mut next_local = 0u32;
    for stmt in &main_fn.body.stmts {
        if let Stmt::Let { name, .. } = stmt {
            if !local_indices.contains_key(name) {
                local_indices.insert(name.clone(), next_local);
                next_local += 1;
            }
        }
    }
    let num_locals = next_local;
    let mut instructions: Vec<Instruction> = Vec::new();
    let mut data = vec![0u8; 16];
    let mut data_len = 16u32;

    let ref_params = HashSet::new();
    for stmt in &main_fn.body.stmts {
        emit_stmt(
            stmt,
            &local_indices,
            num_locals,
            &mut instructions,
            &mut data,
            &mut data_len,
            &ref_params,
            fn_indices,
        )?;
    }

    let mut f = Function::new(vec![(num_locals, ValType::I32)]);
    for inst in &instructions {
        f.instruction(inst);
    }
    f.instruction(&Instruction::End);
    Ok((f, data))
}

#[allow(clippy::too_many_arguments)]
fn emit_stmt(
    stmt: &Stmt,
    local_indices: &HashMap<String, u32>,
    num_locals: u32,
    out: &mut Vec<Instruction>,
    data: &mut Vec<u8>,
    data_len: &mut u32,
    ref_params: &HashSet<String>,
    fn_indices: &HashMap<String, u32>,
) -> Result<(), String> {
    match stmt {
        Stmt::Let { name, init, .. } => {
            emit_expr(init, local_indices, num_locals, out)?;
            let idx = *local_indices.get(name).ok_or("unknown let")?;
            out.push(Instruction::LocalSet(idx));
        }
        Stmt::Assign { name, expr, .. } => {
            emit_expr(expr, local_indices, num_locals, out)?;
            let idx = *local_indices.get(name).ok_or("unknown variable")?;
            out.push(Instruction::LocalSet(idx));
        }
        Stmt::AssignDeref { name, expr, .. } => {
            emit_expr(expr, local_indices, num_locals, out)?;
            if ref_params.contains(name) {
                out.push(Instruction::Return);
            } else {
                let idx = *local_indices
                    .get(name)
                    .ok_or("unknown variable for *x = ...")?;
                out.push(Instruction::LocalSet(idx));
            }
        }
        Stmt::Expr(expr) => {
            if let Expr::Call { callee, args, .. } = expr {
                if callee == "print" && args.len() == 1 {
                    emit_print(&args[0], local_indices, num_locals, out, data, data_len)?;
                    return Ok(());
                }
                if let Some(&fn_idx) = fn_indices.get(callee) {
                    for a in args {
                        emit_expr(a, local_indices, num_locals, out)?;
                    }
                    out.push(Instruction::Call(fn_idx));
                    for a in args {
                        if let Expr::Ref {
                            is_mut: true,
                            target,
                            ..
                        } = a
                        {
                            let idx = *local_indices
                                .get(target)
                                .ok_or(format!("Unknown variable: {}", target))?;
                            out.push(Instruction::LocalSet(idx));
                        }
                    }
                    return Ok(());
                }
            }
            emit_expr(expr, local_indices, num_locals, out)?;
            out.push(Instruction::Drop);
        }
        Stmt::Return(_, _) => {}
    }
    Ok(())
}

fn emit_print(
    arg: &Expr,
    _local_indices: &HashMap<String, u32>,
    _num_locals: u32,
    out: &mut Vec<Instruction>,
    data: &mut Vec<u8>,
    data_len: &mut u32,
) -> Result<(), String> {
    match arg {
        Expr::Literal(Literal::String(s), _) => {
            let mut bytes = s.clone().into_bytes();
            bytes.push(b'\n');
            let len = bytes.len() as u32;
            let offset = *data_len;
            data.resize(*data_len as usize + bytes.len(), 0);
            data[offset as usize..].copy_from_slice(&bytes);
            data[8..12].copy_from_slice(&offset.to_le_bytes());
            data[12..16].copy_from_slice(&len.to_le_bytes());
            *data_len += len;
        }
        Expr::Literal(Literal::Int(n), _) => {
            let s = format!("{}\n", n);
            let bytes = s.into_bytes();
            let len = bytes.len() as u32;
            let offset = *data_len;
            data.resize(*data_len as usize + bytes.len(), 0);
            data[offset as usize..].copy_from_slice(&bytes);
            data[8..12].copy_from_slice(&offset.to_le_bytes());
            data[12..16].copy_from_slice(&len.to_le_bytes());
            *data_len += len;
        }
        _ => {
            return Err(
                "print: only string or int literal supported in full main for now".to_string(),
            )
        }
    }
    out.push(Instruction::I32Const(1));
    out.push(Instruction::I32Const(8));
    out.push(Instruction::I32Const(1));
    out.push(Instruction::I32Const(0));
    out.push(Instruction::Call(0));
    out.push(Instruction::Drop);
    Ok(())
}

fn emit_expr(
    expr: &Expr,
    local_indices: &HashMap<String, u32>,
    _num_locals: u32,
    out: &mut Vec<Instruction>,
) -> Result<(), String> {
    match expr {
        Expr::Literal(Literal::Int(n), _) => {
            out.push(Instruction::I32Const(*n as i32));
        }
        Expr::Literal(Literal::Bool(b), _) => {
            out.push(Instruction::I32Const(if *b { 1 } else { 0 }));
        }
        Expr::Literal(_, _) => return Err("Only int/bool in expr for codegen".into()),
        Expr::Ident(name, _) => {
            let idx = *local_indices
                .get(name)
                .ok_or(format!("Unknown variable: {}", name))?;
            out.push(Instruction::LocalGet(idx));
        }
        Expr::BinaryOp {
            op, left, right, ..
        } => {
            emit_expr(left, local_indices, _num_locals, out)?;
            emit_expr(right, local_indices, _num_locals, out)?;
            match op {
                BinOp::Add => out.push(Instruction::I32Add),
                BinOp::Sub => out.push(Instruction::I32Sub),
                BinOp::Mul => out.push(Instruction::I32Mul),
                BinOp::Div => out.push(Instruction::I32DivS),
                BinOp::Rem => out.push(Instruction::I32RemS),
                BinOp::Eq => out.push(Instruction::I32Eq),
                BinOp::Ne => out.push(Instruction::I32Ne),
                BinOp::Lt => out.push(Instruction::I32LtS),
                BinOp::Le => out.push(Instruction::I32LeS),
                BinOp::Gt => out.push(Instruction::I32GtS),
                BinOp::Ge => out.push(Instruction::I32GeS),
                BinOp::And => out.push(Instruction::I32And),
                BinOp::Or => out.push(Instruction::I32Or),
            }
        }
        Expr::UnaryOp { op, expr, .. } => match op {
            UnOp::Neg => {
                out.push(Instruction::I32Const(0));
                emit_expr(expr, local_indices, _num_locals, out)?;
                out.push(Instruction::I32Sub);
            }
            UnOp::Not => {
                emit_expr(expr, local_indices, _num_locals, out)?;
                out.push(Instruction::I32Eqz);
            }
        },
        Expr::Match {
            scrutinee, arms, ..
        } => {
            emit_expr(scrutinee, local_indices, _num_locals, out)?;
            for arm in arms {
                if let MatchPattern::Literal(Literal::Int(n), _) = arm.pattern {
                    out.push(Instruction::I32Const(n as i32));
                    out.push(Instruction::I32Eq);
                    out.push(Instruction::Drop);
                    emit_expr(&arm.body, local_indices, _num_locals, out)?;
                    return Ok(());
                }
            }
            if let Some(arm) = arms.last() {
                if matches!(arm.pattern, MatchPattern::Wildcard(_)) {
                    out.push(Instruction::Drop);
                    emit_expr(&arm.body, local_indices, _num_locals, out)?;
                    return Ok(());
                }
            }
            return Err("Match: only literal and _ in MVP codegen".into());
        }
        Expr::Call { callee, args, .. } => {
            if *callee != "print" {
                for a in args {
                    emit_expr(a, local_indices, _num_locals, out)?;
                }
                return Err(
                    "Function calls other than print() not yet implemented in codegen".to_string(),
                );
            }
            return Err("print() should be handled in emit_stmt".to_string());
        }
        Expr::Ref { target, .. } => {
            let idx = *local_indices
                .get(target)
                .ok_or(format!("Unknown variable: {}", target))?;
            out.push(Instruction::LocalGet(idx));
        }
        Expr::Deref { expr, .. } => {
            emit_expr(expr, local_indices, _num_locals, out)?;
        }
        _ => return Err(format!("Unsupported expr: {:?}", expr)),
    }
    Ok(())
}

/// Returns (string bytes including newline, Some(callee) if string came from print(callee())).
fn get_main_print_string(root: &Root) -> (Vec<u8>, Option<String>) {
    for item in &root.items {
        let knox_syntax::ast::Item::Fn(f) = item else {
            continue;
        };
        if f.name != "main" {
            continue;
        }
        for stmt in &f.body.stmts {
            let knox_syntax::ast::Stmt::Expr(knox_syntax::ast::Expr::Call { callee, args, .. }) =
                stmt
            else {
                continue;
            };
            if *callee != "print" || args.len() != 1 {
                continue;
            }
            match &args[0] {
                knox_syntax::ast::Expr::Literal(knox_syntax::ast::Literal::String(s), _) => {
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
        let knox_syntax::ast::Item::Fn(f) = item else {
            continue;
        };
        if f.name != fn_name {
            continue;
        }
        let [knox_syntax::ast::Stmt::Return(Some(expr), _)] = f.body.stmts.as_slice() else {
            continue;
        };
        if let knox_syntax::ast::Expr::Literal(knox_syntax::ast::Literal::String(s), _) = expr {
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

fn encode_main_simple(fd_write_idx: u32, via_helper: Option<(u32, u32)>) -> Function {
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
