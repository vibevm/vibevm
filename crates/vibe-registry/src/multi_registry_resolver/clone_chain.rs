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
//! The Locator that answers is the invocation's with this catalog's own
//! registry tables composed into its `[[mirror]] / [[override]]` slot
//! (`Locator::with_registry_tables`, one lowering owned by
//! `vibe_core::remap::rows_from_registry_tables`). The composition root
//! reads the tables up front only for catalog verbs; every other verb
//! demands them here, so a `[[mirror]]` serves `vibe install` exactly as
//! it serves `vibe registry list` — whatever `[[remap]]`, drop or release
//! rows the invocation also carries — and the mirror rows are present
//! exactly once. Without an installed Locator (a library caller, a unit
//! test) the composition is over the empty Locator and the mirror tables
//! alone answer.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#locator");

use vibe_core::manifest::{MirrorSection, RegistrySection};
use vibe_core::remap::{
    Artifact, ArtifactKind, Layer, RemapTarget, Resolution, rows_from_registry_tables,
};

use crate::RegistryError;

/// The alternative clone URLs only, for tests written against the URL list.
#[cfg(test)]
pub(crate) fn clone_chain(
    registry: &RegistrySection,
    mirrors: &[MirrorSection],
    registries: &[RegistrySection],
) -> Result<Vec<String>, RegistryError> {
    clone_chain_scoped(registry, mirrors, registries)
        .map(|routes| routes.into_iter().map(|route| route.url).collect())
}

pub(crate) fn clone_chain_scoped(
    registry: &RegistrySection,
    mirrors: &[MirrorSection],
    registries: &[RegistrySection],
) -> Result<Vec<crate::git_package_registry::PublicationRoute>, RegistryError> {
    let locator = vibe_core::remap::active().with_registry_tables(
        rows_from_registry_tables(registries, mirrors, &[], Layer::Local),
        registries
            .iter()
            .map(|registry| (registry.name.clone(), registry.url.clone()))
            .collect(),
    );
    let artifact = Artifact::new(ArtifactKind::RegistryClone).with_url(&registry.url);
    match locator.locate(&artifact) {
        Resolution::Candidates(candidates) => Ok(candidates
            .into_iter()
            .filter(|candidate| candidate.row.is_some())
            .filter_map(|candidate| match candidate.target {
                RemapTarget::Url(url) => Some(crate::git_package_registry::PublicationRoute {
                    url,
                    publication: candidate
                        .row
                        .as_ref()
                        .map_or(vibe_core::RoutePublication::Local, |row| {
                            row.provenance.publication
                        }),
                }),
                // A `path` target is a directory of artifacts laid out by
                // digest, which is not a git clone; the chain skips it and
                // the local-registry leg of `from_manifest` is what reads
                // a directory as a registry.
                RemapTarget::Path(_) | RemapTarget::Deny => None,
            })
            .filter(|route| route.url != registry.url)
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

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_core::manifest::NamingConvention;

    fn registry(name: &str, url: &str) -> RegistrySection {
        RegistrySection {
            name: name.into(),
            r#type: None,
            url: url.into(),
            r#ref: "main".into(),
            naming: NamingConvention::Fqdn,
            auth: None,
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
                publication: Default::default(),
                of: "vibespecs".into(),
                url: "https://second.example/vibespecs".into(),
                priority: 5,
            },
            MirrorSection {
                publication: Default::default(),
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
            publication: Default::default(),
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
