//! What the registries answer about one bare spelling (PROP-008 §2.6).
//!
//! A bare `qwen` can mean two different things, and the difference is
//! the whole of this module. It can be a package **named** `qwen` — the
//! candidate set `by-name/qwen.json` has always carried — or it can be
//! a short spelling a registry **offers** for a package named something
//! else, the registry-owned alias of [`vibe_index::aliases`]. One GET
//! per registry still answers both, because both ride the same file.
//!
//! **A real name outranks every alias, in every registry.** The rule is
//! structural here rather than a comparison made later: [`exact`] is
//! consulted first and an alias is reached only when it is empty. So no
//! alias anywhere can take a spelling that somebody actually publishes
//! — not in the registry that offers the alias, and not in a registry
//! further down the list. That is the property that makes the feature
//! safe to give to a registry owner: the worst a careless alias can do
//! is fail to be chosen.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name");

use vibe_core::Group;

/// The package a registry's alias points at — qualified, never versioned.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AliasTarget {
    pub group: Group,
    pub name: String,
}

impl std::fmt::Display for AliasTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.group, self.name)
    }
}

/// What ONE registry's `by-name/<name>.json` says about a bare spelling:
/// the groups publishing a package of that name, and the alias — if any
/// — that registry offers for it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameLookup {
    pub groups: Vec<Group>,
    pub alias: Option<AliasTarget>,
}

impl NameLookup {
    pub fn of_groups(groups: Vec<Group>) -> Self {
        NameLookup {
            groups,
            alias: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty() && self.alias.is_none()
    }
}

/// One alias candidate, with every registry that offers it.
///
/// **Two registries offering the same target are one candidate, not a
/// collision.** A mirror and its source agree about what `qwen` means
/// far more often than they disagree, and refusing that agreement would
/// make the feature unusable behind any mirror — which is exactly where
/// it is most wanted. Disagreement is still a collision: different
/// targets are different packages, and PROP-008 §2.7 does not let a
/// resolver guess between those.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasCandidate {
    pub target: AliasTarget,
    /// The `[[registry]].name`s that offer this target, in walk order.
    pub registries: Vec<String>,
}

impl AliasCandidate {
    /// `org.vibevm.qwen/qwen-code (alias in registry vibespecs)` — the
    /// one spelling used in the provenance line, the collision list and
    /// the refusal alike.
    pub fn provenance(&self) -> String {
        match self.registries.as_slice() {
            [] => self.target.to_string(),
            [one] => format!("{} (alias in registry {one})", self.target),
            many => format!("{} (alias in registries {})", self.target, many.join(", ")),
        }
    }
}

/// Every answer the allowed registries gave for one bare spelling.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShortNameCandidates {
    /// Groups publishing a package actually CALLED this name, across
    /// every registry walked — de-duplicated and sorted, exactly the
    /// list short-name resolution has always worked from.
    pub exact: Vec<Group>,
    /// The distinct alias targets offered for it, each naming the
    /// registries that offer it. Empty whenever `exact` is not, because
    /// nothing downstream may ever prefer an alias to a real name.
    pub aliases: Vec<AliasCandidate>,
}

impl ShortNameCandidates {
    /// Fold the per-registry answers into one verdict.
    ///
    /// `answers` is `(registry name, lookup)` in walk order. The alias
    /// side is dropped wholesale when any registry carries the real
    /// name, so a caller cannot accidentally rank them the other way.
    pub fn gather(answers: impl IntoIterator<Item = (String, NameLookup)>) -> Self {
        let mut exact: Vec<Group> = Vec::new();
        let mut aliases: Vec<AliasCandidate> = Vec::new();
        for (registry, lookup) in answers {
            for group in lookup.groups {
                if !exact.contains(&group) {
                    exact.push(group);
                }
            }
            let Some(target) = lookup.alias else {
                continue;
            };
            match aliases.iter_mut().find(|c| c.target == target) {
                Some(candidate) => {
                    if !candidate.registries.contains(&registry) {
                        candidate.registries.push(registry);
                    }
                }
                None => aliases.push(AliasCandidate {
                    target,
                    registries: vec![registry],
                }),
            }
        }
        exact.sort();
        if !exact.is_empty() {
            // A name somebody publishes is never displaced by a spelling
            // somebody offers — so the alias side does not even reach
            // the caller to be weighed.
            aliases.clear();
        }
        aliases.sort_by(|a, b| a.target.cmp(&b.target));
        ShortNameCandidates { exact, aliases }
    }

    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.aliases.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(s: &str) -> Group {
        Group::parse(s).expect("fixture group parses")
    }

    fn alias(group: &str, name: &str) -> NameLookup {
        NameLookup {
            groups: Vec::new(),
            alias: Some(AliasTarget {
                group: g(group),
                name: name.into(),
            }),
        }
    }

    #[test]
    fn one_alias_from_one_registry_is_one_candidate() {
        let found = ShortNameCandidates::gather([(
            "vibespecs".to_string(),
            alias("org.vibevm.qwen", "qwen-code"),
        )]);
        assert!(found.exact.is_empty());
        assert_eq!(found.aliases.len(), 1);
        assert_eq!(
            found.aliases[0].provenance(),
            "org.vibevm.qwen/qwen-code (alias in registry vibespecs)"
        );
    }

    #[test]
    fn the_same_target_from_two_registries_is_still_one_candidate() {
        let found = ShortNameCandidates::gather([
            (
                "vibespecs".to_string(),
                alias("org.vibevm.qwen", "qwen-code"),
            ),
            ("corp".to_string(), alias("org.vibevm.qwen", "qwen-code")),
        ]);
        assert_eq!(
            found.aliases.len(),
            1,
            "a mirror agreeing is not a collision"
        );
        assert_eq!(
            found.aliases[0].provenance(),
            "org.vibevm.qwen/qwen-code (alias in registries vibespecs, corp)"
        );
    }

    #[test]
    fn different_targets_are_two_candidates() {
        let found = ShortNameCandidates::gather([
            (
                "vibespecs".to_string(),
                alias("org.vibevm.qwen", "qwen-code"),
            ),
            ("corp".to_string(), alias("com.acme", "qwen-fork")),
        ]);
        assert_eq!(found.aliases.len(), 2);
    }

    #[test]
    fn a_real_name_anywhere_clears_every_alias() {
        // The precedence rule, as a structural fact rather than a
        // comparison somebody downstream has to remember to make.
        let found = ShortNameCandidates::gather([
            ("corp".to_string(), alias("com.acme", "qwen-fork")),
            (
                "vibespecs".to_string(),
                NameLookup::of_groups(vec![g("org.vibevm.qwen")]),
            ),
        ]);
        assert_eq!(found.exact, vec![g("org.vibevm.qwen")]);
        assert!(
            found.aliases.is_empty(),
            "an alias may never take a name somebody publishes"
        );
    }

    #[test]
    fn nothing_found_is_empty() {
        assert!(ShortNameCandidates::gather([]).is_empty());
    }
}
