//! The mirror's third listing: a per-package source read whole through
//! its host's organisation API (PROP-002 `#registry-mirror`).
//!
//! `org_api` answers "which repositories does this organisation hold?".
//! This is what the mirror does with that answer: address each repository
//! directly, read the coordinate its own manifest declares at its newest
//! `v<semver>` tag, and keep it only when the registry's naming
//! convention would have published that package under that very name.
//! Every kept version is then a tag of the package's own repository —
//! exactly what the `--package` path produces.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use anyhow::{Result, bail};
use vibe_core::Group;
use vibe_core::manifest::Manifest;
use vibe_wire::generated::registry_mirror_report::Listing as ListingKind;

use super::{SourceListing, SourceReader, admitted, tagged_versions};
use crate::commands::registry::mirror::{Filters, org_api};

impl SourceReader {
    /// List a per-package source whole, without an index and without
    /// `--package`: ask the host which repositories its organisation
    /// holds, then let each repository's own manifest state which package
    /// it is and keep it only when this registry's naming convention
    /// would have published that package there. Every kept version is
    /// then read exactly as the `--package` path reads it.
    pub(super) fn list_through_host_api(&self, filters: &Filters) -> Result<SourceListing> {
        let SourceReader::PerPackage {
            registry,
            admission,
            offline,
            ..
        } = self
        else {
            bail!("internal: a layout source has no host API");
        };
        let api = org_api::resolve(registry, *offline)?;
        let mut versions = Vec::new();
        for repo in api.repositories(registry, admission.clone())? {
            // What the name alone says. Under `fqdn` that is the whole
            // coordinate, so a name the convention could not have
            // produced — and a package the filters exclude — costs no
            // request at all.
            let claim = org_api::name_claim(registry.naming, &repo);
            match &claim {
                org_api::NameClaim::Rejected => continue,
                org_api::NameClaim::Coordinate(group, name)
                    if !filters.admits(group.as_str(), name) =>
                {
                    continue;
                }
                _ => {}
            }
            // The host named the repository, so it is addressed directly
            // rather than composed back from a coordinate this listing
            // does not have yet.
            let repository = format!("{}/{repo}.git", registry.url.trim_end_matches('/'));
            let Some(tags) = self.semver_tags(&repository)? else {
                continue;
            };
            let Some(newest) = tags.iter().max() else {
                continue;
            };
            let Some((group, name)) = self.declared_coordinate(&repository, &repo, newest)? else {
                continue;
            };
            // A decidable name and the manifest must agree. Under `fqdn`
            // both have already been composed back into `repo`, so this
            // can only fire if a convention ever stopped being injective
            // — and then guessing is exactly what must not happen.
            if let org_api::NameClaim::Coordinate(claimed_group, claimed_name) = &claim
                && (claimed_group != &group || claimed_name != &name)
            {
                tracing::debug!(
                    target: "vibe_cli::registry::mirror",
                    "skipping `{repo}`: its name claims `{claimed_group}/{claimed_name}` and its \
                     manifest `{group}/{name}`"
                );
                continue;
            }
            if !filters.admits(group.as_str(), &name) {
                continue;
            }
            versions.extend(tagged_versions(&repository, &group, &name, tags));
        }
        Ok(SourceListing {
            kind: ListingKind::Api,
            versions,
            tombstones: Vec::new(),
        })
    }

    /// The `v<semver>` tags of `repository`, or `None` when it cannot be
    /// read at all. An organisation holds repositories a mirror may not
    /// read (private ones, say); a listing leaves those out rather than
    /// claiming them, and `--package` names one explicitly when it must.
    fn semver_tags(&self, repository: &str) -> Result<Option<Vec<semver::Version>>> {
        let SourceReader::PerPackage {
            registry,
            mirrors,
            store,
            denied,
            ..
        } = self
        else {
            bail!("internal: a layout source has no repositories");
        };
        let url = admitted(registry, mirrors, denied, repository)?;
        match store.semver_tags(&url) {
            Ok(tags) => Ok(Some(tags)),
            Err(error) => {
                tracing::debug!(
                    target: "vibe_cli::registry::mirror",
                    "skipping `{repository}`: its tags could not be read: {error:#}"
                );
                Ok(None)
            }
        }
    }

    /// The coordinate `repository` declares at `v<version>` — its
    /// `vibe.toml`'s `[package] group`, `name` and `kind` — confirmed
    /// against the repository's actual name `repo` by this registry's
    /// naming convention. `None` when the repository carries no package
    /// manifest there, or carries one this registry would have published
    /// under a different name.
    ///
    /// The tag is fetched through the same remap Locator and
    /// source-policy gate a copy uses, into the run's own object store,
    /// so a repository this listing admits is one the copy stage can
    /// already reach.
    fn declared_coordinate(
        &self,
        repository: &str,
        repo: &str,
        version: &semver::Version,
    ) -> Result<Option<(Group, String)>> {
        let SourceReader::PerPackage {
            registry,
            mirrors,
            store,
            denied,
            ..
        } = self
        else {
            bail!("internal: a layout source has no repositories");
        };
        let url = admitted(registry, mirrors, denied, repository)?;
        let tag = format!("v{version}");
        let facts = match store.fetch_tag(&url, repo, &tag) {
            Ok(facts) => facts,
            Err(error) => {
                tracing::debug!(
                    target: "vibe_cli::registry::mirror",
                    "skipping `{repo}`: tag `{tag}` could not be read: {error:#}"
                );
                return Ok(None);
            }
        };
        let Some(text) = store.read_blob(&facts.tree, Manifest::FILENAME) else {
            return Ok(None);
        };
        let Some(package) = Manifest::parse_str(&text)
            .ok()
            .and_then(|manifest| manifest.package)
        else {
            return Ok(None);
        };
        if !org_api::produces(
            registry.naming,
            repo,
            package.kind,
            &package.group,
            &package.name,
        ) {
            tracing::debug!(
                target: "vibe_cli::registry::mirror",
                "skipping `{repo}`: registry `{}` would publish its `{}/{}` elsewhere under \
                 its {:?} naming convention",
                registry.name,
                package.group,
                package.name,
                registry.naming,
            );
            return Ok(None);
        }
        Ok(Some((package.group, package.name)))
    }
}
