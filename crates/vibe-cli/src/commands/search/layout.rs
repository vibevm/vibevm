//! `vibe search` over a directory-layout registry (`monorepo`, `localdir`):
//! the registry's own index in its tree, or — absent or stale — the same
//! projection computed in memory (PROP-002 `#registry-index`).
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-index");

use anyhow::Result;
use vibe_core::PackageKind;
use vibe_core::layout_registry::IndexState;
use vibe_core::manifest::{MirrorSection, RegistrySection};
use vibe_registry::SearchHit;

use crate::commands::registry::layout_root;

pub(super) use layout_root::is_layout;

pub(super) struct LayoutSearch {
    pub hits: Vec<SearchHit>,
    /// Why the persisted index was not used, when it was not.
    pub stale: Option<String>,
    /// The registry's issued short names, `(spelling, target)` — read
    /// off the index this search already projected, so marking them
    /// costs a directory-layout registry nothing (PROP-008 §2.6).
    pub issued: Vec<(String, vibe_registry::short_name::AliasTarget)>,
}

pub(super) fn search(
    reg: &RegistrySection,
    mirrors: &[MirrorSection],
    operation: &vibe_package_source::SourceOperation,
    query: &str,
    kind_filter: Option<PackageKind>,
    limit: usize,
    progress: &vibe_core::progress::Progress,
) -> Result<LayoutSearch> {
    let opened = layout_root::open(reg, mirrors, operation, false, progress)?;
    let (index, state) = vibe_index::layout::load_or_project(opened.root(), opened.identity())?;
    let kind = kind_filter.and_then(|k| k.as_str().parse::<vibe_index::types::PackageKind>().ok());
    let hits = vibe_index::index::search(&index, query, kind)
        .into_iter()
        .take(limit)
        .filter_map(|hit| {
            Some(SearchHit {
                kind: hit.kind.to_string().parse().ok()?,
                name: hit.name,
                latest_stable: hit.latest_stable,
                score: u32::try_from(hit.score).unwrap_or(u32::MAX),
                matched_tokens: hit.matched_tokens,
                description: hit.description,
            })
        })
        .collect();
    let stale = match state {
        IndexState::Current => None,
        IndexState::Absent => Some("no index in the registry; computed in memory".to_string()),
        IndexState::Stale { reason } => {
            Some(format!("index is stale ({reason}); computed in memory"))
        }
    };
    let issued = index
        .aliases
        .iter()
        .map(|(alias, target)| {
            (
                alias.clone(),
                vibe_registry::short_name::AliasTarget {
                    group: target.group.clone(),
                    name: target.name.clone(),
                },
            )
        })
        .collect();
    Ok(LayoutSearch {
        hits,
        stale,
        issued,
    })
}
