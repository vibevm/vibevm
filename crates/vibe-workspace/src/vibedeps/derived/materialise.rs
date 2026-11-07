//! Source-admitted preparation and publication of one transformed slot.
use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialise_with_spec_format_report(
    workspace_root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    content_src: &Path,
    mode: CopyMode,
    spec_format: SpecFormat,
    source_hash: &ContentHash,
    known: Option<&vibe_core::manifest::Manifest>,
) -> Result<MaterialiseReport, WorkspaceError> {
    let policy =
        super::super::source_policy::SourcePolicy::observe(content_src, spec_format, known)?;
    if policy.is_declared() {
        let declared = policy
            .manifest()
            .and_then(|manifest| manifest.package.as_ref())
            .ok_or_else(|| {
                super::super::source_policy::refused(
                    content_src,
                    "declared package metadata missing",
                )
            })?;
        if &declared.group != group || declared.name != name || &declared.version != version {
            return Err(super::super::source_policy::refused(
                content_src,
                "declared source identity differs from the requested materialization slot",
            ));
        }
        let package = format!("{group}/{name}");
        let (overlay, overlay_hash) = if spec_format == SpecFormat::Mixed {
            (PackageOverlay::default(), None)
        } else {
            load_overlay(workspace_root, &package)?
        };
        let prepared = super::declared::prepare(&policy, spec_format, mode, &overlay)?;
        let record = SlotRecord {
            schema: super::super::SLOT_RECORD_SCHEMA,
            source_hash: source_hash.clone(),
            spec_format,
            converter_recipe: (spec_format != SpecFormat::Mixed)
                .then(|| CONVERTER_RECIPE.to_owned()),
            overlay_hash,
            derived_hash: if spec_format == SpecFormat::Mixed {
                None
            } else {
                Some(compute_prepared_payload_hash(&prepared.incoming)?)
            },
            files: prepared.rows,
        };
        policy.recheck()?;
        return super::super::slot_diff::reconcile_declared_slot(
            &slot_abs_path(workspace_root, group, name, version),
            &prepared.incoming,
            &record,
        );
    }
    if spec_format == SpecFormat::Mixed {
        return super::super::materialise_observed_report(
            workspace_root,
            group,
            name,
            version,
            &policy,
            mode,
            source_hash,
        );
    }
    let package = format!("{group}/{name}");
    let (overlay, overlay_hash) = load_overlay(workspace_root, &package)?;
    let slot = slot_abs_path(workspace_root, group, name, version);
    if !content_src.is_dir() {
        return Err(WorkspaceError::SpecMaterialization {
            path: content_src.to_path_buf(),
            reason: "source content tree does not exist or is not a directory".to_string(),
        });
    }
    super::super::refuse_reserved_source_record(content_src)?;
    let mut sources = Vec::new();
    collect_files(content_src, content_src, &mut sources)?;
    sources.sort_by(|a, b| a.0.cmp(&b.0));

    let mut outputs = BTreeSet::new();
    let mut incoming = Vec::new();
    let mut files = Vec::new();
    for (rel, source) in sources {
        let transformed = transform_file(&source, &rel, spec_format, &package, &overlay)?;
        let output_rel = transformed
            .as_ref()
            .map_or_else(|| rel.clone(), |(path, _)| path.clone());
        let output_wire = path_to_slash(&output_rel);
        if !outputs.insert(output_wire.clone()) {
            return Err(WorkspaceError::SpecMaterialization {
                path: slot.join(&output_rel),
                reason: format!(
                    "more than one source maps to `{output_wire}` under {} output",
                    spec_format.as_str()
                ),
            });
        }
        let (prepared, disposition) = if let Some((_, bytes)) = transformed {
            (
                PreparedSlotFile::from_bytes(output_rel.clone(), bytes),
                SlotFileDisposition::Converted,
            )
        } else {
            (
                if let Some(observation) = policy.captured(&rel) {
                    PreparedSlotFile::from_observation(
                        output_rel.clone(),
                        source,
                        mode,
                        observation,
                    )
                } else {
                    PreparedSlotFile::from_source(output_rel.clone(), source, mode)?
                },
                SlotFileDisposition::Copied,
            )
        };
        files.push(SlotFile {
            path: output_wire,
            sha256: prepared.sha256().to_string(),
            source: Some(path_to_slash(&rel)),
            disposition: Some(disposition),
        });
        incoming.push(prepared);
    }

    // One canonical order for the payload vector and the persisted rows:
    // the shared flattened forward-slash sorter — exactly the order
    // `validate_file_rows` enforces and `compute_recorded_payload_hash`
    // consumes. Host `Path` order compares component-wise and diverges from
    // it (e.g. `a/x` sorts before `a.md` component-wise but after it
    // flattened), which would desync `derived_hash` from verification.
    sort_prepared_files(&mut incoming);
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let derived_hash = compute_prepared_payload_hash(&incoming)?;
    let record = SlotRecord {
        schema: super::super::SLOT_RECORD_SCHEMA,
        source_hash: source_hash.clone(),
        spec_format,
        converter_recipe: Some(CONVERTER_RECIPE.to_string()),
        overlay_hash,
        derived_hash: Some(derived_hash),
        files,
    };
    policy.validate_prepared(&incoming)?;
    reconcile_slot(&slot, &incoming, &record)
}

pub(in crate::vibedeps) fn load_overlay(
    workspace_root: &Path,
    package: &str,
) -> Result<(PackageOverlay, Option<ContentHash>), WorkspaceError> {
    let registry =
        Registry::load(workspace_root).map_err(|error| WorkspaceError::SpecMaterialization {
            path: workspace_root.join(layout::current_vibefacts_root()),
            reason: format!("package adoption registry cannot be loaded: {error}"),
        })?;
    let overlay = registry.package_overlay(package);
    let overlay_hash = overlay_file_hash(workspace_root, package)
        .map(|hash| {
            ContentHash::parse(&hash).map_err(|error| WorkspaceError::SpecMaterialization {
                path: workspace_root.join(layout::current_vibefacts_root()),
                reason: format!("package overlay hash is invalid: {error}"),
            })
        })
        .transpose()?;
    Ok((overlay, overlay_hash))
}
