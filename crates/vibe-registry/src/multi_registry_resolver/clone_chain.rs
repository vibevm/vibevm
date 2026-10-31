//! The clone chain of one registry, through the Locator
//! (`PROP-066#locator`).
//!
//! PROP-002's `[[mirror]]` chain and PROP-066's rows are one mechanism,
//! not two: the chain a registry is tried through is the Locator's
//! candidate list for a `registry-clone` artifact whose declared origin is
//! the registry's own URL. A `[[mirror]]` user sees exactly what they saw
//! before — the tables are read as rows of the local layer, priority
//! order preserved — and a `prefix`, `registry` or `package` row reaches
//! the clone the same way, which is the whole point of the PROP.
//!
//! Two Locators can answer: the invocation's, when the composition root
//! installed one (it already carries the mirror and override rows of the
//! effective registry config), and otherwise one derived here from the
//! `mirrors` this resolver was handed. The second is what keeps a library
//! caller and every existing registry test working untouched, and it is
//! never both: whichever Locator answers holds the mirror rows exactly
//! once.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#locator");

use vibe_core::manifest::{MirrorSection, RegistrySection};
use vibe_core::remap::{
    Artifact, ArtifactKind, Layer, LoadedSource, Locator, RemapTarget, Resolution, UnmatchedPolicy,
    rows_from_mirrors,
};

use crate::RegistryError;

/// The alternative clone URLs to try for `registry`, most-winning first
/// and never including the registry's own URL (the git layer appends that
/// itself as the origin).
pub(crate) fn clone_chain(
    registry: &RegistrySection,
    mirrors: &[MirrorSection],
    registries: &[RegistrySection],
) -> Result<Vec<String>, RegistryError> {
    let derived;
    let locator = if vibe_core::remap::active().is_empty() {
        derived = derive(mirrors, registries);
        &derived
    } else {
        vibe_core::remap::active()
    };
    let artifact = Artifact::new(ArtifactKind::RegistryClone).with_url(&registry.url);
    match locator.locate(&artifact) {
        Resolution::Candidates(candidates) => Ok(candidates
            .into_iter()
            .filter(|candidate| candidate.row.is_some())
            .filter_map(|candidate| match candidate.target {
                RemapTarget::Url(url) => Some(url),
                // A `path` target is a directory of artifacts laid out by
                // digest, which is not a git clone; the chain skips it and
                // the local-registry leg of `from_manifest` is what reads
                // a directory as a registry.
                RemapTarget::Path(_) | RemapTarget::Deny => None,
            })
            .filter(|url| url != &registry.url)
            .collect()),
        Resolution::Denied { row, reason } => Err(RegistryError::RemapDenied {
            registry: registry.name.clone(),
            url: registry.url.clone(),
            row: row.map_or_else(
                || "the active `[policy] unmatched = \"deny\"`".to_owned(),
                |row| {
                    format!(
                        "row {} of `{}` ({})",
                        row.provenance.index + 1,
                        row.provenance.source,
                        row.provenance.layer.label()
                    )
                },
            ),
            reason: reason.unwrap_or_else(|| "no reason given".to_owned()),
        }),
    }
}

/// A Locator over nothing but the `[[mirror]]` tables this resolver was
/// handed — the answer when no invocation installed one.
fn derive(mirrors: &[MirrorSection], registries: &[RegistrySection]) -> Locator {
    let source = "[[mirror]]";
    Locator::from_sources(
        vec![LoadedSource {
            layer: Layer::Local,
            source: source.into(),
            rows: rows_from_mirrors(mirrors, registries, Layer::Local, source),
            policy: UnmatchedPolicy::Allow,
        }],
        registries
            .iter()
            .map(|registry| (registry.name.clone(), registry.url.clone()))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_core::manifest::{AuthKind, NamingConvention};

    fn registry(name: &str, url: &str) -> RegistrySection {
        RegistrySection {
            name: name.into(),
            url: url.into(),
            r#ref: "main".into(),
            naming: NamingConvention::Fqdn,
            auth: AuthKind::None,
            token_env: None,
            enabled: true,
            index_url: None,
        }
    }

    #[test]
    fn a_named_mirror_chain_keeps_its_priority_order() {
        let declared = [registry("vibespecs", "https://github.com/vibespecs")];
        let mirrors = [
            MirrorSection {
                of: "vibespecs".into(),
                url: "https://second.example/vibespecs".into(),
                priority: 5,
            },
            MirrorSection {
                of: "vibespecs".into(),
                url: "https://first.example/vibespecs".into(),
                priority: 0,
            },
        ];
        let chain = clone_chain(&declared[0], &mirrors, &declared).unwrap();
        assert_eq!(
            chain,
            [
                "https://first.example/vibespecs",
                "https://second.example/vibespecs",
            ]
        );
    }

    #[test]
    fn a_wildcard_mirror_reaches_every_declared_registry() {
        let declared = [
            registry("a", "https://github.com/a"),
            registry("b", "https://gitverse.ru/b"),
        ];
        let mirrors = [MirrorSection {
            of: "*".into(),
            url: "https://nexus.example/all".into(),
            priority: 0,
        }];
        for entry in &declared {
            let chain = clone_chain(entry, &mirrors, &declared).unwrap();
            assert_eq!(chain, ["https://nexus.example/all"], "{}", entry.name);
        }
    }

    #[test]
    fn no_mirror_is_an_empty_chain_and_never_the_registrys_own_url() {
        let declared = [registry("vibespecs", "https://github.com/vibespecs")];
        assert!(
            clone_chain(&declared[0], &[], &declared)
                .unwrap()
                .is_empty()
        );
    }
}
