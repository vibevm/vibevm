//! `vibe prompt build` — explicit regeneration of one node's system-prompt
//! artifacts, through the one existing boot transaction.
//!
//! **There is no second writer and no second atomicity story.** This cell
//! composes exactly what the install seam composes, from the same
//! provider-free node generation, and commits it through
//! `transaction::write_production_with_selectors` with the same four prompt
//! roles and the same redirect selector. A failure leaves the previous
//! coherent set intact, and an idempotent build writes nothing at all —
//! `apply_forward` compares present-and-digest and returns early, so a
//! rebuild with nothing changed moves no mtime and produces no diff
//! (`#SYS-TRANSACTION-ROLES`, `#SYS-NO-EQUAL-REWRITE`).
//!
//! **Why the whole production set is republished and not only the prefix.**
//! The prefix's entrypoint segment is the *prospective* managed block of
//! this same generation, so the block and its witness must be written by one
//! commit; and the sidecar's plan identity is the digest of the `INDEX.md`
//! this generation publishes. Writing the prefix alone would pair new
//! witnesses with another generation's artifacts, which is precisely the
//! window the single transaction exists to close
//! (`#SYS-PROSPECTIVE-ENTRYPOINT`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-SURFACE");

use std::path::PathBuf;

use vibe_core::layout;
use vibe_spec::EmittedArtifact;

use crate::WorkspaceError;
use crate::install::bootgen::node_generation;

use super::{NodePrompt, RouterFacts, redirect, transaction};

/// What one build did, and where.
///
/// `OptedOut` and `Removed` are kept apart deliberately: the first is a node
/// that never had a template and owns nothing, so nothing was read, staged
/// or removed; the second is a node that *had* one and the four owned paths
/// now end absent. Collapsing them would make an opt-out look like a
/// no-operation (`#SYS-OPT-IN`, `#SYS-REMOVAL-AND-CONFLICT`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptBuild {
    /// No authored `SYSTEM-TEMPLATE.md` and nothing of ours on disk.
    OptedOut,
    /// The template is gone and the artifacts it owned have been removed.
    Removed,
    /// The artifacts were written, and the bytes differ from what was there.
    Rebuilt(PromptPaths),
    /// The artifacts are already exactly these bytes; nothing was written.
    Unchanged(PromptPaths),
}

impl PromptBuild {
    /// The artifacts, for the two verdicts that have any.
    pub fn paths(&self) -> Option<&PromptPaths> {
        match self {
            PromptBuild::OptedOut | PromptBuild::Removed => None,
            PromptBuild::Rebuilt(paths) | PromptBuild::Unchanged(paths) => Some(paths),
        }
    }

    /// The verdict's own word, as the summary line spells it.
    pub fn as_str(&self) -> &'static str {
        match self {
            PromptBuild::OptedOut => "opted out",
            PromptBuild::Removed => "removed",
            PromptBuild::Rebuilt(_) => "rebuilt",
            PromptBuild::Unchanged(_) => "unchanged",
        }
    }
}

/// The two artifacts a caller points at, with the sizes a summary line
/// reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptPaths {
    /// `SYSTEM.md` — the API prefix.
    pub system: PathBuf,
    /// Its byte length.
    pub system_len: usize,
    /// `AGENT-LANE.md` — what a launch delivery points at.
    pub lane: PathBuf,
    /// Its byte length.
    pub lane_len: usize,
}

/// Build one node's system-prompt artifacts.
pub fn build_node_prompt(
    workspace: &crate::Workspace,
    node_rel: &str,
) -> Result<PromptBuild, WorkspaceError> {
    let generation = node_generation(workspace, node_rel, None)?;
    let index_text =
        super::render_index_with_spec_format(&generation.effective, None, generation.spec_format)?;
    let compiled = super::compile_static_analyzed(
        &generation.effective,
        &generation.root,
        &generation.self_coord,
        generation.spec_format,
        generation.sources.as_ref(),
        generation.transforms,
        None,
    )?;
    let lane = compiled.map(|compiled| compiled.artifact);
    let prompt = NodePrompt {
        node_rel,
        manifest: &generation.manifest,
    };
    let codex_limit = generation
        .manifest
        .system_prompt
        .as_ref()
        .map_or(redirect::PROJECT_DOC_MAX_BYTES, |section| {
            section.codex_doc_max_bytes()
        });
    let router = RouterFacts {
        node_id: redirect::router_node_id(Some(&generation.manifest), &generation.self_coord),
        static_hash12: lane
            .as_ref()
            .map(|artifact| crate::system_prompt::segment_hash12(artifact.bytes())),
        spec_format: generation.spec_format,
    };
    let publication = super::prompt_publication(
        &generation.node_dir,
        &generation.effective,
        generation.spec_format,
        generation.sources.as_ref(),
        Some(prompt),
        lane.as_ref().map(EmittedArtifact::bytes),
        index_text.as_bytes(),
        &router,
    )?;
    let boot_dir = generation.node_dir.join(layout::current_boot_dir());
    let before = Witness::read(&boot_dir)?;
    let payload = super::prompt_payload(&publication);
    let answer = verdict(&boot_dir, &publication, &before);
    transaction::write_production_with_selectors(
        transaction::ArtifactWrite {
            index_path: &boot_dir.join(super::INDEX_FILE),
            index_bytes: index_text.as_bytes(),
            static_path: &boot_dir.join(super::static_file(generation.spec_format)),
            static_bytes: lane.as_ref().map(EmittedArtifact::bytes),
            stale_path: &boot_dir.join(super::publication::stale_static_file(
                generation.spec_format,
            )),
            prompt: payload,
        },
        |transaction| {
            redirect::write_redirect_blocks_with_transaction(
                &generation.node_dir,
                &router,
                codex_limit,
                transaction,
            )
        },
    )?;
    Ok(answer)
}

/// The three owned artifacts as they stood before this build, so "rebuilt"
/// and "unchanged" are decided by comparing bytes rather than by observing
/// whether the transaction happened to write.
struct Witness {
    system: Option<Vec<u8>>,
    meta: Option<Vec<u8>>,
    lane: Option<Vec<u8>>,
}

impl Witness {
    fn read(boot_dir: &std::path::Path) -> Result<Self, WorkspaceError> {
        let read = |name: &str| -> Result<Option<Vec<u8>>, WorkspaceError> {
            let path = boot_dir.join(name);
            crate::safe_file::preflight_absent_or_regular(&path)
                .and_then(|()| crate::safe_file::read_optional(&path))
                .map_err(|error| WorkspaceError::SystemPromptCapture {
                    path,
                    reason: error.to_string(),
                })
        };
        Ok(Self {
            system: read(layout::SYSTEM_MD)?,
            meta: read(layout::SYSTEM_META_JSON)?,
            lane: read(layout::AGENT_LANE_MD)?,
        })
    }
}

/// Classify the build before the write, from what was composed and what was
/// already there.
fn verdict(
    boot_dir: &std::path::Path,
    publication: &crate::system_prompt::PromptPublication,
    before: &Witness,
) -> PromptBuild {
    use crate::system_prompt::PromptPublication;

    match publication {
        PromptPublication::Untouched => PromptBuild::OptedOut,
        PromptPublication::Removed => PromptBuild::Removed,
        PromptPublication::Published(published) => {
            let paths = PromptPaths {
                system: boot_dir.join(layout::SYSTEM_MD),
                system_len: published.system.len(),
                lane: boot_dir.join(layout::AGENT_LANE_MD),
                lane_len: published.lane.len(),
            };
            let same = before.system.as_deref() == Some(published.system.as_slice())
                && before.meta.as_deref() == Some(published.meta.as_slice())
                && before.lane.as_deref() == Some(published.lane.as_slice());
            if same {
                PromptBuild::Unchanged(paths)
            } else {
                PromptBuild::Rebuilt(paths)
            }
        }
    }
}
