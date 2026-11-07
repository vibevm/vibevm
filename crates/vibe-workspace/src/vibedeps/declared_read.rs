//! The sole slot-record owner prepares data for lower installed-source reads.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-INSTALLED-READ-LAW");
use super::{
    CONVERTER_RECIPE, SLOT_RECORD_FILENAME, SlotFileDisposition, read_slot_record_bytes,
    sha256_bytes, slot_rel_path,
};
use crate::{WorkspaceError, install::ResolvedDep};
use std::path::Path;
use std::sync::Arc;
use vibe_core::manifest::{Manifest, SelectedSource, SourceInterpretation, SpecFormat};
use vibe_safefs::{Pinned, Project};
use vibe_spec::source_policy::observe_package_policy_in;
use vibe_spec::{InstalledSourceExpectation, PreparedInstalledSource, PreparedOutputRow};

fn refused(path: &Path, reason: impl std::fmt::Display) -> WorkspaceError {
    WorkspaceError::SpecMaterialization {
        path: path.to_owned(),
        reason: format!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-INSTALLED-READ-LAW: {reason}; fix surface: selected slot, manifest, record and independent materialized input format"
        ),
    }
}

/// Prepare one selected slot through the supplied workspace capability. An
/// unrecorded legacy slot retains compatibility, not verified original identity.
pub(crate) fn prepare_declared_slot_read(
    project: &Project,
    slot: &Pinned,
    slot_relative: &str,
    selected: &ResolvedDep,
    expected_materialized_format: Option<SpecFormat>,
) -> Result<Option<PreparedInstalledSource>, WorkspaceError> {
    let fail = |reason: String| refused(slot.path(), reason);
    vibe_core::manifest::declarant_path(Path::new(slot_relative))
        .map_err(|e| fail(e.reason().into()))?;
    if slot_relative != slot_rel_path(&selected.group, &selected.name, &selected.version)
        || project.root_path().join(slot_relative) != slot.path()
    {
        return Err(fail(
            "held slot differs from selected coordinate/version".into(),
        ));
    }
    let policy = observe_package_policy_in(project, slot).map_err(|e| fail(e.to_string()))?;
    policy
        .recheck_in(project, slot)
        .map_err(|e| fail(e.to_string()))?;
    let expected_package = selected.manifest.package.as_ref();
    let observed_package = policy.package();
    let expected_declared = expected_package.is_some_and(|p| p.has_explicit_source_policy());
    let observed_declared = observed_package.is_some_and(|p| p.has_explicit_source_policy());
    if expected_declared != observed_declared {
        return Err(fail(
            "expected and observed declaration presence differ".into(),
        ));
    }
    if let Some(package) = observed_package
        && (package.kind != selected.kind
            || package.group != selected.group
            || package.name.as_str() != selected.name
            || package.version != selected.version)
    {
        return Err(fail(
            "observed package identity differs from independent selection".into(),
        ));
    }
    if observed_declared && policy.manifest() != Some(&selected.manifest) {
        return Err(fail(
            "observed full manifest differs from independent selected intent".into(),
        ));
    }
    let record_bytes = project
        .read_source_bytes_in(slot, SLOT_RECORD_FILENAME)
        .map_err(|e| fail(e.to_string()))?;
    let Some(record_bytes) = record_bytes else {
        if expected_declared {
            return Err(fail("declared installed slot has no record".into()));
        }
        policy
            .recheck_in(project, slot)
            .map_err(|e| fail(e.to_string()))?;
        return Ok(None);
    };
    let record =
        read_slot_record_bytes(&slot.join(SLOT_RECORD_FILENAME), &record_bytes).map_err(&fail)?;
    // This coherence check MUST precede legacy downgrade: deleting declarations
    // cannot hide a record that still witnesses a different manifest.
    let manifest_bytes = policy
        .manifest_bytes()
        .ok_or_else(|| fail("recorded slot has no package manifest".into()))?;
    let manifest_row = record
        .files
        .iter()
        .find(|row| row.path == Manifest::FILENAME)
        .ok_or_else(|| fail("record omits package manifest witness".into()))?;
    if sha256_bytes(manifest_bytes) != manifest_row.sha256 {
        return Err(fail(
            "recorded manifest digest differs from observed manifest".into(),
        ));
    }
    if let Some(expected) = &selected.source_hash
        && &record.source_hash != expected
    {
        return Err(fail(
            "record source hash differs from independent selection".into(),
        ));
    }
    if !observed_declared {
        policy
            .recheck_in(project, slot)
            .map_err(|e| fail(e.to_string()))?;
        if project
            .read_source_bytes_in(slot, SLOT_RECORD_FILENAME)
            .map_err(|e| fail(e.to_string()))?
            .as_deref()
            != Some(record_bytes.as_slice())
        {
            return Err(fail("slot record changed during preparation".into()));
        }
        return Ok(None);
    }
    let source_hash = selected
        .source_hash
        .clone()
        .ok_or_else(|| fail("declared slot requires independently selected source hash".into()))?;
    let materialized_format = expected_materialized_format.ok_or_else(|| {
        fail("declared slot requires independently selected materialized input format".into())
    })?;
    if record.spec_format != materialized_format {
        return Err(fail(
            "record materialized format differs from independent selection".into(),
        ));
    }
    if materialized_format != SpecFormat::Mixed
        && record.converter_recipe.as_deref() != Some(CONVERTER_RECIPE)
    {
        return Err(fail("unsupported declared converter recipe".into()));
    }
    let package =
        observed_package.ok_or_else(|| fail("declared package manifest is absent".into()))?;
    let mut rows = Vec::new();
    for output in &record.files {
        let source = if materialized_format == SpecFormat::Mixed {
            output.path.as_str()
        } else {
            output
                .source
                .as_deref()
                .ok_or_else(|| fail("transformed row lacks original path".into()))?
        };
        let descriptor = match package
            .select_source(source)
            .map_err(|e| fail(format!("invalid declared source: {e:?}")))?
        {
            SelectedSource::Descriptor { descriptor, .. } => Some(descriptor),
            SelectedSource::Unclassified { .. } => None,
            SelectedSource::NeedsInput { .. } => {
                return Err(fail(format!("ambiguous declared resource `{source}`")));
            }
        };
        let interpretation = descriptor.map(|d| d.interpretation());
        let native = matches!(
            interpretation,
            Some(SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc)
        );
        let mut media = descriptor
            .map(|d| d.media())
            .unwrap_or("application/octet-stream")
            .to_owned();
        if materialized_format != SpecFormat::Mixed {
            match output.disposition {
                Some(SlotFileDisposition::Copied) if output.path == source => {}
                Some(SlotFileDisposition::Converted) if native => {
                    let extension = match materialized_format {
                        SpecFormat::Xml => "xml",
                        SpecFormat::Markdown => "md",
                        SpecFormat::Mixed => unreachable!(),
                    };
                    if Path::new(source).with_extension(extension) != Path::new(&output.path) {
                        return Err(fail(format!(
                            "converted output `{}` does not match original path",
                            output.path
                        )));
                    }
                    media = if materialized_format == SpecFormat::Xml {
                        "application/xml"
                    } else {
                        "text/markdown"
                    }
                    .into();
                }
                _ => {
                    return Err(fail(format!(
                        "invalid source/output disposition for `{source}`"
                    )));
                }
            }
        }
        let bytes = project
            .read_source_bytes_in(slot, &output.path)
            .map_err(|e| fail(e.to_string()))?
            .ok_or_else(|| fail(format!("recorded output `{}` is absent", output.path)))?;
        if sha256_bytes(&bytes) != output.sha256 {
            return Err(fail(format!(
                "recorded output digest changed for `{}`",
                output.path
            )));
        }
        if let Some(interpretation) = interpretation.filter(|_| native) {
            vibe_spec::project_declared_native(&bytes, &media, interpretation).map_err(&fail)?;
        }
        rows.push(PreparedOutputRow {
            source_path: source.into(),
            output_path: output.path.clone(),
            interpretation,
            media,
            sha256: output.sha256.clone(),
            native_address: native.then(|| vibe_spec::canonical_doc_path(source)),
        });
    }
    policy
        .recheck_in(project, slot)
        .map_err(|e| fail(e.to_string()))?;
    if project
        .read_source_bytes_in(slot, SLOT_RECORD_FILENAME)
        .map_err(|e| fail(e.to_string()))?
        .as_deref()
        != Some(record_bytes.as_slice())
    {
        return Err(fail("slot record changed during preparation".into()));
    }
    let binding = PreparedInstalledSource::from_host_checked(
        slot_relative.into(),
        InstalledSourceExpectation {
            kind: selected.kind,
            group: selected.group.clone(),
            name: selected.name.clone(),
            version: selected.version.to_string(),
            source_hash,
            materialized_format,
        },
        policy,
        SLOT_RECORD_FILENAME.into(),
        Arc::from(record_bytes),
        rows,
    )
    .map_err(|e| fail(e.to_string()))?;
    Ok(Some(binding))
}

#[cfg(test)]
mod tests;
