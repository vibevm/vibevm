//! The Rust T-syn frontend (ENGINE-CONFORM §2): `syn` in-process, the
//! one-page-AST path. Emits the facts the Phase 4 rules consume —
//! items with attribute text, `use` imports, `<Type>::new` construction
//! sites, and `unsafe` uses. B5: an unparseable file yields zero facts
//! rather than an error; the rest of the tree still extracts.

use conform_core::{Fact, Frontend};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;

specmark::scope!("spec://vibevm/neworder/ENGINE-CONFORM-v0.1#frontends");

pub struct RustFrontend;

impl Frontend for RustFrontend {
    fn id(&self) -> &'static str {
        "rust-syn"
    }

    fn version(&self) -> &'static str {
        // Bump when extraction changes shape — the store key includes
        // it, so old cached facts are simply never read again.
        "1"
    }

    fn extract(&self, _file: &str, _crate_name: &str, module: &str, text: &str) -> Vec<Fact> {
        let Ok(ast) = syn::parse_file(text) else {
            return Vec::new();
        };
        let mut v = Extractor {
            module: module.to_string(),
            facts: Vec::new(),
        };
        v.visit_file(&ast);
        v.facts.sort_by_key(|f| match f {
            Fact::Item { line, .. }
            | Fact::Import { line, .. }
            | Fact::Ctor { line, .. }
            | Fact::UnsafeUse { line, .. } => *line,
        });
        v.facts
    }
}

struct Extractor {
    module: String,
    facts: Vec<Fact>,
}

fn attr_text(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter_map(|a| {
            let last = a.path().segments.last()?.ident.to_string();
            match (&a.meta, last.as_str()) {
                (syn::Meta::List(list), "spec" | "cell" | "verifies") => {
                    Some(format!("{last}({})", list.tokens))
                }
                _ => None,
            }
        })
        .collect()
}

fn line_of(spanned: &impl Spanned) -> u32 {
    spanned.span().start().line as u32
}

impl<'ast> Visit<'ast> for Extractor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.facts.push(Fact::Item {
            kind: "fn".into(),
            symbol: format!("{}::{}", self.module, node.sig.ident),
            line: line_of(&node.sig.ident),
            attrs: attr_text(&node.attrs),
        });
        if node.sig.unsafety.is_some() {
            self.facts.push(Fact::UnsafeUse {
                context: format!("fn {}", node.sig.ident),
                line: line_of(&node.sig.ident),
            });
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.facts.push(Fact::Item {
            kind: "struct".into(),
            symbol: format!("{}::{}", self.module, node.ident),
            line: line_of(&node.ident),
            attrs: attr_text(&node.attrs),
        });
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.facts.push(Fact::Item {
            kind: "enum".into(),
            symbol: format!("{}::{}", self.module, node.ident),
            line: line_of(&node.ident),
            attrs: attr_text(&node.attrs),
        });
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.facts.push(Fact::Item {
            kind: "trait".into(),
            symbol: format!("{}::{}", self.module, node.ident),
            line: line_of(&node.ident),
            attrs: attr_text(&node.attrs),
        });
        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        let rendered = node
            .tree
            .to_token_stream()
            .to_string()
            .replace(' ', "")
            .replace(",{", ", {");
        self.facts.push(Fact::Import {
            from_module: self.module.clone(),
            to_path: rendered,
            line: line_of(node),
        });
        syn::visit::visit_item_use(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(p) = node.func.as_ref() {
            let segs: Vec<String> = p
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect();
            if segs.len() >= 2 && segs.last().map(String::as_str) == Some("new") {
                self.facts.push(Fact::Ctor {
                    type_name: segs[segs.len() - 2].clone(),
                    line: line_of(node),
                });
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        self.facts.push(Fact::UnsafeUse {
            context: "block".into(),
            line: line_of(node),
        });
        syn::visit::visit_expr_unsafe(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(src: &str) -> Vec<Fact> {
        RustFrontend.extract("crates/x/src/m.rs", "x", "x::m", src)
    }

    #[test]
    fn extracts_items_with_cell_and_spec_attrs() {
        let facts = extract(
            r#"
            #[cell(seam = "S", variant = "v")]
            #[spec(implements = "spec://p/d#a")]
            pub struct Thing;
            "#,
        );
        let Fact::Item { symbol, attrs, .. } = &facts[0] else {
            panic!("expected item fact, got {facts:?}");
        };
        assert_eq!(symbol, "x::m::Thing");
        assert!(attrs.iter().any(|a| a.starts_with("cell(")));
        assert!(attrs.iter().any(|a| a.starts_with("spec(")));
    }

    #[test]
    fn extracts_imports_ctors_and_unsafe() {
        let facts = extract(
            r#"
            use crate::beta::Beta;
            pub fn build() {
                let _x = Widget::new(1);
                unsafe { core::hint::unreachable_unchecked() }
            }
            pub unsafe fn raw() {}
            "#,
        );
        assert!(
            facts.iter().any(
                |f| matches!(f, Fact::Import { to_path, .. } if to_path == "crate::beta::Beta")
            )
        );
        assert!(
            facts
                .iter()
                .any(|f| matches!(f, Fact::Ctor { type_name, .. } if type_name == "Widget"))
        );
        let unsafes: Vec<_> = facts
            .iter()
            .filter(|f| matches!(f, Fact::UnsafeUse { .. }))
            .collect();
        assert_eq!(unsafes.len(), 2, "block + unsafe fn: {facts:?}");
    }

    #[test]
    fn unparseable_source_yields_no_facts() {
        assert!(extract("pub fn broken( {").is_empty());
    }
}
