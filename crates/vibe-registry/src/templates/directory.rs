//! The three answers a **directory-shaped** registry needs from the
//! template layer, in one place.
//!
//! A local-directory registry, a `monorepo` working copy and a mirrored
//! layout registry are all laid out `<root>/<group>/<name>/v<version>/`,
//! and all three reach the template layer through
//! [`crate::LocalRegistry`] — which is why
//! PROP-075 `#TEMPLATE-DOMAIN-SEAM`'s fourth call site names exactly
//! `LocalRegistry::{list_versions, resolve}`. The three answers are: the
//! version domain, the **skeleton** manifest of a produced version, and
//! the **full** render materialised as an ordinary package directory.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DOMAIN-SEAM");

use std::collections::BTreeSet;
use std::path::Path;

use vibe_core::Group;
use vibe_core::manifest::Manifest;

use super::error::TemplateError;
use super::{ActiveTemplate, TemplateLayer, VersionDomain, lock_record};

/// One package's template, its domain, and what a resolution needs.
pub struct DirectoryTemplate {
    /// The active template revision found on disk.
    pub template: ActiveTemplate,
    /// Its version domain, revisions excluded and concrete versions
    /// unioned in.
    pub domain: VersionDomain,
}

impl TemplateLayer {
    /// The template of `<group>/<name>` in a directory registry, with its
    /// domain already computed — the one entry point the four call sites
    /// share, so a path cannot accidentally consult only half of it.
    ///
    /// `Ok(None)` means this package publishes no template revision, the
    /// ordinary case for every package in the tree.
    pub fn for_directory(
        &self,
        registry_root: &Path,
        group: &Group,
        name: &str,
    ) -> Result<Option<DirectoryTemplate>, TemplateError> {
        let Some(template) = self.discover_in_directory(registry_root, group, name)? else {
            return Ok(None);
        };
        let domain = self.domain(&template)?;
        Ok(Some(DirectoryTemplate { template, domain }))
    }

    /// The union of a directory registry's own version list with a
    /// template's produced versions — **call site 1 and 4** of
    /// `#TEMPLATE-DOMAIN-SEAM`.
    ///
    /// The template revisions are removed: a revision is the revision of
    /// the template, never a produced version, so `install -g tool`
    /// cannot land on one.
    pub fn union_versions(
        &self,
        registry_root: &Path,
        group: &Group,
        name: &str,
        published: Vec<semver::Version>,
    ) -> Result<Vec<semver::Version>, TemplateError> {
        let Some(found) = self.for_directory(registry_root, group, name)? else {
            return Ok(published);
        };
        let revisions: BTreeSet<_> = found.template.revisions.iter().cloned().collect();
        let mut union: BTreeSet<semver::Version> = published
            .into_iter()
            .filter(|version| !revisions.contains(version))
            .collect();
        union.extend(found.domain.all());
        Ok(union.into_iter().collect())
    }

    /// The **skeleton** manifest of a produced version — **call site 2**.
    ///
    /// Costs no digest request at all: it is the stage everything
    /// resolution needs (PROP-075 `#TEMPLATE-TWO-STAGE`). `Ok(None)`
    /// means the registry publishes this version concretely, or there is
    /// no template, and the caller reads the manifest off disk as before.
    pub fn directory_skeleton(
        &self,
        registry_root: &Path,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Option<Manifest>, TemplateError> {
        let Some(found) = self.for_directory(registry_root, group, name)? else {
            return Ok(None);
        };
        if !found.domain.needs_render(version) {
            return Ok(None);
        }
        let Some(record) = found.domain.record(version) else {
            return Ok(None);
        };
        self.skeleton_manifest(&found.template, version, record)
            .map(Some)
    }

    /// The **full** render of a produced version, materialised as an
    /// ordinary package directory — **call site 3**.
    ///
    /// `Ok(None)` means the registry publishes this version concretely
    /// (`#TEMPLATE-CONCRETE-WINS`: the template is then not rendered for
    /// it at all, not rendered and compared), or there is no template.
    /// The lock record is noted on the layer, so the fetch that follows
    /// carries it onto the [`crate::CachedPackage`] without a second
    /// render.
    pub fn directory_materialise(
        &self,
        registry_root: &Path,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Option<crate::ResolvedPackage>, TemplateError> {
        let Some(found) = self.for_directory(registry_root, group, name)? else {
            return Ok(None);
        };
        if !found.domain.needs_render(version) {
            return Ok(None);
        }
        let Some(record) = found.domain.record(version) else {
            return Ok(None);
        };
        let render = self.full(&found.template, version, record)?;
        let resolved = self.materialise(&found.template, &render)?;
        self.record(
            &found.template.coordinate(),
            lock_record(
                &found.template,
                &render,
                found.template.section.version_list_url(),
            ),
            version,
        );
        Ok(Some(resolved))
    }
}
