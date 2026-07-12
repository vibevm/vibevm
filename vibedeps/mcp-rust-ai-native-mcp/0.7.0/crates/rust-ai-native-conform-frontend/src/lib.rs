//! The Rust T-syn frontend (ENGINE-CONFORM §2): `syn` in-process, the
//! one-page-AST path. Emits the facts the Phase 4 rules consume —
//! items with attribute text, `use` imports, `<Type>::new` construction
//! sites, and `unsafe` uses. B5: an unparseable file yields zero facts
//! rather than an error; the rest of the tree still extracts.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#frontends");

use conform_core::{Fact, Frontend};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;

/// The Rust T-syn [`Frontend`](conform_core::Frontend): parse a source
/// string into conform facts in-process.
///
/// `RustFrontend` is a zero-sized seam — construct it directly and call
/// [`extract`](conform_core::Frontend::extract). Every extraction opens
/// with the file's line metrics, then the tagged items in source order;
/// an unparseable file yields zero such facts (B5), never an error.
///
/// ```
/// use conform_core::Frontend;
/// use rust_ai_native_conform_frontend::RustFrontend;
///
/// let facts = RustFrontend.extract(
///     "lib.rs",
///     "demo",
///     "demo",
///     "pub fn answer() -> u32 { 42 }\n",
/// );
/// assert!(!facts.is_empty()); // at least the FileMetrics fact
/// assert_eq!(RustFrontend.id(), "rust-syn");
///
/// // Unparseable input is tolerated: zero facts, no panic.
/// assert!(RustFrontend.extract("x.rs", "demo", "demo", "fn (").is_empty());
/// ```
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
        // v4: UnwrapUse with fn-grain spec(deviates) scoping.
        // v5: UnsafeUse with the same test/deviates scoping, and
        //     unsafe impl methods extracted (they were invisible).
        // v6: EnvRead facts (env::var/var_os/set_var/remove_var) for the
        //     ambient-env rule, with the same test/deviates scoping.
        "6"
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
            deviating_depth: 0,
        };
        v.visit_file(&ast);
        v.facts.sort_by_key(|f| match f {
            Fact::FileMetrics { .. } => 0,
            Fact::Item { line, .. }
            | Fact::Import { line, .. }
            | Fact::Ctor { line, .. }
            | Fact::UnsafeUse { line, .. }
            | Fact::ErrorVariant { line, .. }
            | Fact::UnwrapUse { line, .. }
            | Fact::EnvRead { line, .. }
            // Never produced by rust-syn — the ts-tsc frontend owns it —
            // but the sort is total over the shared fact model.
            | Fact::TsUnsafe { line, .. } => *line,
        });
        v.facts
    }
}

struct Extractor {
    module: String,
    facts: Vec<Fact>,
    /// Nonzero while visiting a `#[cfg(test)]` module or `#[test]`
    /// fn — `UnwrapUse` facts inside carry `in_test: true`.
    test_depth: u32,
    /// Nonzero while visiting a fn (free or impl method) whose attrs
    /// carry `#[spec(deviates = …)]` — `UnwrapUse` and `UnsafeUse`
    /// facts inside carry `in_deviation: true`. Fn-grain only: a
    /// deviates edge on an impl, struct, or mod records a different
    /// deviation (the solver-choice edges on `Sat` / `NaiveDepSolver`
    /// are the live counter-examples) and grants no amnesty.
    deviating_depth: u32,
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
    attrs
        .iter()
        .any(|a| a.path().segments.last().is_some_and(|s| s.ident == "test"))
}

/// `#[spec(deviates = "…", reason = "…")]` — the verb is the first
/// token inside `spec(...)` (specmark-grammar parses verb-first), so
/// only the `deviates` verb matches; `spec(implements = …)` does not.
fn is_spec_deviates(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        if a.path().segments.last().is_none_or(|s| s.ident != "spec") {
            return false;
        }
        match &a.meta {
            syn::Meta::List(list) => matches!(
                list.tokens.clone().into_iter().next(),
                Some(proc_macro2::TokenTree::Ident(i)) if i == "deviates"
            ),
            _ => false,
        }
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
        let in_test = is_test_fn(&node.attrs) || is_cfg_test(&node.attrs);
        if in_test {
            self.test_depth += 1;
        }
        let deviating = is_spec_deviates(&node.attrs);
        if deviating {
            self.deviating_depth += 1;
        }
        // The decl fact for an `unsafe fn` sees the fn's own test and
        // deviates attrs — push after the depths account for them.
        if node.sig.unsafety.is_some() {
            self.facts.push(Fact::UnsafeUse {
                context: format!("fn {}", node.sig.ident),
                line: line_of(&node.sig.ident),
                in_test: self.test_depth > 0,
                in_deviation: self.deviating_depth > 0,
            });
        }
        syn::visit::visit_item_fn(self, node);
        if deviating {
            self.deviating_depth -= 1;
        }
        if in_test {
            self.test_depth -= 1;
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let deviating = is_spec_deviates(&node.attrs);
        if deviating {
            self.deviating_depth += 1;
        }
        // v5: an `unsafe fn` in an impl block is an unsafe use too —
        // until v4 these were invisible to the gate.
        if node.sig.unsafety.is_some() {
            self.facts.push(Fact::UnsafeUse {
                context: format!("fn {}", node.sig.ident),
                line: line_of(&node.sig.ident),
                in_test: self.test_depth > 0,
                in_deviation: self.deviating_depth > 0,
            });
        }
        syn::visit::visit_impl_item_fn(self, node);
        if deviating {
            self.deviating_depth -= 1;
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
                in_deviation: self.deviating_depth > 0,
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
            // `env::{var,var_os,set_var,remove_var}` — the ambient-env
            // signal. Matches `std::env::var(...)` and `env::var(...)` by
            // the trailing `env::<method>` shape; carries the same
            // test/deviates scoping as `UnwrapUse`.
            if segs.len() >= 2
                && segs[segs.len() - 2] == "env"
                && matches!(
                    segs[segs.len() - 1].as_str(),
                    "var" | "var_os" | "set_var" | "remove_var"
                )
            {
                self.facts.push(Fact::EnvRead {
                    method: segs[segs.len() - 1].clone(),
                    line: line_of(node),
                    in_test: self.test_depth > 0,
                    in_deviation: self.deviating_depth > 0,
                });
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        self.facts.push(Fact::UnsafeUse {
            context: "block".into(),
            line: line_of(node),
            in_test: self.test_depth > 0,
            in_deviation: self.deviating_depth > 0,
        });
        syn::visit::visit_expr_unsafe(self, node);
    }
}

#[cfg(test)]
#[path = "lib/tests.rs"]
mod tests;
