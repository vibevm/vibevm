//! `SpecmapEvidence` — the vibevm side of the PROP-043 §6 evidence seam.
//!
//! `progress-core` knows only the [`EvidenceProvider`] trait; the specmap
//! index is a vibevm fact, so the join lives here, on the adapter side. That
//! is the separability law: the core builds and runs with no provider at all,
//! and a consuming project with no `specmap.json` loses nothing but the
//! column.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#evidence");

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context as _, Result};
use progress_core::evidence::{Evidence, EvidenceProvider};
use specmap_core::generated::specmap::{EdgeVerb, Specmap};
use specmap_core::index::INDEX_REL_PATH;
use specmark::spec;

/// The host `specmap.json`, folded once into the two lookups the report
/// needs: what each spec address is backed by, and which address a progress
/// unit (`<file>#<anchor>`) speaks for.
#[derive(Debug)]
pub struct SpecmapEvidence {
    /// Canonical `spec://…#anchor` → the edges that cite it.
    by_uri: HashMap<String, Evidence>,
    /// A progress unit address (`<file>#<anchor>`) → that unit's canonical
    /// URI, as the index itself declares the correspondence.
    by_unit: HashMap<String, String>,
}

impl SpecmapEvidence {
    /// Load `<root>/specmap.json`.
    ///
    /// `Ok(None)` means the index is absent — most consuming projects have
    /// none, and that is not an error (PROP-043 §6). A *malformed* index is
    /// an error naming the file: a corrupt index must never read as an
    /// absent one.
    pub fn load(root: &Path) -> Result<Option<SpecmapEvidence>> {
        let path = root.join(INDEX_REL_PATH);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(e).with_context(|| format!("reading {}", path.display()));
            }
        };
        let index: Specmap = serde_json::from_str(&text)
            .with_context(|| format!("parsing the traceability index {}", path.display()))?;
        Ok(Some(SpecmapEvidence::from_index(&index)))
    }

    /// Fold a parsed index into the lookups.
    fn from_index(index: &Specmap) -> SpecmapEvidence {
        let mut by_uri: HashMap<String, Evidence> = HashMap::new();
        let mut by_unit: HashMap<String, String> = HashMap::new();
        // Seed from the units: an address the index knows but nothing cites
        // answers `Some(zeros)` — a real "nothing implements this" claim,
        // distinct from the `None` an unknown address gets.
        for u in &index.specUnits {
            by_uri.entry(u.uri.clone()).or_default();
            by_unit.insert(format!("{}#{}", u.file, u.anchor), u.uri.clone());
        }
        for e in &index.edges {
            // `deviates` / `documents` / `informs` are not backing evidence;
            // §6 counts the two verbs that claim the unit is realised.
            let slot = match e.verb {
                EdgeVerb::Implements | EdgeVerb::Verifies => {
                    by_uri.entry(e.uri.clone()).or_default()
                }
                _ => continue,
            };
            match e.verb {
                EdgeVerb::Implements => slot.implements += 1,
                _ => slot.verifies += 1,
            }
            slot.refs.push(format!("{}:{}", e.file, e.line));
        }
        SpecmapEvidence { by_uri, by_unit }
    }
}

impl EvidenceProvider for SpecmapEvidence {
    /// Answer the edges citing `unit_addr`.
    ///
    /// Matching is exact on the full `spec://…#anchor` string. A progress
    /// unit address (`<file>#<anchor>`) reaches the same key through the
    /// index's own `spec_units` table, which states the file ↔ URI
    /// correspondence — a join on index data, never a rewrite of the
    /// address (`spec://…#addressing` owns address forms, not this file).
    #[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#evidence")]
    fn evidence_for(&self, unit_addr: &str) -> Option<Evidence> {
        let uri = if unit_addr.starts_with("spec://") {
            unit_addr
        } else {
            self.by_unit.get(unit_addr).map(String::as_str)?
        };
        self.by_uri.get(uri).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
      "code_items": [],
      "edges": [
        {"file": "crates/a/src/lib.rs", "from_symbol": "a", "line": 10,
         "provenance": "authored", "uri": "spec://p/D#one", "verb": "implements"},
        {"file": "crates/a/src/other.rs", "from_symbol": "a::other", "line": 20,
         "provenance": "authored", "uri": "spec://p/D#one", "verb": "implements"},
        {"file": "crates/a/tests/t.rs", "from_symbol": "t", "line": 30,
         "provenance": "authored", "uri": "spec://p/D#one", "verb": "verifies"},
        {"file": "crates/a/src/lib.rs", "from_symbol": "a", "line": 40,
         "provenance": "authored", "uri": "spec://p/D#one", "verb": "deviates",
         "reason": "not counted"}
      ],
      "schema": 2,
      "spec_units": [
        {"anchor": "one", "content_hash": "sha256:aa", "doc_path": "D",
         "file": "spec/d.md", "heading": "One", "line": 5, "uri": "spec://p/D#one"},
        {"anchor": "two", "content_hash": "sha256:bb", "doc_path": "D",
         "file": "spec/d.md", "heading": "Two", "line": 9, "uri": "spec://p/D#two"}
      ],
      "suspects": [],
      "warnings": []
    }"#;

    fn loaded(dir: &Path, body: &str) -> Result<Option<SpecmapEvidence>> {
        std::fs::write(dir.join(INDEX_REL_PATH), body).expect("write index");
        SpecmapEvidence::load(dir)
    }

    #[test]
    fn specmap_evidence_counts_edges() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ev = loaded(tmp.path(), FIXTURE)
            .expect("load")
            .expect("index present");

        let one = ev.evidence_for("spec://p/D#one").expect("known address");
        assert_eq!(one.implements, 2, "two implementing edges");
        assert_eq!(
            one.verifies, 1,
            "one verifying edge — `deviates` is not one"
        );
        assert_eq!(
            one.refs,
            vec![
                "crates/a/src/lib.rs:10",
                "crates/a/src/other.rs:20",
                "crates/a/tests/t.rs:30"
            ],
            "the code-side locators travel as provenance"
        );

        // The same unit reached by the progress address form, via the
        // index's own file ↔ uri table.
        assert_eq!(
            ev.evidence_for("spec/d.md#one")
                .expect("joined address")
                .implements,
            2
        );

        // A unit the index knows but nothing cites: zero edges is an answer.
        let two = ev.evidence_for("spec://p/D#two").expect("known, uncited");
        assert_eq!((two.implements, two.verifies), (0, 0));

        // An address the index never heard of has no answer at all —
        // "no data" and "zero edges" are different claims.
        assert!(ev.evidence_for("spec://p/D#missing").is_none());
        assert!(ev.evidence_for("spec/nowhere.md#x").is_none());
    }

    /// A missing index is silence; a corrupt one is a loud failure naming
    /// the file (PROP-043 §6 — a corrupt index must not read as absent).
    #[test]
    fn absent_index_is_silence_but_malformed_is_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(
            SpecmapEvidence::load(tmp.path())
                .expect("no index")
                .is_none(),
            "no specmap.json ⇒ no provider, no error"
        );
        let err = loaded(tmp.path(), "{ not json").expect_err("malformed index");
        assert!(
            format!("{err:#}").contains(INDEX_REL_PATH),
            "the error names the file: {err:#}"
        );
    }
}
