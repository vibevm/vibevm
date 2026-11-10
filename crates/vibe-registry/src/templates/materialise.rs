//! The two render stages, and the store materialisation that turns a
//! full render into an ordinary package directory (PROP-075
//! `#TEMPLATE-TWO-STAGE`, `#TEMPLATE-DOMAIN-SEAM`).
//!
//! A **skeleton** render uses the version record alone and is enough for
//! everything resolution needs; it is never installed from, never written
//! into a store and never hashed into a lock. A **full** render
//! substitutes the digests and is performed only for a version that is
//! actually selected, locked, installed, described or mirrored — so
//! enumerating a template's versions is one request for the whole
//! package, resolving a requirement against it is none, and installing
//! one version is exactly one more.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-TWO-STAGE");

use std::path::{Path, PathBuf};

use vibe_core::manifest::{Manifest, TemplateRenderChanged, differing_digest_fields};
use vibe_template::{DigestTable, DroppedOption, VersionRecord};

use super::cache::digest_of;
use super::error::TemplateError;
use super::{ActiveTemplate, TemplateLayer};

/// One full render of one produced version: the bytes, their hash, the
/// digest document behind them and the rows `template_optional` dropped.
///
/// ```
/// use vibe_registry::templates::FullRender;
///
/// let render = FullRender {
///     version: "24.22.2".parse().unwrap(),
///     toml: "[package]\n".to_string(),
///     sha256: "ab".repeat(32),
///     digest_source_sha256: "cd".repeat(32),
///     digest_url: "https://nodejs.org/dist/v24.22.2/SHASUMS256.txt".to_string(),
///     dropped: Vec::new(),
/// };
/// assert_eq!(render.sha256.len(), 64);
/// assert!(render.dropped.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullRender {
    /// The produced version.
    pub version: semver::Version,
    /// The rendered manifest's TOML text.
    pub toml: String,
    /// SHA-256 of those bytes — the `rendered_sha256` of a lock record.
    pub sha256: String,
    /// SHA-256 of the digest document's bytes.
    pub digest_source_sha256: String,
    /// The digest document's URL after remap.
    pub digest_url: String,
    /// The `[[installer_options]]` rows dropped because a digest
    /// variable they reference has no value for this version and the row
    /// declared `template_optional`.
    pub dropped: Vec<DroppedOption>,
}

impl TemplateLayer {
    /// A **skeleton** render of one produced version: every
    /// digest-bearing field at the declared placeholder, no digest
    /// document fetched, no request made.
    pub fn skeleton(
        &self,
        template: &ActiveTemplate,
        version: &semver::Version,
        record: &VersionRecord,
    ) -> Result<String, TemplateError> {
        vibe_template::skeleton(&template.template_toml, record)
            .map(|rendered| rendered.toml)
            .map_err(|error| TemplateError::Render {
                package: template.coordinate(),
                version: version.to_string(),
                reason: error.to_string(),
            })
    }

    /// A skeleton render parsed as the ordinary `Manifest` every caller
    /// of the manifest seam expects.
    pub fn skeleton_manifest(
        &self,
        template: &ActiveTemplate,
        version: &semver::Version,
        record: &VersionRecord,
    ) -> Result<Manifest, TemplateError> {
        let toml = self.skeleton(template, version, record)?;
        Manifest::parse_str(&toml).map_err(|error| TemplateError::Render {
            package: template.coordinate(),
            version: version.to_string(),
            reason: format!("the skeleton render is not a valid manifest: {error}"),
        })
    }

    /// A **full** render of one produced version: the digest document is
    /// read (from the cache when it is there, which is always after the
    /// first time) and substituted.
    pub fn full(
        &self,
        template: &ActiveTemplate,
        version: &semver::Version,
        record: &VersionRecord,
    ) -> Result<FullRender, TemplateError> {
        let fetched = self.digest_document(template, version)?;
        let digests = DigestTable::parse(&template.section, &fetched.body).map_err(|error| {
            TemplateError::Render {
                package: template.coordinate(),
                version: version.to_string(),
                reason: error.to_string(),
            }
        })?;
        let rendered =
            vibe_template::render(&template.template_toml, record, &digests).map_err(|error| {
                TemplateError::Render {
                    package: template.coordinate(),
                    version: version.to_string(),
                    reason: error.to_string(),
                }
            })?;
        Ok(FullRender {
            version: version.clone(),
            sha256: rendered.sha256,
            toml: rendered.toml,
            digest_source_sha256: digest_of(fetched.body.as_bytes()),
            digest_url: fetched.chosen_url,
            dropped: rendered.dropped,
        })
    }

    /// Materialise a full render as an **ordinary package directory**:
    /// the template's tree copied with its `vibe.toml` replaced, under
    /// `<settings>/cache/templates/<group>/<name>/rendered/v<version>/`.
    ///
    /// Everything above this returns the `ResolvedPackage` every caller
    /// already expects, which is the whole point: below the four call
    /// sites nothing learns that a manifest was rendered.
    pub fn materialise(
        &self,
        template: &ActiveTemplate,
        render: &FullRender,
    ) -> Result<crate::ResolvedPackage, TemplateError> {
        let target = self
            .cache(&template.group, &template.name)
            .rendered_dir(&render.version);
        let io = |path: &Path, error: std::io::Error| TemplateError::Io {
            path: path.to_path_buf(),
            reason: error.to_string(),
        };
        // **The alarm, raised before any effect** (PROP-075
        // `#TEMPLATE-LOCK-RECORD`). A rendered directory already here is
        // the same produced version rendered before; its digest-bearing
        // fields came from a digest document that is immutable, so a
        // difference in them can only mean the document changed upstream
        // — which is exactly the alarm, and the reason the document is
        // cached forever rather than revalidated. A difference *outside*
        // them is a template fixed without touching a produced version,
        // which TEMPLATE-FLAG exists to allow, so it rewrites silently.
        let previous = target.join(Manifest::FILENAME);
        if let Ok(before) = std::fs::read_to_string(&previous) {
            let fields = differing_digest_fields(&before, &render.toml);
            if !fields.is_empty() {
                return Err(TemplateError::RenderChanged(Box::new(
                    TemplateRenderChanged {
                        package: template.coordinate(),
                        version: render.version.clone(),
                        // A render rewrites `[package].version` to the
                        // produced version, so the earlier bytes do not
                        // say which revision made them; the lock record
                        // does, and `verify_recorded` reports it. Here
                        // both sides are the current revision, which is
                        // honest rather than invented, and the field list
                        // carries what actually differs.
                        recorded_revision: template.revision.clone(),
                        computed_revision: template.revision.clone(),
                        recorded_sha256: super::cache::digest_of(before.as_bytes()),
                        computed_sha256: render.sha256.clone(),
                        fields,
                    },
                )));
            }
        }
        // Rebuild rather than reuse: a rendered directory is derived
        // state whose only input is the render, and a half-written one
        // from an interrupted run must not be installed from.
        if target.exists() {
            std::fs::remove_dir_all(&target).map_err(|error| io(&target, error))?;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| io(parent, error))?;
        }
        copy_tree(&template.tree, &target).map_err(|(path, error)| io(&path, error))?;
        let manifest_path = target.join(Manifest::FILENAME);
        std::fs::write(&manifest_path, &render.toml).map_err(|error| io(&manifest_path, error))?;
        Ok(crate::ResolvedPackage {
            group: template.group.clone(),
            name: template.name.clone(),
            version: render.version.clone(),
            source_dir: target,
        })
    }

    /// Recompute a locked version's full render and compare it with the
    /// record — the alarm of PROP-075 `#TEMPLATE-LOCK-RECORD`, raised
    /// **before any effect**.
    ///
    /// A change of the template *revision* alone, with the same produced
    /// version and the same digests, is **not** an alarm: the hash is
    /// what binds, not the revision.
    pub fn verify_recorded(
        &self,
        template: &ActiveTemplate,
        version: &semver::Version,
        record: &VersionRecord,
        recorded: &vibe_core::manifest::LockedTemplate,
    ) -> Result<FullRender, TemplateError> {
        let render = self.full(template, version, record)?;
        if render.sha256 == recorded.rendered_sha256 {
            return Ok(render);
        }
        let previous = self
            .cache(&template.group, &template.name)
            .rendered_dir(version)
            .join(Manifest::FILENAME);
        let before = std::fs::read_to_string(&previous).unwrap_or_default();
        Err(TemplateError::RenderChanged(Box::new(
            TemplateRenderChanged {
                package: template.coordinate(),
                version: version.clone(),
                recorded_revision: recorded.revision.clone(),
                computed_revision: template.revision.clone(),
                recorded_sha256: recorded.rendered_sha256.clone(),
                computed_sha256: render.sha256.clone(),
                fields: differing_digest_fields(&before, &render.toml),
            },
        )))
    }
}

/// Copy a package tree, skipping nothing: a template's recipes, icons and
/// licence files are the produced version's too.
fn copy_tree(from: &Path, to: &Path) -> Result<(), (PathBuf, std::io::Error)> {
    std::fs::create_dir_all(to).map_err(|error| (to.to_path_buf(), error))?;
    let entries = std::fs::read_dir(from).map_err(|error| (from.to_path_buf(), error))?;
    for entry in entries {
        let entry = entry.map_err(|error| (from.to_path_buf(), error))?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        let kind = entry.file_type().map_err(|error| (source.clone(), error))?;
        if kind.is_dir() {
            copy_tree(&source, &target)?;
        } else {
            std::fs::copy(&source, &target).map_err(|error| (source.clone(), error))?;
        }
    }
    Ok(())
}
