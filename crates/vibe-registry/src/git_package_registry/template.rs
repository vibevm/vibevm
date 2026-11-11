//! The template layer, reached from an **indexed per-package** registry
//! (PROP-075 `#TEMPLATE-IN-A-REPO`).
//!
//! `#TEMPLATE-DISCOVERY` always said a template is discoverable in an
//! indexed registry: the per-version index record carries `template`, so
//! one read says which published versions are revisions, and the active
//! revision's own package version is then fetched "through the ordinary
//! path". This module is that path. It is the per-package-git twin of
//! [`crate::templates::directory`], and it carries the same four call
//! sites: the version union, the channel lowering, the skeleton manifest
//! and the materialised full render.
//!
//! **What it costs.** Discovery is the index read the registry was making
//! anyway, plus **one** `vibe.toml` read at the active revision's tag —
//! which on GitHub and GitVerse is a single HTTPS GET and on every other
//! host one `git archive`. Not one read per tag: that is the cost
//! `#TEMPLATE-DISCOVERY` says an index exists to avoid, and the reason a
//! registry **without** an index still reports `template_index_required`
//! rather than guessing.
//!
//! **A clone happens only to render.** The revision's whole tree is the
//! produced version's too, so materialisation needs it; nothing else
//! does. Listing versions, lowering a channel and reading a skeleton
//! manifest never check anything out.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-IN-A-REPO");

use std::collections::BTreeSet;

use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef, VersionSpec};

use super::pick::pick_version;
use super::*;
use crate::index_client::IndexVersion;
use crate::store;
use crate::templates::{RepoReader, RepoTemplate};

impl GitPerPackageRegistry {
    /// The active template of `<group>/<name>`, with its domain — the
    /// one entry point this registry's call sites share, so a path
    /// cannot consult only half of it.
    ///
    /// `Ok(None)` when the index named no template revision, which is
    /// the ordinary case for every package; also `Ok(None)` when this
    /// registry has no index at all, because a tag list cannot tell a
    /// revision from a version and this path will not guess.
    pub fn active_template(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Option<RepoTemplate>, RegistryError> {
        let Some(layer) = crate::templates::active() else {
            return Ok(None);
        };
        let (entries, _) = self.list_candidates_with_route(group, name)?;
        let (revisions, published) = split_revisions(&entries);
        if revisions.is_empty() {
            return Ok(None);
        }
        let read_manifest = |revision: &semver::Version| -> Result<String, String> {
            let tag = format!("v{revision}");
            let bytes = self
                .read_package_file(group, name, &tag, Manifest::FILENAME)
                .map_err(|error| error.to_string())?;
            String::from_utf8(bytes).map_err(|error| format!("invalid UTF-8: {error}"))
        };
        let reader = RepoReader {
            revisions,
            published,
            read_manifest: &read_manifest,
            // Where the revision will be checked out if a render needs
            // its tree. Named now, populated by
            // [`Self::materialise_from_template`] and nowhere else.
            tree: self.package_clone_dir(group, name),
        };
        layer
            .for_package_repo(group, name, &reader)
            .map_err(RegistryError::from)
    }

    /// **Call site 1**: the concrete publications unioned with the
    /// versions the template's source lists, the revisions excluded.
    pub(crate) fn union_with_template(
        &self,
        group: &Group,
        name: &str,
        produced: Vec<semver::Version>,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        let Some(found) = self.active_template(group, name)? else {
            return Ok(produced);
        };
        let revisions: BTreeSet<_> = found.template.revisions.iter().cloned().collect();
        let mut union: BTreeSet<semver::Version> = produced
            .into_iter()
            .filter(|version| !revisions.contains(version))
            .collect();
        union.extend(found.domain.all());
        Ok(union.into_iter().collect())
    }

    /// Lower a channel requirement against this package's template, if
    /// it has one. `Ok(None)` means the requirement was not a channel,
    /// or there is no template to lower it against — and an unlowered
    /// channel then matches nothing, which is the refusal
    /// `#CHANNEL-SYNTAX` prescribes rather than a silent miss.
    pub(crate) fn lower_through_template(
        &self,
        pkgref: &PackageRef,
    ) -> Result<Option<PackageRef>, RegistryError> {
        let VersionSpec::Channel(_) = &pkgref.version else {
            return Ok(None);
        };
        let group = pkgref
            .group
            .as_ref()
            .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
        let Some(layer) = crate::templates::active() else {
            return Ok(None);
        };
        let Some(found) = self.active_template(group, pkgref.name.as_str())? else {
            return Ok(None);
        };
        let resolver = crate::templates::TemplateChannels::new(&layer, &found.template);
        let mut lowered = pkgref.clone();
        let channel =
            vibe_core::channel::lower_channel(&mut lowered, &resolver).map_err(|error| {
                RegistryError::MalformedMeta {
                    path: std::path::PathBuf::from(format!("{group}/{}", pkgref.name)),
                    reason: error.to_string(),
                }
            })?;
        // The channel is noted on the layer so the lock row carries
        // `via_channel` — the same note the directory path makes
        // (PROP-075 `#CHANNEL-SYNTAX`, `#TEMPLATE-LOCK-RECORD`).
        if let (Some(channel), Some(group)) = (&channel, lowered.group.as_ref())
            && let VersionSpec::Req(req) = &lowered.version
            && let Some(comparator) = req.comparators.first()
        {
            let version = semver::Version::new(
                comparator.major,
                comparator.minor.unwrap_or(0),
                comparator.patch.unwrap_or(0),
            );
            layer.record_channel(&format!("{group}/{}", lowered.name), &version, channel);
        }
        Ok(Some(lowered))
    }

    /// The pick, over the produced candidates **and** the versions only
    /// the template names.
    ///
    /// A candidate the index named keeps its `must_understand`
    /// quarantine (B-080); a version only the template produces has no
    /// index record and so declares nothing, exactly as a git-tag
    /// candidate does.
    pub(crate) fn pick_with_template(
        &self,
        group: &Group,
        name: &str,
        candidates: &[IndexVersion],
        spec: &VersionSpec,
    ) -> Result<semver::Version, RegistryError> {
        let Some(found) = self.active_template(group, name)? else {
            return pick_version(group, name, candidates, spec);
        };
        let known: BTreeSet<semver::Version> =
            candidates.iter().map(|c| c.version.clone()).collect();
        let mut widened: Vec<IndexVersion> = candidates.to_vec();
        for version in found.domain.all() {
            if known.contains(&version) {
                continue;
            }
            widened.push(IndexVersion {
                version,
                must_understand: Vec::new(),
                template: false,
            });
        }
        widened.sort_by(|a, b| a.version.cmp(&b.version));
        pick_version(group, name, &widened, spec)
    }

    /// **Call site 2**: the skeleton manifest of a produced version —
    /// every digest-bearing field at its declared placeholder, no digest
    /// document fetched, no request made (PROP-075
    /// `#TEMPLATE-TWO-STAGE`).
    ///
    /// `Ok(None)` means the registry publishes this version concretely,
    /// or there is no template, and the caller reads the manifest at the
    /// version's own tag as before.
    pub(crate) fn template_skeleton(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Option<Manifest>, RegistryError> {
        let Some(layer) = crate::templates::active() else {
            return Ok(None);
        };
        let Some(found) = self.active_template(group, name)? else {
            return Ok(None);
        };
        if !found.domain.needs_render(version) {
            return Ok(None);
        }
        let Some(record) = found.domain.record(version) else {
            return Ok(None);
        };
        layer
            .skeleton_manifest(&found.template, version, record)
            .map(Some)
            .map_err(RegistryError::from)
    }

    /// **Call site 3**: the full render of a produced version,
    /// materialised as an ordinary package directory.
    ///
    /// This is the one path that checks the revision out: the rendered
    /// directory is a copy of the revision's tree with its `vibe.toml`
    /// replaced, so the tree has to be there. `Ok(None)` means the
    /// registry publishes this version concretely
    /// (`#TEMPLATE-CONCRETE-WINS`: the template is then not rendered for
    /// it at all) or there is no template.
    pub(crate) fn materialise_from_template(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Option<ResolvedPackage>, RegistryError> {
        let Some(layer) = crate::templates::active() else {
            return Ok(None);
        };
        let Some(found) = self.active_template(group, name)? else {
            return Ok(None);
        };
        if !found.domain.needs_render(version) {
            return Ok(None);
        }
        let Some(record) = found.domain.record(version) else {
            return Ok(None);
        };
        // The render needs the revision's whole tree, and this is the
        // only place that wants it, so this is the only place that pays
        // for it.
        let tag = format!("v{}", found.template.revision);
        self.refresh_package(group, name, &tag)?;
        let render = layer.full(&found.template, version, record)?;
        let resolved = layer.materialise(&found.template, &render)?;
        layer.record(
            &found.template.coordinate(),
            crate::templates::lock_record(
                &found.template,
                &render,
                found.template.section.version_list_url(),
            ),
            version,
        );
        Ok(Some(resolved))
    }
}

/// Split the index rows into the template's revisions and the concrete
/// publications, each ascending.
fn split_revisions(entries: &[IndexVersion]) -> (Vec<semver::Version>, Vec<semver::Version>) {
    let mut revisions: BTreeSet<semver::Version> = BTreeSet::new();
    let mut published: BTreeSet<semver::Version> = BTreeSet::new();
    for entry in entries {
        if entry.template {
            revisions.insert(entry.version.clone());
        } else {
            published.insert(entry.version.clone());
        }
    }
    (
        revisions.into_iter().collect(),
        published.into_iter().collect(),
    )
}

impl GitPerPackageRegistry {
    /// Insert a **rendered** version's directory into the machine-global
    /// store, exactly as a directory registry inserts an ordinary
    /// package (PROP-075 `#TEMPLATE-DOMAIN-SEAM`, call site 3's tail).
    ///
    /// There is no clone, no tag and no URL to record, because upstream
    /// has no such version: it exists as a row in the template's version
    /// source and as the revision whose tree this is a copy of. The
    /// provenance the lock keeps is therefore the template record the
    /// layer noted while rendering, which is what `#TEMPLATE-LOCK-RECORD`
    /// says binds — the rendered hash, not a commit.
    pub(crate) fn fetch_materialised(
        &self,
        resolved: &ResolvedPackage,
        store_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        let content_hash = compute_content_hash(&resolved.source_dir)?;
        let inserted = store::insert_current_at(
            store_root,
            &resolved.source_dir,
            &resolved.group,
            &resolved.name,
            &resolved.version,
            &content_hash,
        )?;
        let reused = matches!(&inserted, store::InsertOutcome::AlreadyPresent(_));
        let cache_dir = inserted.into_entry();
        let manifest_path = cache_dir.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path,
                reason: "a rendered package manifest must carry a [package] table".to_string(),
            });
        }
        let noted = crate::templates::active()
            .map(|layer| layer.noted(&resolved.group, &resolved.name, &resolved.version))
            .unwrap_or_default();
        Ok(CachedPackage {
            template: noted.template,
            via_channel: noted.via_channel,
            locked_selection: None,
            route_publication: vibe_core::RoutePublication::Local,
            recorded_source_kind: None,
            store_reused: Some(reused),
            acquisition: Some(vibe_wire::generated::shared::SourceAcquisition::Directory),
            resolved: resolved.clone(),
            cache_dir,
            manifest,
            content_hash,
            // The template package's own repository, which is where the
            // revision that rendered this came from — the one upstream
            // identity a rendered version honestly has.
            source_uri: self.package_repo_url(&resolved.group, &resolved.name)?,
            fetched_from: None,
            registry_name: Some(self.name.clone()),
            source_ref: None,
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            is_local: false,
            via_redirect: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The split is by the flag the record carries and by nothing else —
    /// not by a version spelling, not by sort order.
    #[test]
    fn the_split_is_by_the_flag_the_record_carries() {
        let row = |version: &str, template: bool| IndexVersion {
            version: version.parse().unwrap(),
            must_understand: Vec::new(),
            template,
        };
        let (revisions, published) = split_revisions(&[
            row("1.1.0", true),
            row("0.41.0", false),
            row("1.0.0", true),
            row("22.23.4", false),
        ]);
        assert_eq!(
            revisions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["1.0.0", "1.1.0"]
        );
        assert_eq!(
            published
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["0.41.0", "22.23.4"]
        );
        // The newest revision is the active one, whatever the order the
        // rows arrived in.
        assert_eq!(revisions.last().unwrap().to_string(), "1.1.0");
    }

    /// No template rows at all: nothing is a revision, so the package
    /// reads exactly as it did before this module existed.
    #[test]
    fn a_package_with_no_template_row_has_no_revisions() {
        let (revisions, published) = split_revisions(&[IndexVersion {
            version: "1.0.0".parse().unwrap(),
            must_understand: Vec::new(),
            template: false,
        }]);
        assert!(revisions.is_empty());
        assert_eq!(published.len(), 1);
    }
}
