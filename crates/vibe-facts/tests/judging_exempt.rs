//! Documentation is observed and never judged (PROP-057
//! `##OBS-NOT-JUDGED`), and the list that says so cannot go stale.
//!
//! `[judging] exempt` in `facts.toml` is an enumerated list of globs, and
//! an enumerated list rots the moment the thing it enumerates grows: a
//! second documentation package — a translation, the site's own manual,
//! anything of kind `doc` — would enter the judging debt silently, and the
//! debt is precisely the number nobody watches per file. So the tree is
//! walked here for packages of kind `doc`, and every one of them must be
//! covered.
//!
//! The assertion is on package SLOTS rather than on the observed corpus on
//! purpose: the exemption must be declared before the include glob that
//! makes a package observable, never after, so that no package spends a
//! single commit both observed and judged.

use std::path::{Path, PathBuf};

use progress_core::scope::{self, JudgingExemption};
use vibe_core::PackageKind;

/// The repository root: this crate sits at `crates/vibe-facts`.
///
/// Left uncanonicalised deliberately — on Windows the canonical form is an
/// extended-length path, which refuses the `/` separators every glob in
/// `facts.toml` is written with.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// Every in-tree package slot declaring `kind = "doc"`, as `/`-separated
/// repo-relative paths of the slot directory.
///
/// The regenerated dependency copies under `vibedeps/` and `.vibe/` are
/// skipped by the engine's own always-on rule: marking a copy is marking
/// nothing, and the copy is not this project's to exempt either.
fn doc_package_slots(root: &Path) -> Vec<String> {
    let mut slots = Vec::new();
    let mut stack = vec![root.join("vibevm").join("vibepacks")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let skip = path.file_name().is_some_and(|n| {
                    let name = n.to_string_lossy();
                    scope::DEFAULT_EXCLUDES.iter().any(|e| name == *e)
                });
                if !skip {
                    stack.push(path);
                }
                continue;
            }
            if path.file_name().and_then(|n| n.to_str()) != Some("vibe.toml") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if declared_kind(&text) != Some(PackageKind::Doc) {
                continue;
            }
            let slot = path.parent().expect("a manifest has a directory");
            let rel = slot.strip_prefix(root).expect("under the repository root");
            slots.push(scope::rel_str(rel));
        }
    }
    slots.sort();
    slots
}

/// The `[package] kind` of one manifest, read as the closed vocabulary it
/// is — so renaming the kind moves this test with it rather than leaving a
/// string behind.
fn declared_kind(manifest: &str) -> Option<PackageKind> {
    // `toml::from_str` and not `str::parse`: at toml 0.9 the `FromStr` of
    // `Value` reads a bare VALUE, and a whole manifest handed to it fails
    // with "unexpected content, expected nothing" — silently, if the error
    // is discarded, which would leave this guard passing over a tree it
    // never read.
    let value: toml::Value = toml::from_str(manifest).ok()?;
    value
        .get("package")?
        .get("kind")?
        .as_str()?
        .parse::<PackageKind>()
        .ok()
}

fn exemption(root: &Path) -> JudgingExemption {
    scope::load_config(root)
        .expect("facts.toml loads")
        .judging_exemption()
        .expect("the judging exempt globs compile")
}

/// The guard: no package of kind `doc` can enter the judging debt, because
/// every one of them is covered by `[judging] exempt`.
///
/// A failure here has exactly one repair — add the package's glob to
/// `[judging] exempt` in `facts.toml` — and exactly one thing it must not
/// be: a `exclude` entry, which would hide the pages instead of freeing
/// them from judgement.
#[test]
fn every_in_tree_doc_package_is_exempt_from_judging() {
    let root = repo_root();
    let exemption = exemption(&root);
    let slots = doc_package_slots(&root);
    assert!(
        !slots.is_empty(),
        "no package of kind `doc` found under vibevm/vibepacks — this test \
         guards a list against going stale and cannot do that against an \
         empty tree; if the last documentation package really is gone, \
         delete this test with it"
    );
    let uncovered: Vec<&String> = slots
        .iter()
        .filter(|slot| !exemption.covers(Path::new(&format!("{slot}/any-page.xml"))))
        .collect();
    assert!(
        uncovered.is_empty(),
        "these packages declare `kind = \"doc\"` and would enter the judging \
         debt: {uncovered:?} — add a glob for each to `[judging] exempt` in \
         facts.toml (never to `exclude`: documentation is observed, only \
         never judged)"
    );
}

/// The files, not merely the directory: a glob that names a slot but not
/// what is under it would pass the slot check and still leave every page
/// judged, because the debt is counted per file.
#[test]
fn the_exemption_reaches_the_pages_and_not_just_the_slot() {
    let root = repo_root();
    let exemption = exemption(&root);
    for slot in doc_package_slots(&root) {
        let mut pages = 0usize;
        let mut stack = vec![root.join(&slot)];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let ext = path.extension().and_then(|e| e.to_str());
                if ext != Some("xml") && ext != Some("md") {
                    continue;
                }
                let rel = path.strip_prefix(&root).expect("under the root");
                assert!(
                    exemption.covers(rel),
                    "{}: inside the exempt package `{slot}`, yet the globs do \
                     not reach it — the debt is counted per file, so a glob \
                     that names only the slot exempts nothing",
                    scope::rel_str(rel)
                );
                pages += 1;
            }
        }
        assert!(pages > 0, "{slot}: a documentation package with no pages");
    }
}

/// An exemption is not an exclusion, and the two must never be spelled for
/// the same path: `exclude` removes a file from the corpus, `exempt` keeps
/// it there and frees it from judgement alone. A package named by both
/// would be invisible while looking cared for.
#[test]
fn no_documentation_package_is_excluded_as_well_as_exempt() {
    let root = repo_root();
    let config = scope::load_config(&root).expect("facts.toml loads");
    let excluded = JudgingExemption::compile(&config.exclude).expect("the exclude globs compile");
    for slot in doc_package_slots(&root) {
        assert!(
            !excluded.covers(Path::new(&format!("{slot}/any-page.xml"))),
            "{slot}: exempt from judgement AND excluded from the corpus — \
             documentation is observed, checked and mapped; only its \
             verdicts are waived"
        );
    }
}
