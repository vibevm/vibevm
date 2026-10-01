//! Selected Checkout citation source, captured under a held project capability.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use vibe_core::manifest::{
    Manifest, SelectedSource, SourceInterpretation, SourceLoading, SourceSelector,
    select_source_rows,
};
use vibe_safefs::Project;
use vibe_spec::{Authority, SpecAddress};

use super::{CapturedMedia, CapturedSource};

/// Resolve an opt-in checkout only from authored Vibe rows. `None` means the
/// checkout has no descriptor authority and retains the old resolver route.
pub(super) fn declared_checkout_citation(
    root: &Path,
    group: &str,
    name: &str,
    address: &SpecAddress,
) -> Result<Option<(PathBuf, CapturedSource)>, String> {
    const MANIFEST_CAP: usize = 1024 * 1024;
    const SOURCE_CAP: usize = 16 * 1024 * 1024;
    let absolute = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("checkout current directory: {error}"))?
            .join(root)
    };
    let pinned =
        Project::open(&absolute).map_err(|error| format!("cannot pin checkout root: {error}"))?;
    let manifest_snapshot = pinned
        .read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP)
        .map_err(|error| format!("cannot capture checkout manifest: {error}"))?;
    let Some(manifest_snapshot) = manifest_snapshot else {
        return Ok(None);
    };
    let text = std::str::from_utf8(&manifest_snapshot.bytes)
        .map_err(|error| format!("checkout manifest is not UTF-8: {error}"))?;
    let manifest =
        Manifest::parse_str(text).map_err(|error| format!("invalid checkout manifest: {error}"))?;
    let (declared_group, declared_name, rows) = if let Some(project) = manifest.project.as_ref() {
        (
            project.group.as_ref().map(ToString::to_string),
            project.name.as_str(),
            &project.sources,
        )
    } else if let Some(package) = manifest.package.as_ref() {
        (
            Some(package.group.to_string()),
            package.name.as_str(),
            &package.sources,
        )
    } else {
        return Ok(None);
    };
    if rows.is_empty() {
        return Ok(None);
    }
    if declared_group.as_deref() != Some(group) || declared_name != name {
        return Err(format!(
            "checkout descriptor coordinate differs from cited `{group}/{name}`"
        ));
    }
    if let Authority::Package {
        version: Some(version),
        ..
    } = &address.authority
    {
        return Err(format!(
            "checkout self coordinate is unversioned; remove `@{version}`"
        ));
    }
    if rows
        .iter()
        .any(|row| matches!(row.selector(), SourceSelector::Directory(_)))
    {
        return Err("checkout directory source descriptors need a complete captured census".into());
    }

    let coordinate = format!("{group}/{name}");
    let mut chosen = None;
    let mut seen = BTreeSet::new();
    for row in rows {
        let SourceSelector::File(path) = row.selector() else {
            unreachable!("directory descriptors refused above")
        };
        if vibe_spec::canonical_doc_path(path) != address.doc_path || !seen.insert(path) {
            continue;
        }
        let selected = select_source_rows(&coordinate, rows, path)
            .map_err(|error| format!("invalid checkout source selector: {error:?}"))?;
        let descriptor = match selected {
            SelectedSource::Descriptor { descriptor, .. } => descriptor,
            SelectedSource::NeedsInput {
                declaration_indices,
                ..
            } => {
                return Err(format!(
                    "checkout cited source has ambiguous declaration rows {declaration_indices:?}"
                ));
            }
            SelectedSource::Unclassified { .. } => {
                return Err("checkout cited source is unclassified".into());
            }
        };
        if !matches!(
            descriptor.interpretation(),
            SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc
        ) {
            continue;
        }
        if descriptor.loading() != SourceLoading::Scan {
            continue;
        }
        let spec_root = vibe_core::layout::current_specs_root();
        let legacy_root = Path::new("spec");
        if !Path::new(path).starts_with(&spec_root) && !Path::new(path).starts_with(&legacy_root) {
            return Err(format!(
                "checkout Vibe source `{path}` is outside the specs root"
            ));
        }
        let media = match descriptor.media() {
            "application/xml" => CapturedMedia::Xml,
            "text/markdown" => CapturedMedia::Markdown,
            _ => {
                return Err(format!(
                    "checkout Vibe source `{path}` has unsupported media"
                ));
            }
        };
        let vocabulary = match descriptor.interpretation() {
            SourceInterpretation::VibeSpec => vibe_specdoc::doc::Vocabulary::Spec,
            SourceInterpretation::VibeDoc => vibe_specdoc::doc::Vocabulary::Doc,
            _ => unreachable!("raw interpretations skipped above"),
        };
        if chosen.replace((path.as_str(), media, vocabulary)).is_some() {
            return Err(format!(
                "two selected Vibe sources claim cited document `{}`",
                address.doc_path
            ));
        }
    }
    let (path, media, vocabulary) = chosen.ok_or_else(|| {
        format!(
            "checkout cited document `{}` has no declared scan-loaded Vibe source",
            address.doc_path
        )
    })?;
    let captured = pinned
        .read_file_snapshot_bounded(path, SOURCE_CAP)
        .map_err(|error| {
            format!("checkout cited source `{path}` cannot be captured safely: {error}")
        })?
        .ok_or_else(|| format!("checkout cited source `{path}` is unavailable"))?;
    let text = String::from_utf8(captured.bytes)
        .map_err(|error| format!("checkout cited source `{path}` is not UTF-8: {error}"))?;
    let manifest_after = pinned
        .read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP)
        .map_err(|error| format!("checkout manifest recheck: {error}"))?
        .ok_or_else(|| "checkout manifest disappeared during citation read".to_owned())?;
    if manifest_after != manifest_snapshot {
        return Err("checkout manifest changed during citation read".into());
    }
    Ok(Some((
        absolute.join(path),
        CapturedSource {
            text,
            media,
            vocabulary,
        },
    )))
}
