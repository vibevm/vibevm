//! The transform cells' fence RULE FAMILIES and the AST classifier that
//! applies them (ABI §6.3, and every per-cell law added since).
//!
//! Split out of `schedule_fence_tests` when that cell crossed the file
//! budget, along its own seam: the families and the classifier are shared
//! INFRASTRUCTURE — `schedule_fence_tests` and `transform_cells_fence_tests`
//! both consume them — while the tests that APPLY them are per-cell claims.
//! Keeping one home for the families is what makes the module-tree fence's
//! "every production cell is classified" assertion checkable at all.
//!
//! Sources are parsed as an AST with `syn`, so grouped/renamed imports,
//! qualified paths, type paths and macros are classified structurally; prose
//! and string literals never reach the AST and never trip a fence.

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
pub(super) struct CellRules {
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

pub(super) const WRAPPER_RULES: CellRules = CellRules {
    forbidden_segments: WRAPPER_SEGMENTS,
    forbidden_methods: WRAPPER_METHODS,
    forbidden_macros: WRAPPER_MACROS,
    allows_trait_objects: true,
    forbids_boxed_trait_objects: true,
};

pub(super) const PLAN_CARRIER_RULES: CellRules = CellRules {
    forbidden_segments: &["Arc", "Box"],
    forbidden_methods: &[],
    forbidden_macros: &[],
    allows_trait_objects: false,
    forbids_boxed_trait_objects: true,
};

/// The selector-admission cell's law: the kernel selector crate and
/// `.matches()` are its REASON to exist, so they are the two surfaces this
/// family — and only this family — admits. Everything else the wrapper cell
/// is banned from stays banned here, and the cell is held to more besides:
/// it is a pure decision over borrowed values, so it owns no behavior
/// channel of any spelling (`Arc`, `Box`, `dyn`), and it eliminates no fault
/// by panic.
const SELECTOR_SEGMENTS: &[&str] = &[
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
    "ArtifactCompileError",
    "builtin",
    "Arc",
    "Box",
];

pub(super) const SELECTOR_RULES: CellRules = CellRules {
    forbidden_segments: SELECTOR_SEGMENTS,
    forbidden_methods: &["unwrap", "expect"],
    forbidden_macros: WRAPPER_MACROS,
    allows_trait_objects: false,
    forbids_boxed_trait_objects: true,
};

/// The T10B lowering cell's law: a borrowed kernel ROW is its input
/// contract, so `ExtensionRegistryRow` and the kernel crate it comes from
/// are the two surfaces this family admits — and, exactly like the selector
/// family, it is held to MORE than the wrapper elsewhere.
///
/// The three identifiers that would make it a second COLLECTOR stay banned:
/// `ExtensionRegistry`, `RegistryView` and `collect_extensions`. That ban is
/// the fence's whole point here, because it is the executable form of the
/// split §5.3 froze — the workspace filters and hands rows over, this cell
/// only maps and refuses. It owns no behavior channel of any spelling, reads
/// no filesystem or codec, and eliminates no fault by panic.
const LOWERING_SEGMENTS: &[&str] = &[
    "serde",
    "serde_json",
    "toml",
    "json",
    "Path",
    "PathBuf",
    "fs",
    "ExtensionRegistry",
    "RegistryView",
    "collect_extensions",
    "ArtifactCompileError",
    "builtin",
    "Arc",
    "Box",
];

pub(super) const LOWERING_RULES: CellRules = CellRules {
    forbidden_segments: LOWERING_SEGMENTS,
    forbidden_methods: &["unwrap", "expect"],
    forbidden_macros: WRAPPER_MACROS,
    allows_trait_objects: false,
    forbids_boxed_trait_objects: true,
};

/// The T10C header cell's law: rendering the ACTIVE list is its reason to
/// exist, so the shared generated-comment codec is the surface this family
/// admits — and the OTHER percent codec in this workspace
/// (`vibe_core::HostOwner`'s host-segment pair) is banned by name, because
/// §7.1's rule is "one shared cell", not "some codec". Everything the wrapper
/// is banned from stays banned; the cell owns no behavior channel of any
/// spelling, reads no collector or row, and eliminates no fault by panic.
const HEADER_SEGMENTS: &[&str] = &[
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
    "Arc",
    "Box",
    "HostOwner",
    "encode_host_segment",
    "decode_host_segment",
];

pub(super) const HEADER_RULES: CellRules = CellRules {
    forbidden_segments: HEADER_SEGMENTS,
    forbidden_methods: &["unwrap", "expect"],
    forbidden_macros: WRAPPER_MACROS,
    allows_trait_objects: false,
    forbids_boxed_trait_objects: true,
};

/// The R4.2 `xml-minify` binding cell's law: it exists to read the EMIT
/// cell's own framing back off a tape, so `framing` is what this family
/// admits — and the ban that matters most is the one it keeps.
/// `vibe_specdoc` is refused here even though the segmenter must recognise a
/// codec-encoded marker: the codec is the emit cell's to spell, so the
/// binding asks `framing::hoisted_origin_in_comment` instead of decoding a
/// comment itself. One framing grammar, one codec call site, and this fence
/// is what makes "never a second grammar" (R4 architecture §2.2) mechanical.
///
/// Everything the wrapper cell is banned from stays banned, and the cell is
/// held to more besides: it owns no behavior channel of any spelling, reads
/// no collector, row or filesystem, and eliminates no fault by panic — a
/// tape is attacker-controlled input, so every refusal is typed.
const MINIFY_SEGMENTS: &[&str] = &[
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
    "Arc",
    "Box",
    "vibe_specdoc",
    "encode_generated_xml_comment",
    "decode_generated_xml_comment",
    "HostOwner",
];

pub(super) const MINIFY_RULES: CellRules = CellRules {
    forbidden_segments: MINIFY_SEGMENTS,
    forbidden_methods: &["unwrap", "expect"],
    forbidden_macros: WRAPPER_MACROS,
    allows_trait_objects: false,
    forbids_boxed_trait_objects: true,
};

/// Classify one source under one cell's rules; an unparsable source reports
/// itself as the offender so the fence names the file, never aborts.
pub(super) fn offenders(source: &str, rules: &CellRules) -> Vec<String> {
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
