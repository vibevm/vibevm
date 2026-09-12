//! How a package is delivered, projected onto the wire: the boot snippet
//! it contributes, the sources embedded in it, the workspace it
//! originated from, and the delivery mode a consumer resolves it under.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use vibe_core::manifest::{
    BootCategory, BootSnippet, DeliveryMode as CoreDeliveryMode, EmbeddedSourceDecl,
    EmbeddedSourceKind, OriginSection,
};

use crate::types::{BootSnippetEntry, DeliveryMode, EmbeddedSourceEntry, WorkspaceOriginEntry};

pub fn boot_snippet_from(b: &Option<BootSnippet>) -> Option<BootSnippetEntry> {
    b.as_ref().map(|bs| BootSnippetEntry {
        source: bs.source.to_string_lossy().replace('\\', "/"),
        category: bs.category.map(boot_category_str),
    })
}

/// Project immutable external-source declarations into the public catalog.
/// These are provenance rows, not dependency nodes: their independent hash
/// and upstream licence remain visibly separate from the bridge package's
/// own content hash and licence.
pub fn embedded_sources_from(sources: &[EmbeddedSourceDecl]) -> Vec<EmbeddedSourceEntry> {
    sources
        .iter()
        .map(|source| EmbeddedSourceEntry {
            name: source.name.clone(),
            kind: match source.kind {
                EmbeddedSourceKind::Git => "git".to_string(),
            },
            source_url: source.url.clone(),
            source_ref: source.ref_hint.clone(),
            resolved_commit: source.commit.clone(),
            content_hash: source.content_hash.to_string(),
            upstream_license: source.upstream_license.clone(),
            upstream_authors: source.upstream_authors.clone(),
            license_path: source.license_path.to_string_lossy().replace('\\', "/"),
            license_url: source.license_url.clone(),
        })
        .collect()
}

/// Project the `[origin]` provenance marker (PROP-007 §2.8) — present
/// only on a copy `vibe workspace publish` generated from a workspace
/// member — into the index entry's `workspace_origin` (PROP-008 §2.8).
pub fn workspace_origin_from(origin: &Option<OriginSection>) -> Option<WorkspaceOriginEntry> {
    origin.as_ref().map(|o| WorkspaceOriginEntry {
        upstream: o.upstream.clone(),
        path: o.path.clone(),
        commit: o.commit.clone(),
        generated_by: o.generated_by.clone(),
        generated_at: o.generated_at.clone(),
    })
}

/// The kebab-case wire string for a boot category — the form `vibe-core`
/// serialises and the [`BootSnippetEntry`] records.
fn boot_category_str(c: BootCategory) -> String {
    match c {
        BootCategory::Foundation => "foundation",
        BootCategory::Flow => "flow",
        BootCategory::Stack => "stack",
        BootCategory::Tool => "tool",
        BootCategory::App => "app",
        BootCategory::UserOverride => "user-override",
    }
    .to_string()
}

pub(super) fn delivery_from(d: CoreDeliveryMode) -> DeliveryMode {
    match d {
        CoreDeliveryMode::Eager => DeliveryMode::Eager,
        CoreDeliveryMode::LazyPush => DeliveryMode::LazyPush,
        CoreDeliveryMode::LazyPull => DeliveryMode::LazyPull,
    }
}
