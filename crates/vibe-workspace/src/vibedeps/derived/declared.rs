//! Pure decisions over captured declared payloads, shared by preflight/publish.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DECLARED-MATERIALIZATION");
use super::super::source_policy::{SourcePolicy, refused};
use super::*;
use std::collections::BTreeMap;
use vibe_core::manifest::{SelectedSource, SourceInterpretation};

pub(in crate::vibedeps) struct DeclaredPreparation {
    pub(in crate::vibedeps) incoming: Vec<PreparedSlotFile>,
    pub(in crate::vibedeps) rows: Vec<SlotFile>,
}

pub(in crate::vibedeps) fn prepare(
    policy: &SourcePolicy,
    format: SpecFormat,
    mode: CopyMode,
    overlay: &PackageOverlay,
) -> Result<DeclaredPreparation, WorkspaceError> {
    let package = policy
        .manifest()
        .and_then(|manifest| manifest.package.as_ref())
        .ok_or_else(|| refused(policy.root(), "declared package metadata missing"))?;
    let mut outputs = BTreeMap::new();
    let mut addresses = BTreeMap::new();
    let mut native_sources = BTreeSet::new();
    let mut incoming = Vec::new();
    let mut rows = Vec::new();
    for (original, observation) in policy.inventory() {
        let relative = Path::new(original);
        let descriptor = match package
            .select_source(original)
            .map_err(|error| refused(relative, format!("source selection failed: {error:?}")))?
        {
            SelectedSource::Descriptor { descriptor, .. } => Some(descriptor),
            SelectedSource::Unclassified { .. } => None,
            SelectedSource::NeedsInput {
                declaration_indices,
                ..
            } => {
                return Err(refused(
                    relative,
                    format!("source declaration conflict at rows {declaration_indices:?}"),
                ));
            }
        };
        let native = descriptor.filter(|descriptor| {
            matches!(
                descriptor.interpretation(),
                SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc
            )
        });
        let converted = if let Some(descriptor) = native {
            native_sources.insert(original.to_owned());
            if original == vibe_core::manifest::Manifest::FILENAME {
                return Err(refused(
                    relative,
                    "the captured manifest must retain its original metadata path/bytes",
                ));
            }
            let address = vibe_spec::canonical_doc_path(original);
            let address_text = format!("spec://{}/{}/{}", package.group, package.name, address);
            let parsed = vibe_spec::SpecAddress::parse(&address_text).map_err(|error| {
                refused(
                    relative,
                    format!("original native address is unsupported: {error}"),
                )
            })?;
            if parsed.doc_path != address || !parsed.anchor.is_empty() || parsed.pinned_r.is_some()
            {
                return Err(refused(
                    relative,
                    "original native path cannot be represented by the existing logical address grammar",
                ));
            }
            reserve(
                &mut addresses,
                &address,
                original,
                "native address",
                policy.root(),
            )?;
            let text = std::str::from_utf8(observation.bytes())
                .map_err(|_| refused(relative, "selected native source is not UTF-8"))?;
            let mut doc = match (descriptor.interpretation(), descriptor.media()) {
                (SourceInterpretation::VibeSpec, "text/markdown") => vibe_specdoc::from_markdown(text),
                (SourceInterpretation::VibeSpec, "application/xml") => vibe_specdoc::from_xml(text),
                (SourceInterpretation::VibeDoc, "application/xml") if package.kind == vibe_core::PackageKind::Doc => {
                    if format == SpecFormat::Markdown { return Err(refused(relative, "VibeDoc Markdown projection is one-way and is not a native Doc representation")); }
                    vibe_specdoc::from_xml_with(text, vibe_specdoc::doc::Vocabulary::Doc)
                },
                _ => return Err(refused(relative, "selected native media/vocabulary/package kind is unsupported")),
            }.map_err(|error| refused(relative, format!("selected native source cannot enter the pivot: {error}")))?;
            let prefix = format!("spec://{}/{}/{}#", package.group, package.name, address);
            let overlay_applies = overlay.contains_document(&prefix);
            let target = match format {
                SpecFormat::Mixed => None,
                SpecFormat::Xml => Some(("xml", "application/xml")),
                SpecFormat::Markdown => Some(("md", "text/markdown")),
            };
            if let Some((suffix, media)) = target
                && (descriptor.media() != media
                    || relative.extension().and_then(|value| value.to_str()) != Some(suffix)
                    || overlay_applies)
            {
                super::apply_overlay(&mut doc, &prefix, overlay);
                let output = relative.with_extension(suffix);
                if vibe_core::layout::is_reserved_boot_source(&output) {
                    return Err(refused(
                        &output,
                        "converted output collides with a reserved generated boot destination",
                    ));
                }
                let bytes = if format == SpecFormat::Xml {
                    vibe_specdoc::to_xml(&doc)
                } else {
                    vibe_specdoc::to_markdown(&doc)
                };
                Some((output, bytes.into_bytes()))
            } else {
                None
            }
        } else {
            // Not a native spec source, so nothing enters the pivot — but an
            // instruction file's managed boot block is still neutralised on
            // the way into the slot, by the same rule and the same function
            // the other two walks use (PROP-076 `#SYS-NO-RECURSION`).
            super::super::instruction_block::for_file(relative, observation.bytes())
                .map(|neutralised| (relative.to_path_buf(), neutralised))
        };
        let (file, disposition) = if let Some((output, bytes)) = converted {
            (
                PreparedSlotFile::from_bytes(output, bytes),
                SlotFileDisposition::Converted,
            )
        } else {
            (
                PreparedSlotFile::from_observation(
                    relative.to_owned(),
                    policy.root().join(relative),
                    mode,
                    observation.clone(),
                ),
                SlotFileDisposition::Copied,
            )
        };
        let output = file.path_wire();
        if output.eq_ignore_ascii_case(SLOT_RECORD_FILENAME) {
            return Err(refused(
                Path::new(&output),
                "payload collides with reserved destination .vibe-slot.toml",
            ));
        }
        reserve(
            &mut outputs,
            &output,
            original,
            "output path",
            policy.root(),
        )?;
        rows.push(SlotFile {
            path: output,
            sha256: file.sha256().to_owned(),
            source: (format != SpecFormat::Mixed).then(|| original.to_owned()),
            disposition: (format != SpecFormat::Mixed).then_some(disposition),
        });
        incoming.push(file);
    }
    if let Some(snippet) = policy
        .manifest()
        .and_then(|manifest| manifest.boot_snippet.as_ref())
    {
        for path in std::iter::once(&snippet.source)
            .chain(snippet.fragments.iter().map(|fragment| &fragment.source))
            .filter(|path| !path.as_os_str().is_empty())
        {
            let original = path
                .to_str()
                .ok_or_else(|| refused(path, "declared boot input path is not UTF-8"))?
                .to_owned();
            if !native_sources.contains(&original) {
                return Err(refused(
                    path,
                    "classified boot input must name a present selected native source in the published payload",
                ));
            }
        }
    }
    for row in &rows {
        for parent in Path::new(&row.path)
            .ancestors()
            .skip(1)
            .filter(|path| !path.as_os_str().is_empty())
        {
            if let Some(first) =
                outputs.get(&vibe_safefs::path_identity_key(&path_to_slash(parent)))
            {
                return Err(refused(
                    &policy.root().join(&row.path),
                    format!(
                        "source `{first}` maps a file onto an ancestor of output `{}`",
                        row.path
                    ),
                ));
            }
        }
    }
    sort_prepared_files(&mut incoming);
    rows.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(DeclaredPreparation { incoming, rows })
}

fn reserve(
    used: &mut BTreeMap<String, String>,
    value: &str,
    original: &str,
    kind: &str,
    root: &Path,
) -> Result<(), WorkspaceError> {
    let key = vibe_safefs::path_identity_key(value);
    if let Some(first) = used.insert(key, original.to_owned()) {
        return Err(refused(
            &root.join(value),
            format!(
                "sources `{first}` and `{original}` collide at {kind} `{value}` under the incumbent portable alias key"
            ),
        ));
    }
    Ok(())
}
