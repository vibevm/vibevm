//! Syntax-aware fences over the plan cells (R4-TRANSFORM-PLAN-ABI §§1–2,
//! central-review round 1): the classifier parses production cells as an
//! AST with `syn`, so grouped and renamed imports, fully-qualified paths,
//! type paths, macros and trait objects are classified structurally — a
//! string literal or comment containing a needle cannot hide real Rust
//! from the fence, and a real use cannot be stripped away by prose.
//!
//! Two fences live here: the forbidden-use fence over the identity, digest,
//! refusal and config cells, and the opacity fence over the provider and
//! implementation values (structs over private fields, no widened
//! constructor — the regression a future visibility change could bring). The
//! manifest DAG proof moved to `dependency_dag_fence_tests` at R4.2: it reads
//! MANIFESTS rather than source, and it is the one home for that fact.

use std::collections::BTreeSet;

use syn::visit::{self, Visit};
use syn::{File, Item, Type, UseTree, Visibility};

/// Flatten one `use` tree into complete imported paths — grouped and
/// nested trees never appear as one whole `syn::Path`, so the tree is
/// walked explicitly (the registry-kernel fence's idiom). A rename binds
/// BOTH spellings locally, so both identifiers enter the flattened path; a
/// glob still names its whole prefix.
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
    /// Every path-segment identifier in any position (expressions, types,
    /// patterns, generic arguments).
    segments: BTreeSet<String>,
    /// Every complete flattened import path — not a bag of segments: the
    /// `std::path` sequence is refused as a sequence, so an alias or glob
    /// of the module cannot smuggle it through as two harmless names.
    imports: Vec<Vec<String>>,
    /// Every `extern crate` item's original identifier.
    extern_crates: Vec<String>,
    /// Every fully-qualified path's segment sequence, for the same
    /// sequence-level refusal where no import exists at all.
    path_sequences: Vec<Vec<String>>,
    /// Every macro invocation's terminal identifier (`format!` ⇒ `format`).
    macros: BTreeSet<String>,
    /// Every method-call name.
    methods: BTreeSet<String>,
    /// How many trait-object types (`dyn Trait`, in any position) appear.
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
    /// Flatten EVERY `use` item at any nesting depth — a `use` inside a
    /// function body binds just as reachable a name as a top-level one.
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        flatten_use_tree(&item.tree, Vec::new(), &mut self.imports);
        visit::visit_item_use(self, item);
    }
    /// `extern crate x as y;` rebinds a crate root under a local name; the
    /// ORIGINAL identifier is what the fence must judge.
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
}

/// Which rule family one cell is fenced under.
struct CellRules {
    /// Extra identifiers forbidden as path/import segments on top of the
    /// common set.
    extra_segments: &'static [&'static str],
    /// Macro names forbidden on top of the common set.
    extra_macros: &'static [&'static str],
    /// Method names forbidden outright in this cell.
    forbidden_methods: &'static [&'static str],
    /// Identifiers REMOVED from the common set for this cell, each because
    /// naming it is the cell's own declared job (T10B).
    ///
    /// A removal is a deliberate act with a reason, never a way to make a
    /// new file pass: a cell is admitted to a permission only when its whole
    /// purpose is the thing the common set bans, and every other member of
    /// the set still binds it.
    permitted_segments: &'static [&'static str],
}

/// The common forbidden set, every plan cell: parser/serializer identity
/// surfaces, filesystem path types, registry row/view/registry objects and
/// collection calls, behavior-carriage smart pointers.
const COMMON_SEGMENTS: &[&str] = &[
    "serde",
    "serde_json",
    "toml",
    "json",
    "Path",
    "PathBuf",
    "ExtensionRegistryRow",
    "RegistryView",
    "ExtensionRegistry",
    "collect_extensions",
    "Arc",
    "Box",
];

/// The identity and digest cells additionally refuse every rendered-identity
/// surface: no trait rendering, no formatting macros of any spelling, and
/// no string-conversion method (the one legitimate manual projection,
/// `PlanDigest::sha256_hex`, builds its text with `push`/`push_str` and
/// stays clean).
const RENDERING_SEGMENTS: &[&str] = &["Display", "fmt"];
const RENDERING_MACROS: &[&str] = &["format", "write", "format_args", "writeln"];
const RENDERING_METHODS: &[&str] = &["to_string"];

/// The refusal cell additionally proves its checks stay borrowed: no
/// `to_owned` clone and no `parse` call (whose refusal allocates the full
/// input back) anywhere in the cell.
const REFUSAL_METHODS: &[&str] = &["to_owned", "parse"];

/// Classify one source under one cell's rules. An unparsable source reports
/// itself as an offender rather than panicking, so the production fence
/// names the file instead of aborting.
fn offenders(source: &str, rules: &CellRules) -> Vec<String> {
    let Ok(file) = syn::parse_file(source) else {
        return vec!["<unparsable source>".to_string()];
    };
    let mut classified = Classified::default();
    classified.visit_file(&file);

    let mut forbidden_segments: BTreeSet<&str> = COMMON_SEGMENTS.iter().copied().collect();
    forbidden_segments.extend(rules.extra_segments.iter().copied());
    for permitted in rules.permitted_segments {
        forbidden_segments.remove(permitted);
    }
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
        .filter(|segment| forbidden_segments.contains(segment.as_str()))
        .map(|segment| format!("identifier `{segment}`"))
        .collect();
    // The module sequence itself is forbidden, spelled as a sequence, so
    // aliases and globs of `std::path` refuse even though neither `std`
    // nor `path` is a forbidden identifier alone.
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
        if rules.extra_macros.iter().any(|forbidden| mac == forbidden) {
            found.push(format!("macro `{mac}!`"));
        }
    }
    for method in &classified.methods {
        if rules
            .forbidden_methods
            .iter()
            .any(|forbidden| method == forbidden)
        {
            found.push(format!("method `.{method}()`"));
        }
    }
    if classified.trait_objects > 0 {
        found.push("trait object (`dyn`)".to_string());
    }
    found
}

const IDENTITY_RULES: CellRules = CellRules {
    extra_segments: RENDERING_SEGMENTS,
    extra_macros: RENDERING_MACROS,
    forbidden_methods: RENDERING_METHODS,
    permitted_segments: &[],
};
const COMMON_RULES: CellRules = CellRules {
    extra_segments: &[],
    extra_macros: &[],
    forbidden_methods: &[],
    permitted_segments: &[],
};
const REFUSAL_RULES: CellRules = CellRules {
    extra_segments: &[],
    extra_macros: &[],
    forbidden_methods: REFUSAL_METHODS,
    permitted_segments: &[],
};

/// The T10B lowering cell. Its input contract IS a borrowed kernel row, so
/// `ExtensionRegistryRow` is the one identifier it may name — and the two
/// identifiers that would make it a second COLLECTOR, `ExtensionRegistry`
/// and `collect_extensions`, stay forbidden: the caller filters, this cell
/// only refuses. Everything else in the common set still binds it, so no
/// parser, path, serializer or behavior pointer can enter the lowering.
const LOWERING_RULES: CellRules = CellRules {
    extra_segments: &[],
    extra_macros: &[],
    forbidden_methods: &[],
    permitted_segments: &["ExtensionRegistryRow"],
};

/// The R4.2 effective-configuration cell. Its whole purpose IS reading the
/// TOML value tower — ABI §3 forbids generic JSON and forbids a render/parse
/// round trip, so a lossless lowering has to name `toml::Value`'s variants —
/// which makes it the one cell admitted to the `toml` identifier. Everything
/// else in the common set still binds it, and the permission is exactly one
/// name: no serializer, no path, no registry row, no behavior pointer.
///
/// The permission is narrow in the other direction too: every OTHER plan cell
/// is still checked under a family that bans `toml`, so the crate's new
/// runtime edge cannot spread from here by being merely available.
const CONFIG_LOWERING_RULES: CellRules = CellRules {
    extra_segments: &[],
    extra_macros: &[],
    forbidden_methods: &[],
    permitted_segments: &["toml"],
};

/// The forbidden-use fence over the production cells: identity and digest
/// cells refuse the common set plus every rendered-identity surface; the
/// refusal cell refuses the common set plus the borrowed-check law; the
/// config cell refuses the common set (its numeric error messages are
/// bounded diagnostics, not identity rendering).
#[test]
fn the_plan_cells_admit_no_parser_renderer_path_row_or_trait_object() {
    for (cell, source, rules) in [
        ("plan.rs", include_str!("plan.rs"), &IDENTITY_RULES),
        (
            "plan_digest.rs",
            include_str!("plan_digest.rs"),
            &IDENTITY_RULES,
        ),
        (
            "plan_validate.rs",
            include_str!("plan_validate.rs"),
            &REFUSAL_RULES,
        ),
        ("config.rs", include_str!("config.rs"), &COMMON_RULES),
        // T10B's two new cells. The lowering names the borrowed kernel row
        // it consumes and nothing else the common set bans; the effective
        // configuration lowering names `toml` and NOTHING else the common set
        // bans — R4.2 turned its interim refusal into the real value-tower
        // walk, which is exactly what that one permitted identifier buys.
        ("lowering.rs", include_str!("lowering.rs"), &LOWERING_RULES),
        (
            "config_lowering.rs",
            include_str!("config_lowering.rs"),
            &CONFIG_LOWERING_RULES,
        ),
    ] {
        let found = offenders(source, rules);
        assert!(
            found.is_empty(),
            "{cell} is fenced: {found:#?} — the plan cells are typed owned values, not parser/renderer/path/registry surfaces"
        );
    }
}

/// The RED fixtures for the fence itself: every ordinary Rust spelling of a
/// forbidden use is detected — direct, grouped and renamed imports,
/// fully-qualified paths with no import at all, type paths, macros and
/// trait objects — while comments and string literals containing every
/// needle never reach the AST and stay clean.
#[test]
fn the_forbidden_use_fence_detects_every_spelling() {
    let cases: &[(&str, &str)] = &[
        ("use serde::Serialize;", "identifier `serde`"),
        ("use std::path::{Path, PathBuf};", "identifier `Path`"),
        ("use toml as manifest_codec;", "identifier `toml`"),
        ("fn f() { let _ = std::fmt::Display; }", "identifier `fmt`"),
        ("fn f(p: std::path::PathBuf) {}", "identifier `PathBuf`"),
        ("fn f() -> String { format!(\"x\") }", "macro `format!`"),
        ("fn f(h: Box<dyn Pass>) {}", "identifier `Box`"),
        (
            "use vibe_extension_registry::collect_extensions;",
            "identifier `collect_extensions`",
        ),
        ("fn f(a: Arc<dyn Behavior>) {}", "identifier `Arc`"),
        // Alias and glob of the module itself: neither `std` nor `path` is
        // a forbidden identifier alone — the SEQUENCE is what refuses.
        ("use std::path as p;", "import of `std::path`"),
        ("use std::path::*;", "import of `std::path`"),
        ("use std::{path::{self, PathBuf}};", "import of `std::path`"),
        // A nested use binds just as reachable a name as a top-level one.
        (
            "fn f() { use serde as codec; let _ = codec::de; }",
            "identifier `serde`",
        ),
        // extern-crate aliases rebind a crate root under a local name.
        ("extern crate serde as codec;", "identifier `serde`"),
        ("extern crate toml as cfglib;", "identifier `toml`"),
        // Rendered identity, further spellings: formatting macros and the
        // string-conversion method.
        (
            "fn f() -> String { let _ = format_args!(\"x\"); String::new() }",
            "macro `format_args!`",
        ),
        (
            "use std::io::stdout;\nfn f() { writeln!(\"x\").ok(); }",
            "macro `writeln!`",
        ),
        (
            "fn f(n: &str) -> String { n.to_string() }",
            "method `.to_string()`",
        ),
    ];
    for (source, expected) in cases {
        let found = offenders(source, &IDENTITY_RULES);
        assert!(
            found.iter().any(|finding| finding.contains(expected)),
            "fixture `{source}` must report {expected:?}, got {found:?}"
        );
    }

    // A `std::pathological` spelling never trips the sequence rule.
    assert!(
        offenders(
            "mod m { pub fn pathological() {} }\nfn f() { m::pathological(); }",
            &IDENTITY_RULES
        )
        .is_empty(),
        "the sequence refusal is exact, not prefix-greedy"
    );

    // The clean fixture: every needle, in comments and in string literals,
    // under the strictest rule family — nothing reaches the AST.
    let clean = concat!(
        "// serde serde_json toml json Path PathBuf Display fmt format! write!\n",
        "// format_args! writeln! to_string() to_owned parse std::path\n",
        "// ExtensionRegistryRow RegistryView ExtensionRegistry collect_extensions\n",
        "// Arc Box dyn\n",
        "const NEEDLES: &str = \"serde toml PathBuf Display fmt format! write! Arc Box\n",
        "std::path format_args! writeln! to_string()\n",
        "ExtensionRegistryRow RegistryView ExtensionRegistry collect_extensions\";\n",
        "fn f() { let _ = NEEDLES; }\n",
    );
    assert!(
        offenders(clean, &IDENTITY_RULES).is_empty(),
        "prose and strings must never trip the AST fence"
    );
    assert!(
        offenders(clean, &REFUSAL_RULES).is_empty(),
        "prose and strings must never trip the method fence"
    );
}

/// Whether one visibility keeps an item private to the transform module:
/// inherited, or `pub(super)` exactly — never `pub`, `pub(crate)` or a
/// wider `pub(in …)` (syn models every restricted visibility as
/// `Visibility::Restricted`).
fn is_module_private(visibility: &Visibility) -> bool {
    match visibility {
        Visibility::Inherited => true,
        Visibility::Restricted(restricted) => restricted
            .path
            .get_ident()
            .is_some_and(|ident| ident == "super"),
        _ => false,
    }
}

/// The opacity fence: `TransformProvider` and `TransformImplementation`
/// are structs over private fields (a `pub(crate)` enum with named variant
/// fields would be constructible crate-wide the moment T10 widens the
/// type), no method on them is visible outside the transform module, and
/// `TransformPlan::capacity` stays test-only — so the visibility widening
/// a future atom performs cannot silently reopen direct construction.
#[test]
fn provider_and_implementation_values_stay_opaque_and_constructors_stay_module_private() {
    let file: File = syn::parse_file(include_str!("plan.rs")).expect("plan.rs parses");
    for name in ["TransformProvider", "TransformImplementation"] {
        let mut seen = false;
        for item in &file.items {
            match item {
                Item::Struct(item_struct) if item_struct.ident == name => {
                    seen = true;
                    assert!(
                        !item_struct.fields.is_empty(),
                        "{name} must be an opaque struct over fields, not a unit or tuple"
                    );
                    for field in &item_struct.fields {
                        assert!(
                            matches!(field.vis, Visibility::Inherited),
                            "{name} field `{}` must stay private",
                            field
                                .ident
                                .as_ref()
                                .map(|ident| ident.to_string())
                                .unwrap_or_default()
                        );
                    }
                }
                Item::Enum(item_enum) if item_enum.ident == name => {
                    panic!(
                        "{name} must not be an enum: enum variant fields inherit the enum's \
                         visibility and become constructible wherever the type is nameable"
                    );
                }
                _ => {}
            }
        }
        assert!(seen, "{name} must be declared in plan.rs");
    }

    // The only inherent constructors are module-private: every associated
    // function that returns `Self` must stay private to the transform
    // module, while read accessors may stay `pub(crate)` because they
    // cannot author the value. The `From<&ExtensionProvider>` conversion
    // is a trait implementation with no inherent visibility and passes the
    // same rule.
    for item in &file.items {
        let Item::Impl(item_impl) = item else {
            continue;
        };
        let Type::Path(path) = item_impl.self_ty.as_ref() else {
            continue;
        };
        let Some(last) = path.path.segments.last() else {
            continue;
        };
        if last.ident != "TransformProvider" && last.ident != "TransformImplementation" {
            continue;
        }
        for item in &item_impl.items {
            let syn::ImplItem::Fn(method) = item else {
                continue;
            };
            let constructs = matches!(
                &method.sig.output,
                syn::ReturnType::Type(_, return_type)
                    if matches!(return_type.as_ref(), Type::Path(type_path)
                        if type_path.path.segments.last().is_some_and(|segment| segment.ident == "Self"))
            );
            if constructs {
                assert!(
                    is_module_private(&method.vis),
                    "{}::{} constructs the value and must stay private to the transform module",
                    last.ident,
                    method.sig.ident
                );
            }
        }
    }

    // `capacity` is a test-only observation of the empty-plan allocation
    // law, not production API: it must carry `#[cfg(test)]` in the AST.
    let mut found_test_only = false;
    for item in &file.items {
        let Item::Impl(item_impl) = item else {
            continue;
        };
        let Type::Path(path) = item_impl.self_ty.as_ref() else {
            continue;
        };
        let is_plan_impl = path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "TransformPlan");
        if !is_plan_impl {
            continue;
        }
        for item in &item_impl.items {
            if let syn::ImplItem::Fn(method) = item
                && method.sig.ident == "capacity"
            {
                found_test_only = method.attrs.iter().any(|attribute| {
                    matches!(&attribute.meta, syn::Meta::List(list)
                        if list.path.is_ident("cfg")
                            && list.tokens.to_string().contains("test"))
                });
            }
        }
    }
    assert!(
        found_test_only,
        "TransformPlan::capacity must carry #[cfg(test)]"
    );
}
