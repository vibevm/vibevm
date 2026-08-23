//! Install-time resolution of boot-snippet contribution predicates.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-049#installed-predicate");

use std::collections::HashSet;
use std::path::Path;

use vibe_core::Group;
use vibe_core::manifest::{BootSnippet, WhenCondition};

use crate::boot::BootContribution;

use super::super::ResolvedDep;
use super::snippet_source::resolve_snippet_source;

pub(super) struct ActiveSnippet {
    pub(super) main: Option<BootContribution>,
    pub(super) fragments: Vec<BootContribution>,
}

/// The exact `(group, name)` set present in the current unified resolution.
pub(super) fn installed_identities(resolution: &[ResolvedDep]) -> HashSet<(Group, String)> {
    resolution
        .iter()
        .map(|dep| (dep.group.clone(), dep.name.clone()))
        .collect()
}

pub(super) fn active_snippet(
    workspace_root: &Path,
    slot: &str,
    snippet: Option<&BootSnippet>,
    installed: &HashSet<(Group, String)>,
) -> ActiveSnippet {
    let main = snippet.and_then(|boot| {
        resolve_condition(boot.when.as_ref(), installed).map(|when| BootContribution {
            path: resolve_snippet_source(workspace_root, slot, &boot.source),
            when,
        })
    });
    let fragments = snippet
        .into_iter()
        .flat_map(|boot| &boot.fragments)
        .filter_map(|fragment| {
            resolve_condition(fragment.when.as_ref(), installed).map(|when| BootContribution {
                path: resolve_snippet_source(workspace_root, slot, &fragment.source),
                when,
            })
        })
        .collect();
    ActiveSnippet { main, fragments }
}

/// `None` omits the contribution; `Some(None)` includes it unconditionally;
/// `Some(Some(os))` preserves the session-time predicate for INDEX emission.
fn resolve_condition(
    when: Option<&WhenCondition>,
    installed: &HashSet<(Group, String)>,
) -> Option<Option<WhenCondition>> {
    match when {
        None => Some(None),
        Some(WhenCondition::Os(os)) => Some(Some(WhenCondition::Os(*os))),
        Some(WhenCondition::Installed { group, name }) => installed
            .contains(&(group.clone(), name.to_string()))
            .then_some(None),
    }
}
