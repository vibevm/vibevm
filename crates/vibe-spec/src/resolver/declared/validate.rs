use super::*;
use std::collections::BTreeSet;
use vibe_core::manifest::{
    SelectedSource, SourceSelector, declarant_path, declarant_path_component,
};

pub(super) fn prepared(
    slot: &str,
    expected: &InstalledSourceExpectation,
    policy: &PackagePolicyCheckpoint,
    record: &str,
    rows: &[PreparedOutputRow],
) -> Result<(), ResolveError> {
    for path in [slot, record] {
        declarant_path(Path::new(path)).map_err(|e| fail(path, e.reason()))?;
    }
    declarant_path_component(record).map_err(|e| fail(record, e.reason()))?;
    paths::package_component(
        expected.group.as_str(),
        &expected.name,
        Some(&expected.version),
    )?;
    let expected_slot = vibe_core::machine_json_path(
        &vibe_core::layout::current_vibedeps_root()
            .join(format!("{}.{}", expected.group, expected.name))
            .join(&expected.version),
    );
    if slot != expected_slot {
        return Err(fail(
            slot,
            "slot path differs from selected coordinate/version",
        ));
    }
    let package = policy
        .package()
        .ok_or_else(|| fail(slot, "selected package manifest absent"))?;
    if !matches!(
        policy.mode(),
        PackageSourceMode::Explicit | PackageSourceMode::ExplicitEmpty
    ) {
        return Err(fail(slot, "prepared declared binding has legacy policy"));
    }
    if package.kind != expected.kind
        || package.group != expected.group
        || package.name.as_str() != expected.name
        || package.version.to_string() != expected.version
    {
        return Err(fail(
            slot,
            "package identity differs from independent selection",
        ));
    }
    let mut physical = BTreeSet::new();
    let mut originals = BTreeSet::new();
    let mut native = BTreeSet::new();
    for row in rows {
        for path in [&row.source_path, &row.output_path] {
            declarant_path(Path::new(path)).map_err(|e| fail(path, e.reason()))?;
            if vibe_core::layout::is_generated_boot_output(Path::new(path)) || path == record {
                return Err(fail(
                    path,
                    "generated/control output is not an authored source",
                ));
            }
        }
        if !physical.insert(vibe_safefs::path_identity_key(&row.output_path))
            || !originals.insert(&row.source_path)
        {
            return Err(fail(&row.output_path, "duplicate source/output row"));
        }
        if row.sha256.len() != 64
            || !row
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(fail(&row.output_path, "invalid recorded SHA-256"));
        }
        let descriptor = match package
            .select_source(&row.source_path)
            .map_err(|e| fail(&row.source_path, format!("{e:?}")))?
        {
            SelectedSource::Descriptor { descriptor, .. } => Some(descriptor),
            SelectedSource::Unclassified { .. } => None,
            SelectedSource::NeedsInput { .. } => {
                return Err(fail(&row.source_path, "ambiguous declared source"));
            }
        };
        if row.interpretation != descriptor.map(|d| d.interpretation()) {
            return Err(fail(
                &row.source_path,
                "prepared interpretation differs from policy",
            ));
        }
        let is_native = matches!(
            row.interpretation,
            Some(SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc)
        );
        if is_native {
            let source = descriptor.expect("native interpretation requires descriptor");
            let address = canonical_doc_path(&row.source_path);
            paths::source(&address, &[])?;
            let parsed = SpecAddress::parse(&format!(
                "spec://{}/{}/{address}",
                expected.group, expected.name
            ))
            .map_err(|error| fail(&row.source_path, error))?;
            if parsed.doc_path != address || !parsed.anchor.is_empty() || parsed.pinned_r.is_some()
            {
                return Err(fail(
                    &row.source_path,
                    "native logical path cannot be represented without changing URI selectors",
                ));
            }
            if row.native_address.as_deref() != Some(address.as_str()) || !native.insert(address) {
                return Err(fail(
                    &row.source_path,
                    "native address is inconsistent or ambiguous",
                ));
            }
            if row.interpretation == Some(SourceInterpretation::VibeDoc)
                && (expected.kind != PackageKind::Doc
                    || expected.materialized_format == SpecFormat::Markdown)
            {
                return Err(fail(
                    &row.source_path,
                    "unsupported documentation vocabulary/target pair",
                ));
            }
            let media = match expected.materialized_format {
                SpecFormat::Mixed => source.media(),
                SpecFormat::Markdown => "text/markdown",
                SpecFormat::Xml => "application/xml",
            };
            let path = match expected.materialized_format {
                SpecFormat::Mixed => row.source_path.clone(),
                SpecFormat::Markdown => {
                    vibe_core::machine_json_path(&Path::new(&row.source_path).with_extension("md"))
                }
                SpecFormat::Xml => {
                    vibe_core::machine_json_path(&Path::new(&row.source_path).with_extension("xml"))
                }
            };
            if row.media != media || row.output_path != path {
                return Err(fail(
                    &row.source_path,
                    "native media/output differs from selected materialization",
                ));
            }
        } else if row.native_address.is_some()
            || row.source_path != row.output_path
            || row.media
                != descriptor
                    .map(|d| d.media())
                    .unwrap_or("application/octet-stream")
        {
            return Err(fail(
                &row.source_path,
                "raw resource must preserve original path/media without native address",
            ));
        }
    }
    for descriptor in policy.package_sources().unwrap_or(&[]) {
        if let SourceSelector::File(file) = descriptor.selector()
            && !originals.contains(file)
        {
            return Err(fail(file, "exact declared resource has no output witness"));
        }
    }
    for key in &physical {
        let mut path = key.as_str();
        while let Some((parent, _)) = path.rsplit_once('/') {
            if physical.contains(parent) {
                return Err(fail(
                    key,
                    "an output file is also another output's ancestor",
                ));
            }
            path = parent;
        }
    }
    Ok(())
}
