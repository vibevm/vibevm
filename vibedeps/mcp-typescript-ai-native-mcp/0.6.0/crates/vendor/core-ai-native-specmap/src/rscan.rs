//! Rust side of the scanner: `#[spec]` / `#[verifies]` attributes and
//! `specmark::scope!` markers, read as AST out of the source tree —
//! never expanded (PROP-014 §2.5: "no macro expansion needed").
//!
//! B5 (monotone utility): a file `syn` cannot parse becomes a warning,
//! not an error; the rest of the tree still indexes.

use std::path::Path;

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#addressing-code");

use crate::fingerprint::fingerprint_of;
use crate::generated::specmap::{CodeItem, Edge, EdgeProvenance, EdgeVerb, Warning};
use quote::ToTokens;
use specmark_grammar::{EdgeSpec, SpecArgs, UriArgs};
use syn::spanned::Spanned;
use walkdir::WalkDir;

use crate::config::Config;
use crate::cscan;
use crate::fwd;

fn verb_to_wire(v: specmark_grammar::Verb) -> EdgeVerb {
    match v {
        specmark_grammar::Verb::Implements => EdgeVerb::Implements,
        specmark_grammar::Verb::Verifies => EdgeVerb::Verifies,
        specmark_grammar::Verb::Documents => EdgeVerb::Documents,
        specmark_grammar::Verb::Deviates => EdgeVerb::Deviates,
        specmark_grammar::Verb::Informs => EdgeVerb::Informs,
    }
}

/// Compact token rendering: `Foo < T >` → `Foo<T>`.
fn compact_tokens(t: impl ToTokens) -> String {
    t.to_token_stream().to_string().replace(' ', "")
}

struct FileScan<'a> {
    file: &'a str,
    crate_name: &'a str,
    items: Vec<CodeItem>,
    edges: Vec<Edge>,
    warnings: Vec<Warning>,
    /// `#[derive(Parser)]` roots seen this file — half of the crate-wide
    /// command join (`cscan`); resolved against `cmd_enums` after the walk.
    roots: Vec<cscan::CommandRoot>,
    /// `#[derive(Subcommand)]` enums seen this file — the other half.
    cmd_enums: Vec<cscan::CommandEnum>,
}

impl FileScan<'_> {
    fn record_item(
        &mut self,
        symbol: &str,
        item_kind: &str,
        line: u32,
        end_line: u32,
        fingerprint: String,
    ) {
        self.items.push(CodeItem {
            symbol: symbol.to_string(),
            itemKind: item_kind.to_string(),
            crateName: self.crate_name.to_string(),
            file: self.file.to_string(),
            line,
            endLine: Some(Box::new(end_line)),
            fingerprint: Some(Box::new(fingerprint)),
        });
    }

    fn record_edge(&mut self, symbol: &str, edge: EdgeSpec, line: u32) {
        self.edges.push(Edge {
            fromSymbol: symbol.to_string(),
            verb: verb_to_wire(edge.verb),
            uri: edge.uri.without_pin(),
            provenance: EdgeProvenance::Authored,
            file: self.file.to_string(),
            line,
            pinnedR: edge.r.map(Box::new),
            reason: edge.reason.map(Box::new),
        });
    }

    fn warn(&mut self, code: &str, message: String, line: u32) {
        self.warnings.push(Warning {
            code: code.to_string(),
            message,
            file: self.file.to_string(),
            line,
        });
    }

    /// Extract edges from an attribute list. Returns `(edges, line)` of
    /// recognised specmark attributes; grammar errors become warnings.
    fn edges_from_attrs(&mut self, attrs: &[syn::Attribute]) -> Vec<(EdgeSpec, u32)> {
        let mut out = Vec::new();
        for attr in attrs {
            let last = attr
                .path()
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();
            let line = attr.span().start().line as u32;
            match last.as_str() {
                "spec" => match &attr.meta {
                    syn::Meta::List(list) => match syn::parse2::<SpecArgs>(list.tokens.clone()) {
                        Ok(args) => out.push((args.edge, line)),
                        Err(e) => self.warn(
                            "invalid-spec-attr",
                            format!("#[spec(...)] does not parse: {e}"),
                            line,
                        ),
                    },
                    _ => self.warn(
                        "invalid-spec-attr",
                        "#[spec] without arguments".to_string(),
                        line,
                    ),
                },
                "verifies" => match &attr.meta {
                    syn::Meta::List(list) => match syn::parse2::<UriArgs>(list.tokens.clone()) {
                        Ok(args) => out.push((args.into_verifies_edge(), line)),
                        Err(e) => self.warn(
                            "invalid-verifies-attr",
                            format!("#[verifies(...)] does not parse: {e}"),
                            line,
                        ),
                    },
                    _ => self.warn(
                        "invalid-verifies-attr",
                        "#[verifies] without arguments".to_string(),
                        line,
                    ),
                },
                _ => {}
            }
        }
        out
    }

    fn tag_item<T: ToTokens + Spanned>(
        &mut self,
        attrs: &[syn::Attribute],
        item: &T,
        symbol: &str,
        item_kind: &str,
        line: u32,
    ) {
        let edges = self.edges_from_attrs(attrs);
        if edges.is_empty() {
            return;
        }
        // `line` is the attribute-inclusive start of the span; `end_line` is
        // the matching far end of the same span — the item's closing brace
        // or semicolon. Same convention, opposite end.
        let end_line = item.span().end().line as u32;
        let fingerprint = fingerprint_of(item);
        self.record_item(symbol, item_kind, line, end_line, fingerprint);
        for (edge, edge_line) in edges {
            self.record_edge(symbol, edge, edge_line);
        }
    }

    fn walk_items(&mut self, items: &[syn::Item], module: &str) {
        for item in items {
            let line = item.span().start().line as u32;
            match item {
                syn::Item::Fn(f) => {
                    let symbol = format!("{module}::{}", f.sig.ident);
                    self.tag_item(&f.attrs, f, &symbol, "fn", line);
                }
                syn::Item::Struct(s) => {
                    let symbol = format!("{module}::{}", s.ident);
                    self.tag_item(&s.attrs, s, &symbol, "struct", line);
                    if let Some(root) = cscan::observe_root(s, self.file, self.crate_name) {
                        self.roots.push(root);
                    }
                }
                syn::Item::Enum(e) => {
                    let symbol = format!("{module}::{}", e.ident);
                    self.tag_item(&e.attrs, e, &symbol, "enum", line);
                    if let Some(en) = cscan::observe_enum(e, self.file, self.crate_name) {
                        self.cmd_enums.push(en);
                    }
                }
                syn::Item::Union(u) => {
                    let symbol = format!("{module}::{}", u.ident);
                    self.tag_item(&u.attrs, u, &symbol, "union", line);
                }
                syn::Item::Trait(t) => {
                    let symbol = format!("{module}::{}", t.ident);
                    self.tag_item(&t.attrs, t, &symbol, "trait", line);
                    for ti in &t.items {
                        if let syn::TraitItem::Fn(tf) = ti {
                            let msym = format!("{module}::{}::{}", t.ident, tf.sig.ident);
                            let mline = tf.span().start().line as u32;
                            self.tag_item(&tf.attrs, tf, &msym, "fn", mline);
                        }
                    }
                }
                syn::Item::Const(c) => {
                    let symbol = format!("{module}::{}", c.ident);
                    self.tag_item(&c.attrs, c, &symbol, "const", line);
                }
                syn::Item::Static(s) => {
                    let symbol = format!("{module}::{}", s.ident);
                    self.tag_item(&s.attrs, s, &symbol, "static", line);
                }
                syn::Item::Type(t) => {
                    let symbol = format!("{module}::{}", t.ident);
                    self.tag_item(&t.attrs, t, &symbol, "type", line);
                }
                syn::Item::Impl(im) => {
                    let ty = compact_tokens(&im.self_ty);
                    let symbol = match &im.trait_ {
                        Some((_, path, _)) => {
                            format!("{module}::<impl {} for {ty}>", compact_tokens(path))
                        }
                        None => format!("{module}::<impl {ty}>"),
                    };
                    self.tag_item(&im.attrs, im, &symbol, "impl", line);
                    for ii in &im.items {
                        if let syn::ImplItem::Fn(mf) = ii {
                            let msym = format!("{module}::{ty}::{}", mf.sig.ident);
                            let mline = mf.span().start().line as u32;
                            self.tag_item(&mf.attrs, mf, &msym, "fn", mline);
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    let sub = format!("{module}::{}", m.ident);
                    let mline = m.span().start().line as u32;
                    self.tag_item(&m.attrs, m, &sub, "mod", mline);
                    if let Some((_, items)) = &m.content {
                        self.walk_items(items, &sub);
                    }
                }
                syn::Item::Macro(mc) => {
                    // `specmark::scope!("uri")` — the module-level
                    // inheritance marker. Identified by the path's last
                    // segment being `scope` AND the tokens parsing as
                    // the URI grammar; other `scope!` macros won't carry
                    // a `spec://` literal and fall through silently.
                    let is_scope = mc
                        .mac
                        .path
                        .segments
                        .last()
                        .map(|s| s.ident == "scope")
                        .unwrap_or(false);
                    if is_scope && let Ok(args) = syn::parse2::<UriArgs>(mc.mac.tokens.clone()) {
                        let end_line = mc.span().end().line as u32;
                        let fingerprint = fingerprint_of(mc);
                        self.record_item(module, "mod", line, end_line, fingerprint);
                        self.record_edge(module, args.into_scope_edge(), line);
                    }
                }
                _ => {}
            }
        }
    }
}

/// One file's full scan: items, edges, warnings, plus the command roots /
/// enums collected for the crate-wide join (`cscan`). `pub(crate)` — these
/// are the join's inputs, not a per-file result a public caller sees; the
/// public [`scan_source`] returns only the file's items / edges / warnings.
pub(crate) struct FileScanOutput {
    pub(crate) items: Vec<CodeItem>,
    pub(crate) edges: Vec<Edge>,
    pub(crate) warnings: Vec<Warning>,
    pub(crate) roots: Vec<cscan::CommandRoot>,
    pub(crate) cmd_enums: Vec<cscan::CommandEnum>,
}

/// Scan one source file into its items / edges / warnings **and** the
/// command roots / enums it declares, for the crate-wide join (`cscan`).
/// `pub(crate)` — the join's inputs are internal; [`scan_source`] is the
/// public, three-tuple seam. The root and the enum it points at may live in
/// different files, so the join runs after the walk, not here
/// (command-nodes.md `##x-the-join-is-crate-wide`).
pub(crate) fn scan_file(file: &str, crate_name: &str, module: &str, text: &str) -> FileScanOutput {
    let mut scan = FileScan {
        file,
        crate_name,
        items: Vec::new(),
        edges: Vec::new(),
        warnings: Vec::new(),
        roots: Vec::new(),
        cmd_enums: Vec::new(),
    };
    match syn::parse_file(text) {
        Ok(ast) => scan.walk_items(&ast.items, module),
        Err(e) => {
            let line = e.span().start().line as u32;
            scan.warn("unparseable-source", format!("syn cannot parse: {e}"), line);
        }
    }
    FileScanOutput {
        items: scan.items,
        edges: scan.edges,
        warnings: scan.warnings,
        roots: scan.roots,
        cmd_enums: scan.cmd_enums,
    }
}

/// Scan one source file (testable on strings).
///
/// The public per-file seam: items / edges / warnings only. It does not
/// produce command nodes — those come from the crate-wide `cscan` join in
/// [`scan_workspace`] — so the join's intermediate `roots` / `cmd_enums`
/// stay out of the public contract (a public caller such as a tracing
/// fragment reader destructures exactly this three-tuple).
pub fn scan_source(
    file: &str,
    crate_name: &str,
    module: &str,
    text: &str,
) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
    let o = scan_file(file, crate_name, module, text);
    (o.items, o.edges, o.warnings)
}

/// Module path for a source file inside one crate.
///
/// `src/lib.rs` / `src/main.rs` → the crate root; `src/foo.rs` and
/// `src/foo/mod.rs` → `foo`; `tests/x.rs` → `tests::x` (integration
/// test targets compile as their own crates; this scheme keeps their
/// symbols stable and readable).
pub(crate) fn module_path(crate_ident: &str, rel_in_crate: &Path) -> Option<String> {
    let comps: Vec<String> = rel_in_crate
        .iter()
        .map(|c| c.to_string_lossy().to_string())
        .collect();
    let (head, rest) = comps.split_first()?;
    let mut parts: Vec<String> = vec![crate_ident.to_string()];
    match head.as_str() {
        "src" => {}
        "tests" => parts.push("tests".to_string()),
        _ => return None,
    }
    for (i, comp) in rest.iter().enumerate() {
        let is_last = i + 1 == rest.len();
        if is_last {
            let stem = comp.strip_suffix(".rs")?;
            match stem {
                "lib" | "main" | "mod" => {}
                other => parts.push(other.to_string()),
            }
        } else {
            parts.push(comp.clone());
        }
    }
    Some(parts.join("::"))
}

/// Walk every code root named by [`Config::scan_dirs`] (`crates/*` by
/// default — each crate's `src` and `tests`). Generated code is excluded
/// (PROP-014 §2.3: the generator *input* is the taggable unit).
pub fn scan_workspace(root: &Path, cfg: &Config) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
    let mut items = Vec::new();
    let mut edges = Vec::new();
    let mut warnings = Vec::new();
    let mut roots = Vec::new();
    let mut cmd_enums = Vec::new();

    for crate_dir in cfg.scan_dirs(root) {
        let crate_name = crate_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let crate_ident = crate_name.replace('-', "_");
        for entry in WalkDir::new(&crate_dir)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let rel_in_crate = path.strip_prefix(&crate_dir).unwrap_or(path);
            // Generated trees are excluded from scanning wholesale.
            if fwd(rel_in_crate).contains("/generated/") {
                continue;
            }
            let Some(module) = module_path(&crate_ident, rel_in_crate) else {
                continue;
            };
            let rel = path.strip_prefix(root).unwrap_or(path);
            let file = fwd(rel);
            match std::fs::read_to_string(path) {
                Ok(text) => {
                    let mut o = scan_file(&file, &crate_name, &module, &text);
                    items.append(&mut o.items);
                    edges.append(&mut o.edges);
                    warnings.append(&mut o.warnings);
                    roots.append(&mut o.roots);
                    cmd_enums.append(&mut o.cmd_enums);
                }
                Err(err) => warnings.push(Warning {
                    code: "unreadable-file".to_string(),
                    message: format!("could not read: {err}"),
                    file,
                    line: 0,
                }),
            }
        }
    }
    // Crate-wide join: every file has been walked, so roots collected across
    // files can now be matched against enums collected across files — the
    // join cannot run per-file because a root names an enum in another file
    // (command-nodes.md `##x-the-join-is-crate-wide`).
    let (cmd_items, cmd_warns) = cscan::join_commands(&roots, &cmd_enums);
    items.extend(cmd_items);
    warnings.extend(cmd_warns);
    (items, edges, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const URI: &str = "spec://project/modules/vibe-resolver/PROP-003#req-conditional-fixpoint";

    fn fmt_warnings(w: &[Warning]) -> String {
        w.iter()
            .map(|x| format!("{}:{} [{}] {}", x.file, x.line, x.code, x.message))
            .collect::<Vec<_>>()
            .join("; ")
    }

    #[test]
    fn tagged_items_yield_edges() {
        let src = format!(
            r#"
#[spec(implements = "{URI}", r = 2)]
pub enum ConditionalPredicate {{ A }}

#[spec(deviates = "{URI}", r = 2, reason = "boolean composition unimplemented")]
impl ConditionalPredicate {{
    pub fn parse(_raw: &str) -> Self {{ Self::A }}
}}

#[cfg(test)]
mod tests {{
    #[test]
    #[verifies("{URI}", r = 2)]
    fn fixed_point_is_monotone() {{}}
}}
"#
        );
        let (items, edges, warnings) =
            scan_source("crates/x/src/conditional.rs", "x", "x::conditional", &src);
        assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
        assert_eq!(edges.len(), 3);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].symbol, "x::conditional::ConditionalPredicate");
        assert_eq!(
            items[1].symbol,
            "x::conditional::<impl ConditionalPredicate>"
        );
        assert_eq!(
            items[2].symbol,
            "x::conditional::tests::fixed_point_is_monotone"
        );
        assert!(matches!(edges[0].verb, EdgeVerb::Implements));
        assert!(matches!(edges[1].verb, EdgeVerb::Deviates));
        assert_eq!(
            edges[1].reason.as_deref(),
            Some(&"boolean composition unimplemented".to_string())
        );
        assert!(matches!(edges[2].verb, EdgeVerb::Verifies));
        assert_eq!(edges[2].pinnedR.as_deref(), Some(&2));
        // Edges carry the pin-free canonical URI.
        assert_eq!(edges[0].uri, URI);
    }

    #[test]
    fn scope_marker_records_a_module_edge() {
        let src = format!("specmark::scope!(\"{URI}\", r = 1);\npub fn helper() {{}}\n");
        let (items, edges, warnings) = scan_source("crates/x/src/m.rs", "x", "x::m", &src);
        assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].symbol, "x::m");
        assert_eq!(items[0].itemKind, "mod");
        assert_eq!(edges.len(), 1);
        assert!(matches!(edges[0].verb, EdgeVerb::Implements));
    }

    #[test]
    fn foreign_scope_macros_are_ignored() {
        let src = "tracing::scope!(\"not a spec uri\");\n";
        let (items, edges, warnings) = scan_source("f.rs", "x", "x", src);
        assert!(items.is_empty() && edges.is_empty() && warnings.is_empty());
    }

    #[test]
    fn untagged_items_are_not_inventoried() {
        let src = "pub fn plain() {}\npub struct Plain;\n";
        let (items, edges, _) = scan_source("f.rs", "x", "x", src);
        assert!(items.is_empty());
        assert!(edges.is_empty());
    }

    #[test]
    fn bad_grammar_becomes_a_warning_not_an_error() {
        let src = format!("#[spec(fulfills = \"{URI}\")]\npub fn f() {{}}\n");
        let (items, edges, warnings) = scan_source("f.rs", "x", "x", &src);
        assert!(items.is_empty() && edges.is_empty());
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "invalid-spec-attr");
    }

    #[test]
    fn unparseable_source_degrades_to_a_warning() {
        let (items, edges, warnings) = scan_source("f.rs", "x", "x", "fn broken( {");
        assert!(items.is_empty() && edges.is_empty());
        assert_eq!(warnings[0].code, "unparseable-source");
    }

    #[test]
    fn module_paths_follow_the_scheme() {
        use std::path::Path;
        assert_eq!(module_path("x", Path::new("src/lib.rs")).unwrap(), "x");
        assert_eq!(module_path("x", Path::new("src/main.rs")).unwrap(), "x");
        assert_eq!(module_path("x", Path::new("src/foo.rs")).unwrap(), "x::foo");
        assert_eq!(
            module_path("x", Path::new("src/foo/mod.rs")).unwrap(),
            "x::foo"
        );
        assert_eq!(
            module_path("x", Path::new("src/foo/bar.rs")).unwrap(),
            "x::foo::bar"
        );
        assert_eq!(
            module_path("x", Path::new("tests/cli_e2e.rs")).unwrap(),
            "x::tests::cli_e2e"
        );
        assert!(module_path("x", Path::new("benches/b.rs")).is_none());
    }

    /// §2.3 proof: on a multi-line tagged item the recorded `end_line`
    /// strictly exceeds `line`, which can only happen if `proc-macro2`'s
    /// `span-locations` feature yields real line numbers (the zero-span
    /// trap). Values printed for the verbatim record.
    #[test]
    fn end_line_exceeds_line_for_multiline_item() {
        let src = format!(
            "#[spec(implements = \"{URI}\", r = 1)]\n\
             pub fn fold(\n    a: u32,\n    b: u32,\n    c: u32,\n) -> u32 {{\n    a + b + c\n}}\n"
        );
        let (items, _, warnings) = scan_source("crates/x/src/m.rs", "x", "x::m", &src);
        assert!(warnings.is_empty(), "{}", fmt_warnings(&warnings));
        assert_eq!(items.len(), 1, "exactly one tagged item");
        let it = &items[0];
        let end = it.endLine.as_deref().copied().expect("end_line present");
        let fp = it.fingerprint.as_deref().expect("fingerprint present");
        println!("line={} end_line={} fingerprint={}", it.line, end, fp);
        assert!(
            end > it.line,
            "end_line {end} must exceed line {} — spans read as zero (span-locations off?)",
            it.line
        );
    }
}
