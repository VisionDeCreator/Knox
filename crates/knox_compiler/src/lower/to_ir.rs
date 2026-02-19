//! Lower AST to IR. One pass over main + accessors; produces Program.

use knox_syntax::ast::{Block, Expr, Item, Root, Stmt, Type};
use knox_syntax::{AccessorSpec, StructLayout};
use std::collections::HashMap;

use crate::ir::{IrFunction, IrInstr, Program, StructLayoutIr};

/// Lower main module + deps + layouts + accessors into a single IR Program.
/// Function index 0 = main; then getters/setters in deterministic order.
pub fn lower_to_ir(
    main_root: &Root,
    deps: &[(String, Root)],
    layouts: &[StructLayout],
    accessors: &[AccessorSpec],
) -> Result<Program, String> {
    let mut program = Program::default();

    // 1. Struct layouts
    for l in layouts {
        program.struct_layouts.push(StructLayoutIr {
            module: l.module.clone(),
            struct_name: l.struct_name.clone(),
            fields: l.fields.clone(),
            total_size: l.total_size,
        });
    }

    // 2. Function order: main first, then accessors (getters then setters, sorted by module/struct/field)
    let main_fn = main_root
        .items
        .iter()
        .find_map(|i| {
            if let Item::Fn(f) = i {
                if f.name == "main" {
                    return Some(f);
                }
            }
            None
        })
        .ok_or_else(|| "main function not found".to_string())?;

    // One entry per getter and per setter (a field with both get and set yields two entries).
    let mut accessor_list: Vec<(&AccessorSpec, bool)> = accessors
        .iter()
        .flat_map(|a| {
            let mut v = Vec::new();
            if a.get {
                v.push((a, true));
            }
            if a.set {
                v.push((a, false));
            }
            v
        })
        .collect();
    accessor_list.sort_by(|(a, is_get), (b, is_get_b)| {
        (&a.module, &a.struct_name, !*is_get, &a.field_name).cmp(&(
            &b.module,
            &b.struct_name,
            !*is_get_b,
            &b.field_name,
        ))
    });

    let mut func_index: HashMap<(String, String, String, bool), u32> = HashMap::new();
    let mut idx = 0u32;
    idx += 1; // main = 0
    for (a, is_getter) in &accessor_list {
        let key = (
            a.module.clone(),
            a.struct_name.clone(),
            a.field_name.clone(),
            *is_getter,
        );
        func_index.insert(key, idx);
        idx += 1;
    }

    let layout_id: HashMap<(String, String), u32> = program
        .struct_layouts
        .iter()
        .enumerate()
        .map(|(i, l)| ((l.module.clone(), l.struct_name.clone()), i as u32))
        .collect();

    // 3. Lower main
    let main_ir = lower_function(
        main_fn.name.as_str(),
        &main_fn.params,
        &main_fn.body,
        deps,
        &layout_id,
        &program.struct_layouts,
        &func_index,
        &mut program.string_data,
    )?;
    program.functions.push(main_ir);

    // 4. Lower accessors
    for (a, is_getter) in &accessor_list {
        let layout_id = *layout_id
            .get(&(a.module.clone(), a.struct_name.clone()))
            .ok_or_else(|| format!("layout not found for {}/{}", a.module, a.struct_name))?;
        let (_, _, byte_offset) = program.struct_layouts[layout_id as usize]
            .fields
            .iter()
            .find(|(n, _, _)| n == &a.field_name)
            .cloned()
            .unwrap_or_else(|| (a.field_name.clone(), a.ty.clone(), 0));

        let body = if *is_getter {
            if matches!(a.ty, Type::String) {
                vec![
                    IrInstr::StructGetStr(0, byte_offset, 1, 2),
                    IrInstr::ReturnStr(1, 2),
                ]
            } else {
                vec![IrInstr::StructGet(0, byte_offset, 1), IrInstr::ReturnInt(1)]
            }
        } else {
            vec![IrInstr::StructSet(0, byte_offset, 1), IrInstr::Return]
        };

        let params = if *is_getter {
            vec![Type::Int] // ptr
        } else {
            vec![Type::Int, Type::Int] // ptr, value
        };
        let locals: Vec<Type> = if *is_getter {
            if matches!(a.ty, Type::String) {
                vec![Type::Int, Type::Int] // ptr_dest, len_dest
            } else {
                vec![Type::Int]
            }
        } else {
            vec![]
        };

        let name = if *is_getter {
            format!("{}_{}_{}", a.module, a.struct_name, a.field_name)
        } else {
            format!("{}_{}_set_{}", a.module, a.struct_name, a.field_name)
        };
        program.functions.push(IrFunction {
            name,
            params,
            locals,
            body,
        });
    }

    Ok(program)
}

/// (module, struct) for a variable's type (e.g. p -> ("product", "Product")).
type VarType = (String, String);

/// Lower one function body. Tracks local indices: params 0..params.len(), then new locals for lets and temps.
fn lower_function(
    name: &str,
    params: &[knox_syntax::ast::Param],
    body: &Block,
    deps: &[(String, Root)],
    layout_id: &HashMap<(String, String), u32>,
    struct_layouts: &[StructLayoutIr],
    func_index: &HashMap<(String, String, String, bool), u32>,
    string_data: &mut Vec<String>,
) -> Result<IrFunction, String> {
    let mut instructions = Vec::new();
    let mut local_types: Vec<Type> = params.iter().map(|p| p.ty.clone()).collect();
    let mut var_to_local: HashMap<String, u32> = params
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.clone(), i as u32))
        .collect();
    let mut var_to_type: HashMap<String, VarType> = HashMap::new();

    let mut next_local = |local_types: &mut Vec<Type>| {
        let idx = local_types.len() as u32;
        local_types.push(Type::Int); // temps and ptrs are i32 in wasm
        idx
    };

    for stmt in &body.stmts {
        lower_stmt(
            stmt,
            &mut instructions,
            &mut var_to_local,
            &mut var_to_type,
            &mut local_types,
            &mut next_local,
            deps,
            layout_id,
            struct_layouts,
            func_index,
            string_data,
        )?;
    }

    instructions.push(IrInstr::Return);

    Ok(IrFunction {
        name: name.to_string(),
        params: params.iter().map(|p| p.ty.clone()).collect(),
        locals: local_types.split_off(params.len()),
        body: instructions,
    })
}

fn lower_stmt(
    stmt: &Stmt,
    out: &mut Vec<IrInstr>,
    var_to_local: &mut HashMap<String, u32>,
    var_to_type: &mut HashMap<String, VarType>,
    local_types: &mut Vec<Type>,
    mut next_local: &mut dyn FnMut(&mut Vec<Type>) -> u32,
    deps: &[(String, Root)],
    layout_id: &HashMap<(String, String), u32>,
    struct_layouts: &[StructLayoutIr],
    func_index: &HashMap<(String, String, String, bool), u32>,
    string_data: &mut Vec<String>,
) -> Result<(), String> {
    match stmt {
        Stmt::Let { name, init, .. } => {
            let local = next_local(local_types);
            var_to_local.insert(name.clone(), local);
            if let Expr::StructLiteral { path, .. } = init {
                if path.len() == 2 {
                    var_to_type.insert(name.clone(), (path[0].clone(), path[1].clone()));
                }
            }
            lower_expr_to_local(
                init,
                local,
                out,
                local_types,
                &mut next_local,
                deps,
                layout_id,
                struct_layouts,
                func_index,
                string_data,
                var_to_type,
                var_to_local,
            )?;
        }
        Stmt::Expr { expr, .. } => {
            let tmp = next_local(local_types);
            lower_expr_to_local(
                expr,
                tmp,
                out,
                local_types,
                &mut next_local,
                deps,
                layout_id,
                struct_layouts,
                func_index,
                string_data,
                var_to_type,
                var_to_local,
            )?;
        }
        Stmt::Return { value, .. } => {
            if let Some(expr) = value {
                let tmp = next_local(local_types);
                lower_expr_to_local(
                    expr,
                    tmp,
                    out,
                    local_types,
                    &mut next_local,
                    deps,
                    layout_id,
                    struct_layouts,
                    func_index,
                    string_data,
                    var_to_type,
                    var_to_local,
                )?;
                out.push(IrInstr::ReturnInt(tmp));
            } else {
                out.push(IrInstr::Return);
            }
            return Ok(());
        }
    }
    Ok(())
}

/// Lower expr and ensure its value ends up in dest_local. May push instructions that leave value in dest.
fn lower_expr_to_local(
    expr: &Expr,
    dest_local: u32,
    out: &mut Vec<IrInstr>,
    local_types: &mut Vec<Type>,
    next_local: &mut dyn FnMut(&mut Vec<Type>) -> u32,
    deps: &[(String, Root)],
    layout_id: &HashMap<(String, String), u32>,
    struct_layouts: &[StructLayoutIr],
    func_index: &HashMap<(String, String, String, bool), u32>,
    string_data: &mut Vec<String>,
    var_to_type: &HashMap<String, VarType>,
    var_to_local: &HashMap<String, u32>,
) -> Result<(), String> {
    match expr {
        Expr::IntLiteral { value, .. } => {
            out.push(IrInstr::ConstInt(*value));
            out.push(IrInstr::LocalSet(dest_local));
        }
        Expr::StringLiteral { value, .. } => {
            let data_id = string_data.len() as u32;
            string_data.push(value.clone());
            let len_local = next_local(local_types);
            out.push(IrInstr::ConstString {
                ptr_local: dest_local,
                len_local,
                data_id,
            });
        }
        Expr::StructLiteral { path, fields, .. } => {
            if path.len() != 2 {
                return Err("struct literal path must be module::Struct".to_string());
            }
            let key = (path[0].clone(), path[1].clone());
            let lid = *layout_id
                .get(&key)
                .ok_or_else(|| format!("layout not found for {}::{}", path[0], path[1]))?;
            let layout = struct_layouts
                .iter()
                .find(|l| l.module == key.0 && l.struct_name == key.1)
                .ok_or_else(|| format!("struct layout not found for {}::{}", key.0, key.1))?;
            out.push(IrInstr::StructAlloc(lid));
            out.push(IrInstr::LocalSet(dest_local));
            for (fname, fexpr) in fields {
                let (_, fty, offset) = layout
                    .fields
                    .iter()
                    .find(|(n, _, _)| n == fname)
                    .cloned()
                    .ok_or_else(|| format!("field {} not in {}::{}", fname, key.0, key.1))?;
                if matches!(fty, Type::String) {
                    let ptr_local = next_local(local_types);
                    let len_local = next_local(local_types);
                    if let Expr::StringLiteral { value, .. } = fexpr {
                        let data_id = string_data.len() as u32;
                        string_data.push(value.clone());
                        out.push(IrInstr::ConstString {
                            ptr_local,
                            len_local,
                            data_id,
                        });
                        out.push(IrInstr::StructSetStr(dest_local, offset, ptr_local, len_local));
                    } else {
                        return Err("string field in struct literal must be a string literal".to_string());
                    }
                } else {
                    let val_local = next_local(local_types);
                    lower_expr_to_local(
                        fexpr,
                        val_local,
                        out,
                        local_types,
                        next_local,
                        deps,
                        layout_id,
                        struct_layouts,
                        func_index,
                        string_data,
                        var_to_type,
                        var_to_local,
                    )?;
                    out.push(IrInstr::StructSet(dest_local, offset, val_local));
                }
            }
        }
        Expr::Ident { name, .. } => {
            let &local = var_to_local
                .get(name)
                .ok_or_else(|| format!("variable not found: {}", name))?;
            out.push(IrInstr::LocalGet(local));
            out.push(IrInstr::LocalSet(dest_local));
        }
        Expr::Call {
            receiver,
            name,
            args,
            ..
        } => {
            if name == "print" && args.len() == 1 && receiver.is_none() {
                match &args[0] {
                    Expr::StringLiteral { value, .. } => {
                        let ptr_local = next_local(local_types);
                        let len_local = next_local(local_types);
                        let data_id = string_data.len() as u32;
                        string_data.push(value.clone());
                        out.push(IrInstr::ConstString {
                            ptr_local,
                            len_local,
                            data_id,
                        });
                        out.push(IrInstr::PrintStr(ptr_local, len_local));
                    }
                    Expr::Call {
                        receiver: Some(receiver_expr),
                        name: method,
                        args: margs,
                        ..
                    } if margs.is_empty() => {
                        let arg_local = next_local(local_types);
                        let rec_local = next_local(local_types);
                        lower_expr_to_local(
                            receiver_expr,
                            rec_local,
                            out,
                            local_types,
                            next_local,
                            deps,
                            layout_id,
                            struct_layouts,
                            func_index,
                            string_data,
                            var_to_type,
                            var_to_local,
                        )?;
                        let (mod_name, struct_name) =
                            resolve_receiver_type(receiver_expr, var_to_type)?;
                        let key_get = (mod_name.clone(), struct_name.clone(), method.clone(), true);
                        let &idx = func_index.get(&key_get).ok_or_else(|| {
                            format!("getter not found: {} for {}", method, struct_name)
                        })?;
                        let is_string = is_getter_string(deps, &key_get.0, &key_get.1, method);
                        if is_string {
                            let len_local = next_local(local_types);
                            out.push(IrInstr::LocalGet(rec_local));
                            out.push(IrInstr::CallStr(idx, arg_local, len_local));
                            out.push(IrInstr::PrintStr(arg_local, len_local));
                        } else {
                            out.push(IrInstr::LocalGet(rec_local));
                            out.push(IrInstr::Call(idx));
                            out.push(IrInstr::LocalSet(arg_local));
                            out.push(IrInstr::PrintInt(arg_local));
                        }
                    }
                    _ => {
                        let arg_local = next_local(local_types);
                        lower_expr_to_local(
                            &args[0],
                            arg_local,
                            out,
                            local_types,
                            next_local,
                            deps,
                            layout_id,
                            struct_layouts,
                            func_index,
                            string_data,
                            var_to_type,
                            var_to_local,
                        )?;
                        out.push(IrInstr::PrintInt(arg_local));
                    }
                }
                return Ok(());
            }

            if let Some(receiver) = receiver {
                if name.starts_with("set_") && args.len() == 1 {
                    let field = name.strip_prefix("set_").unwrap_or(name);
                    let rec_local = next_local(local_types);
                    lower_expr_to_local(
                        receiver,
                        rec_local,
                        out,
                        local_types,
                        next_local,
                        deps,
                        layout_id,
                        struct_layouts,
                        func_index,
                        string_data,
                        var_to_type,
                        var_to_local,
                    )?;
                    let val_local = next_local(local_types);
                    lower_expr_to_local(
                        &args[0],
                        val_local,
                        out,
                        local_types,
                        next_local,
                        deps,
                        layout_id,
                        struct_layouts,
                        func_index,
                        string_data,
                        var_to_type,
                        var_to_local,
                    )?;
                    let (mod_name, struct_name) = resolve_receiver_type(receiver, var_to_type)?;
                    let key_set = (mod_name, struct_name.clone(), field.to_string(), false);
                    let &idx = func_index.get(&key_set).ok_or_else(|| {
                        format!("setter not found: set_{} for {}", field, struct_name)
                    })?;
                    out.push(IrInstr::LocalGet(rec_local));
                    out.push(IrInstr::LocalGet(val_local));
                    out.push(IrInstr::Call(idx));
                    return Ok(());
                }
                let (mod_name, struct_name) = resolve_receiver_type(receiver, var_to_type)?;
                let key_get = (mod_name.clone(), struct_name.clone(), name.clone(), true);
                if let Some(&idx) = func_index.get(&key_get) {
                    let rec_local = next_local(local_types);
                    lower_expr_to_local(
                        receiver,
                        rec_local,
                        out,
                        local_types,
                        next_local,
                        deps,
                        layout_id,
                        struct_layouts,
                        func_index,
                        string_data,
                        var_to_type,
                        var_to_local,
                    )?;
                    let is_string = is_getter_string(deps, &mod_name, &struct_name, name);
                    if is_string {
                        let len_local = next_local(local_types);
                        out.push(IrInstr::LocalGet(rec_local));
                        out.push(IrInstr::CallStr(idx, dest_local, len_local));
                    } else {
                        out.push(IrInstr::LocalGet(rec_local));
                        out.push(IrInstr::Call(idx));
                        out.push(IrInstr::LocalSet(dest_local));
                    }
                    return Ok(());
                }
            }

            return Err(format!(
                "unsupported call: {} (receiver: {:?})",
                name, receiver
            ));
        }
        _ => return Err(format!("unsupported expression: {:?}", expr)),
    }
    Ok(())
}

fn resolve_receiver_type(
    receiver: &Expr,
    var_to_type: &HashMap<String, VarType>,
) -> Result<(String, String), String> {
    match receiver {
        Expr::Ident { name, .. } => var_to_type.get(name).cloned().ok_or_else(|| {
            format!(
                "variable '{}' has unknown type (not a struct literal?)",
                name
            )
        }),
        _ => Err("receiver must be ident".to_string()),
    }
}

fn is_getter_string(
    deps: &[(String, Root)],
    module: &str,
    struct_name: &str,
    field_name: &str,
) -> bool {
    for (mod_name, root) in deps.iter() {
        if mod_name.as_str() != module {
            continue;
        }
        for item in &root.items {
            if let Item::Struct(s) = item {
                if s.name == struct_name {
                    for f in &s.fields {
                        if f.name == field_name {
                            return matches!(f.ty, Type::String);
                        }
                    }
                }
            }
        }
    }
    false
}
