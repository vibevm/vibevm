//! Ephemeral terminal-artifact state projection.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#TERMINAL-OBSERVATION-SURFACE"
);

use crate::cache::{now_utc, write_if_changed};
use crate::doc::ParsedDoc;
use crate::evidence::EvidenceProvider;
use crate::terminal::{TerminalCounts, counts_for_doc};
use anyhow::Result;
use std::path::Path;

use super::{TERMINAL_STATE_SCHEMA, TerminalFileState, TerminalState};

/// Write `terminal.json` from parsed documents and one provider snapshot.
/// Invalid documents are omitted rather than assigned an invented outcome.
pub fn write_terminal_state<'a>(
    state_dir: &Path,
    docs: impl IntoIterator<Item = &'a ParsedDoc>,
    provider: &dyn EvidenceProvider,
) -> Result<bool> {
    let mut aggregate = TerminalCounts::default();
    let files = docs
        .into_iter()
        .filter(|doc| doc.error_count() == 0)
        .map(|doc| {
            let counts = counts_for_doc(doc, provider);
            aggregate.classified += counts.classified;
            aggregate.terminal += counts.terminal;
            aggregate.pending += counts.pending;
            aggregate.unclassified += counts.unclassified;
            TerminalFileState {
                path: doc.path.clone(),
                counts,
            }
        })
        .collect();
    let state = TerminalState {
        schema: TERMINAL_STATE_SCHEMA,
        updated_at: now_utc(),
        files,
        aggregate,
    };
    write_if_changed(
        &state_dir.join("terminal.json"),
        &serde_json::to_string_pretty(&state)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{Evidence, EvidenceProvider};
    use crate::model::ArtifactKind;
    use crate::parse::parse_document;
    use crate::terminal::ProviderArtifactEvidence;

    struct Count(usize);

    impl EvidenceProvider for Count {
        fn evidence_for(&self, _unit_addr: &str) -> Option<Evidence> {
            None
        }

        fn artifact_evidence_for(
            &self,
            _unit_addr: &str,
            artifact: ArtifactKind,
            _reference: Option<&str>,
        ) -> ProviderArtifactEvidence {
            assert_eq!(artifact, ArtifactKind::Implementation);
            ProviderArtifactEvidence::Known {
                count: self.0,
                locators: None,
            }
        }
    }

    #[test]
    fn provider_snapshot_moves_only_the_derived_projection() {
        let dir = tempfile::tempdir().unwrap();
        let doc = parse_document("a.md", "@fact:A body @requires:implementation @impl/done\n");
        assert!(write_terminal_state(dir.path(), [&doc], &Count(0)).unwrap());
        let pending = std::fs::read_to_string(dir.path().join("terminal.json")).unwrap();
        assert!(pending.contains("\"pending\": 1"), "{pending}");

        assert!(write_terminal_state(dir.path(), [&doc], &Count(1)).unwrap());
        let terminal = std::fs::read_to_string(dir.path().join("terminal.json")).unwrap();
        assert!(terminal.contains("\"terminal\": 1"), "{terminal}");
        assert_ne!(pending, terminal);
        assert!(!write_terminal_state(dir.path(), [&doc], &Count(1)).unwrap());
    }

    #[test]
    fn invalid_documents_are_not_given_terminal_counts() {
        let dir = tempfile::tempdir().unwrap();
        let invalid = parse_document("bad.md", "@fact:A body @requires:external @impl/done\n");
        assert!(invalid.error_count() > 0);
        write_terminal_state(dir.path(), [&invalid], &Count(1)).unwrap();
        let text = std::fs::read_to_string(dir.path().join("terminal.json")).unwrap();
        assert!(!text.contains("bad.md"), "{text}");
    }
}
