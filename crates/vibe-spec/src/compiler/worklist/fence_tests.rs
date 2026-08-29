//! Syntax-aware fences for the T6a discovery substrate
//! (`R4-TRANSFORM-PLAN-ABI-v0.1.md` §6.2): the cells are parsed as an AST
//! with `syn`, so grouped imports, aliases, macros and trait objects are
//! classified structurally — prose cannot smuggle a needle past the fence
//! and a real use cannot be stripped away by a comment.
//!
//! Fence A: production discovery stays behavior-neutral — no
//! transform/registry/behavior/pass vocabulary, no trait-object or boxed
//! error erasure, no `unwrap`/`expect`/`panic` for `E`, and `Infallible`
//! itself stays out (the seam is generic; caller adapters live elsewhere).
//! Fence B (T6b state): the three existing callers propagate the generic
//! discovery error genuinely — no `Infallible` eliminator remains and no
//! adapter body eliminates `E` by `unwrap`/`expect`/`panic`.

use syn::visit::{self, Visit};
use syn::{File, Item};

/// Everything the classifier collects from one parsed cell.
#[derive(Default)]
struct Classified {
    segments: std::collections::BTreeSet<String>,
    macros: std::collections::BTreeSet<String>,
    methods: std::collections::BTreeSet<String>,
    trait_objects: usize,
    /// Call counts for the two adapter spellings the fences reason about.
    infallible_calls: usize,
    discover_calls: usize,
    /// Whether an exhaustive `match impossible { … }` elimination exists.
    exhaustive_elimination: bool,
}

impl<'ast> Visit<'ast> for Classified {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        for segment in &path.segments {
            self.segments.insert(segment.ident.to_string());
        }
        visit::visit_path(self, path);
    }
    /// A `use` tree's identifiers never appear as one whole `syn::Path`, so
    /// the tree is walked explicitly — an import-only use of a forbidden
    /// name must not be invisible to the fence (the registry-kernel fence's
    /// idiom).
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.collect_use_tree(&item.tree);
        visit::visit_item_use(self, item);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(last) = mac.path.segments.last() {
            self.macros.insert(last.ident.to_string());
        }
        visit::visit_macro(self, mac);
    }
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        self.methods.insert(call.method.to_string());
        visit::visit_expr_method_call(self, call);
    }
    fn visit_type_trait_object(&mut self, ty: &'ast syn::TypeTraitObject) {
        self.trait_objects += 1;
        visit::visit_type_trait_object(self, ty);
    }
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = call.func.as_ref()
            && let Some(last) = path.path.segments.last()
        {
            match last.ident.to_string().as_str() {
                "infallible_worklist" => self.infallible_calls += 1,
                "discover" => self.discover_calls += 1,
                _ => {}
            }
        }
        visit::visit_expr_call(self, call);
    }
    fn visit_expr_match(&mut self, expr: &'ast syn::ExprMatch) {
        if let syn::Expr::Path(path) = expr.expr.as_ref()
            && path.path.is_ident("impossible")
        {
            self.exhaustive_elimination = true;
        }
        visit::visit_expr_match(self, expr);
    }
}

impl Classified {
    fn collect_use_tree(&mut self, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(path) => {
                self.segments.insert(path.ident.to_string());
                self.collect_use_tree(&path.tree);
            }
            syn::UseTree::Name(name) => {
                self.segments.insert(name.ident.to_string());
            }
            syn::UseTree::Rename(rename) => {
                // A rename binds BOTH spellings locally, so both enter.
                self.segments.insert(rename.ident.to_string());
                self.segments.insert(rename.rename.to_string());
            }
            syn::UseTree::Group(group) => {
                for nested in &group.items {
                    self.collect_use_tree(nested);
                }
            }
            syn::UseTree::Glob(_) => {}
        }
    }
}

fn classify(source: &str) -> Classified {
    let file: File = syn::parse_file(source).expect("the fenced cell parses as Rust");
    let mut classified = Classified::default();
    classified.visit_file(&file);
    classified
}

/// Classify ONLY the body of the named function, so pre-existing
/// schedule-construction `expect`s elsewhere in the cell do not weaken a
/// rule that is about the adapter itself.
fn classify_fn_body(source: &str, name: &str) -> Classified {
    let file: File = syn::parse_file(source).expect("the fenced cell parses as Rust");
    for item in &file.items {
        if let Item::Fn(function) = item
            && function.sig.ident == name
        {
            let mut classified = Classified::default();
            for stmt in &function.block.stmts {
                classified.visit_stmt(stmt);
            }
            return classified;
        }
    }
    panic!("function `{name}` not found in the fenced cell");
}

const FORBIDDEN_SEGMENTS: &[&str] = &[
    "TransformRegistry",
    "TransformBehavior",
    "transform",
    "registry",
    "behavior",
    "Infallible",
    "Box",
    "Arc",
    "todo",
    "unimplemented",
];

const FORBIDDEN_METHODS: &[&str] = &["unwrap", "expect"];

const FORBIDDEN_MACROS: &[&str] = &["panic", "todo", "unimplemented"];

/// The fence-A verdict, factored so the mutation fixtures below can prove
/// the classifier really turns each banned shape into a violation.
fn first_fence_a_violation(classified: &Classified) -> Option<&'static str> {
    for &needle in FORBIDDEN_SEGMENTS {
        if classified.segments.contains(needle) {
            return Some(needle);
        }
    }
    for &method in FORBIDDEN_METHODS {
        if classified.methods.contains(method) {
            return Some(method);
        }
    }
    for &mac in FORBIDDEN_MACROS {
        if classified.macros.contains(mac) {
            return Some(mac);
        }
    }
    if classified.trait_objects > 0 {
        return Some("dyn");
    }
    None
}

/// Fence A: the production discovery cell is behavior-neutral and keeps `E`
/// unerased.
#[test]
fn worklist_production_stays_behavior_neutral_and_generically_fallible() {
    let classified = classify(include_str!("../worklist.rs"));
    assert_eq!(
        first_fence_a_violation(&classified),
        None,
        "worklist production stays behavior-neutral and keeps `E` unerased"
    );
}

/// Fence B (T6b state): the three `discover` adapters propagate the parse
/// error GENUINELY — T6a's tripwire fired, `infallible_worklist` is gone,
/// and no `Infallible`, `unwrap`, `expect` or panic spelling survives in the
/// adapter bodies, which route `parse_source`'s `Result` straight through.
#[test]
fn caller_adapters_propagate_the_discovery_error_genuinely() {
    let builtin = classify(include_str!("../builtin.rs"));
    let driver = classify(include_str!("../builtin/driver.rs"));

    // Exactly the three historical call sites remain; the eliminator is gone.
    assert_eq!(builtin.discover_calls, 2, "prefix and lane adapters");
    assert_eq!(driver.discover_calls, 1, "the driver `run` adapter");
    assert_eq!(
        builtin.infallible_calls, 0,
        "no eliminator remains in builtin.rs"
    );
    assert_eq!(
        driver.infallible_calls, 0,
        "no eliminator remains in driver.rs"
    );
    for (cell, classified) in [("builtin.rs", &builtin), ("builtin/driver.rs", &driver)] {
        assert!(
            !classified.segments.contains("Infallible"),
            "{cell} names no impossible error type anymore"
        );
    }

    // The adapter bodies themselves: no elimination spelling, and the fallible
    // `parse_source` result really flows through each closure.
    for adapter in ["compile_artifact_prefix", "compile_artifact_lane"] {
        let body = classify_fn_body(include_str!("../builtin.rs"), adapter);
        assert!(
            body.methods.contains("parse_source"),
            "`{adapter}` routes the fallible parse result"
        );
        for &method in FORBIDDEN_METHODS {
            assert!(
                !body.methods.contains(method),
                "`{adapter}` must not use `.{method}()`"
            );
        }
        for &mac in FORBIDDEN_MACROS {
            assert!(
                !body.macros.contains(mac),
                "`{adapter}` must not invoke `{mac}!`"
            );
        }
    }
    let run = classify_fn_body(include_str!("../builtin/driver.rs"), "run");
    assert!(
        run.methods.contains("parse_source"),
        "the driver `run` adapter routes the fallible parse result"
    );
    for &method in FORBIDDEN_METHODS {
        assert!(
            !run.methods.contains(method),
            "`run` must not use `.{method}()`"
        );
    }
    for &mac in FORBIDDEN_MACROS {
        assert!(!run.macros.contains(mac), "`run` must not invoke `{mac}!`");
    }
}

/// Mutation fixtures over synthetic Rust: every banned shape a future edit
/// could introduce is VISIBLE to the classifier under the spelling it would
/// really use — erasure types, elimination calls, panic macros, and
/// transform/registry/behavior vocabulary including grouped and plain
/// imports — so each one turns the fence-A verdict red. Prose needles in
/// comments and string literals stay invisible, which is what keeps the
/// fence syntax-aware rather than a grep.
#[test]
fn fence_fixtures_prove_the_classifier_sees_every_banned_shape() {
    // Erasure: `Box<dyn …>` and `Arc<dyn …>` are seen as both a forbidden
    // segment and a trait object.
    let boxed = classify("fn f(e: Box<dyn std::error::Error>) {}");
    assert!(boxed.segments.contains("Box"));
    assert_eq!(boxed.trait_objects, 1);
    let shared = classify("fn f(e: std::sync::Arc<dyn std::error::Error>) {}");
    assert!(shared.segments.contains("Arc"));
    assert_eq!(shared.trait_objects, 1);
    assert_eq!(first_fence_a_violation(&boxed), Some("Box"));
    assert_eq!(first_fence_a_violation(&shared), Some("Arc"));

    // Elimination spellings land in methods/macros and flip the verdict.
    let unwrapped = classify("fn f(r: Result<u8, E>) { r.unwrap() }");
    assert!(unwrapped.methods.contains("unwrap"));
    assert_eq!(first_fence_a_violation(&unwrapped), Some("unwrap"));
    let expected = classify("fn f(r: Result<u8, E>) { r.expect(\"boom\") }");
    assert!(expected.methods.contains("expect"));
    assert_eq!(first_fence_a_violation(&expected), Some("expect"));
    let panicked = classify("fn f() { panic!(\"boom\"); }");
    assert!(panicked.macros.contains("panic"));
    assert_eq!(first_fence_a_violation(&panicked), Some("panic"));
    let deferred = classify("fn f() { todo!() }");
    assert!(deferred.macros.contains("todo"));
    assert_eq!(first_fence_a_violation(&deferred), Some("todo"));

    // Transform vocabulary: plain and grouped imports, qualified paths and
    // plain identifiers are all seen — including the registry-kernel
    // use-tree walk this fence adopted for exactly this reason.
    for (source, needle) in [
        ("use a::TransformRegistry;", "TransformRegistry"),
        (
            "use b::{TransformBehavior, transform};",
            "TransformBehavior",
        ),
        ("use b::{TransformBehavior, transform};", "transform"),
        ("fn g(r: c::registry) {}", "registry"),
        ("fn h(r: behavior::Thing) {}", "behavior"),
    ] {
        let classified = classify(source);
        assert!(
            classified.segments.contains(needle),
            "the classifier sees `{needle}` in `{source}`"
        );
        assert!(
            first_fence_a_violation(&classified).is_some(),
            "`{needle}` in `{source}` must flip the fence-A verdict"
        );
    }

    // Prose immunity: every needle inside comments and string literals is
    // invisible to the classifier, so documentation cannot trip the fence
    // and a stripped comment cannot hide a real use.
    let prose = classify(
        "fn f() {\n    // TransformRegistry TransformBehavior transform registry behavior\n    \
         // Infallible Box Arc unwrap expect panic todo\n    let documented = \"use \
         a::TransformRegistry; r.unwrap(); panic!(); todo!()\";\n    let _ = documented;\n}\n",
    );
    assert_eq!(
        first_fence_a_violation(&prose),
        None,
        "prose needles stay invisible to the syntax-aware fence"
    );
}
