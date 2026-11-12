//! What the registries answer about one bare spelling (PROP-008 §2.6).
//!
//! **A bare spelling exists only because a registry issued it.** The
//! owner's ruling of 2026-11-12: «если мы разрешим назначать короткие
//! имена держателям пакетов, то всё сломается, потому что все будут
//! бороться за эти короткие имена. Это то же самое что с доменами в
//! интернете - мы не можем дать людям самим выбирать любое имя.» So a
//! short name is an ISSUED name — a row in the alias table of a
//! registry that holds the package ([`vibe_index::aliases`]) — and
//! being *called* `qwen` grants a package nothing. The one GET per
//! registry still answers both halves, because both ride the same
//! `by-name/<name>.json` file; what changed is which half resolves.
//!
//! So this module carries two lists with two different jobs:
//!
//! - [`ShortNameCandidates::aliases`] — the issued spellings. These and
//!   only these are resolution candidates.
//! - [`ShortNameCandidates::named`] — the groups publishing a package
//!   that actually bears the spelling. Not a candidate at any rank: it
//!   is the HINT a refusal prints, so an operator who typed `pxnode`
//!   can copy the qualified name out of the message.
//!
//! The precedence rule that used to live here — a published name
//! outranking every offered spelling — is gone with the candidacy that
//! made it necessary. There is nothing left for an alias to outrank.

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
/// the groups publishing a package of that name, and the spelling — if
/// any — that registry has issued for it.
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

/// One issued-name candidate, with every registry that issues it.
///
/// **Two registries issuing the same target are one candidate, not a
/// collision.** A mirror and its source agree about what `qwen` means
/// far more often than they disagree, and refusing that agreement would
/// make the feature unusable behind any mirror — which is exactly where
/// it is most wanted. Disagreement is still a collision: different
/// targets are different packages, and PROP-008 §2.7 does not let a
/// resolver guess between those.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasCandidate {
    pub target: AliasTarget,
    /// The `[[registry]].name`s that issue this target, in walk order.
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
    /// The distinct issued targets, each naming the registries that
    /// issue it. **This is the resolution candidate set** — one entry
    /// resolves, several are the §2.7 collision, none is the refusal.
    pub aliases: Vec<AliasCandidate>,
    /// Groups publishing a package actually CALLED this name, across
    /// every registry walked — de-duplicated and sorted. **Never a
    /// resolution candidate** (PROP-008 §2.6 `##ISSUED-NAMES-ONLY`):
    /// the refusal prints these as the qualified names the operator can
    /// copy, and nothing picks one.
    pub named: Vec<Group>,
}

impl ShortNameCandidates {
    /// Fold the per-registry answers into one verdict.
    ///
    /// `answers` is `(registry name, lookup)` in walk order. Both lists
    /// are built independently, because they no longer compete: a
    /// package bearing the spelling neither displaces an issued one nor
    /// is displaced by it.
    pub fn gather(answers: impl IntoIterator<Item = (String, NameLookup)>) -> Self {
        let mut named: Vec<Group> = Vec::new();
        let mut aliases: Vec<AliasCandidate> = Vec::new();
        for (registry, lookup) in answers {
            for group in lookup.groups {
                if !named.contains(&group) {
                    named.push(group);
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
        named.sort();
        aliases.sort_by(|a, b| a.target.cmp(&b.target));
        ShortNameCandidates { aliases, named }
    }

    /// Whether anything at all answered — an issued spelling or a
    /// package bearing the name. A chain link short-circuits on
    /// [`Self::has_candidates`], not on this: a registry that merely
    /// *publishes* the name resolves nothing and must not stop the walk.
    pub fn is_empty(&self) -> bool {
        self.aliases.is_empty() && self.named.is_empty()
    }

    /// Whether any consulted registry issued this spelling — the only
    /// question resolution asks.
    pub fn has_candidates(&self) -> bool {
        !self.aliases.is_empty()
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
        assert!(found.named.is_empty());
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

    /// The inverted rule. A package bearing the spelling used to clear
    /// the alias side wholesale; since the owner's 2026-11-12 ruling it
    /// is not a candidate at all, so it clears nothing — the issued
    /// spelling stands and the name goes to the hint list.
    #[test]
    fn a_published_name_no_longer_clears_an_issued_one() {
        let found = ShortNameCandidates::gather([
            ("corp".to_string(), alias("com.acme", "qwen-fork")),
            (
                "vibespecs".to_string(),
                NameLookup::of_groups(vec![g("org.vibevm.qwen")]),
            ),
        ]);
        assert_eq!(found.named, vec![g("org.vibevm.qwen")]);
        assert_eq!(
            found.aliases.len(),
            1,
            "a name somebody publishes is no longer a candidate, so it displaces nothing"
        );
        assert!(found.has_candidates());
    }

    /// A registry that only publishes the name issues nothing: the
    /// lookup is non-empty — there is a hint to print — but it carries
    /// no candidate, so resolution refuses.
    #[test]
    fn a_published_name_alone_is_a_hint_and_not_a_candidate() {
        let found = ShortNameCandidates::gather([(
            "vibespecs".to_string(),
            NameLookup::of_groups(vec![g("org.vibevm.opencode")]),
        )]);
        assert!(found.aliases.is_empty());
        assert_eq!(found.named, vec![g("org.vibevm.opencode")]);
        assert!(!found.has_candidates());
        assert!(!found.is_empty(), "there is still a hint to print");
    }

    /// An issued name MAY equal the package's own name, and that is now
    /// the primary case: `node → org.vibevm.node/node`.
    #[test]
    fn an_issued_name_equal_to_the_package_name_is_one_candidate() {
        let found = ShortNameCandidates::gather([(
            "vibespecs".to_string(),
            NameLookup {
                groups: vec![g("org.vibevm.node")],
                alias: Some(AliasTarget {
                    group: g("org.vibevm.node"),
                    name: "node".into(),
                }),
            },
        )]);
        assert_eq!(found.aliases.len(), 1);
        assert_eq!(found.aliases[0].target.to_string(), "org.vibevm.node/node");
        assert_eq!(found.named, vec![g("org.vibevm.node")]);
    }

    #[test]
    fn nothing_found_is_empty() {
        assert!(ShortNameCandidates::gather([]).is_empty());
    }
}
