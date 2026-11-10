//! Install-time resolution of boot-snippet contribution predicates.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-049#installed-predicate");

use std::collections::HashSet;
use std::path::Path;

use vibe_core::Group;
use vibe_core::manifest::{BootSnippet, Manifest, TargetSelection, WhenCondition};

use crate::boot::BootContribution;

use super::super::ResolvedDep;
use super::snippet_source::resolve_snippet_source;

pub(super) struct ActiveSnippet {
    pub(super) main: Option<BootContribution>,
    pub(super) fragments: Vec<BootContribution>,
}

/// Everything a contribution predicate is resolved against at generation
/// time: the exact resolution an `installed:` gate asks about, and the
/// project's declared product platforms a `target:` gate asks about.
///
/// The two are bundled because they answer at the same moment and must
/// answer once per regeneration, never twice with two verdicts: a package
/// unit's lane is published once for the whole resolution, so one `target:`
/// row cannot be included for one node and dropped for another
/// (PROP-076 `#SYS-HOST-AND-TARGET`).
pub(super) struct ConditionWorld {
    installed: HashSet<(Group, String)>,
    targets: TargetSelection,
}

impl ConditionWorld {
    pub(super) fn new(resolution: &[ResolvedDep], targets: TargetSelection) -> Self {
        Self {
            installed: installed_identities(resolution),
            targets,
        }
    }
}

/// The product platforms a `target:` gate is resolved against — the
/// project's own authored declaration, nothing observed. Absent selects
/// every operating system, so a project that declares nothing sees no
/// `target:` row dropped and its boot is unchanged.
pub(super) fn project_targets(manifest: &Manifest) -> TargetSelection {
    manifest
        .system_prompt
        .as_ref()
        .map(|section| section.target.clone())
        .unwrap_or_default()
}

/// The exact `(group, name)` set present in the current unified resolution.
fn installed_identities(resolution: &[ResolvedDep]) -> HashSet<(Group, String)> {
    resolution
        .iter()
        .map(|dep| (dep.group.clone(), dep.name.clone()))
        .collect()
}

pub(super) fn active_snippet(
    workspace_root: &Path,
    slot: &str,
    snippet: Option<&BootSnippet>,
    world: &ConditionWorld,
) -> ActiveSnippet {
    let result = select_snippet(snippet, world, |source| {
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
    world: &ConditionWorld,
    sources: &crate::boot_artifacts::declared_sources::DeclaredBootSources,
) -> Result<ActiveSnippet, crate::WorkspaceError> {
    select_snippet(dep.manifest.boot_snippet.as_ref(), world, |source| {
        sources.snippet(dep, source).map(|selected| {
            selected.unwrap_or_else(|| resolve_snippet_source(workspace_root, slot, source))
        })
    })
}

fn select_snippet<E>(
    snippet: Option<&BootSnippet>,
    world: &ConditionWorld,
    mut resolve: impl FnMut(&Path) -> Result<String, E>,
) -> Result<ActiveSnippet, E> {
    let main = match snippet
        .and_then(|boot| resolve_condition(boot.when.as_ref(), world).map(|when| (boot, when)))
    {
        Some((boot, when)) => Some(BootContribution {
            path: resolve(&boot.source)?,
            when,
        }),
        None => None,
    };
    let mut fragments = Vec::new();
    for fragment in snippet.into_iter().flat_map(|boot| &boot.fragments) {
        if let Some(when) = resolve_condition(fragment.when.as_ref(), world) {
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
///
/// Two of the three gates are decided here and are therefore never residual.
/// `installed:` has always been (PROP-049 §3). `target:` joins it, because
/// the platform the product addresses is a property of the project and never
/// of a session: a matching row becomes unconditional, a non-matching one is
/// physically absent, and no `target:` string can reach `INDEX.md` — so the
/// generated `kind` expression keeps exactly the meaning it has today
/// (PROP-076 `#SYS-HOST-AND-TARGET`). Only `os:` survives for the reading
/// session.
fn resolve_condition(
    when: Option<&WhenCondition>,
    world: &ConditionWorld,
) -> Option<Option<WhenCondition>> {
    match when {
        None => Some(None),
        Some(WhenCondition::Os(os)) => Some(Some(WhenCondition::Os(*os))),
        Some(WhenCondition::Installed { group, name }) => world
            .installed
            .contains(&(group.clone(), name.to_string()))
            .then_some(None),
        Some(WhenCondition::Target(os)) => world.targets.includes(*os).then_some(None),
    }
}

#[cfg(test)]
#[path = "conditions/tests.rs"]
mod tests;
