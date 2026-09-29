//! The tree walks — the seam where a project's spec sources (either
//! serialisation) become parsed units. Split out of `mdspec.rs` for the
//! file-length budget (the same split as `lines` and `excludes`), and
//! because the walks are the one place the two readers meet: routing by
//! extension, one address space, and the one-document-one-form law.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#spec-units");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::excludes;
use super::{parse_units, parse_units_with};
use crate::config::Config;
use crate::fwd;
use crate::generated::specmap::{SpecUnit, Warning};

/// Whether a path/name carries a spec-source serialisation extension — the
/// two forms a spec document ships in (Markdown or dialect XML). Routing is
/// by extension; the address is not (see [`super::canonical_doc_path`]).
fn is_spec_ext(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md" | "xml")
    )
}

/// Whether a `/`-separated file string names the XML form.
fn is_xml_form(file: &str) -> bool {
    file.ends_with(".xml")
}

/// The `X.md` + `X.xml` pairs among `sources` (`(repo-relative string, path)`
/// in walk order) — one logical document found in both serialisations. The
/// mixed tree holds each document in ONE form; a pair is a split brain, and
/// split brains are reported, never resolved by guessing which half to read.
/// Keyed on `(directory, stem)`, so the same stem in two directories is two
/// documents, not a pair.
fn pair_collisions(sources: &[(String, PathBuf)]) -> Vec<(String, String)> {
    let mut by_stem: BTreeMap<(String, String), (Option<String>, Option<String>)> = BTreeMap::new();
    for (file_rel, _) in sources {
        let (dir, name) = match file_rel.rsplit_once('/') {
            Some((d, n)) => (d.to_string(), n.to_string()),
            None => (String::new(), file_rel.clone()),
        };
        let stem = name
            .strip_suffix(".md")
            .or_else(|| name.strip_suffix(".xml"))
            .unwrap_or(&name)
            .to_string();
        let slot = by_stem.entry((dir, stem)).or_default();
        if is_xml_form(file_rel) {
            slot.1 = Some(file_rel.clone());
        } else {
            slot.0 = Some(file_rel.clone());
        }
    }
    by_stem
        .into_values()
        .filter_map(|(md, xml)| Some((md?, xml?)))
        .collect()
}

/// The loud warning every pair collision carries — one wording, both paths,
/// the law itself.
fn pair_collision_warning(md: &str, xml: &str) -> Warning {
    Warning {
        code: "pair-collision".to_string(),
        message: format!(
            "`{md}` and `{xml}` are one logical document in two forms — \
             one document, one form; delete one of the pair or rename one"
        ),
        file: md.to_string(),
        line: 0,
    }
}

/// Walk each `<spec_root>/**/*.md` and `<spec_root>/**/*.xml` under the repo
/// root, then the explicit [`Config::root_spec_docs`] (either form). The two
/// serialisations route to their readers by extension and mint into ONE
/// address space — [`super::canonical_doc_path`] strips either extension, so
/// a document's address never depends on its serialisation. A `X.md` +
/// `X.xml` pair in one directory is a loud `pair-collision` warning naming
/// both paths, and BOTH halves are skipped — the walker never guesses which
/// form of a split brain to read. Deterministic order. [
/// `Config::spec_exclude`] is applied to **both** halves — a match leaves
/// the inventory before it is parsed — by the same law the progress gate
/// applies its `exclude` after its includes. A pattern that matched nothing,
/// or that is not a valid glob, speaks up through its own warning (see [
/// `SpecExcludes`]).
pub fn scan_spec_tree(root: &Path, cfg: &Config) -> (Vec<SpecUnit>, Vec<Warning>) {
    let mut units = Vec::new();
    let mut warnings = Vec::new();
    let (mut excludes, bad_globs) = excludes::SpecExcludes::compile(&cfg.spec_exclude);
    // Bad globs are discovered before any walk, so they lead the warnings.
    warnings.extend(bad_globs);
    let mut sources: Vec<(String, PathBuf)> = Vec::new();
    for spec_root_rel in &cfg.spec_roots {
        let spec_root = root.join(spec_root_rel);
        for entry in WalkDir::new(&spec_root)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !is_spec_ext(path) {
                continue;
            }
            let rel = path.strip_prefix(root).unwrap_or(path);
            let file_rel = fwd(rel);
            // `file_rel` is the exact string the SpecUnit would carry as
            // `file`; matching it (not the OS path) is what the exclude key
            // pays for — the printed path and the matched path are one.
            if excludes.matches(&file_rel) {
                continue;
            }
            sources.push((file_rel, path.to_path_buf()));
        }
    }
    let mut root_sources: Vec<(String, PathBuf)> = Vec::new();
    for name in &cfg.root_spec_docs {
        let path = root.join(name);
        if !path.exists() {
            continue;
        }
        // The exclude applies to this half too: `name` is the exact string a
        // SpecUnit minted from a root doc carries as `file`, so it is what the
        // pattern is tested against — uniformly with the spec-roots half.
        if excludes.matches(name) {
            continue;
        }
        root_sources.push((name.clone(), path));
    }
    // The one-document-one-form law is judged over the union of both halves
    // before any parse, so a pair is skipped whole — never double-read into
    // duplicate units and a wall of synthetic duplicate-anchor warnings.
    let mut all: Vec<(String, PathBuf)> = sources;
    all.extend(root_sources.iter().cloned());
    let collisions = pair_collisions(&all);
    for (md, xml) in &collisions {
        warnings.push(pair_collision_warning(md, xml));
    }
    let is_colliding = |file_rel: &str| {
        collisions
            .iter()
            .any(|(md, xml)| md == file_rel || xml == file_rel)
    };
    for (file_rel, path) in &all {
        if is_colliding(file_rel) {
            continue;
        }
        // Root docs keep the bare seam (no long-section policy), exactly as
        // they did in the markdown-only walk.
        let root_doc = root_sources.iter().any(|(n, _)| n == file_rel);
        let parsed = match std::fs::read_to_string(path) {
            Ok(text) => {
                if is_xml_form(file_rel) {
                    if root_doc {
                        crate::xmlspec::parse_units(file_rel, &text, &cfg.namespace)
                    } else {
                        crate::xmlspec::parse_units_with(
                            file_rel,
                            &text,
                            &cfg.namespace,
                            cfg.max_section_lines,
                            cfg.section_grain,
                        )
                    }
                } else if root_doc {
                    parse_units(file_rel, &text, &cfg.namespace)
                } else {
                    parse_units_with(
                        file_rel,
                        &text,
                        &cfg.namespace,
                        cfg.max_section_lines,
                        cfg.section_grain,
                    )
                }
            }
            Err(e) => {
                warnings.push(Warning {
                    code: "unreadable-file".to_string(),
                    message: format!("could not read: {e}"),
                    file: file_rel.clone(),
                    line: 0,
                });
                continue;
            }
        };
        let (mut u, mut w) = parsed;
        units.append(&mut u);
        warnings.append(&mut w);
    }
    // Stale patterns can only be known once both halves have walked, so they
    // trail the warnings.
    warnings.extend(excludes.stale_warnings());
    (units, warnings)
}

/// Scan each [`Config::external_specs`] tree — an installed package's spec
/// directory, either serialisation — and mint its units under that package's
/// namespace. These units participate in **resolution only** (dangling
/// suppression, suspect revisions, queries); the caller never serialises
/// them into the project's own index, and their parse warnings are the
/// package's business, not this project's, so they are dropped. A missing
/// root is reported to stderr and skipped (the package may simply not be
/// installed yet), never a failure. A `X.md` + `X.xml` pair inside one
/// external tree is the same split brain as in the project tree; this walk
/// has no warning channel, so the pair is reported to stderr and both halves
/// skipped — the same loud law, the walk's own idiom.
pub fn scan_external_units(root: &Path, cfg: &Config) -> Vec<SpecUnit> {
    let mut units = Vec::new();
    for ext in &cfg.external_specs {
        let base = root.join(&ext.root);
        if !base.is_dir() {
            eprintln!(
                "specmap: external spec root `{}` (namespace `{}`) not found — \
                 skipped; install the package to resolve its units",
                ext.root, ext.namespace
            );
            continue;
        }
        let mut sources: Vec<(String, PathBuf)> = Vec::new();
        for entry in WalkDir::new(&base)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !is_spec_ext(path) {
                continue;
            }
            // Doc-paths are minted relative to the external tree itself, so
            // `<ext.root>/mechanisms/X.md` reads `spec://<ns>/mechanisms/X#…`.
            let rel = path.strip_prefix(&base).unwrap_or(path);
            sources.push((fwd(rel), path.to_path_buf()));
        }
        let collisions = pair_collisions(&sources);
        for (md, xml) in &collisions {
            eprintln!(
                "specmap: external spec pair `{md}` + `{xml}` — one document, one \
                 form; both skipped (namespace `{}`)",
                ext.namespace
            );
        }
        for (file_rel, path) in &sources {
            if collisions
                .iter()
                .any(|(md, xml)| md == file_rel || xml == file_rel)
            {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(path) {
                let parse = if is_xml_form(file_rel) {
                    crate::xmlspec::parse_units
                } else {
                    parse_units
                };
                let (mut u, _w) = parse(file_rel, &text, &ext.namespace);
                units.append(&mut u);
            }
        }
    }
    units
}
