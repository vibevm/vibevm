//! The Rust T-syn frontend (ENGINE-CONFORM §2): `syn` in-process, the
//! one-page-AST path. Emits the facts the Phase 4 rules consume —
//! items with attribute text, `use` imports, `<Type>::new` construction
//! sites, and `unsafe` uses. B5: an unparseable file yields zero facts
//! rather than an error; the rest of the tree still extracts.

use conform_core::{Fact, Frontend};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;

specmark::scope!("spec://vibevm/discipline/ENGINE-CONFORM-v0.1#frontends");

pub struct RustFrontend;

impl Frontend for RustFrontend {
    fn id(&self) -> &'static str {
        "rust-syn"
    }

    fn version(&self) -> &'static str {
        // Bump when extraction changes shape — the store key includes
        // it, so old cached facts are simply never read again.
        // v2: is_pub + has_doctest on Item; ErrorVariant facts.
        // v3: FileMetrics per file; UnwrapUse with cfg(test) scoping.
        "3"
    }

    fn extract(&self, _file: &str, _crate_name: &str, module: &str, text: &str) -> Vec<Fact> {
        let Ok(ast) = syn::parse_file(text) else {
            return Vec::new();
        };
        let mut v = Extractor {
            module: module.to_string(),
            facts: vec![Fact::FileMetrics {
                lines: text.lines().count() as u32,
            }],
            test_depth: 0,
        };
        v.visit_file(&ast);
        v.facts.sort_by_key(|f| match f {
            Fact::FileMetrics { .. } => 0,
            Fact::Item { line, .. }
            | Fact::Import { line, .. }
            | Fact::Ctor { line, .. }
            | Fact::UnsafeUse { line, .. }
            | Fact::ErrorVariant { line, .. }
            | Fact::UnwrapUse { line, .. } => *line,
        });
        v.facts
    }
}

struct Extractor {
    module: String,
    facts: Vec<Fact>,
    /// > 0 while visiting a `#[cfg(test)]` module or `#[test]` fn —
    /// `UnwrapUse` facts inside carry `in_test: true`.
    test_depth: u32,
}

/// `#[cfg(test)]` / `#[cfg(any(test, ...))]` — the same shape the
/// specmap ratchet skips.
fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        if !a.path().is_ident("cfg") {
            return false;
        }
        match &a.meta {
            syn::Meta::List(list) => list.tokens.to_string().contains("test"),
            _ => false,
        }
    })
}

/// `#[test]`, `#[tokio::test]`, and friends — the last path segment
/// is `test`.
fn is_test_fn(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path()
            .segments
            .last()
            .is_some_and(|s| s.ident == "test")
    })
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

/// True when the item's doc comment carries a fenced code block — the
/// compiled-doctest signal Class G consumes. rustdoc treats a fence
/// with no language (or `rust`) as a doctest; `text`/`ignore` fences
/// are prose, but distinguishing them is the rule's refinement, not
/// the fact's — the fact records "a fence exists".
fn has_doc_fence(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        if !a.path().is_ident("doc") {
            return false;
        }
        if let syn::Meta::NameValue(nv) = &a.meta
            && let syn::Expr::Lit(lit) = &nv.value
            && let syn::Lit::Str(s) = &lit.lit
        {
            return s.value().trim_start().starts_with("```");
        }
        false
    })
}

fn is_pub(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
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
            is_pub: is_pub(&node.vis),
            has_doctest: has_doc_fence(&node.attrs),
        });
        if node.sig.unsafety.is_some() {
            self.facts.push(Fact::UnsafeUse {
                context: format!("fn {}", node.sig.ident),
                line: line_of(&node.sig.ident),
            });
        }
        let in_test = is_test_fn(&node.attrs) || is_cfg_test(&node.attrs);
        if in_test {
            self.test_depth += 1;
        }
        syn::visit::visit_item_fn(self, node);
        if in_test {
            self.test_depth -= 1;
        }
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        let in_test = is_cfg_test(&node.attrs);
        if in_test {
            self.test_depth += 1;
        }
        syn::visit::visit_item_mod(self, node);
        if in_test {
            self.test_depth -= 1;
        }
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let m = node.method.to_string();
        if m == "unwrap" || m == "expect" {
            self.facts.push(Fact::UnwrapUse {
                method: m,
                line: line_of(&node.method),
                in_test: self.test_depth > 0,
            });
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.facts.push(Fact::Item {
            kind: "struct".into(),
            symbol: format!("{}::{}", self.module, node.ident),
            line: line_of(&node.ident),
            attrs: attr_text(&node.attrs),
            is_pub: is_pub(&node.vis),
            has_doctest: has_doc_fence(&node.attrs),
        });
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.facts.push(Fact::Item {
            kind: "enum".into(),
            symbol: format!("{}::{}", self.module, node.ident),
            line: line_of(&node.ident),
            attrs: attr_text(&node.attrs),
            is_pub: is_pub(&node.vis),
            has_doctest: has_doc_fence(&node.attrs),
        });
        // thiserror variants: #[error("...")] on each variant, the
        // enum's own attrs travel with every variant fact (Class F).
        let enum_attrs = attr_text(&node.attrs);
        for v in &node.variants {
            for a in &v.attrs {
                if !a.path().is_ident("error") {
                    continue;
                }
                let syn::Meta::List(list) = &a.meta else {
                    continue;
                };
                // First string literal in the error(...) tokens is the
                // display template; transparent variants have none.
                let message = list
                    .tokens
                    .clone()
                    .into_iter()
                    .find_map(|t| match t {
                        proc_macro2::TokenTree::Literal(l) => {
                            let s = l.to_string();
                            s.starts_with('"').then(|| s.trim_matches('"').to_string())
                        }
                        _ => None,
                    })
                    .unwrap_or_default();
                self.facts.push(Fact::ErrorVariant {
                    enum_symbol: format!("{}::{}", self.module, node.ident),
                    variant: v.ident.to_string(),
                    message,
                    line: line_of(&v.ident),
                    enum_attrs: enum_attrs.clone(),
                });
            }
        }
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.facts.push(Fact::Item {
            kind: "trait".into(),
            symbol: format!("{}::{}", self.module, node.ident),
            line: line_of(&node.ident),
            attrs: attr_text(&node.attrs),
            is_pub: is_pub(&node.vis),
            has_doctest: has_doc_fence(&node.attrs),
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
        let Some(Fact::Item { symbol, attrs, .. }) =
            facts.iter().find(|f| matches!(f, Fact::Item { .. }))
        else {
            panic!("expected an item fact, got {facts:?}");
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

    #[test]
    fn emits_file_metrics_for_parsed_files() {
        let facts = extract("pub fn a() {}\npub fn b() {}\n");
        assert!(
            facts
                .iter()
                .any(|f| matches!(f, Fact::FileMetrics { lines: 2 })),
            "{facts:?}"
        );
    }

    #[test]
    fn unwrap_in_domain_vs_test_scopes() {
        let facts = extract(
            r#"
            pub fn domain() { Some(1).unwrap(); }
            pub fn hinted() { std::fs::read("x").expect("io"); }
            #[test]
            fn in_test_fn() { Some(1).unwrap(); }
            #[cfg(test)]
            mod tests {
                fn helper() { Some(2).unwrap(); }
            }
            "#,
        );
        let unwraps: Vec<(&str, bool)> = facts
            .iter()
            .filter_map(|f| match f {
                Fact::UnwrapUse {
                    method, in_test, ..
                } => Some((method.as_str(), *in_test)),
                _ => None,
            })
            .collect();
        assert_eq!(
            unwraps,
            vec![
                ("unwrap", false),
                ("expect", false),
                ("unwrap", true),
                ("unwrap", true),
            ],
            "{facts:?}"
        );
    }

    #[test]
    fn extracts_visibility_and_doctest_presence() {
        let facts = extract(
            r#"
            /// Canonical use:
            ///
            /// ```
            /// assert_eq!(1, 1);
            /// ```
            pub fn documented() {}

            /// Prose only.
            pub fn bare() {}

            fn private() {}
            "#,
        );
        let item = |name: &str| {
            facts
                .iter()
                .find_map(|f| match f {
                    Fact::Item {
                        symbol,
                        is_pub,
                        has_doctest,
                        ..
                    } if symbol.ends_with(name) => Some((*is_pub, *has_doctest)),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(item("documented"), (true, true));
        assert_eq!(item("bare"), (true, false));
        assert_eq!(item("private"), (false, false));
    }

    #[test]
    fn extracts_thiserror_variants_with_enum_attrs() {
        let facts = extract(
            r#"
            #[spec(implements = "spec://p/d#err")]
            #[derive(Debug)]
            pub enum Error {
                #[error("file `{0}` missing")]
                Missing(String),
                #[error(transparent)]
                Io(std::io::Error),
            }
            "#,
        );
        let variants: Vec<_> = facts
            .iter()
            .filter_map(|f| match f {
                Fact::ErrorVariant {
                    variant,
                    message,
                    enum_attrs,
                    ..
                } => Some((variant.clone(), message.clone(), enum_attrs.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(variants.len(), 2, "{facts:?}");
        assert_eq!(variants[0].0, "Missing");
        assert!(variants[0].1.contains("missing"));
        assert!(variants[0].2.iter().any(|a| a.starts_with("spec(")));
        // transparent carries no display template
        assert_eq!(variants[1].1, "");
    }
}
