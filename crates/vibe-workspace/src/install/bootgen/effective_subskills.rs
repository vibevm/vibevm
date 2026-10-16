//! Subskill declarations joined to original captured bytes, independently of
//! Agent Skills. This value is inert: the caller supplies admitted source
//! authority and selection; construction does not publish or modify files.
use std::collections::{BTreeMap, BTreeSet};

use vibe_core::manifest::SubskillManifest;

use crate::source_query::CapturedSourceFile;

/// Durable-only entrypoints may obtain the published selection here. Ready
/// install passes its preflight record explicitly and never calls this reader.
pub(crate) fn durable_configuration(
    root: &std::path::Path,
) -> Result<Option<vibe_core::manifest::LockedConfiguration>, crate::WorkspaceError> {
    let path = root.join(vibe_core::manifest::Lockfile::FILENAME);
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(super::io_err(&path, e)),
        Ok(meta) if !meta.file_type().is_file() => {
            return Err(crate::WorkspaceError::Io {
                path,
                reason: "non-regular lock".into(),
            });
        }
        Ok(_) => (),
    }
    vibe_core::manifest::Lockfile::read(&path)
        .map(|lock| lock.meta.configuration)
        .map_err(|e| crate::WorkspaceError::Manifest {
            path,
            source: Box::new(e),
        })
}

/// Derive effective exports from the existing exact requested/resolved record.
/// No legacy copy rows or package-directory names supply activation authority.
pub fn selected_exports(
    record: Option<&vibe_core::manifest::LockedConfiguration>,
    package: &str,
    content: &str,
) -> Result<BTreeSet<String>, String> {
    let Some(record) = record else {
        return Ok(BTreeSet::new());
    };
    let (_, resolved) = vibe_wire::behaviour::lock_configuration::decode(record)?;
    let mut rows = resolved
        .packages
        .iter()
        .filter(|row| row.package == package);
    let row = rows
        .next()
        .ok_or_else(|| format!("configuration missing selected package {package}"))?;
    if rows.next().is_some() || row.content != content {
        return Err(format!(
            "configuration capture disagrees with selected package {package}"
        ));
    }
    #[derive(serde::Deserialize)]
    struct Effects {
        subskills: BTreeSet<String>,
    }
    serde_json::from_str::<Effects>(&row.effects)
        .map(|effects| effects.subskills)
        .map_err(|error| format!("{package}: invalid selected effects: {error}"))
}

/// Read a lock-selected provider through the workspace's strict slot admission.
pub fn from_locked_package(
    root: &std::path::Path,
    lock: &vibe_core::manifest::Lockfile,
    package: &vibe_core::manifest::LockedPackage,
) -> Result<Vec<CapturedSubskill>, String> {
    let selected = selected_exports(
        lock.meta.configuration.as_ref(),
        &format!("{}/{}@{}", package.group, package.name, package.version),
        &package.content_hash.to_string(),
    )?;
    if selected.is_empty() {
        return Ok(Vec::new());
    }
    let dep = super::materialised_read::durable_row(root, package).map_err(|e| e.to_string())?;
    capture_selected_subskills(&dep.content_dir, &selected)
}

/// Add selected content to a node's actual loading lane. Lazy-push exposes
/// only its captured declaration (description and content references); the
/// content itself remains behind MCP read_subskill. Lazy-pull has no boot row.
pub(crate) fn append_loading(
    root: &std::path::Path,
    resolution: &[super::ResolvedDep],
    dependencies: &[crate::boot::DependencyBoot],
    record: Option<&vibe_core::manifest::LockedConfiguration>,
    effective: &mut crate::boot::EffectiveBoot,
) -> Result<(), crate::WorkspaceError> {
    use crate::boot::{BootBand, BootEntry, BootProvenance};
    use vibe_core::manifest::{DeliveryMode, LinkType, PackageFormat};
    for dep in resolution {
        if !dependencies
            .iter()
            .any(|d| d.group == dep.group && d.name == dep.name)
        {
            continue;
        }
        let package = format!("{}/{}@{}", dep.group, dep.name, dep.version);
        let fail = |reason| crate::WorkspaceError::SlotLifecycle {
            phase: "effective-subskills",
            package: package.clone(),
            reason,
        };
        let selected = selected_exports(
            record,
            &package,
            &dep.source_hash
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
        .map_err(&fail)?;
        if selected.is_empty() {
            continue;
        }
        let slot = if dep
            .manifest
            .package
            .as_ref()
            .is_some_and(|p| p.materialization.is_in_place())
        {
            crate::vibedeps::in_place_slot_rel_path(&dep.group, &dep.name)
        } else {
            crate::vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version)
        };
        for sub in capture_selected_subskills(&root.join(&slot), &selected).map_err(&fail)? {
            let provenance = if sub.declaration.subskill.delivery == DeliveryMode::LazyPush {
                BootProvenance::SubskillOffer {
                    group: dep.group.clone(),
                    name: dep.name.clone(),
                    subskill: sub.declaration.subskill.path.clone(),
                    description: sub
                        .declaration
                        .subskill
                        .description
                        .clone()
                        .unwrap_or_default(),
                }
            } else {
                BootProvenance::Dependency {
                    group: dep.group.clone(),
                    name: dep.name.clone(),
                }
            };
            let (paths, link) = match sub.declaration.subskill.delivery {
                DeliveryMode::Eager => (
                    sub.content.keys().cloned().collect::<Vec<_>>(),
                    LinkType::Static,
                ),
                DeliveryMode::LazyPush => (vec![sub.definition_path], LinkType::Dynamic),
                DeliveryMode::LazyPull => continue,
            };
            for path in paths {
                effective.entries.push(BootEntry {
                    path: format!("{slot}/{path}"),
                    band: BootBand::Dependency,
                    link,
                    when: None,
                    origin: format!("{}/{}", dep.group, dep.name),
                    provenance: provenance.clone(),
                    use_ref: false,
                    format: PackageFormat::Simple,
                    unit_substituted: false,
                    elided: false,
                });
            }
        }
    }
    effective.entries.sort_by_key(|entry| entry.band);
    Ok(())
}

/// Attach explicit declaration evidence to the existing selected-slot carriage.
pub(crate) fn bind_selected_slots(
    witnesses: &mut [crate::boot_artifacts::SelectedSlotWitness],
    record: Option<&vibe_core::manifest::LockedConfiguration>,
) -> Result<(), crate::WorkspaceError> {
    for witness in witnesses {
        let package = format!("{}/{}@{}", witness.group, witness.name, witness.version);
        let fail = |reason| crate::WorkspaceError::SlotLifecycle {
            phase: "effective-subskills",
            package: package.clone(),
            reason,
        };
        let selected = selected_exports(
            record,
            &package,
            &witness
                .source_hash
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
        .map_err(&fail)?;
        witness.subskills = capture_selected_subskills(&witness.root, &selected).map_err(fail)?;
    }
    Ok(())
}

/// Capture selected declarations from the caller's admitted package root.
/// Path reads use the shared no-follow filesystem capability. The caller
/// retains responsibility for binding this root to the selected provider.
pub fn capture_selected_subskills(
    root: &std::path::Path,
    selected: &BTreeSet<String>,
) -> Result<Vec<CapturedSubskill>, String> {
    if selected.is_empty() {
        return Ok(Vec::new());
    }
    let project = vibe_safefs::Project::open(root).map_err(|e| e.to_string())?;
    let mut files = BTreeMap::new();
    for name in selected {
        vibe_safefs::split_relative(name).map_err(|e| e.to_string())?;
        let path = format!("subskills/{name}/{}", SubskillManifest::FILENAME);
        let snapshot = project
            .read_file_snapshot_bounded(&path, 1024 * 1024)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("missing captured declaration: {path}"))?;
        let declaration: SubskillManifest =
            toml::from_str(std::str::from_utf8(&snapshot.bytes).map_err(|e| e.to_string())?)
                .map_err(|e| format!("{path}: {e}"))?;
        files.insert(
            path.clone(),
            CapturedSourceFile::new(path, snapshot).map_err(|e| e.to_string())?,
        );
        for relative in &declaration.content.files_written {
            let relative = relative.to_str().ok_or("non-UTF8 subskill content path")?;
            vibe_safefs::split_relative(relative).map_err(|e| e.to_string())?;
            let path = format!("subskills/{name}/{relative}");
            let snapshot = project
                .read_file_snapshot_bounded(&path, 16 * 1024 * 1024)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("missing captured content: {path}"))?;
            files.insert(
                path.clone(),
                CapturedSourceFile::new(path, snapshot).map_err(|e| e.to_string())?,
            );
        }
    }
    select_captured_subskills(&files.into_values().collect::<Vec<_>>(), selected)
}

/// A selected declaration and its exact original content, retained together.
#[derive(Debug, Clone)]
pub struct CapturedSubskill {
    pub declaration: SubskillManifest,
    pub definition_path: String,
    pub definition_digest: String,
    /// Package-relative source paths and captured bytes (never project paths).
    pub content: BTreeMap<String, Vec<u8>>,
}

/// Validate the complete child catalogue before selecting contributions.
/// Provider/content identity remains with the caller's admitted capture.
pub fn select_captured_subskills(
    files: &[CapturedSourceFile],
    selected: &BTreeSet<String>,
) -> Result<Vec<CapturedSubskill>, String> {
    let mut inventory = BTreeMap::new();
    for file in files {
        if inventory
            .insert(file.relative_path(), file.snapshot())
            .is_some()
        {
            return Err(format!("duplicate captured path: {}", file.relative_path()));
        }
    }
    let mut catalogue = BTreeMap::new();
    for (path, snapshot) in &inventory {
        if !path.starts_with("subskills/")
            || !path.ends_with(&format!("/{}", SubskillManifest::FILENAME))
        {
            continue;
        }
        let text = std::str::from_utf8(&snapshot.bytes).map_err(|e| format!("{path}: {e}"))?;
        let declaration: SubskillManifest =
            toml::from_str(text).map_err(|e| format!("{path}: {e}"))?;
        let name = &declaration.subskill.path;
        vibe_safefs::split_relative(name).map_err(|e| format!("{path}: {e}"))?;
        if *path != format!("subskills/{name}/{}", SubskillManifest::FILENAME) {
            return Err(format!(
                "{path}: declared subskill path disagrees with capture"
            ));
        }
        let findings = declaration.validation_findings();
        if !findings.is_empty() {
            return Err(format!("{path}: {}", findings.join("; ")));
        }
        if declaration.subskill.delivery.requires_description()
            && declaration
                .subskill
                .description
                .as_deref()
                .is_none_or(|s| s.trim().is_empty())
        {
            return Err(format!(
                "{path}: lazy delivery requires a nonempty description"
            ));
        }
        let mut content = BTreeMap::new();
        for relative in &declaration.content.files_written {
            let relative = relative
                .to_str()
                .ok_or_else(|| format!("{path}: non-UTF8 content path"))?;
            vibe_safefs::split_relative(relative).map_err(|e| format!("{path}: {e}"))?;
            let source = format!("subskills/{name}/{relative}");
            let bytes = inventory
                .get(source.as_str())
                .ok_or_else(|| format!("{path}: missing captured content {source}"))?;
            if content
                .insert(source.clone(), bytes.bytes.clone())
                .is_some()
            {
                return Err(format!("{path}: duplicate content {source}"));
            }
        }
        let name = name.clone();
        catalogue.insert(
            name,
            CapturedSubskill {
                declaration,
                definition_path: (*path).into(),
                definition_digest: snapshot.sha256.clone(),
                content,
            },
        );
    }
    selected
        .iter()
        .map(|name| {
            catalogue
                .remove(name)
                .ok_or_else(|| format!("selected subskill has no captured declaration: {name}"))
        })
        .collect()
}

#[cfg(test)]
mod m_05_d_2_cases;
