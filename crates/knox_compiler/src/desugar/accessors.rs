//! Desugaring pass: collect struct layouts and generate accessor specs from @pub(get)/@pub(set) annotations.
//! Runs after parsing; output is consumed by codegen to emit getter/setter functions generically.

use knox_syntax::ast::{Item, Root, StructDecl, Visibility};
use knox_syntax::{AccessorSpec, StructLayout};

/// Build layout for an exported struct: field order and byte offsets.
pub fn build_struct_layout(module: &str, s: &StructDecl) -> StructLayout {
    let mut offset = 0u32;
    let mut fields = Vec::with_capacity(s.fields.len());
    for f in &s.fields {
        let size = knox_syntax::field_byte_size(&f.ty);
        fields.push((f.name.clone(), f.ty.clone(), offset));
        offset += size;
    }
    StructLayout {
        module: module.to_string(),
        struct_name: s.name.clone(),
        total_size: offset,
        fields,
    }
}

/// Collect all exported struct layouts and their accessor specs from a module graph.
pub fn collect_struct_layouts_and_accessors(
    deps: &[(String, Root)],
) -> (Vec<StructLayout>, Vec<AccessorSpec>) {
    let mut layouts = Vec::new();
    let mut accessors = Vec::new();

    for (mod_name, root) in deps {
        for item in &root.items {
            if let Item::Struct(s) = item {
                if s.vis != Visibility::Exported {
                    continue;
                }
                let layout = build_struct_layout(mod_name, s);
                let field_attrs: Vec<(String, bool, bool)> = s
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), f.attrs.has_pub_get(), f.attrs.has_pub_set()))
                    .collect();
                for (field_name, get, set) in field_attrs {
                    if !get && !set {
                        continue;
                    }
                    let (_, ty, byte_offset) = layout
                        .fields
                        .iter()
                        .find(|(n, _, _)| n == &field_name)
                        .cloned()
                        .unwrap_or_else(|| (field_name.clone(), knox_syntax::ast::Type::Unit, 0));
                    accessors.push(AccessorSpec {
                        module: mod_name.clone(),
                        struct_name: s.name.clone(),
                        field_name,
                        get,
                        set,
                        ty,
                        byte_offset,
                    });
                }
                layouts.push(layout);
            }
        }
    }
    (layouts, accessors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use knox_syntax::ast::{FieldAttrs, StructField, Type};
    use knox_syntax::span::Span;

    fn span() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn setter_name_snake_case() {
        assert_eq!(knox_syntax::setter_name("age"), "set_age");
        assert_eq!(knox_syntax::setter_name("price"), "set_price");
    }

    #[test]
    fn field_byte_size_types() {
        assert_eq!(knox_syntax::field_byte_size(&Type::String), 8);
        assert_eq!(knox_syntax::field_byte_size(&Type::Int), 4);
        assert_eq!(knox_syntax::field_byte_size(&Type::Bool), 4);
    }

    #[test]
    fn layout_and_accessors_arbitrary_struct() {
        let s = StructDecl {
            span: span(),
            vis: Visibility::Exported,
            name: "Product".to_string(),
            fields: vec![
                StructField {
                    span: span(),
                    name: "id".to_string(),
                    ty: Type::Int,
                    attrs: FieldAttrs {
                        get: true,
                        set: false,
                    },
                },
                StructField {
                    span: span(),
                    name: "price".to_string(),
                    ty: Type::Int,
                    attrs: FieldAttrs {
                        get: true,
                        set: true,
                    },
                },
            ],
        };
        let layout = build_struct_layout("mymod", &s);
        assert_eq!(layout.struct_name, "Product");
        assert_eq!(layout.fields.len(), 2);
        assert_eq!(layout.fields[0].0, "id");
        assert_eq!(layout.fields[0].2, 0);
        assert_eq!(layout.fields[1].0, "price");
        assert_eq!(layout.fields[1].2, 4);
        assert_eq!(layout.total_size, 8);

        let root = Root {
            items: vec![Item::Struct(s.clone())],
        };
        let (layouts, accessors) =
            collect_struct_layouts_and_accessors(&[("mymod".to_string(), root)]);
        assert_eq!(layouts.len(), 1);
        assert_eq!(accessors.len(), 2);
        let id_acc = accessors.iter().find(|a| a.field_name == "id").unwrap();
        assert!(id_acc.get);
        assert!(!id_acc.set);
        let price_acc = accessors.iter().find(|a| a.field_name == "price").unwrap();
        assert!(price_acc.get);
        assert!(price_acc.set);
    }
}
