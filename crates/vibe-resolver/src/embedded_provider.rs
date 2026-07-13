//! `DepProvider` composing an embedded local-directory registry with the
//! declared multi-registry walk, at the origin-selected precedence
//! (PROP-030 §3). A source-installed developer resolves **embedded-first**
//! (their in-tree edits win a coordinate clash); a distribution's end user
//! resolves **embedded-last** (the bundle only fills gaps). The discovery
//! that produces the embedded registry, and the choice of precedence from
//! the install `origin`, live in the CLI (PROP-030 §7); this cell is only
//! the composition.

use specmark::{cell, spec};
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef};

use crate::{
    DepProvider, DepProviderError, LocalRegistryProvider, MultiRegistryProvider, VersionEnumerator,
};

/// Which side wins a coordinate both the embedded registry and the declared
/// registries can serve (PROP-030 §1.1, §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddedPrecedence {
    /// Developer (`origin = external`): the embedded registry is consulted
    /// before the declared walk, so a local edit shadows a published copy
    /// of the same `(group, name, version)`.
    EmbeddedFirst,
    /// Distribution / end user: the declared walk is consulted first, so a
    /// bundled package only fills a gap the declared registries cannot.
    EmbeddedLast,
}

/// A `DepProvider` over the embedded local-directory registry plus an
/// optional declared multi-registry walk, delegating **per coordinate** at
/// the [`EmbeddedPrecedence`] the caller selected from the install origin.
///
/// Both sub-providers are owned (each holds a `&'a` borrow of its registry),
/// so the composed provider moves into a solver cell exactly like the plain
/// providers do — the only borrow that outlives it is the registries'.
#[cell(seam = "DepProvider", variant = "embedded", flag = "provider")]
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-030#precedence")]
pub struct EmbeddedProvider<'a> {
    embedded: LocalRegistryProvider<'a>,
    declared: Option<MultiRegistryProvider<'a>>,
    precedence: EmbeddedPrecedence,
}

impl<'a> EmbeddedProvider<'a> {
    /// Compose the embedded registry with an optional declared walk. With
    /// `declared = None` (a project that declares no `[[registry]]`), the
    /// embedded registry answers alone — the case that lifts PROP-002's
    /// "no registry configured" bail (PROP-030 §3).
    pub fn new(
        embedded: LocalRegistryProvider<'a>,
        declared: Option<MultiRegistryProvider<'a>>,
        precedence: EmbeddedPrecedence,
    ) -> Self {
        EmbeddedProvider {
            embedded,
            declared,
            precedence,
        }
    }

    /// The sub-providers in resolution order for this precedence — always
    /// non-empty (the embedded provider is always present).
    fn ordered(&self) -> Vec<&dyn VersionEnumerator> {
        order_providers(
            &self.embedded,
            self.declared.as_ref().map(|d| d as &dyn VersionEnumerator),
            self.precedence,
        )
    }
}

impl<'a> DepProvider for EmbeddedProvider<'a> {
    #[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-030#precedence")]
    fn resolve_version(&self, pkgref: &PackageRef) -> Result<semver::Version, DepProviderError> {
        resolve_first(&self.ordered(), |p| p.resolve_version(pkgref))
    }

    #[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-030#precedence")]
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        resolve_first(&self.ordered(), |p| p.fetch_manifest(group, name, version))
    }
}

impl<'a> VersionEnumerator for EmbeddedProvider<'a> {
    #[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-017#provider-enrichment")]
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        union_versions(&self.ordered(), group, name)
    }
}

// ---------------------------------------------------------------------------
// The composition, factored out of the trait impls so the precedence,
// fall-through, and union logic is unit-testable without a live registry.
// ---------------------------------------------------------------------------

/// Order the sub-providers for a precedence. The embedded provider is
/// always present; the declared walk is threaded in front of or behind it.
fn order_providers<'p>(
    embedded: &'p dyn VersionEnumerator,
    declared: Option<&'p dyn VersionEnumerator>,
    precedence: EmbeddedPrecedence,
) -> Vec<&'p dyn VersionEnumerator> {
    match (declared, precedence) {
        (Some(d), EmbeddedPrecedence::EmbeddedFirst) => vec![embedded, d],
        (Some(d), EmbeddedPrecedence::EmbeddedLast) => vec![d, embedded],
        (None, _) => vec![embedded],
    }
}

/// Does this error mean "this provider does not serve the coordinate"? On
/// these the combiner falls through to the next provider; any other error
/// (a real provider failure) propagates immediately rather than being
/// masked by a fall-through.
fn is_absent(err: &DepProviderError) -> bool {
    matches!(
        err,
        DepProviderError::UnknownPackage { .. }
            | DepProviderError::NoMatchingVersion { .. }
            | DepProviderError::AggregateNotFound { .. }
    )
}

/// Run `op` against each provider in order, returning the first success.
/// A provider that is merely absent for the coordinate is skipped; a real
/// failure short-circuits. If every provider is absent, the last absence is
/// returned so the caller sees a genuine "not found" rather than a fabricated
/// one. `providers` is never empty (see [`order_providers`]).
fn resolve_first<T>(
    providers: &[&dyn VersionEnumerator],
    op: impl Fn(&dyn VersionEnumerator) -> Result<T, DepProviderError>,
) -> Result<T, DepProviderError> {
    let mut last_absent = None;
    for &p in providers {
        match op(p) {
            Ok(value) => return Ok(value),
            Err(e) if is_absent(&e) => last_absent = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last_absent.unwrap_or_else(|| {
        DepProviderError::Other("embedded composition consulted no providers".into())
    }))
}

/// The union of every provider's versions for `(group, name)`, sorted and
/// de-duplicated — so a candidate-enumerating solver (resolvo) sees the
/// embedded and declared versions together and picks among them, then
/// fetches the chosen version from the precedence-first provider that has it
/// via [`resolve_first`]. Absent providers contribute nothing; only when
/// *every* provider is absent does the union report "not found".
fn union_versions(
    providers: &[&dyn VersionEnumerator],
    group: &Group,
    name: &str,
) -> Result<Vec<semver::Version>, DepProviderError> {
    let mut all = Vec::new();
    let mut last_absent = None;
    let mut any_served = false;
    for &p in providers {
        match p.list_versions(group, name) {
            Ok(mut versions) => {
                any_served = true;
                all.append(&mut versions);
            }
            Err(e) if is_absent(&e) => last_absent = Some(e),
            Err(e) => return Err(e),
        }
    }
    if !any_served {
        return Err(last_absent.unwrap_or_else(|| {
            DepProviderError::Other("embedded composition consulted no providers".into())
        }));
    }
    all.sort();
    all.dedup();
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A canned provider: it either serves a fixed version set, is absent
    /// for every coordinate, or fails hard — enough to exercise precedence,
    /// fall-through, propagation, and the union, without a live registry.
    /// `label` is the package name it stamps into a served manifest, so a
    /// test can tell *which* provider answered a `fetch_manifest`.
    struct Canned {
        answer: Answer,
        label: &'static str,
    }

    enum Answer {
        Serves(Vec<semver::Version>),
        Absent,
        Fails,
    }

    fn v(s: &str) -> semver::Version {
        semver::Version::parse(s).unwrap()
    }

    fn absent_err() -> DepProviderError {
        DepProviderError::UnknownPackage {
            group: Group::parse("org.vibevm").unwrap(),
            name: "x".into(),
        }
    }

    impl DepProvider for Canned {
        fn resolve_version(&self, _: &PackageRef) -> Result<semver::Version, DepProviderError> {
            match &self.answer {
                Answer::Serves(vs) => Ok(vs.iter().max().cloned().unwrap()),
                Answer::Absent => Err(absent_err()),
                Answer::Fails => Err(DepProviderError::Other("boom".into())),
            }
        }

        fn fetch_manifest(
            &self,
            _: &Group,
            _: &str,
            _: &semver::Version,
        ) -> Result<Manifest, DepProviderError> {
            match &self.answer {
                Answer::Serves(_) => Ok(Manifest::parse_str(&format!(
                    "[package]\ngroup = \"org.vibevm\"\nname = \"{}\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
                    self.label
                ))
                .unwrap()),
                Answer::Absent => Err(absent_err()),
                Answer::Fails => Err(DepProviderError::Other("boom".into())),
            }
        }
    }

    impl VersionEnumerator for Canned {
        fn list_versions(
            &self,
            _: &Group,
            _: &str,
        ) -> Result<Vec<semver::Version>, DepProviderError> {
            match &self.answer {
                Answer::Serves(vs) => Ok(vs.clone()),
                Answer::Absent => Err(absent_err()),
                Answer::Fails => Err(DepProviderError::Other("boom".into())),
            }
        }
    }

    fn pkgref() -> PackageRef {
        PackageRef::parse("org.vibevm/wal").unwrap()
    }

    fn group() -> Group {
        Group::parse("org.vibevm").unwrap()
    }

    #[test]
    fn embedded_first_lets_the_embedded_registry_win_a_clash() {
        let emb = Canned {
            answer: Answer::Serves(vec![v("9.9.9")]),
            label: "emb",
        };
        let dec = Canned {
            answer: Answer::Serves(vec![v("1.0.0")]),
            label: "dec",
        };
        let order = order_providers(&emb, Some(&dec), EmbeddedPrecedence::EmbeddedFirst);
        assert_eq!(
            resolve_first(&order, |p| p.resolve_version(&pkgref())).unwrap(),
            v("9.9.9")
        );
        // …and the manifest comes from the embedded copy, not the declared one.
        let m = resolve_first(&order, |p| p.fetch_manifest(&group(), "wal", &v("0.1.0"))).unwrap();
        assert_eq!(m.require_package().unwrap().name, "emb");
    }

    #[test]
    fn embedded_last_lets_the_declared_walk_win_a_clash() {
        let emb = Canned {
            answer: Answer::Serves(vec![v("9.9.9")]),
            label: "emb",
        };
        let dec = Canned {
            answer: Answer::Serves(vec![v("1.0.0")]),
            label: "dec",
        };
        let order = order_providers(&emb, Some(&dec), EmbeddedPrecedence::EmbeddedLast);
        assert_eq!(
            resolve_first(&order, |p| p.resolve_version(&pkgref())).unwrap(),
            v("1.0.0")
        );
        let m = resolve_first(&order, |p| p.fetch_manifest(&group(), "wal", &v("0.1.0"))).unwrap();
        assert_eq!(m.require_package().unwrap().name, "dec");
    }

    #[test]
    fn an_absent_provider_is_skipped_not_fatal() {
        let emb = Canned {
            answer: Answer::Absent,
            label: "emb",
        };
        let dec = Canned {
            answer: Answer::Serves(vec![v("2.0.0")]),
            label: "dec",
        };
        let order = order_providers(&emb, Some(&dec), EmbeddedPrecedence::EmbeddedFirst);
        assert_eq!(
            resolve_first(&order, |p| p.resolve_version(&pkgref())).unwrap(),
            v("2.0.0")
        );
    }

    #[test]
    fn a_real_provider_failure_propagates_rather_than_falling_through() {
        let emb = Canned {
            answer: Answer::Fails,
            label: "emb",
        };
        let dec = Canned {
            answer: Answer::Serves(vec![v("2.0.0")]),
            label: "dec",
        };
        let order = order_providers(&emb, Some(&dec), EmbeddedPrecedence::EmbeddedFirst);
        let err = resolve_first(&order, |p| p.resolve_version(&pkgref())).unwrap_err();
        assert!(matches!(err, DepProviderError::Other(_)), "got {err:?}");
    }

    #[test]
    fn list_versions_unions_and_dedups_across_both() {
        let emb = Canned {
            answer: Answer::Serves(vec![v("1.0.0"), v("2.0.0")]),
            label: "emb",
        };
        let dec = Canned {
            answer: Answer::Serves(vec![v("2.0.0"), v("3.0.0")]),
            label: "dec",
        };
        let order = order_providers(&emb, Some(&dec), EmbeddedPrecedence::EmbeddedFirst);
        assert_eq!(
            union_versions(&order, &group(), "wal").unwrap(),
            vec![v("1.0.0"), v("2.0.0"), v("3.0.0")]
        );
    }

    #[test]
    fn with_no_declared_walk_the_embedded_registry_answers_alone() {
        let emb = Canned {
            answer: Answer::Serves(vec![v("1.2.3")]),
            label: "emb",
        };
        let order = order_providers(&emb, None, EmbeddedPrecedence::EmbeddedFirst);
        assert_eq!(order.len(), 1);
        assert_eq!(
            resolve_first(&order, |p| p.resolve_version(&pkgref())).unwrap(),
            v("1.2.3")
        );
        assert_eq!(
            union_versions(&order, &group(), "wal").unwrap(),
            vec![v("1.2.3")]
        );
    }
}
