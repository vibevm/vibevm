//! The T6b wrapper-cell fence (ABI §6.3): the schedule cell may render its
//! mandated pass name and hold the one `Arc<dyn TransformBehavior>` channel,
//! and NOTHING else — no manifest/collector/row/path/codec surface, no `Box`
//! behavior ownership, no `SelectorSubject` or selector `matches` (the
//! unscoped-subject trap, made mechanical). Parsed as an AST with `syn`, so
//! grouped/renamed imports, qualified paths and macros are classified
//! structurally; prose never trips it.
//!
//! The module-tree assertion keeps the rule families exhaustive: every
//! production transform cell is declared in `mod.rs`, and a future cell
//! cannot ship unclassified.

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
            for nested in &group.items {
                flatten_use_tree(nested, prefix.clone(), out);
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
    /// `Box<dyn …>` spellings: the behavior-ownership channel that is not
    /// `Arc`. A `Box<ConcreteType>` (error boxing) is not this.
    boxed_trait_objects: usize,
}

/// Whether an identifier sequence begins with the `std::path` module
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
    fn visit_type_trait_object(&mut self, ty: &'ast syn::TypeTraitObject) {
        self.trait_objects += 1;
        visit::visit_type_trait_object(self, ty);
    }
    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        if let Some(segment) = ty.path.segments.last()
            && segment.ident == "Box"
            && matches!(
                &segment.arguments,
                syn::PathArguments::AngleBracketed(arguments)
                    if arguments.args.iter().any(|argument| matches!(
                        argument,
                        syn::GenericArgument::Type(syn::Type::TraitObject(_))
                    ))
            )
        {
            self.boxed_trait_objects += 1;
        }
        visit::visit_type_path(self, ty);
    }
}

/// One cell's fence rules.
struct CellRules {
    forbidden_segments: &'static [&'static str],
    forbidden_methods: &'static [&'static str],
    forbidden_macros: &'static [&'static str],
    allows_trait_objects: bool,
    /// Whether `Box<dyn …>` refuses in this cell: it does everywhere, but the
    /// wrapper cell may still box CONCRETE error types.
    forbids_boxed_trait_objects: bool,
}

/// The wrapper cell's law: the mandated pass-name rendering and the one
/// `Arc<dyn …>` channel are legal; manifest/collector/row/path/codec
/// surfaces, `Box` behavior ownership, the WHOLE kernel selector crate
/// (the unscoped-subject trap — the cell imports nothing from it, so a
/// direct/renamed/grouped/glob/qualified use is a subject evaluation trying
/// to happen), and every upward builtin/driver spelling are not. And the
/// production schedule cell never eliminates a fault by panic spelling.
const WRAPPER_SEGMENTS: &[&str] = &[
    "serde",
    "serde_json",
    "toml",
    "json",
    "Path",
    "PathBuf",
    "fs",
    "ExtensionRegistry",
    "RegistryView",
    "ExtensionRegistryRow",
    "collect_extensions",
    "vibe_extension_registry",
    "ArtifactCompileError",
    "builtin",
];

const WRAPPER_METHODS: &[&str] = &["matches", "unwrap", "expect"];

const WRAPPER_MACROS: &[&str] = &["panic", "todo", "unimplemented"];

const WRAPPER_RULES: CellRules = CellRules {
    forbidden_segments: WRAPPER_SEGMENTS,
    forbidden_methods: WRAPPER_METHODS,
    forbidden_macros: WRAPPER_MACROS,
    allows_trait_objects: true,
    forbids_boxed_trait_objects: true,
};

const PLAN_CARRIER_RULES: CellRules = CellRules {
    forbidden_segments: &["Arc", "Box"],
    forbidden_methods: &[],
    forbidden_macros: &[],
    allows_trait_objects: false,
    forbids_boxed_trait_objects: true,
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
    if rules.forbids_boxed_trait_objects && classified.boxed_trait_objects > 0 {
        found.push("boxed trait object (`Box<dyn …>`)".to_string());
    }
    found.sort();
    found
}

/// The wrapper cell admits exactly its mandated surfaces.
#[test]
fn the_schedule_cell_renders_names_and_holds_one_behavior_channel() {
    let found = offenders(include_str!("schedule.rs"), &WRAPPER_RULES);
    assert!(
        found.is_empty(),
        "schedule.rs is fenced: {found:#?} — the wrapper cell renders pass names and holds Arc<dyn …>, nothing else"
    );
}

/// The plan cells remain behavior- and registry-free under the stronger
/// carrier rules — re-asserted here so this fence stays self-contained.
#[test]
fn the_plan_cells_stay_behavior_free_under_the_carrier_rules() {
    for (cell, source) in [
        ("plan.rs", include_str!("plan.rs")),
        ("plan_digest.rs", include_str!("plan_digest.rs")),
        ("plan_validate.rs", include_str!("plan_validate.rs")),
        ("config.rs", include_str!("config.rs")),
    ] {
        let found = offenders(source, &PLAN_CARRIER_RULES);
        assert!(
            found.is_empty(),
            "{cell} stays Arc/Box/dyn-free: {found:#?}"
        );
    }
}

/// Mutation fixtures: every banned spelling a future edit could introduce is
/// VISIBLE to the classifier — direct, GROUPED, RENAMED, GLOB and
/// fully-qualified imports alike — while the two mandated surfaces (name
/// rendering, `Arc<dyn …>`) pass the wrapper rules and refuse the plan-carrier
/// rules, proving the two families genuinely differ. Boxing a CONCRETE error
/// stays legal; `Box<dyn …>` behavior ownership never does; and the whole
/// kernel selector crate refuses under any import shape.
#[test]
fn the_wrapper_fence_detects_every_banned_spelling_and_admits_its_two_surfaces() {
    let cases: &[(&str, &str)] = &[
        (
            "use vibe_extension_registry::collect_extensions;",
            "identifier `collect_extensions`",
        ),
        // The kernel selector crate itself refuses — renamed, grouped, glob
        // and qualified alike (the unscoped-subject trap made mechanical).
        (
            "use vibe_extension_registry::SelectorSubject as Subject;",
            "identifier `vibe_extension_registry`",
        ),
        (
            "use vibe_extension_registry::{CompiledSelector, SelectorSubject};",
            "identifier `vibe_extension_registry`",
        ),
        (
            "use vibe_extension_registry::*;",
            "identifier `vibe_extension_registry`",
        ),
        (
            "fn f(s: &vibe_extension_registry::SelectorSubject) {}",
            "identifier `vibe_extension_registry`",
        ),
        (
            "fn f(s: &Subject) { s.matches(&t); }",
            "method `.matches()`",
        ),
        // The upward boundary is load-bearing: no builtin/driver spelling.
        (
            "use crate::compiler::builtin::ArtifactCompileError;",
            "identifier `builtin`",
        ),
        (
            "fn f(e: ArtifactCompileError) {}",
            "identifier `ArtifactCompileError`",
        ),
        // Manifest/collector/row/path/codec surfaces, grouped and renamed.
        (
            "use vibe_extension_registry::ExtensionRegistry;",
            "identifier `ExtensionRegistry`",
        ),
        ("use std::path::{Path, PathBuf};", "import of `std::path`"),
        ("use std::path as p;", "import of `std::path`"),
        ("use std::path::*;", "import of `std::path`"),
        (
            "fn f(x: std::path::PathBuf) {}",
            "fully-qualified `std::path`",
        ),
        ("use toml as codec;", "identifier `toml`"),
        ("use serde_json::Value;", "identifier `serde_json`"),
        (
            "fn f() { std::fs::read_to_string(\"x\").ok(); }",
            "identifier `fs`",
        ),
        // The production schedule cell never eliminates a fault by panic.
        (
            "fn f(r: Result<u8, E>) { r.unwrap() }",
            "method `.unwrap()`",
        ),
        (
            "fn f(r: Result<u8, E>) { r.expect(\"x\") }",
            "method `.expect()`",
        ),
        ("fn f() { panic!(\"boom\"); }", "macro `panic!`"),
        ("fn f() { todo!() }", "macro `todo!`"),
        // `Box<dyn …>` is not a behavior channel even inside the wrapper
        // cell, while boxing a CONCRETE error type stays legal there.
        (
            "fn f(a: Box<dyn TransformBehavior>) {}",
            "boxed trait object (`Box<dyn …>`)",
        ),
    ];
    for (source, expected) in cases {
        let found = offenders(source, &WRAPPER_RULES);
        assert!(
            found.iter().any(|finding| finding.contains(expected)),
            "fixture `{source}` must report {expected:?}, got {found:?}"
        );
    }

    // The classifier DISTINGUISHES the two Box spellings: a boxed concrete
    // error passes the wrapper rules, the boxed trait object refuses.
    let boxed_concrete = "fn f(e: Box<VerificationError>) -> Box<VerificationError> { e }";
    assert!(offenders(boxed_concrete, &WRAPPER_RULES).is_empty());
    assert!(!offenders("fn f(a: Box<dyn TransformBehavior>) {}", &WRAPPER_RULES).is_empty());

    // The clean fixture: BOTH mandated surfaces of the wrapper cell — the
    // one `Arc<dyn …>` channel and the rendered pass name — stay legal here.
    let clean = concat!(
        "use std::sync::Arc;\n",
        "fn name(stage: &str, key: &str) -> String { format!(\"transform:{stage}:{key}\") }\n",
        "fn f(a: Arc<dyn TransformBehavior>) -> Arc<dyn TransformBehavior> { a }\n",
    );
    assert!(
        offenders(clean, &WRAPPER_RULES).is_empty(),
        "name rendering and Arc<dyn …> are the wrapper cell's two mandated surfaces"
    );
    assert!(
        !offenders(clean, &PLAN_CARRIER_RULES).is_empty(),
        "the same fixture must refuse under the plan-carrier rules"
    );

    // Prose immunity: every needle in comments and string literals is
    // invisible to the AST fence.
    let prose = concat!(
        "// serde toml PathBuf fs collect_extensions ExtensionRegistry Box SelectorSubject matches\n",
        "// vibe_extension_registry builtin ArtifactCompileError unwrap expect panic todo\n",
        "const NEEDLES: &str = \"collect_extensions SelectorSubject matches std::path Box\n",
        "vibe_extension_registry builtin ArtifactCompileError unwrap expect panic todo\";\n",
        "fn f() { let _ = NEEDLES; }\n",
    );
    assert!(offenders(prose, &WRAPPER_RULES).is_empty());
}

/// The rule families stay exhaustive over the module tree: the production
/// transform cells are exactly the seven declared `pub(crate) mod`s, every
/// cfg-test cell is declared too, and no undeclared `.rs` sibling can ship
/// unclassified (a new production cell must be added to a family here).
#[test]
fn the_module_tree_declares_every_transform_cell_under_a_rule_family() {
    let module_tree =
        syn::parse_file(include_str!("mod.rs")).expect("transform/mod.rs parses as Rust");
    let mut production = BTreeSet::new();
    let mut test_only = BTreeSet::new();
    for item in &module_tree.items {
        let syn::Item::Mod(item_mod) = item else {
            continue;
        };
        let is_test = item_mod.attrs.iter().any(|attribute| {
            matches!(&attribute.meta, syn::Meta::List(list)
                if list.path.is_ident("cfg")
                    && list.tokens.to_string().contains("test"))
        });
        if is_test {
            test_only.insert(item_mod.ident.to_string());
        } else {
            production.insert(item_mod.ident.to_string());
        }
    }
    assert_eq!(
        production,
        BTreeSet::from([
            "behavior".to_owned(),
            "config".to_owned(),
            "plan".to_owned(),
            "plan_digest".to_owned(),
            "plan_validate".to_owned(),
            "registry".to_owned(),
            "schedule".to_owned(),
        ]),
        "a new production transform cell must be declared AND classified"
    );
    assert_eq!(
        test_only,
        BTreeSet::from([
            "carriage".to_owned(),
            "config_tests".to_owned(),
            "plan_digest_tests".to_owned(),
            "plan_fence_tests".to_owned(),
            "plan_refusal_tests".to_owned(),
            "plan_test_support".to_owned(),
            "plan_tests".to_owned(),
            "registry_fence_tests".to_owned(),
            "registry_test_support".to_owned(),
            "registry_tests".to_owned(),
            "schedule_execution_tests".to_owned(),
            "schedule_execution_vehicles".to_owned(),
            "schedule_fence_tests".to_owned(),
            "schedule_tests".to_owned(),
        ]),
        "a new test cell must be declared too — undeclared files do not compile"
    );

    // The classification itself: the wrapper cell under wrapper rules, the
    // plan cells under the stronger carrier rules.
    assert!(offenders(include_str!("schedule.rs"), &WRAPPER_RULES).is_empty());
    assert!(offenders(include_str!("plan.rs"), &PLAN_CARRIER_RULES).is_empty());
}
