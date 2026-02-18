//! Desugar pass: expand @pub(get, set) on struct fields into generated getter/setter methods.

use knox_syntax::ast::*;
use knox_syntax::span::Span;
use std::mem;

/// Convert snake_case to camelCase and prefix with "set" for setter name.
/// e.g. age -> setAge, user_id -> setUserId.
pub fn setter_name(field_name: &str) -> String {
    let mut out = String::from("set");
    let mut cap_next = true;
    for c in field_name.chars() {
        if c == '_' {
            cap_next = true;
        } else if cap_next {
            out.push(c.to_ascii_uppercase());
            cap_next = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Run desugaring: for each struct with @pub(get) or @pub(set) fields, generate
/// getter/setter methods and append them to the module. Struct decls are kept;
/// generated methods are marked pub.
pub fn desugar_root(root: &mut Root) {
    let items = mem::take(&mut root.items);
    let mut new_items = Vec::new();
    for item in items {
        match item {
            Item::Struct(s) => {
                new_items.push(Item::Struct(s.clone()));
                for gen in generate_accessors(&s) {
                    new_items.push(Item::Fn(gen));
                }
            }
            other => new_items.push(other),
        }
    }
    root.items = new_items;
}

fn generate_accessors(s: &StructDecl) -> Vec<FnDecl> {
    let mut out = Vec::new();
    for field in &s.fields {
        let Some(ref attrs) = field.attrs else { continue };
        let span = field.span;
        if attrs.get {
            out.push(generate_getter(s, field, span));
        }
        if attrs.set {
            out.push(generate_setter(s, field, span));
        }
    }
    out
}

fn generate_getter(s: &StructDecl, field: &StructField, span: Span) -> FnDecl {
    let ret_expr = Expr::FieldAccess {
        receiver: Box::new(Expr::Ident("self".to_string(), span)),
        field: field.name.clone(),
        span,
    };
    FnDecl {
        name: field.name.clone(),
        params: vec![Param {
            name: "self".to_string(),
            ty: Type::Named(s.name.clone()),
            span,
        }],
        return_type: field.ty.clone(),
        body: Block {
            stmts: vec![Stmt::Return(Some(ret_expr), span)],
            span,
        },
        span,
        pub_vis: true,
    }
}

fn generate_setter(s: &StructDecl, field: &StructField, span: Span) -> FnDecl {
    let setter_name = setter_name(&field.name);
    FnDecl {
        name: setter_name,
        params: vec![
            Param {
                name: "self".to_string(),
                ty: Type::Named(s.name.clone()),
                span,
            },
            Param {
                name: "v".to_string(),
                ty: field.ty.clone(),
                span,
            },
        ],
        return_type: Type::Unit,
        body: Block {
            stmts: vec![Stmt::Return(Some(Expr::Literal(Literal::Unit, span)), span)],
            span,
        },
        span,
        pub_vis: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setter_name_camel_case() {
        assert_eq!(setter_name("age"), "setAge");
        assert_eq!(setter_name("user_id"), "setUserId");
    }
}
