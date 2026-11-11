//! The three answers an **indexed per-package** registry needs from the
//! template layer (PROP-075 `#TEMPLATE-DISCOVERY`, `#TEMPLATE-IN-A-REPO`).
//!
//! A per-package registry has no tree on disk to read manifests off, so
//! the directory entry point in [`super::directory`] cannot serve it.
//! What it has instead is exactly what `#TEMPLATE-DISCOVERY` says it has:
//! an index whose per-version records carry `template`, so **one read**
//! says which published versions are revisions and which are concrete,
//! and a package repository whose tags can be read one file at a time.
//! Discovery is therefore one index read the registry was making anyway
//! plus **one** file read at the active revision's tag — not one read per
//! tag, which is the cost `#TEMPLATE-DISCOVERY` says the index exists to
//! avoid.
//!
//! The tree is a separate matter and a later one. A render needs the
//! revision's whole tree (its recipes, icons and payload files are the
//! produced version's too), and that costs a clone; so the tree is named
//! here and populated by the registry **only** when a version is actually
//! materialised. Enumerating versions, lowering a channel and reading a
//! skeleton manifest all cost nothing but the two reads above.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-IN-A-REPO");

use std::path::PathBuf;

use vibe_core::Group;

use super::error::TemplateError;
use super::{ActiveTemplate, TemplateLayer, VersionDomain};

/// One package's template as an indexed registry sees it, with its
/// domain already computed — the same pairing
/// [`super::directory::DirectoryTemplate`] carries, so the two registry
/// shapes answer the same question with the same type.
pub struct RepoTemplate {
    /// The active template revision, whose `tree` is where the registry
    /// will check the revision out when something must be rendered.
    pub template: ActiveTemplate,
    /// Its version domain: the produced versions unioned with the
    /// concrete publications, revisions excluded.
    pub domain: VersionDomain,
}

/// What the registry must hand the layer, because only the registry
/// knows how to get it: the rows the index gave, and how to read one
/// file at one tag.
pub struct RepoReader<'a> {
    /// Every published version the index named that carries
    /// `[package] template = true` — the template's revisions.
    pub revisions: Vec<semver::Version>,
    /// Every published version the index named that does not — the
    /// concrete publications (`#TEMPLATE-CONCRETE-WINS`).
    pub published: Vec<semver::Version>,
    /// Read `vibe.toml` at tag `v<revision>`, as **text**. Text and not
    /// a typed `Manifest`, because a template that uses the numeric
    /// coercion does not deserialise into one (PROP-075
    /// `#TEMPLATE-DOCUMENT-READING`).
    pub read_manifest: &'a dyn Fn(&semver::Version) -> Result<String, String>,
    /// Where the registry will check that revision out when a render
    /// needs its tree. Named now, populated later and only if needed.
    pub tree: PathBuf,
}

impl TemplateLayer {
    /// The template of `<group>/<name>` in an indexed per-package
    /// registry, with its domain already computed — the one entry point
    /// that registry's call sites share, so a path cannot consult only
    /// half of it.
    ///
    /// `Ok(None)` means the index named no template revision for this
    /// package, which is the ordinary case for every package in a
    /// registry.
    pub fn for_package_repo(
        &self,
        group: &Group,
        name: &str,
        reader: &RepoReader<'_>,
    ) -> Result<Option<RepoTemplate>, TemplateError> {
        let Some(revision) = reader.revisions.iter().max() else {
            return Ok(None);
        };
        let coordinate = format!("{group}/{name}");
        let template_toml =
            (reader.read_manifest)(revision).map_err(|reason| TemplateError::Declaration {
                package: coordinate.clone(),
                reason: format!("the template revision {revision} could not be read: {reason}"),
            })?;
        // A row the index marked `template` whose manifest does not
        // declare the flag is a disagreement between the index and the
        // tree. The tree wins — it is the thing that was published — and
        // the package reads as having no template, which is the same
        // answer a registry with no index would give.
        if !super::declares_template(&template_toml) {
            tracing::warn!(
                target: "vibe_registry::templates",
                "{coordinate}: the index marks {revision} as a template revision and its \
                 manifest does not declare `[package] template = true`; read as an ordinary \
                 version"
            );
            return Ok(None);
        }
        let section = vibe_template::template_section(&template_toml).map_err(|error| {
            TemplateError::Declaration {
                package: coordinate,
                reason: error.to_string(),
            }
        })?;
        let cache = self.cache(group, name);
        // Keep the declaration beside its inputs, exactly as the
        // directory path does, so a channel can be lowered later from
        // the cache alone — the budget a prompt hook has
        // (PROP-075 `#ACTIVATION-HOOK`).
        cache.write_declaration(&template_toml)?;
        let template = ActiveTemplate {
            group: group.clone(),
            name: name.to_string(),
            revision: revision.clone(),
            tree: reader.tree.clone(),
            template_toml,
            section,
            published: reader.published.clone(),
            revisions: reader.revisions.clone(),
        };
        let domain = self.domain(&template)?;
        Ok(Some(RepoTemplate { template, domain }))
    }
}
