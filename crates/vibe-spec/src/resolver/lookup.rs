//! Doc-path → file, and the layout roots that lookup happens inside.
//!
//! Split from `resolver.rs` along the seam the two halves already had:
//! the parent decides *which `spec/` root* an authority means (self
//! coordinate, selected world, slot), this cell decides *which file* a
//! doc-path means inside one root — including the lossy `PROP-NNN` /
//! `FEAT-NNN` prefix-scan and the one-document-one-form refusal.

use std::fs;
use std::path::{Path, PathBuf};

use super::ResolveError;

/// Resolve a doc-path (relative to a `spec/` root) to a spec-source file —
/// either serialisation (`.md` or `.xml`; a document's address does not
/// depend on its form, PROP-045 ##ADDRESSING-UNCHANGED) — inverting the
/// `PROP-NNN` / `FEAT-NNN` truncation by a prefix-scan. `X.md` + `X.xml`
/// beside each other is a [`ResolveError::PairCollision`]: one document,
/// one form, and the resolver never guesses which half of a split brain
/// to read.
pub(super) fn resolve_doc(base_spec: &Path, doc_path: &str) -> Result<PathBuf, ResolveError> {
    resolve_doc_with_formats(base_spec, doc_path, &[]).map(|resolved| resolved.path)
}

pub(super) struct ResolvedDoc {
    pub(super) path: PathBuf,
    pub(super) extension: String,
    pub(super) physical_stem: String,
}

pub(super) fn resolve_doc_with_formats(
    base_spec: &Path,
    doc_path: &str,
    active_formats: &[String],
) -> Result<ResolvedDoc, ResolveError> {
    let (dir, last) = match doc_path.rsplit_once('/') {
        Some((d, l)) => (base_spec.join(d), l),
        None => (base_spec.to_path_buf(), doc_path),
    };

    if is_id_stem(last) {
        let matches: Vec<PathBuf> = read_dir_or_empty(&dir)
            .map(|e| e.path())
            .filter(|p| id_file_matches_with_formats(p, last, active_formats))
            .collect();
        if let Some(collision) = active_same_stem_collision(doc_path, &matches, active_formats) {
            return Err(collision);
        }
        if let Some((md, xml)) = pair_among(&matches) {
            return Err(ResolveError::PairCollision { markdown: md, xml });
        }
        match matches.as_slice() {
            [] => Err(ResolveError::DocNotFound {
                doc_path: doc_path.to_string(),
                base: base_spec.display().to_string(),
            }),
            [one] => resolved(one),
            many => Err(ResolveError::AmbiguousDoc {
                id: last.to_string(),
                count: many.len(),
                dir: dir.display().to_string(),
            }),
        }
    } else {
        let mut candidates = vec![
            base_spec.join(format!("{doc_path}.md")),
            base_spec.join(format!("{doc_path}.xml")),
        ];
        candidates.extend(
            active_formats
                .iter()
                .map(|format| base_spec.join(format!("{doc_path}.{format}"))),
        );
        candidates.sort();
        candidates.dedup();
        candidates.retain(|candidate| candidate.is_file());
        if candidates.len() > 1
            && candidates.iter().any(|candidate| {
                candidate
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|extension| active_formats.iter().any(|value| value == extension))
            })
        {
            return Err(active_collision(doc_path, &candidates));
        }
        if let Some((md, xml)) = pair_among(&candidates) {
            return Err(ResolveError::PairCollision { markdown: md, xml });
        }
        match candidates.as_slice() {
            [one] => resolved(one),
            [] => Err(ResolveError::DocNotFound {
                doc_path: doc_path.to_string(),
                base: base_spec.display().to_string(),
            }),
            many => Err(active_collision(doc_path, many)),
        }
    }
}

/// The first same-stem `.md` + `.xml` pair among `files`, if any — the
/// one-document-one-form law over an already-filtered candidate list.
pub(super) fn pair_among(files: &[PathBuf]) -> Option<(PathBuf, PathBuf)> {
    for a in files {
        if !is_md(a) {
            continue;
        }
        for b in files {
            if is_xml(b) && a.file_stem() == b.file_stem() {
                return Some((a.clone(), b.clone()));
            }
        }
    }
    None
}

pub(super) fn is_md(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()) == Some("md")
}

fn is_xml(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()) == Some("xml")
}

/// Does a file stem (either serialisation's extension stripped) equal `id`
/// or start with `id-` (the descriptive-slug form)?
#[cfg(test)]
pub(super) fn id_file_matches(path: &Path, id: &str) -> bool {
    id_file_matches_with_formats(path, id, &[])
}

fn id_file_matches_with_formats(path: &Path, id: &str, active_formats: &[String]) -> bool {
    let extension = path.extension().and_then(|value| value.to_str());
    if !matches!(extension, Some("md" | "xml"))
        && !active_formats
            .iter()
            .any(|format| Some(format.as_str()) == extension)
    {
        return false;
    }
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return false;
    };
    stem == id
        || stem
            .strip_prefix(id)
            .is_some_and(|rest| rest.starts_with('-'))
}

fn resolved(path: &Path) -> Result<ResolvedDoc, ResolveError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let physical_stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    Ok(ResolvedDoc {
        path: path.to_owned(),
        extension: extension.to_owned(),
        physical_stem: physical_stem.to_owned(),
    })
}

fn active_collision(doc_path: &str, files: &[PathBuf]) -> ResolveError {
    ResolveError::ActiveFormatCollision {
        doc_path: doc_path.to_owned(),
        files: files
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn active_same_stem_collision(
    doc_path: &str,
    files: &[PathBuf],
    active_formats: &[String],
) -> Option<ResolveError> {
    for file in files {
        let stem = file.file_stem()?;
        let same = files
            .iter()
            .filter(|candidate| candidate.file_stem() == Some(stem))
            .cloned()
            .collect::<Vec<_>>();
        if same.len() > 1
            && same.iter().any(|candidate| {
                candidate
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|extension| active_formats.iter().any(|value| value == extension))
            })
        {
            return Some(active_collision(doc_path, &same));
        }
    }
    None
}

// --- The PROP-052 relayout seam — the one sanctioned duplication --------
//
// vibe-spec cannot depend on vibe-core (no new dependencies; and the
// core's layout module — `crates/vibe-core/src/layout.rs` — is the ONE
// home of the root names, PROP-052 L2). These four names are the single
// sanctioned duplication outside that module: the forward half
// (`canonical_doc_path`) strips both specs-root prefixes, and the
// reverse half (the disk walk below) probes the new root first and
// falls back to the legacy one — so behaviour on the legacy tree
// (today's) is byte-identical, and the R4 flip needs no edit here.
pub(crate) const NEW_SPECS_ROOT: &str = "vibevm/vibespecs";
pub(crate) const LEGACY_SPECS_ROOT: &str = "spec";
pub(crate) const NEW_VIBEDEPS_ROOT: &str = "vibevm/vibedeps";
pub(crate) const LEGACY_VIBEDEPS_ROOT: &str = "vibedeps";

/// The specs root of whichever layout is live under `base`: the new
/// `vibevm/vibespecs` when it exists on disk, else the legacy `spec`
/// (the fallback also names the legacy root in `DocNotFound` errors on
/// a tree that carries neither — the pre-relayout message, byte for
/// byte).
pub(crate) fn specs_root_under(base: &Path) -> PathBuf {
    let new = base.join(NEW_SPECS_ROOT);
    if new.is_dir() {
        new
    } else {
        base.join(LEGACY_SPECS_ROOT)
    }
}

/// The dependency-slot root of whichever layout is live under `base`:
/// `vibevm/vibedeps` when it exists, else the legacy `vibedeps`
/// (same probe discipline as [`specs_root_under`]).
pub(crate) fn vibedeps_root_under(base: &Path) -> PathBuf {
    let new = base.join(NEW_VIBEDEPS_ROOT);
    if new.is_dir() {
        new
    } else {
        base.join(LEGACY_VIBEDEPS_ROOT)
    }
}

/// The forward half of this router's law: a spec file's canonical
/// citation doc-path — relative to the specs root, the serialisation
/// extension stripped, and a `PROP-NNN` / `FEAT-NNN` descriptive-slug
/// filename truncated to its id (`spec/modules/x/PROP-003-dep-evolution.md`
/// → `modules/x/PROP-003`), so `resolve_doc` inverts it. Files without
/// a document id keep their full stem (`boot/00-core`, `WAL`).
///
/// The specs root is two-shaped today (PROP-052, the relayout): the
/// physical file may still sit under the legacy `spec/` prefix or under
/// the new `vibevm/vibespecs/` one — the LONG prefix is checked first,
/// and both canonicalise to the same doc-path (L1: physics moves,
/// addresses do not). vibe-spec cannot depend on vibe-core, so the two
/// prefixes are spelled here as the one sanctioned duplication of the
/// layout names outside `vibe_core::layout` (see
/// `crates/vibe-core/src/layout.rs`, PROP-052 L2).
pub fn canonical_doc_path(file_rel: &str) -> String {
    canonical_doc_path_with_formats(file_rel, &[])
}

pub fn canonical_doc_path_with_formats(file_rel: &str, active_formats: &[String]) -> String {
    let rel = file_rel
        .strip_prefix(&format!("{NEW_SPECS_ROOT}/"))
        .or_else(|| file_rel.strip_prefix(&format!("{LEGACY_SPECS_ROOT}/")))
        .unwrap_or(file_rel);
    let (dir, name) = match rel.rsplit_once('/') {
        Some((d, n)) => (Some(d), n),
        None => (None, rel),
    };
    let builtin = name
        .strip_suffix(".md")
        .or_else(|| name.strip_suffix(".xml"));
    let custom = active_formats
        .iter()
        .find_map(|format| name.strip_suffix(&format!(".{format}")));
    let stem = builtin.or(custom).unwrap_or(name);
    let mut parts = stem.split('-');
    let canonical = match (parts.next(), parts.next()) {
        (Some(kind @ ("PROP" | "FEAT")), Some(num))
            if !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()) =>
        {
            format!("{kind}-{num}")
        }
        _ => stem.to_string(),
    };
    match dir {
        Some(d) => format!("{d}/{canonical}"),
        None => canonical,
    }
}

/// A `PROP-NNN` / `FEAT-NNN` id stem (the truncated doc-path tail).
pub(super) fn is_id_stem(s: &str) -> bool {
    let Some((kind, num)) = s.split_once('-') else {
        return false;
    };
    (kind == "PROP" || kind == "FEAT") && !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit())
}

/// Iterate a directory's entries, yielding nothing if it is unreadable or
/// absent (the resolver degrades to "not found", never panics).
pub(super) fn read_dir_or_empty(dir: &Path) -> impl Iterator<Item = fs::DirEntry> {
    fs::read_dir(dir).into_iter().flatten().flatten()
}

/// The PROP-052 half of [`canonical_doc_path`]: both physical layouts
/// canonicalise to one doc-path (L1 — addresses survive the move).
/// Kept inline in this file — not in the sibling `tests` module — so the
/// relayout slice touches exactly this one file.
#[cfg(test)]
mod canonical_doc_path_layout_tests {
    use super::{LEGACY_SPECS_ROOT, NEW_SPECS_ROOT, canonical_doc_path};

    #[test]
    fn one_doc_path_for_both_layouts() {
        // The PROP-052 ##ADDRESSES-SURVIVE-THE-MOVE proof: the same
        // document reached under either physical prefix canonicalises
        // to the identical citation doc-path. The prefixes are built
        // from the module's sanctioned duplication pair — no fresh
        // literals here.
        for doc in [
            "common/PROP-000.xml",
            "common/PROP-000.md",
            "common/PROP-046-adoption-facts-registry.md",
            "modules/x/PROP-003-dep-evolution.md",
            "modules/x/FEAT-012-thing.xml",
            "boot/00-core.md",
            "WAL.xml",
        ] {
            let new = format!("{NEW_SPECS_ROOT}/{doc}");
            let old = format!("{LEGACY_SPECS_ROOT}/{doc}");
            assert_eq!(
                canonical_doc_path(&new),
                canonical_doc_path(&old),
                "diverged for {doc}"
            );
        }
    }

    #[test]
    fn new_prefix_strips_and_truncates_like_the_old() {
        // Not just equality of the two shapes: each new-layout input
        // lands on the exact expected doc-path (slug truncation, the
        // id-less stem, nested dirs, either serialisation).
        assert_eq!(
            canonical_doc_path("vibevm/vibespecs/modules/x/PROP-003-dep-evolution.md"),
            "modules/x/PROP-003"
        );
        assert_eq!(
            canonical_doc_path("vibevm/vibespecs/common/FEAT-012-thing.xml"),
            "common/FEAT-012"
        );
        assert_eq!(
            canonical_doc_path("vibevm/vibespecs/boot/00-core.md"),
            "boot/00-core"
        );
        assert_eq!(canonical_doc_path("vibevm/vibespecs/WAL.xml"), "WAL");
    }

    #[test]
    fn long_prefix_wins_over_the_short_one() {
        // The strip order is longest-first: the new root is checked
        // before the legacy one, so neither prefix can eat into the
        // other's documents. Both serialisations of the same document
        // canonicalise identically under the new root…
        assert_eq!(
            canonical_doc_path("vibevm/vibespecs/common/PROP-000.xml"),
            canonical_doc_path("vibevm/vibespecs/common/PROP-000.md"),
        );
        // A bare root name without the trailing separator is NOT a doc
        // path and keeps its shape (no partial strip).
        assert_eq!(canonical_doc_path("vibevm/vibespecs"), "vibevm/vibespecs");
        assert_eq!(canonical_doc_path("vibespecs"), "vibespecs");
    }

    #[test]
    fn unprefixed_and_legacy_shapes_are_unchanged() {
        // Additivity: with no prefix at all, or with the legacy one,
        // behaviour is byte-for-byte the pre-relayout function's.
        assert_eq!(canonical_doc_path("PROP-000.xml"), "PROP-000");
        assert_eq!(
            canonical_doc_path("notes/random-file.md"),
            "notes/random-file"
        );
        assert_eq!(
            canonical_doc_path("vibevm/vibespecs/common/PROP-000.xml"),
            "common/PROP-000"
        );
        assert_eq!(canonical_doc_path("spec/WAL.md"), "WAL");
    }
}
