//! The specmap orphan ratchet — the Phase 2 "flip" (PLAYBOOK
//! `#phase2`): `specmap --check` becomes blocking for non-exempt
//! crates whose public surface carries untagged items.
//!
//! v0 semantics:
//!
//! - `specmap-ratchet.json` at the workspace root lists the crates
//!   **exempt** from the orphan gate; a crate not listed is gated.
//!   Absent file = gate off everywhere (pre-flip behaviour).
//! - An **orphan** is a `pub` top-level item (`fn` / `struct` / `enum`
//!   / `trait` / `type`) in a gated crate with no own edge and no
//!   `scope!`-inherited module edge. Impl methods and
//!   `pub(crate)`-and-narrower visibility are out of scope —
//!   PROP-014 §2.3: private helpers need no annotation; method-level
//!   precision is opt-in via `#[spec]` where it pays.
//! - A symbol may carry a recorded **disposition** (a debt id): the
//!   orphan is reported but does not block — the
//!   "empty or dispositioned into debt.json" arm of the Phase 2
//!   acceptance.
//!
//! Orphans are computed at gate time and deliberately not serialised
//! into `specmap.json`; the full PROP-014 §2.5 orphan table lands when
//! PROP-014 itself is unit-ified after ratification.

specmark::scope!("spec://vibevm/neworder/PROP-014#index");

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use syn::spanned::Spanned;
use vibe_wire::generated::specmap::Specmap;
use walkdir::WalkDir;

use crate::fwd;

/// `specmap-ratchet.json`, deserialised.
#[derive(Debug, Deserialize)]
pub struct Ratchet {
    pub schema: u32,
    /// Crate directory names exempt from the orphan gate.
    #[serde(default)]
    pub exempt: Vec<String>,
    /// Orphans allowed to stand, each carrying its debt id.
    #[serde(default)]
    pub dispositioned: Vec<Disposition>,
}

#[derive(Debug, Deserialize)]
pub struct Disposition {
    pub symbol: String,
    pub debt: String,
}

pub fn ratchet_path(root: &Path) -> PathBuf {
    root.join("specmap-ratchet.json")
}

/// Load the ratchet file; `None` when it does not exist (gate off).
pub fn load(root: &Path) -> Result<Option<Ratchet>> {
    let path = ratchet_path(root);
    if !path.exists() {
        return Ok(None);
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let ratchet: Ratchet =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    Ok(Some(ratchet))
}

/// One untagged public item found by the gate.
#[derive(Debug)]
pub struct Orphan {
    pub symbol: String,
    pub item_kind: String,
    pub crate_name: String,
    pub file: String,
    pub line: u32,
    /// `Some(debt-id)` when the ratchet file dispositions this symbol.
    pub disposition: Option<String>,
}

/// Compute the orphan list for every gated (non-exempt) crate, checked
/// against the already-built index `map`.
pub fn orphans(root: &Path, map: &Specmap, ratchet: &Ratchet) -> Vec<Orphan> {
    let tagged: HashSet<&str> = map.edges.iter().map(|e| e.fromSymbol.as_str()).collect();
    let mut out = Vec::new();

    let mut crate_dirs: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(root.join("crates")) {
        for entry in rd.filter_map(Result::ok) {
            if entry.path().is_dir() {
                crate_dirs.push(entry.path());
            }
        }
    }
    crate_dirs.push(root.join("xtask"));
    crate_dirs.sort();

    for crate_dir in crate_dirs {
        let crate_name = crate_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if ratchet.exempt.iter().any(|e| e == &crate_name) {
            continue;
        }
        let crate_ident = crate_name.replace('-', "_");
        // Public API surface lives under src/; tests/ targets are
        // their own (never-consumed) crates and are not gated.
        let src = crate_dir.join("src");
        for entry in WalkDir::new(&src)
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
            if fwd(rel_in_crate).contains("/generated/") {
                continue;
            }
            let Some(module) = crate::rscan::module_path(&crate_ident, rel_in_crate) else {
                continue;
            };
            let rel = path.strip_prefix(root).unwrap_or(path);
            let file = fwd(rel);
            let Ok(text) = std::fs::read_to_string(path) else {
                continue;
            };
            // B5: an unparseable file is rscan's warning already; the
            // gate just skips it.
            let Ok(ast) = syn::parse_file(&text) else {
                continue;
            };
            collect_orphans(
                &ast.items,
                &module,
                &crate_name,
                &file,
                &tagged,
                ratchet,
                &mut out,
            );
        }
    }
    out
}

fn is_pub(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

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

#[allow(clippy::too_many_arguments)]
fn collect_orphans(
    items: &[syn::Item],
    module: &str,
    crate_name: &str,
    file: &str,
    tagged: &HashSet<&str>,
    ratchet: &Ratchet,
    out: &mut Vec<Orphan>,
) {
    // A scope!-style edge on the module itself covers every item in it.
    if tagged.contains(module) {
        return;
    }
    for item in items {
        let line = item.span().start().line as u32;
        let found: Option<(String, &str)> = match item {
            syn::Item::Fn(f) if is_pub(&f.vis) => {
                Some((format!("{module}::{}", f.sig.ident), "fn"))
            }
            syn::Item::Struct(s) if is_pub(&s.vis) => {
                Some((format!("{module}::{}", s.ident), "struct"))
            }
            syn::Item::Enum(e) if is_pub(&e.vis) => {
                Some((format!("{module}::{}", e.ident), "enum"))
            }
            syn::Item::Trait(t) if is_pub(&t.vis) => {
                Some((format!("{module}::{}", t.ident), "trait"))
            }
            syn::Item::Type(t) if is_pub(&t.vis) => {
                Some((format!("{module}::{}", t.ident), "type"))
            }
            syn::Item::Mod(m) if is_pub(&m.vis) && !is_cfg_test(&m.attrs) => {
                if let Some((_, inner)) = &m.content {
                    let sub = format!("{module}::{}", m.ident);
                    collect_orphans(inner, &sub, crate_name, file, tagged, ratchet, out);
                }
                None
            }
            _ => None,
        };
        let Some((symbol, item_kind)) = found else {
            continue;
        };
        if tagged.contains(symbol.as_str()) {
            continue;
        }
        let disposition = ratchet
            .dispositioned
            .iter()
            .find(|d| d.symbol == symbol)
            .map(|d| d.debt.clone());
        out.push(Orphan {
            symbol,
            item_kind: item_kind.to_string(),
            crate_name: crate_name.to_string(),
            file: file.to_string(),
            line,
            disposition,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ratchet_with(exempt: &[&str]) -> Ratchet {
        Ratchet {
            schema: 1,
            exempt: exempt.iter().map(|s| s.to_string()).collect(),
            dispositioned: Vec::new(),
        }
    }

    fn scan_str(src: &str, tagged: &[&str], ratchet: &Ratchet) -> Vec<Orphan> {
        let ast = syn::parse_file(src).unwrap();
        let tagged: HashSet<&str> = tagged.iter().copied().collect();
        let mut out = Vec::new();
        collect_orphans(
            &ast.items,
            "demo",
            "demo-crate",
            "crates/demo/src/lib.rs",
            &tagged,
            ratchet,
            &mut out,
        );
        out
    }

    #[test]
    fn untagged_pub_item_is_orphan() {
        let r = ratchet_with(&[]);
        let o = scan_str("pub fn naked() {}", &[], &r);
        assert_eq!(o.len(), 1);
        assert_eq!(o[0].symbol, "demo::naked");
    }

    #[test]
    fn tagged_item_is_not_orphan() {
        let r = ratchet_with(&[]);
        let o = scan_str("pub fn covered() {}", &["demo::covered"], &r);
        assert!(o.is_empty());
    }

    #[test]
    fn private_and_pub_crate_items_are_ignored() {
        let r = ratchet_with(&[]);
        let o = scan_str("fn private() {}\npub(crate) fn internal() {}", &[], &r);
        assert!(o.is_empty());
    }

    #[test]
    fn scope_edge_on_module_covers_items() {
        let r = ratchet_with(&[]);
        let o = scan_str("pub fn helper() {}", &["demo"], &r);
        assert!(o.is_empty());
    }

    #[test]
    fn cfg_test_mod_is_skipped() {
        let r = ratchet_with(&[]);
        let o = scan_str(
            "#[cfg(test)]\npub mod tests { pub fn helper() {} }",
            &[],
            &r,
        );
        assert!(o.is_empty());
    }

    #[test]
    fn disposition_is_carried() {
        let mut r = ratchet_with(&[]);
        r.dispositioned.push(Disposition {
            symbol: "demo::naked".to_string(),
            debt: "DBT-9999".to_string(),
        });
        let o = scan_str("pub fn naked() {}", &[], &r);
        assert_eq!(o.len(), 1);
        assert_eq!(o[0].disposition.as_deref(), Some("DBT-9999"));
    }
}
