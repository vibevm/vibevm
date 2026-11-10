//! One replayed node lane's system-prompt publication.
//!
//! A replay republishes a **changed** `STATIC.*`, so it is one of the paths
//! that must refresh the node's prefix and the managed block whose expected
//! lane digest names it. The composition happens here, at preparation time,
//! together with the node selector the publication will run — the publisher
//! is consume-only and derives nothing (PROP-076
//! `#SYS-TRANSACTION-ROLES`, `#SYS-PROSPECTIVE-ENTRYPOINT`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-TRANSACTION-ROLES");

use vibe_spec::SelfCoordinate;

use crate::WorkspaceError;
use crate::boot_artifacts;
use crate::boot_artifacts::declared_sources::DeclaredBootSources;

use super::{NodeReplayCandidate, ReplayCandidate, prepared};

/// The prompt publication of one replayed lane, or `None` for a unit slot.
///
/// A package unit slot publishes no prefix, and `None` here is the journal's
/// "no prompt member at all" rather than a removal — the two statements are
/// kept apart everywhere (`#SYS-OPT-IN`).
pub(super) fn for_lane(
    candidate: &ReplayCandidate,
    self_coord: &SelfCoordinate,
    sources: Option<&DeclaredBootSources>,
    lane: &[u8],
    index: &[u8],
) -> Result<Option<prepared::NodePromptPublication>, WorkspaceError> {
    match candidate {
        ReplayCandidate::Node(node) => {
            node_prompt_publication(node, self_coord, sources, lane, index).map(Some)
        }
        ReplayCandidate::Unit(_) => Ok(None),
    }
}

/// One node lane's prompt publication and the managed-block facts its
/// selector needs, composed from this replay's own compiled bytes.
pub(super) fn node_prompt_publication(
    candidate: &NodeReplayCandidate,
    self_coord: &SelfCoordinate,
    sources: Option<&DeclaredBootSources>,
    lane: &[u8],
    index: &[u8],
) -> Result<prepared::NodePromptPublication, WorkspaceError> {
    let router = boot_artifacts::RouterFacts {
        node_id: boot_artifacts::router_node_id(Some(&candidate.manifest), self_coord),
        static_hash12: Some(crate::system_prompt::segment_hash12(lane)),
        spec_format: candidate.spec_format,
    };
    let publication = boot_artifacts::prompt_publication(
        &candidate.node_dir,
        &candidate.effective,
        candidate.spec_format,
        sources,
        Some(boot_artifacts::NodePrompt {
            node_rel: &candidate.rel,
            manifest: &candidate.manifest,
        }),
        Some(lane),
        index,
        &router,
    )?;
    let node_dir = candidate.node_dir.clone();
    let limit = candidate
        .manifest
        .system_prompt
        .as_ref()
        .map_or(boot_artifacts::PROJECT_DOC_MAX_BYTES, |section| {
            section.codex_doc_max_bytes()
        });
    Ok(prepared::NodePromptPublication {
        publication,
        selectors: Box::new(move |transaction: &str| {
            boot_artifacts::write_redirect_blocks_in_transaction(
                &node_dir,
                &router,
                limit,
                transaction,
            )
            .map(|_| ())
        }),
    })
}
