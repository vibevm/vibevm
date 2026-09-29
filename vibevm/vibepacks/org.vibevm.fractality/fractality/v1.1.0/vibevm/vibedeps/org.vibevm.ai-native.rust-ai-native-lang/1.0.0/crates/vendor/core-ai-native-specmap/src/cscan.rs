//! Command nodes — the `command` item kind (B-019(б), slice 1).
//!
//! A clap `#[derive(Parser)]` root names a binary; its
//! `#[command(subcommand)]` field names a `#[derive(Subcommand)]` enum, and
//! each variant of that enum is a top-level command the user types. The
//! binary half of the path is the root's `#[command(name = "…")]`; the
//! variant half is clap's own kebab rename, unless the variant carries its
//! own `#[command(name = "…")]`, which wins.
//!
//! The root and the enum it points at need not share a file (`cli.rs` vs
//! `cli/registry.rs`), so the scanner collects both during the per-file walk
//! and [`join_commands`] resolves them after the workspace walk — a
//! crate-wide join, the one structural cost this node type adds
//! (command-nodes.md `##x-the-join-is-crate-wide`). Slice 1 only: the join
//! is root → its *direct* subcommand enum. A variant whose payload carries
//! its own `#[command(subcommand)]` (nesting) is slice 2 and is not
//! descended into here.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#addressing-code");

use crate::fingerprint::fingerprint_of;
use crate::generated::specmap::{CodeItem, Warning};
use syn::parse::Parser;
use syn::spanned::Spanned;

/// A `#[derive(Parser)]` root as seen during one file's walk — half of the
/// crate-wide join's input.
#[derive(Clone)]
pub(crate) struct CommandRoot {
    /// Binary name from `#[command(name = "…")]`. `None` when the root
    /// declares none: clap would then fall back to `CARGO_PKG_NAME`, a
    /// compile-time value the source AST does not carry, so the root is
    /// skipped with a warning rather than given an invented name
    /// (command-nodes.md `##n-binary-name-is-declared`).
    pub binary_name: Option<String>,
    /// The crate the root lives in — half of the crate-local join key. Two
    /// crates may both declare `pub enum Command`; matching on the type name
    /// alone would resolve both roots to whichever enum was walked first, so
    /// a root joins only an enum in its own crate.
    pub crate_name: String,
    /// Last path segment of the root's `#[command(subcommand)]` field's
    /// type — the other half of the join key (paired with `crate_name`).
    pub subcommand_type: String,
    /// The root's file + the subcommand field's line, for the skip warning.
    pub file: String,
    pub line: u32,
}

/// One variant of a `#[derive(Subcommand)]` enum, captured during the walk.
#[derive(Clone)]
pub(crate) struct VariantObs {
    /// The variant's PascalCase ident — kebab-renamed unless `explicit_name`
    /// is set.
    pub ident: String,
    /// `#[command(name = "…")]` on the variant — wins over the derived name
    /// (command-nodes.md `##n-variant-name-rule`).
    pub explicit_name: Option<String>,
    pub line: u32,
    pub end_line: u32,
    pub fingerprint: String,
}

/// A `#[derive(Subcommand)]` enum as seen during one file's walk — the other
/// half of the join's input. Carries the file / crate its command nodes will
/// inherit (a command is declared where its variant is).
#[derive(Clone)]
pub(crate) struct CommandEnum {
    /// The enum's ident — the key the join matches a root's
    /// `subcommand_type` against.
    pub type_name: String,
    pub variants: Vec<VariantObs>,
    pub crate_name: String,
    pub file: String,
}

/// True if any `#[derive(...)]` path's **last segment** is `segment`.
///
/// Catches both spellings — `#[derive(Subcommand)]` and
/// `#[derive(clap::Subcommand)]` — by matching the path's last segment,
/// exactly the mechanic `rscan::FileScan::edges_from_attrs` applies to
/// attribute paths (command-nodes.md `##r-both-spellings`).
fn derive_has_segment(attrs: &[syn::Attribute], segment: &str) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let syn::Meta::List(list) = &attr.meta else {
            continue;
        };
        let Ok(paths) = syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated
            .parse2(list.tokens.clone())
        else {
            continue;
        };
        for path in paths {
            if path.segments.last().is_some_and(|s| s.ident == segment) {
                return true;
            }
        }
    }
    false
}

/// The `name = "lit"` value carried by a `#[command(...)]` attribute, if any.
///
/// clap's `#[command(...)]` is a comma-separated list of `key = value` /
/// `key(value)` / bare-`key` entries; only the `name = "string"` form is
/// read, which is the form both host roots declare (`vibe`, `vibe-index`).
/// `name = <const>` or `name("…")` are left unmatched and fall through.
fn command_name(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("command") {
            continue;
        }
        let syn::Meta::List(list) = &attr.meta else {
            continue;
        };
        let Ok(metas) = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
            .parse2(list.tokens.clone())
        else {
            continue;
        };
        for meta in metas {
            if let syn::Meta::NameValue(nv) = meta
                && nv.path.is_ident("name")
                && let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = nv.value
            {
                return Some(s.value());
            }
        }
    }
    None
}

/// True if the field carries `#[command(subcommand)]` — the marker that ties
/// a root (or, in slice 2, an args struct) to a command enum.
fn field_is_subcommand(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("command") {
            continue;
        }
        let syn::Meta::List(list) = &attr.meta else {
            continue;
        };
        let Ok(metas) = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
            .parse2(list.tokens.clone())
        else {
            continue;
        };
        for meta in metas {
            if let syn::Meta::Path(p) = meta
                && p.is_ident("subcommand")
            {
                return true;
            }
        }
    }
    false
}

/// Last path segment of a bare type (`Command`) — `None` for wrapped forms
/// (`Option<Command>`, `Vec<…>`) the top-level surface does not use.
fn field_type_last_segment(ty: &syn::Type) -> Option<String> {
    if let syn::Type::Path(tp) = ty {
        return tp.path.segments.last().map(|s| s.ident.to_string());
    }
    None
}

/// Observe a struct as a command root if it `#[derive(Parser)]` and has a
/// `#[command(subcommand)]` field. `file` / `crate_name` are the per-file
/// scan's forward-slash path and crate (the skip warning's location and the
/// crate-local join key); the caller knows both.
pub(crate) fn observe_root(
    s: &syn::ItemStruct,
    file: &str,
    crate_name: &str,
) -> Option<CommandRoot> {
    if !derive_has_segment(&s.attrs, "Parser") {
        return None;
    }
    for f in &s.fields {
        if field_is_subcommand(&f.attrs)
            && let Some(seg) = field_type_last_segment(&f.ty)
        {
            return Some(CommandRoot {
                binary_name: command_name(&s.attrs),
                crate_name: crate_name.to_string(),
                subcommand_type: seg,
                file: file.to_string(),
                line: f.span().start().line as u32,
            });
        }
    }
    None
}

/// Observe an enum as a command enum if it `#[derive(Subcommand)]`, capturing
/// each variant's span + token fingerprint (the same attribute-inclusive span
/// and `tok1:<sha256>` fingerprint every other item carries).
pub(crate) fn observe_enum(e: &syn::ItemEnum, file: &str, crate_name: &str) -> Option<CommandEnum> {
    if !derive_has_segment(&e.attrs, "Subcommand") {
        return None;
    }
    let variants = e
        .variants
        .iter()
        .map(|v| VariantObs {
            ident: v.ident.to_string(),
            explicit_name: command_name(&v.attrs),
            line: v.span().start().line as u32,
            end_line: v.span().end().line as u32,
            fingerprint: fingerprint_of(v),
        })
        .collect();
    Some(CommandEnum {
        type_name: e.ident.to_string(),
        variants,
        crate_name: crate_name.to_string(),
        file: file.to_string(),
    })
}

/// clap's own subcommand rename: PascalCase → kebab-case. Reimplemented here
/// because `heck` (the crate clap delegates to) is not a dependency of this
/// crate, and adding a workspace dep is outside this engine crate's
/// perimeter. Matches `heck::ToKebabCase` on the cases that arise: word
/// boundaries fall between lowercase-and-uppercase, between an uppercase run
/// and the last uppercase that begins a lowercase tail (`RedirectSync` →
/// `redirect-sync`, `HTTPRequest` → `http-request`), and between a letter and
/// a digit. No variant in this tree has a digit or an abbreviation, so the
/// digit branch is faithful-but-unexercised here (command-nodes.md
/// `##n-variant-name-rule`).
fn to_kebab_case(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + 4);
    for (i, &c) in chars.iter().enumerate() {
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let next = chars.get(i + 1).copied();
        let boundary = match (prev, c) {
            (Some(p), c) if p.is_ascii_lowercase() && c.is_ascii_uppercase() => true,
            (Some(p), c)
                if p.is_ascii_uppercase()
                    && c.is_ascii_uppercase()
                    && next.is_some_and(|n| n.is_ascii_lowercase()) =>
            {
                true
            }
            (Some(p), c) if p.is_ascii_alphabetic() && c.is_ascii_digit() => true,
            (Some(p), c) if p.is_ascii_digit() && c.is_ascii_alphabetic() => true,
            _ => false,
        };
        if boundary {
            out.push('-');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// Resolve collected roots and enums into `command` items plus warnings.
///
/// A root joins only an enum in its **own crate** with a matching type name —
/// the pair `(crate_name, type_name)`. Matching on the type name alone would
/// collapse two crates that both declare `pub enum Command` (`vibe-cli`,
/// `vibe-index`) into whichever enum the walk visited first, so each root
/// would inherit the other's variants. Each matched root yields one `command`
/// item per variant, recorded **unconditionally** — a command exists whether
/// or not anyone tagged it, so this is the `record_item` path (not `tag_item`,
/// which gates on an edge) (command-nodes.md `##x-not-through-tag-item`). A
/// root with no declared name, or whose enum was not collected, is a warning,
/// not an invention.
pub(crate) fn join_commands(
    roots: &[CommandRoot],
    enums: &[CommandEnum],
) -> (Vec<CodeItem>, Vec<Warning>) {
    let mut items = Vec::new();
    let mut warnings = Vec::new();
    for root in roots {
        let Some(name) = &root.binary_name else {
            warnings.push(Warning {
                code: "command-root-unnamed".to_string(),
                message: "a #[derive(Parser)] root declares no #[command(name)]; \
                          clap's CARGO_PKG_NAME default is not visible to the AST, \
                          so its commands are skipped"
                    .to_string(),
                file: root.file.clone(),
                line: root.line,
            });
            continue;
        };
        let Some(en) = enums
            .iter()
            .find(|e| e.crate_name == root.crate_name && e.type_name == root.subcommand_type)
        else {
            warnings.push(Warning {
                code: "command-root-no-enum".to_string(),
                message: format!(
                    "no #[derive(Subcommand)] enum named `{}` was collected in crate `{}`",
                    root.subcommand_type, root.crate_name
                ),
                file: root.file.clone(),
                line: root.line,
            });
            continue;
        };
        for v in &en.variants {
            let command = v
                .explicit_name
                .clone()
                .unwrap_or_else(|| to_kebab_case(&v.ident));
            // Same field shape as `FileScan::record_item`: the command is
            // declared in the enum's crate / file (where the variant lives),
            // carries the variant's attribute-inclusive span, and a
            // `tok1:<sha256>` fingerprint over the variant's tokens.
            items.push(CodeItem {
                symbol: format!("{name} {command}"),
                itemKind: "command".to_string(),
                crateName: en.crate_name.clone(),
                file: en.file.clone(),
                line: v.line,
                endLine: Some(Box::new(v.end_line)),
                fingerprint: Some(Box::new(v.fingerprint.clone())),
            });
        }
    }
    (items, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_enum(src: &str) -> syn::ItemEnum {
        syn::parse_str(src).expect("test enum parses")
    }
    fn parse_struct(src: &str) -> syn::ItemStruct {
        syn::parse_str(src).expect("test struct parses")
    }

    #[test]
    fn both_derive_spellings_are_recognised() {
        let bare = parse_enum("#[derive(Debug, Subcommand)]\nenum C { A, B }");
        let qualified = parse_enum("#[derive(clap::Subcommand, Debug)]\nenum D { A }");
        assert!(derive_has_segment(&bare.attrs, "Subcommand"));
        assert!(derive_has_segment(&qualified.attrs, "Subcommand"));
        // An enum without Subcommand is not a command enum.
        let plain = parse_enum("#[derive(Debug)]\nenum E { A }");
        assert!(!derive_has_segment(&plain.attrs, "Subcommand"));
    }

    #[test]
    fn parser_root_with_subcommand_field_is_observed() {
        let s = parse_struct(
            r#"
#[derive(Debug, Parser)]
#[command(name = "vibe")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
"#,
        );
        let root = observe_root(&s, "cli.rs", "vibe-cli").expect("observed");
        assert_eq!(root.binary_name.as_deref(), Some("vibe"));
        assert_eq!(root.crate_name, "vibe-cli");
        assert_eq!(root.subcommand_type, "Command");
    }

    #[test]
    fn root_without_subcommand_field_is_not_a_root() {
        let s = parse_struct(
            r#"
#[derive(Debug, Parser)]
#[command(name = "vibe")]
struct Cli {
    #[arg(long)]
    json: bool,
}
"#,
        );
        assert!(observe_root(&s, "cli.rs", "vibe-cli").is_none());
    }

    #[test]
    fn variant_without_explicit_name_is_kebab() {
        let e = parse_enum("#[derive(Debug, Subcommand)]\nenum C { Install, RedirectSync }");
        let en = observe_enum(&e, "cli.rs", "vibe-cli").expect("observed");
        let names: Vec<String> = en
            .variants
            .iter()
            .map(|v| {
                v.explicit_name
                    .clone()
                    .unwrap_or_else(|| to_kebab_case(&v.ident))
            })
            .collect();
        assert_eq!(names, vec!["install", "redirect-sync"]);
    }

    #[test]
    fn explicit_variant_name_wins_over_derived() {
        let e = parse_enum(
            r#"
#[derive(Debug, Subcommand)]
enum C {
    #[command(name = "command")]
    Drain,
    #[command(name = "self")]
    Vvm,
}
"#,
        );
        let en = observe_enum(&e, "cli.rs", "vibe-cli").expect("observed");
        let names: Vec<String> = en
            .variants
            .iter()
            .map(|v| {
                v.explicit_name
                    .clone()
                    .unwrap_or_else(|| to_kebab_case(&v.ident))
            })
            .collect();
        assert_eq!(names, vec!["command", "self"]);
    }

    #[test]
    fn enum_without_subcommand_derive_yields_nothing() {
        let e = parse_enum("#[derive(Debug)]\nenum C { A, B }");
        assert!(observe_enum(&e, "cli.rs", "x").is_none());
    }

    #[test]
    fn join_emits_one_command_per_variant_with_binary_prefix() {
        let root = CommandRoot {
            binary_name: Some("vibe".to_string()),
            crate_name: "vibe-cli".to_string(),
            subcommand_type: "Command".to_string(),
            file: "cli.rs".to_string(),
            line: 90,
        };
        let en = observe_enum(
            &parse_enum("#[derive(Debug, Subcommand)]\nenum Command { Install, Version }"),
            "cli.rs",
            "vibe-cli",
        )
        .expect("observed");
        let (items, warnings) = join_commands(&[root], &[en]);
        assert!(warnings.is_empty());
        let symbols: Vec<&str> = items.iter().map(|i| i.symbol.as_str()).collect();
        assert_eq!(symbols, vec!["vibe install", "vibe version"]);
        assert!(items.iter().all(|i| i.itemKind == "command"));
        assert!(items.iter().all(|i| i.crateName == "vibe-cli"));
        assert!(items.iter().all(|i| i.fingerprint.is_some()));
    }

    #[test]
    fn command_is_recorded_even_with_no_spec_edge() {
        // The `##x-not-through-tag-item` point: a variant with no `#[spec]`
        // is still recorded, because the join uses the unconditional path.
        let root = CommandRoot {
            binary_name: Some("vibe".to_string()),
            crate_name: "vibe-cli".to_string(),
            subcommand_type: "Command".to_string(),
            file: "cli.rs".to_string(),
            line: 90,
        };
        let en = observe_enum(
            &parse_enum("#[derive(Debug, Subcommand)]\nenum Command { Init }"),
            "cli.rs",
            "vibe-cli",
        )
        .expect("observed");
        let (items, _) = join_commands(&[root], &[en]);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].symbol, "vibe init");
    }

    /// Two crates both declare `pub enum Command`; the join must resolve each
    /// root to its own enum, not whichever the walk visited first. The index
    /// enum is placed first in the slice on purpose, so a type-name-only join
    /// would hand the `vibe` root the index enum's variants.
    #[test]
    fn join_is_crate_local_two_crates_same_enum_name() {
        let vibe_root = CommandRoot {
            binary_name: Some("vibe".to_string()),
            crate_name: "vibe-cli".to_string(),
            subcommand_type: "Command".to_string(),
            file: "cli.rs".to_string(),
            line: 90,
        };
        let index_root = CommandRoot {
            binary_name: Some("vibe-index".to_string()),
            crate_name: "vibe-index".to_string(),
            subcommand_type: "Command".to_string(),
            file: "cli.rs".to_string(),
            line: 55,
        };
        let vibe_enum = observe_enum(
            &parse_enum("#[derive(Debug, Subcommand)]\nenum Command { Agentic, Term }"),
            "cli.rs",
            "vibe-cli",
        )
        .expect("observed");
        let index_enum = observe_enum(
            &parse_enum("#[derive(Debug, Subcommand)]\nenum Command { Reindex, Serve }"),
            "cli.rs",
            "vibe-index",
        )
        .expect("observed");
        // `index_enum` first: a type-name-only join would match both roots to
        // it, leaking `vibe-index`'s variants under the `vibe` binary.
        let (items, warnings) = join_commands(&[vibe_root, index_root], &[index_enum, vibe_enum]);
        assert!(warnings.is_empty());
        let symbols: Vec<&str> = items.iter().map(|i| i.symbol.as_str()).collect();
        assert_eq!(
            symbols,
            vec![
                "vibe agentic",
                "vibe term",
                "vibe-index reindex",
                "vibe-index serve",
            ]
        );
    }

    #[test]
    fn unnamed_root_is_skipped_with_a_warning() {
        let root = CommandRoot {
            binary_name: None,
            crate_name: "x".to_string(),
            subcommand_type: "Command".to_string(),
            file: "cli.rs".to_string(),
            line: 47,
        };
        let en = observe_enum(
            &parse_enum("#[derive(Debug, Subcommand)]\nenum Command { Init }"),
            "cli.rs",
            "x",
        )
        .expect("observed");
        let (items, warnings) = join_commands(&[root], &[en]);
        assert!(items.is_empty());
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "command-root-unnamed");
    }

    #[test]
    fn kebab_handles_acronyms_and_digits() {
        assert_eq!(to_kebab_case("RedirectSync"), "redirect-sync");
        assert_eq!(to_kebab_case("HTTPRequest"), "http-request");
        assert_eq!(to_kebab_case("Install"), "install");
        assert_eq!(to_kebab_case("Bin"), "bin");
        assert_eq!(to_kebab_case("Vvm"), "vvm");
    }
}
