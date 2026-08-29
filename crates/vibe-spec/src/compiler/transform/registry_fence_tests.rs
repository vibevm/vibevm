//! The T5 registry/behavior syntax fence (R4-TRANSFORM-PLAN-ABI §6.1), split
//! from `plan_fence_tests` along its file-length seam: the two new behavior
//! cells are parsed as an AST with `syn` so grouped/renamed imports,
//! fully-qualified paths, type paths and macros are classified structurally —
//! prose and string literals never reach the AST and never trip the fence.
//!
//! Four laws live here: the new cells admit no manifest/row/collector/
//! filesystem/codec/Display surface, no aliased/globbed/qualified `std::path`
//! sequence, and no `Box` — `Arc<dyn …>` trait objects are the ONE legal
//! behavior channel, exactly there; the existing plan cells stay
//! `Arc`/`Box`/`dyn`-free; `plan.rs` carries no registry lookup; the crate
//! root reexports nothing of the registry; the reusable test catalog module
//! is `#[cfg(test)]`-gated in the module tree.
//!
//! The manifest dependency sets were a FIFTH law here until R4.2 gave them a
//! single home in `dependency_dag_fence_tests`.

use std::collections::BTreeSet;

use syn::UseTree;
use syn::visit::{self, Visit};

/// Flatten one `use` tree into complete imported paths (the established
/// fence idiom): grouped and nested trees never appear as one whole path, a
/// rename binds both spellings, a glob still names its whole prefix.
fn flatten_use_tree(tree: &UseTree, mut prefix: Vec<String>, out: &mut Vec<Vec<String>>) {
    match tree {
        UseTree::Name(name) => {
            prefix.push(name.ident.to_string());
            out.push(prefix);
        }
        UseTree::Rename(rename) => {
            prefix.push(rename.ident.to_string());
            prefix.push(rename.rename.to_string());
            out.push(prefix);
        }
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            flatten_use_tree(&path.tree, prefix, out);
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use_tree(item, prefix.clone(), out);
            }
        }
        UseTree::Glob(_) => out.push(prefix),
    }
}

/// Everything the AST classifier collects from one parsed source.
#[derive(Default)]
struct Classified {
    segments: BTreeSet<String>,
    imports: Vec<Vec<String>>,
    extern_crates: Vec<String>,
    path_sequences: Vec<Vec<String>>,
    macros: BTreeSet<String>,
    methods: BTreeSet<String>,
    trait_objects: usize,
}

/// Whether one identifier sequence begins with the `std::path` module
/// sequence — exact segment match, so `std::pathological` never trips it.
fn is_std_path_sequence(segments: &[String]) -> bool {
    segments.len() >= 2 && segments[0] == "std" && segments[1] == "path"
}

impl<'ast> Visit<'ast> for Classified {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        let sequence: Vec<String> = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        for segment in &sequence {
            self.segments.insert(segment.clone());
        }
        self.path_sequences.push(sequence);
        visit::visit_path(self, path);
    }
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        flatten_use_tree(&item.tree, Vec::new(), &mut self.imports);
        visit::visit_item_use(self, item);
    }
    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        self.extern_crates.push(item.ident.to_string());
        visit::visit_item_extern_crate(self, item);
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
    fn visit_expr_field(&mut self, field: &'ast syn::ExprField) {
        // A named field access (`self.registry`) binds a name just as
        // reachable as an import, so it enters the segment set.
        if let syn::Member::Named(ident) = &field.member {
            self.segments.insert(ident.to_string());
        }
        visit::visit_expr_field(self, field);
    }
    fn visit_type_trait_object(&mut self, ty: &'ast syn::TypeTraitObject) {
        self.trait_objects += 1;
        visit::visit_type_trait_object(self, ty);
    }
}

/// One cell's fence rules.
struct CellRules {
    /// Identifiers forbidden as path/import segments.
    forbidden_segments: &'static [&'static str],
    /// Macro names forbidden outright.
    forbidden_macros: &'static [&'static str],
    /// Method names forbidden outright.
    forbidden_methods: &'static [&'static str],
    /// Whether `dyn` trait objects are legal in this cell: they are, exactly
    /// in the two behavior cells.
    allows_trait_objects: bool,
}

/// The behavior cells' forbidden segments: the manifest/row/collector/
/// filesystem/codec/Display surface every transform cell refuses, PLUS
/// `Box` — the ABI's one behavior-ownership channel is
/// `Arc<dyn TransformBehavior>`, so an alternative Box channel is refused
/// even inside the two cells allowed to hold behavior objects.
const BEHAVIOR_SEGMENTS: &[&str] = &[
    "serde",
    "serde_json",
    "toml",
    "json",
    "Path",
    "PathBuf",
    "fs",
    "ExtensionRegistryRow",
    "RegistryView",
    "ExtensionRegistry",
    "collect_extensions",
    "Display",
    "fmt",
    "Box",
];

/// The behavior cells additionally refuse rendered identity: no formatting
/// macro of any spelling and no string-conversion method.
const BEHAVIOR_MACROS: &[&str] = &["format", "write", "format_args", "writeln"];
const BEHAVIOR_METHODS: &[&str] = &["to_string"];

const BEHAVIOR_RULES: CellRules = CellRules {
    forbidden_segments: BEHAVIOR_SEGMENTS,
    forbidden_macros: BEHAVIOR_MACROS,
    forbidden_methods: BEHAVIOR_METHODS,
    allows_trait_objects: true,
};

const PLAN_CARRIER_RULES: CellRules = CellRules {
    forbidden_segments: &["Arc", "Box"],
    forbidden_macros: &[],
    forbidden_methods: &[],
    allows_trait_objects: false,
};

const PLAN_LOOKUP_RULES: CellRules = CellRules {
    forbidden_segments: &["TransformRegistry", "TransformBehavior", "registry"],
    forbidden_macros: &[],
    forbidden_methods: &["resolve", "register", "catalog"],
    allows_trait_objects: false,
};

/// Classify one source under one cell's rules; an unparsable source reports
/// itself as the offender so the fence names the file, never aborts.
fn offenders(source: &str, rules: &CellRules) -> Vec<String> {
    let Ok(file) = syn::parse_file(source) else {
        return vec!["<unparsable source>".to_string()];
    };
    let mut classified = Classified::default();
    classified.visit_file(&file);
    for extern_crate in &classified.extern_crates {
        classified.segments.insert(extern_crate.clone());
    }
    for import in &classified.imports {
        for segment in import {
            classified.segments.insert(segment.clone());
        }
    }
    let mut found: Vec<String> = classified
        .segments
        .iter()
        .filter(|segment| rules.forbidden_segments.contains(&segment.as_str()))
        .map(|segment| format!("identifier `{segment}`"))
        .collect();
    // The module sequence itself is refused as a sequence — spelled as an
    // import (aliased, globbed or grouped) or fully qualified with no import
    // at all — so an alias or glob of `std::path` cannot smuggle the module
    // in through two individually harmless identifiers.
    for import in &classified.imports {
        if is_std_path_sequence(import) {
            found.push("import of `std::path`".to_string());
        }
    }
    if classified
        .path_sequences
        .iter()
        .any(|sequence| is_std_path_sequence(sequence))
    {
        found.push("fully-qualified `std::path`".to_string());
    }
    for mac in &classified.macros {
        if rules.forbidden_macros.contains(&mac.as_str()) {
            found.push(format!("macro `{mac}!`"));
        }
    }
    for method in &classified.methods {
        if rules.forbidden_methods.contains(&method.as_str()) {
            found.push(format!("method `.{method}()`"));
        }
    }
    if !rules.allows_trait_objects && classified.trait_objects > 0 {
        found.push("trait object (`dyn`)".to_string());
    }
    found.sort();
    found
}

/// The two behavior cells admit no manifest/row/collector/filesystem/codec/
/// Display surface; `Arc<dyn …>` stays legal exactly here while `Box` refuses,
/// because the registry has one behavior-ownership channel the plan never sees.
#[test]
fn the_behavior_cells_admit_no_collector_codec_or_rendered_surface() {
    for (cell, source) in [
        ("behavior.rs", include_str!("behavior.rs")),
        ("registry.rs", include_str!("registry.rs")),
    ] {
        let found = offenders(source, &BEHAVIOR_RULES);
        assert!(
            found.is_empty(),
            "{cell} is fenced: {found:#?} — behavior cells hold Arc<dyn …> and nothing else"
        );
    }
}

/// The existing plan cells remain `Arc`/`dyn`-free — the independent T5
/// re-assert, so behavior objects cannot migrate into plan identity even if
/// the original fence were weakened.
#[test]
fn the_plan_cells_stay_free_of_behavior_objects() {
    for (cell, source) in [
        ("plan.rs", include_str!("plan.rs")),
        ("plan_digest.rs", include_str!("plan_digest.rs")),
        ("plan_validate.rs", include_str!("plan_validate.rs")),
        ("config.rs", include_str!("config.rs")),
    ] {
        let found = offenders(source, &PLAN_CARRIER_RULES);
        assert!(
            found.is_empty(),
            "{cell} stays behavior-object-free: {found:#?}"
        );
    }
}

/// `TransformPlan::build` never consults the registry: no registry type,
/// module or lookup verb appears anywhere in the plan cell, so the plan stays
/// grammar-only and off-catalog candidates remain legal plan values.
#[test]
fn the_plan_cell_contains_no_registry_lookup() {
    let found = offenders(include_str!("plan.rs"), &PLAN_LOOKUP_RULES);
    assert!(
        found.is_empty(),
        "plan.rs must not consult the behavior registry: {found:#?}"
    );
}

/// The crate root reexports nothing of the registry: neither type name, nor
/// the private module path, reaches `vibe-spec`'s public surface.
#[test]
fn the_crate_root_reexports_nothing_of_the_registry() {
    let root = syn::parse_file(include_str!("../../lib.rs")).expect("lib.rs parses");
    let mut classified = Classified::default();
    classified.visit_file(&root);
    for needle in [
        "TransformRegistry",
        "TransformBehavior",
        "behavior",
        "registry",
    ] {
        assert!(
            !classified.segments.contains(needle),
            "lib.rs must not name `{needle}` — the registry is crate-private"
        );
    }
}

/// The RED fixtures for this fence: every ordinary spelling of a forbidden
/// surface is detected under the behavior rules, the carrier fixtures refuse
/// under the plan-carrier rules, and a clean fixture using `Arc<dyn …>`
/// proves behavior objects are the legal surface of a behavior-ruled cell.
#[test]
fn the_registry_fence_detects_every_spelling_and_allows_behavior_objects() {
    let behavior_cases: &[(&str, &str)] = &[
        ("use serde::Serialize;", "identifier `serde`"),
        ("use std::path::PathBuf;", "identifier `PathBuf`"),
        ("use toml as codec;", "identifier `toml`"),
        (
            "use vibe_extension_registry::collect_extensions;",
            "identifier `collect_extensions`",
        ),
        ("fn f() { let _ = std::fmt::Display; }", "identifier `fmt`"),
        ("fn f() -> String { format!(\"x\") }", "macro `format!`"),
        (
            "fn f(n: &str) -> String { n.to_string() }",
            "method `.to_string()`",
        ),
        (
            "fn f() { std::fs::read_to_string(\"x\").ok(); }",
            "identifier `fs`",
        ),
        ("use serde_json::Value;", "identifier `serde_json`"),
        // `Box` is not a behavior channel even inside the behavior cells.
        ("fn f(a: Box<dyn Behavior>) {}", "identifier `Box`"),
        // The `std::path` module sequence refuses under every spelling.
        ("use std::path as p;", "import of `std::path`"),
        ("use std::path::*;", "import of `std::path`"),
        ("use std::{path::Path};", "import of `std::path`"),
        (
            "fn f(x: std::path::PathBuf) {}",
            "fully-qualified `std::path`",
        ),
    ];
    for (source, expected) in behavior_cases {
        let found = offenders(source, &BEHAVIOR_RULES);
        assert!(
            found.iter().any(|finding| finding.contains(expected)),
            "fixture `{source}` must report {expected:?}, got {found:?}"
        );
    }

    // The sequence refusal is exact, not prefix-greedy: `std::pathological`
    // never trips it.
    assert!(
        offenders(
            "mod m { pub fn pathological() {} }\nfn f() { m::pathological(); }",
            &BEHAVIOR_RULES
        )
        .is_empty(),
        "the sequence refusal is exact"
    );

    let carrier_cases: &[(&str, &str)] = &[
        ("fn f(a: Arc<dyn Behavior>) {}", "identifier `Arc`"),
        ("fn f(a: Box<dyn Behavior>) {}", "identifier `Box`"),
        ("fn f(a: &dyn Behavior) {}", "trait object (`dyn`)"),
    ];
    for (source, expected) in carrier_cases {
        let found = offenders(source, &PLAN_CARRIER_RULES);
        assert!(
            found.iter().any(|finding| finding.contains(expected)),
            "plan fixture `{source}` must report {expected:?}, got {found:?}"
        );
    }

    // The clean fixture: the Arc<dyn …> channel is exactly the legal
    // surface of a behavior-ruled cell, and prose never trips the AST fence.
    let clean = concat!(
        "// serde toml PathBuf Display fmt format! to_string collect_extensions Box\n",
        "use std::sync::Arc;\n",
        "fn f(a: Arc<dyn Behavior>) -> Arc<dyn Behavior> { a }\n",
    );
    assert!(
        offenders(clean, &BEHAVIOR_RULES).is_empty(),
        "Arc<dyn> is the one legal behavior channel in the behavior cells"
    );
    assert!(
        !offenders(clean, &PLAN_CARRIER_RULES).is_empty(),
        "the same fixture must refuse under the plan-carrier rules"
    );

    // The registry-lookup rules catch a consulted registry by type, module or
    // verb, wherever it hides.
    let lookup_cases: &[(&str, &str)] = &[
        (
            "use super::registry::TransformRegistry;",
            "identifier `TransformRegistry`",
        ),
        (
            "fn f(r: &TransformRegistry) {}",
            "identifier `TransformRegistry`",
        ),
        (
            "fn f() { self.registry.resolve(&i, &s); }",
            "identifier `registry`",
        ),
        (
            "fn f(x: &dyn TransformBehavior) {}",
            "identifier `TransformBehavior`",
        ),
    ];
    for (source, expected) in lookup_cases {
        let found = offenders(source, &PLAN_LOOKUP_RULES);
        assert!(
            found.iter().any(|finding| finding.contains(expected)),
            "lookup fixture `{source}` must report {expected:?}, got {found:?}"
        );
    }
}

/// The reusable test catalog stays test-only: `registry_test_support` is
/// declared `#[cfg(test)] pub(crate)` in `transform/mod.rs`, so T6's tests
/// can consume the same authority while no production build can name it.
#[test]
fn the_test_support_module_is_cfg_test_gated_in_the_module_tree() {
    let module_tree = syn::parse_file(include_str!("mod.rs")).expect("mod.rs parses");
    let mut found = false;
    for item in &module_tree.items {
        let syn::Item::Mod(item_mod) = item else {
            continue;
        };
        if item_mod.ident != "registry_test_support" {
            continue;
        }
        found = true;
        let gated = item_mod.attrs.iter().any(|attribute| {
            matches!(&attribute.meta, syn::Meta::List(list)
                if list.path.is_ident("cfg")
                    && list.tokens.to_string().contains("test"))
        });
        assert!(gated, "registry_test_support must carry #[cfg(test)]");
        assert!(
            matches!(&item_mod.vis, syn::Visibility::Restricted(restricted)
                if restricted.path.get_ident().is_some_and(|ident| ident == "crate")),
            "registry_test_support must stay pub(crate) for T6's tests"
        );
    }
    assert!(
        found,
        "mod.rs must declare the registry_test_support module"
    );
}

// The manifest dependency sets used to be re-asserted here as well. R4.2
// moved that fact to `dependency_dag_fence_tests`, its one home: two copies of
// one expected set is two places to update and one place to forget, and the
// registry family's own claim — that registering the first production behavior
// is a CODE change, never a dependency one — is exactly what that single
// assertion says.
