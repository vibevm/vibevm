//! The layout half of the codegen: where schemas live, which generated
//! tree each schema home owns, what a path segment must look like to
//! become a module name, and the `mod.rs` tree those rules imply. Split
//! from `mod.rs` (the driver: binary lookup, home routing, emission, the
//! drift check) along the responsibility seam — every test the old
//! single file carried exercises exactly this half — and because
//! `mod.rs` sat at 586 of its 600-line budget with a post-processing
//! layer still to be wired into `generate_into`.
//!
//! Two homes, one rule of routing: the host wire contracts under
//! `schemas/` at the repo root, and the specmap schemas inside the
//! `core-ai-native` package (the traceability engine owns its own data
//! model, schema included). A schema's generated module mirrors its path
//! relative to its home, so a schema directory (which carries its epoch —
//! PROP-044 §4.6) becomes a module path instead of being flattened away.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

/// The authored specmap engine package: both the `specmap` schema and the
/// generated types live inside `core-ai-native` (vendored stack copies catch
/// up at release events, never from here).
///
/// The `packages/` prefix is a layout-root literal kept here because
/// xtask carries no vibe-core edge; the single home of the root names
/// is `crates/vibe-core/src/layout.rs` (PROP-052 L2) — the R4 relayout
/// sweep retires this duplication.
const SPECMAP_ENGINE_SLOT: &str = "packages/org.vibevm.ai-native/core-ai-native/v0.8.0";

/// The engine package's own `schemas/` — the second schema home the
/// codegen scans besides the host `schemas/` at the repo root.
pub(crate) fn specmap_schema_dir(root: &Path) -> PathBuf {
    root.join(SPECMAP_ENGINE_SLOT).join("schemas")
}

/// The engine crate's generated tree — the output home for every schema
/// under the engine's own `schemas/` (`specmap` owns its data model there,
/// so the traceability engine carries its types and can relocate without an
/// engine → `vibe-wire` edge; Traceability Relocation Plan, Phase 1). One
/// function so schema-home routing and `check-codegen`'s diff set agree on
/// the exact path, never divergent literals.
pub(crate) fn specmap_generated_dir(root: &Path) -> PathBuf {
    root.join(SPECMAP_ENGINE_SLOT)
        .join("crates/core-ai-native-specmap/src/generated")
}

/// The vibe-wire generated tree — the home of every host wire type and, since
/// `FormatId` routes wire I/O through `wire::publish/load` (PROP-044 §4.1, gate
/// G1), the home of the generated format-id enum too. One function so every
/// site that needs the path (schema-home routing, the `format_id` emission
/// branch, `check-codegen`'s diff set) compares against exactly one literal,
/// never a divergent copy.
pub(crate) fn vibe_wire_generated_dir(root: &Path) -> PathBuf {
    root.join("crates/vibe-wire/src/generated")
}

/// `foo.jtd.json` → `foo`.
fn schema_stem(schema: &Path) -> Result<String> {
    schema
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".jtd.json"))
        .map(str::to_string)
        .with_context(|| format!("schema name not `*.jtd.json`: {}", schema.display()))
}

/// Every `*.jtd.json` anywhere under `dir`, depth-first — subdirectories
/// carry epochs (PROP-044 §4.6), so schemas live nested and the scan must
/// reach them. The result is sorted: the walk order a filesystem hands back
/// is not guaranteed, and everything downstream (grouping, emission,
/// `check-codegen`) must stay byte-stable across platforms.
pub(crate) fn schemas_under(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut schemas: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(dir) {
        let path = entry
            .with_context(|| format!("reading {}", dir.display()))?
            .into_path();
        if !path.is_file() {
            continue;
        }
        let is_schema = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".jtd.json"))
            .unwrap_or(false);
        if is_schema {
            schemas.push(path);
        }
    }
    schemas.sort();
    Ok(schemas)
}

/// Where a schema's generated module lives: the schema's path relative to
/// its schema home, mirrored under the output tree —
/// `<root>/<rel_dir>/<stem>.jtd.json` → `<out_dir>/<rel_dir>/<stem>/`.
/// Mirroring rather than flattening is contract, not taste: the schema's
/// directory carries its epoch (PROP-044 §4.6 — a heavy break mints the new
/// world as a new path), so `index/e1/entry` and a future `index/e2/entry`
/// must stay distinct modules.
///
/// Every path segment that becomes a module name is checked here, because
/// nothing downstream would: an illegal segment is emitted verbatim into a
/// `pub mod …;` line, codegen exits 0, and the refusal finally arrives as a
/// parse error inside another crate. Measured — a `by-cap.jtd.json` produces
/// `pub mod by-cap;` and breaks `vibe-wire`, naming neither the schema nor
/// the fix. PROP-044 §8 `##AGENT-MESSAGES` asks the opposite: state what was
/// violated and the exact next command.
pub(crate) fn schema_module_dir(
    schema_root: &Path,
    out_dir: &Path,
    schema: &Path,
) -> Result<PathBuf> {
    let rel = schema.strip_prefix(schema_root).with_context(|| {
        format!(
            "schema {} is not under its schema home {}",
            schema.display(),
            schema_root.display()
        )
    })?;
    let rel_dir = rel.parent().unwrap_or(Path::new(""));
    for segment in rel_dir {
        check_module_ident(&segment.to_string_lossy(), schema, "directory")?;
    }
    let stem = schema_stem(schema)?;
    check_module_ident(&stem, schema, "file name")?;
    Ok(out_dir.join(rel_dir).join(stem))
}

/// Rust keywords (2021 edition, including the reserved set) — a schema or
/// directory named after one of these is as unusable as a hyphenated one,
/// and `type`, `crate` or `match` are ordinary words a schema author reaches
/// for. Listed rather than approximated so the check closes the class.
const RUST_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in",
    "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "try", "type",
    "typeof", "union", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Refuse a path segment that cannot be a module name, naming the schema,
/// the offending segment and the fix — instead of letting the generator emit
/// `pub mod <segment>;` and leaving a compiler in another crate to complain
/// about a file nobody wrote by hand.
fn check_module_ident(segment: &str, schema: &Path, what: &str) -> Result<()> {
    let legal_shape = !segment.is_empty()
        && segment
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !legal_shape {
        bail!(
            "schema {schema}: the {what} `{segment}` is not a Rust module name.\n\
             Every directory and file name under a schema home becomes a `pub mod` \
             segment, so it must be ASCII letters, digits and `_`, starting with a \
             letter or `_`.\n\
             Fix: rename it (a hyphen becomes an underscore — `by-cap` → `by_cap`), \
             then run `cargo xtask codegen`.",
            schema = schema.display(),
        );
    }
    if RUST_KEYWORDS.contains(&segment) {
        bail!(
            "schema {schema}: the {what} `{segment}` is a Rust keyword and cannot \
             be a module name.\n\
             Fix: rename it (`{segment}` → `{segment}_`), then run \
             `cargo xtask codegen`.",
            schema = schema.display(),
        );
    }
    Ok(())
}

/// The `mod.rs` tree for an output directory: every directory from `out_dir`
/// down to each generated leaf gets a `mod.rs` listing its direct submodule
/// children. Built by walking up from every leaf so intermediate directories
/// (the epoch dirs between root and leaf) are registered even though no
/// schema of their own sits there. `BTreeMap` / `BTreeSet` keep both the
/// directory order and each child list sorted — determinism across platforms
/// (the filesystem walk order is not guaranteed).
pub(crate) fn module_tree(
    out_dir: &Path,
    leaves: &[PathBuf],
) -> BTreeMap<PathBuf, BTreeSet<String>> {
    let mut tree: BTreeMap<PathBuf, BTreeSet<String>> = BTreeMap::new();
    for leaf in leaves {
        let mut child = leaf.clone();
        while child != *out_dir {
            let (Some(parent), Some(name)) = (child.parent(), child.file_name()) else {
                break;
            };
            tree.entry(parent.to_path_buf())
                .or_default()
                .insert(name.to_string_lossy().into_owned());
            child = parent.to_path_buf();
        }
    }
    tree
}

#[cfg(test)]
#[path = "layout/tests.rs"]
mod tests;
