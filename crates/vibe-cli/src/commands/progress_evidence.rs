//! `SpecmapEvidence` — the vibevm side of the PROP-043 §6 evidence seam.
//!
//! `progress-core` knows only the [`EvidenceProvider`] trait; the specmap
//! index is a vibevm fact, so the join lives here, on the adapter side. That
//! is the separability law: the core builds and runs with no provider at all,
//! and a consuming project with no `specmap.json` loses nothing but the
//! column.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#evidence");

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context as _, Result};
use progress_core::evidence::{Evidence, EvidenceProvider};
use progress_core::model::ArtifactKind;
use progress_core::terminal::ProviderArtifactEvidence;
use specmap_core::generated::specmap::{EdgeVerb, Specmap};
use specmap_core::index::INDEX_REL_PATH;
use specmark::spec;

static NO_EVIDENCE: progress_core::evidence::NoEvidence = progress_core::evidence::NoEvidence;

/// One immutable provider snapshot shared by every evidence-consuming write
/// in a command. Loading is a loud preflight; an absent map is the null
/// provider rather than an error.
pub(crate) struct EvidenceSnapshot {
    specmap: Option<SpecmapEvidence>,
}

impl EvidenceSnapshot {
    pub(crate) fn load(root: &Path) -> Result<Self> {
        Ok(Self {
            specmap: SpecmapEvidence::load(root)?,
        })
    }

    pub(crate) fn provider(&self) -> &dyn EvidenceProvider {
        match &self.specmap {
            Some(provider) => provider,
            None => &NO_EVIDENCE,
        }
    }
}

/// The host `specmap.json`, folded once into the two lookups the report
/// needs: what each spec address is backed by, and which address a progress
/// unit (`<file>#<anchor>`) speaks for.
#[derive(Debug)]
pub struct SpecmapEvidence {
    /// Canonical addresses explicitly owned by `spec_units`.
    known: HashSet<String>,
    /// A progress unit address (`<file>#<anchor>`) → that unit's canonical
    /// URI, as the index itself declares the correspondence.
    by_unit: HashMap<String, String>,
    implements: HashMap<String, Vec<String>>,
    verifies: HashMap<String, Vec<String>>,
    documents: HashMap<String, Vec<String>>,
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
        let mut known = HashSet::new();
        let mut by_unit: HashMap<String, String> = HashMap::new();
        let mut implements: HashMap<String, Vec<String>> = HashMap::new();
        let mut verifies: HashMap<String, Vec<String>> = HashMap::new();
        let mut documents: HashMap<String, Vec<String>> = HashMap::new();
        // Seed from the units: an address the index knows but nothing cites
        // answers `Some(zeros)` — a real "nothing implements this" claim,
        // distinct from the `None` an unknown address gets.
        for u in &index.specUnits {
            known.insert(u.uri.clone());
            by_unit.insert(format!("{}#{}", u.file, u.anchor), u.uri.clone());
        }
        for e in &index.edges {
            let relation = match e.verb {
                EdgeVerb::Implements => &mut implements,
                EdgeVerb::Verifies => &mut verifies,
                EdgeVerb::Documents => &mut documents,
                EdgeVerb::Deviates | EdgeVerb::Informs => continue,
            };
            relation
                .entry(e.uri.clone())
                .or_default()
                .push(format!("{}:{}", e.file, e.line));
        }
        SpecmapEvidence {
            known,
            by_unit,
            implements,
            verifies,
            documents,
        }
    }

    fn uri<'a>(&'a self, address: &'a str) -> Option<&'a str> {
        let uri = if address.starts_with("spec://") {
            address
        } else {
            self.by_unit.get(address)?.as_str()
        };
        self.known.contains(uri).then_some(uri)
    }

    fn relation(&self, address: &str, kind: ArtifactKind) -> ProviderArtifactEvidence {
        let Some(uri) = self.uri(address) else {
            return ProviderArtifactEvidence::Unavailable;
        };
        let index = match kind {
            ArtifactKind::Implementation => &self.implements,
            ArtifactKind::Verification => &self.verifies,
            ArtifactKind::Documentation => &self.documents,
            _ => return ProviderArtifactEvidence::Unavailable,
        };
        let locators = index.get(uri).cloned().unwrap_or_default();
        ProviderArtifactEvidence::Known {
            count: locators.len(),
            locators: Some(locators),
        }
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
        let uri = self.uri(unit_addr)?;
        let implements = self.implements.get(uri).cloned().unwrap_or_default();
        let verifies = self.verifies.get(uri).cloned().unwrap_or_default();
        Some(Evidence {
            implements: implements.len(),
            verifies: verifies.len(),
            refs: implements.into_iter().chain(verifies).collect(),
        })
    }

    fn artifact_evidence_for(
        &self,
        unit_addr: &str,
        artifact: ArtifactKind,
        _reference: Option<&str>,
    ) -> ProviderArtifactEvidence {
        self.relation(unit_addr, artifact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A spec file's project-relative path on the live layout — the
    /// fixture index's `file` values and the joined addresses below
    /// build on it, so the R4 flip carries the whole cell.
    fn spec_rel(name: &str) -> String {
        format!(
            "{}/{}",
            vibe_core::machine_json_path(&vibe_core::layout::current_specs_root()),
            name
        )
    }

    fn fixture() -> String {
        let d = spec_rel("d.md");
        format!(
            r#"{{
      "code_items": [],
      "edges": [
        {{"file": "crates/a/src/lib.rs", "from_symbol": "a", "line": 10,
         "provenance": "authored", "uri": "spec://p/D#one", "verb": "implements"}},
        {{"file": "crates/a/src/other.rs", "from_symbol": "a::other", "line": 20,
         "provenance": "authored", "uri": "spec://p/D#one", "verb": "implements"}},
        {{"file": "crates/a/tests/t.rs", "from_symbol": "t", "line": 30,
         "provenance": "authored", "uri": "spec://p/D#one", "verb": "verifies"}},
        {{"file": "docs/guide.md", "from_symbol": "guide", "line": 35,
         "provenance": "authored", "uri": "spec://p/D#one", "verb": "documents"}},
        {{"file": "crates/a/src/lib.rs", "from_symbol": "a", "line": 40,
         "provenance": "authored", "uri": "spec://p/D#one", "verb": "deviates",
         "reason": "not counted"}}
      ],
      "schema": 2,
      "spec_units": [
        {{"anchor": "one", "content_hash": "sha256:aa", "doc_path": "D",
         "file": "{d}", "heading": "One", "line": 5, "uri": "spec://p/D#one"}},
        {{"anchor": "two", "content_hash": "sha256:bb", "doc_path": "D",
         "file": "{d}", "heading": "Two", "line": 9, "uri": "spec://p/D#two"}}
      ],
      "suspects": [],
      "warnings": []
    }}"#
        )
    }

    fn loaded(dir: &Path, body: &str) -> Result<Option<SpecmapEvidence>> {
        std::fs::write(dir.join(INDEX_REL_PATH), body).expect("write index");
        SpecmapEvidence::load(dir)
    }

    #[test]
    fn specmap_evidence_counts_edges() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ev = loaded(tmp.path(), &fixture())
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
            ev.evidence_for(&format!("{}#one", spec_rel("d.md")))
                .expect("joined address")
                .implements,
            2
        );

        // A unit the index knows but nothing cites: zero edges is an answer.
        let two = ev.evidence_for("spec://p/D#two").expect("known, uncited");
        assert_eq!((two.implements, two.verifies), (0, 0));

        assert_eq!(
            ev.artifact_evidence_for("spec://p/D#one", ArtifactKind::Implementation, None),
            ProviderArtifactEvidence::Known {
                count: 2,
                locators: Some(vec![
                    "crates/a/src/lib.rs:10".into(),
                    "crates/a/src/other.rs:20".into(),
                ]),
            }
        );
        assert_eq!(
            ev.artifact_evidence_for(
                &format!("{}#one", spec_rel("d.md")),
                ArtifactKind::Documentation,
                None
            ),
            ProviderArtifactEvidence::Known {
                count: 1,
                locators: Some(vec!["docs/guide.md:35".into()]),
            },
            "the exact local form joins to the same relation-specific index"
        );
        assert_eq!(
            ev.artifact_evidence_for("spec://p/D#two", ArtifactKind::Verification, None),
            ProviderArtifactEvidence::Known {
                count: 0,
                locators: Some(Vec::new()),
            },
            "known zero stays distinct from provider absence"
        );

        // An address the index never heard of has no answer at all —
        // "no data" and "zero edges" are different claims.
        assert!(ev.evidence_for("spec://p/D#missing").is_none());
        assert_eq!(
            ev.artifact_evidence_for("spec://p/D#missing", ArtifactKind::Implementation, None),
            ProviderArtifactEvidence::Unavailable
        );
        assert!(
            ev.evidence_for(&format!("{}#x", spec_rel("nowhere.md")))
                .is_none()
        );
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
