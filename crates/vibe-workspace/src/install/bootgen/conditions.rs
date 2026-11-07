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
    let result = select_snippet(snippet, installed, |source| {
        Ok::<_, std::convert::Infallible>(resolve_snippet_source(workspace_root, slot, source))
    });
    match result {
        Ok(active) => active,
        Err(never) => match never {},
    }
}

pub(super) fn active_snippet_bound(
    workspace_root: &Path,
    slot: &str,
    dep: &ResolvedDep,
    installed: &HashSet<(Group, String)>,
    sources: &crate::boot_artifacts::declared_sources::DeclaredBootSources,
) -> Result<ActiveSnippet, crate::WorkspaceError> {
    select_snippet(dep.manifest.boot_snippet.as_ref(), installed, |source| {
        sources.snippet(dep, source).map(|selected| {
            selected.unwrap_or_else(|| resolve_snippet_source(workspace_root, slot, source))
        })
    })
}

fn select_snippet<E>(
    snippet: Option<&BootSnippet>,
    installed: &HashSet<(Group, String)>,
    mut resolve: impl FnMut(&Path) -> Result<String, E>,
) -> Result<ActiveSnippet, E> {
    let main = match snippet
        .and_then(|boot| resolve_condition(boot.when.as_ref(), installed).map(|when| (boot, when)))
    {
        Some((boot, when)) => Some(BootContribution {
            path: resolve(&boot.source)?,
            when,
        }),
        None => None,
    };
    let mut fragments = Vec::new();
    for fragment in snippet.into_iter().flat_map(|boot| &boot.fragments) {
        if let Some(when) = resolve_condition(fragment.when.as_ref(), installed) {
            fragments.push(BootContribution {
                path: resolve(&fragment.source)?,
                when,
            });
        }
    }
    Ok(ActiveSnippet { main, fragments })
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
