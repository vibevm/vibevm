//! Descriptor selection and bounded selected-source reads for progress grounding.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use progress_core::scope;
use vibe_core::manifest::{
    Manifest, SelectedSource, SourceDescriptor, SourceInterpretation, SourceLoading,
    select_source_rows,
};
use vibe_safefs::{Project, StableFileSnapshot};

pub(super) enum SourceAdmission {
    Legacy,
    Selected(SourceDescriptor),
    Raw,
}
#[derive(Default)]
pub(super) struct SourceDeclarations {
    manifests: BTreeMap<PathBuf, (Manifest, StableFileSnapshot)>,
}

impl SourceDeclarations {
    pub(super) fn classify(&mut self, root: &Path, rel: &Path) -> Result<SourceAdmission> {
        let full = root.join(rel);
        let mut dir = full.parent();
        while let Some(current) = dir {
            let manifest_path = current.join(Manifest::FILENAME);
            match std::fs::symlink_metadata(&manifest_path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("inspecting `{}`", manifest_path.display()));
                }
                Ok(metadata) if !metadata.is_file() => {
                    bail!(
                        "source manifest `{}` is not a regular file",
                        manifest_path.display()
                    )
                }
                Ok(_) => {
                    if !self.manifests.contains_key(current) {
                        let pinned = Project::open(current)?;
                        let snapshot = pinned
                            .read_file_snapshot_bounded(Manifest::FILENAME, 1024 * 1024)?
                            .with_context(|| {
                                format!("source manifest `{}` disappeared", manifest_path.display())
                            })?;
                        let text = std::str::from_utf8(&snapshot.bytes).with_context(|| {
                            format!("source manifest `{}` is not UTF-8", manifest_path.display())
                        })?;
                        let manifest = Manifest::parse_str(text).with_context(|| {
                            format!("parsing source manifest `{}`", manifest_path.display())
                        })?;
                        self.manifests
                            .insert(current.to_path_buf(), (manifest, snapshot));
                    }
                    let manifest = &self.manifests[current].0;
                    let relative = scope::rel_str(full.strip_prefix(current)?);
                    let (coordinate, rows, doc_package) =
                        if let Some(package) = manifest.package.as_ref() {
                            (
                                format!("{}/{}", package.group, package.name),
                                package.sources.as_slice(),
                                package.kind == vibe_core::PackageKind::Doc,
                            )
                        } else if let Some(project) = manifest.project.as_ref() {
                            if project.sources.is_empty() {
                                return Ok(SourceAdmission::Legacy);
                            }
                            let group = project.group.as_ref().context(
                                "project source descriptors require a self-coordinate group",
                            )?;
                            (
                                format!("{group}/{}", project.name),
                                project.sources.as_slice(),
                                false,
                            )
                        } else {
                            return Ok(SourceAdmission::Legacy);
                        };
                    if rows.is_empty() {
                        return Ok(SourceAdmission::Legacy);
                    }
                    return match select_source_rows(&coordinate, rows, &relative)
                        .map_err(|error| anyhow::anyhow!("source `{relative}`: {error:?}"))?
                    {
                        SelectedSource::Unclassified { .. } => Ok(SourceAdmission::Raw),
                        SelectedSource::NeedsInput {
                            declaration_indices,
                            ..
                        } => {
                            bail!(
                                "source `{relative}` has ambiguous descriptor rows {declaration_indices:?}"
                            )
                        }
                        SelectedSource::Descriptor { descriptor, .. } => {
                            if descriptor.loading() != SourceLoading::Scan {
                                return Ok(SourceAdmission::Raw);
                            }
                            match descriptor.interpretation() {
                                SourceInterpretation::VibeSpec => {
                                    Ok(SourceAdmission::Selected(descriptor.clone()))
                                }
                                SourceInterpretation::VibeDoc if doc_package => {
                                    Ok(SourceAdmission::Selected(descriptor.clone()))
                                }
                                SourceInterpretation::VibeDoc => {
                                    bail!("source `{relative}` requires a doc package")
                                }
                                _ => Ok(SourceAdmission::Raw),
                            }
                        }
                    };
                }
            }
            if current == root {
                break;
            }
            dir = current.parent();
        }
        Ok(SourceAdmission::Legacy)
    }

    pub(super) fn recheck(&self) -> Result<()> {
        for (dir, (_, before)) in &self.manifests {
            let pinned = Project::open(dir)?;
            let after = pinned
                .read_file_snapshot_bounded(Manifest::FILENAME, 1024 * 1024)?
                .with_context(|| {
                    format!("source manifest under `{}` disappeared", dir.display())
                })?;
            if &after != before {
                bail!(
                    "source manifest under `{}` changed during progress grounding",
                    dir.display()
                );
            }
        }
        Ok(())
    }
}

pub(crate) fn read_selected(
    root: &Path,
    rel: &Path,
    descriptor: &SourceDescriptor,
    expected_sha: Option<&str>,
) -> Result<(String, String)> {
    let pinned = Project::open(root)?;
    let relative = scope::rel_str(rel);
    let snapshot = pinned
        .read_file_snapshot_bounded(&relative, 16 * 1024 * 1024)?
        .with_context(|| format!("selected source `{relative}` is unavailable"))?;
    if expected_sha.is_some_and(|sha| sha != snapshot.sha256) {
        bail!("selected source `{relative}` changed after grounding");
    }
    let sha = snapshot.sha256.clone();
    let text = std::str::from_utf8(&snapshot.bytes)
        .with_context(|| format!("selected source `{relative}` is not UTF-8"))?;
    match descriptor.media() {
        "text/markdown" => Ok((text.to_owned(), sha)),
        "application/xml" => {
            let vocabulary = if descriptor.interpretation() == SourceInterpretation::VibeDoc {
                vibe_specdoc::Vocabulary::Doc
            } else {
                vibe_specdoc::Vocabulary::Spec
            };
            let document = vibe_specdoc::from_xml_with(text, vocabulary)
                .with_context(|| format!("selected XML source `{relative}` is invalid"))?;
            Ok((vibe_specdoc::to_markdown(&document), sha))
        }
        media => bail!("selected source `{relative}` has unsupported media `{media}`"),
    }
}
