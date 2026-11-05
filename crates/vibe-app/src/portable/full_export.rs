//! One locked portable snapshot; selected jobs and carried ownership are distinct.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY");

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::manifest::RequirementScope;
use vibe_wire::generated::{application::e2::apps as full, shared as w};

use super::{PortableExport, arrangement, document::PortableDocument, row_of};
use crate::catalog::UserLaunchers;
use crate::index::{Index, Installation, Mark, StepOutcome};
use crate::operations::{import, pending};
use crate::steps::StepsFile;
use crate::{AppError, SettingsRoot};

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
pub(super) fn export(
    settings: &SettingsRoot,
    request: &PortableExport,
    force_full: bool,
) -> Result<PortableDocument, AppError> {
    let _held = settings.lock()?;
    let index = Index::load(settings)?;
    let selected: Vec<_> = index
        .installations
        .values()
        .filter(|r| r.lifecycle.superseded_by.is_none())
        .filter(|r| request.all || !r.lifecycle.hidden)
        .filter(|r| request.all || r.lifecycle.mark == Mark::Manual)
        .collect();
    let roots: Vec<_> = selected
        .iter()
        .filter(|r| r.lifecycle.mark == Mark::Manual)
        .map(|r| r.package.key())
        .collect();
    // Collect gaps first: automatic legacy mode historically does not require
    // a complete closure. Known rich rows, including --all orphan Auto data,
    // select full mode before its stronger admission is applied.
    let mut included: BTreeSet<_> = selected.iter().map(|r| r.package.key()).collect();
    let mut queue: Vec<_> = included.iter().cloned().collect();
    let mut missing = BTreeSet::new();
    while let Some(key) = queue.pop() {
        let Some(record) = index.installations.get(&key) else {
            missing.insert(key);
            continue;
        };
        for edge in &record.lifecycle.requires {
            if included.insert(edge.key.clone()) {
                queue.push(edge.key.clone());
            }
        }
    }
    let rich = included
        .iter()
        .filter_map(|key| index.installations.get(key))
        .any(|r| r.configuration.is_some() || !r.secret_origins.is_empty());
    let mut legacy = vibe_wire::generated::apps_manifest::AppsManifest {
        protocol: super::APPS_PROTOCOL.into(),
        exported: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        home: if settings.is_project() {
            vibe_wire::generated::apps_manifest::AppsManifestHome::Project
        } else {
            vibe_wire::generated::apps_manifest::AppsManifestHome::User
        },
        platform: crate::detected_platform()
            .unwrap_or(std::env::consts::OS)
            .into(),
        arch: crate::detected_arch().into(),
        application: selected.into_iter().map(row_of).collect(),
        collection: Vec::new(),
        pin: Vec::new(),
        launcher: Vec::new(),
    };
    if request.arrangement {
        arrangement::export_into(&UserLaunchers::load(settings)?, &mut legacy);
    }
    if !force_full && !rich {
        return Ok(PortableDocument::Legacy(legacy));
    }
    if !missing.is_empty() {
        return Err(refuse(format!(
            "required installation records are missing: {}",
            missing.into_iter().collect::<Vec<_>>().join(", ")
        )));
    }
    for key in &included {
        super::document::exact_key(key).map_err(refuse)?;
    }
    ready(settings, &index, &included, request.arrangement)?;
    let application = included
        .iter()
        .map(|key| {
            index.installations.get(key).map(full_row).ok_or_else(|| {
                refuse(format!(
                    "required installation `{key}` disappeared from the snapshot"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let set = full::AppsManifest {
        protocol: "vibe-apps/2".into(),
        exported: legacy.exported,
        home: match legacy.home {
            vibe_wire::generated::apps_manifest::AppsManifestHome::Project => {
                full::AppsHome::Project
            }
            vibe_wire::generated::apps_manifest::AppsManifestHome::User => full::AppsHome::User,
        },
        platform: legacy.platform,
        arch: legacy.arch,
        roots,
        application,
        collection: legacy
            .collection
            .into_iter()
            .map(|r| full::Collection {
                name: r.name,
                launchers: r.launchers,
            })
            .collect(),
        pin: legacy
            .pin
            .into_iter()
            .map(|r| full::Pin {
                target: match r.target {
                    vibe_wire::generated::apps_manifest::PinTarget::Taskbar => {
                        full::PinTarget::Taskbar
                    }
                    vibe_wire::generated::apps_manifest::PinTarget::Favorites => {
                        full::PinTarget::Favorites
                    }
                },
                launchers: r.launchers,
            })
            .collect(),
        launcher: legacy
            .launcher
            .into_iter()
            .map(|r| full::Launcher {
                key: r.key,
                name: r.name,
                hidden: r.hidden,
                icon: r.icon,
            })
            .collect(),
    };
    super::document::validate_full(&set)
        .map_err(|reason| super::document::invalid("export", &reason))?;
    Ok(PortableDocument::Full(set))
}

fn full_row(record: &Installation) -> w::PortableApplication {
    w::PortableApplication {
        key: record.package.key(),
        installer: record.option.clone(),
        platform: record.platform.clone(),
        arch: record.arch.clone(),
        mark: match record.lifecycle.mark {
            Mark::Manual => w::PortableApplicationMark::Manual,
            Mark::Auto => w::PortableApplicationMark::Auto,
        },
        held: record.lifecycle.held,
        hidden: record.lifecycle.hidden,
        license_accepted: record.license.is_some(),
        configuration: match &record.configuration {
            Some(record) => w::PortableConfigurationState::Configured(Box::new(
                w::PortableConfigurationStateConfigured {
                    history: crate::configuration::portable::encode(record),
                },
            )),
            None => w::PortableConfigurationState::LegacyEffective(Box::new(
                w::PortableConfigurationStateLegacyEffective {
                    features: record.features.clone(),
                    parameters: record.parameters.clone(),
                    secret_origins: record.secret_origins.clone(),
                },
            )),
        },
        requires: record
            .lifecycle
            .requires
            .iter()
            .map(|r| w::PortableRequirement {
                package: r.package.clone(),
                key: r.key.clone(),
                scope: match r.scope {
                    RequirementScope::Build => w::PortableRequirementScope::Build,
                    RequirementScope::Runtime => w::PortableRequirementScope::Runtime,
                },
                step: r.step.clone(),
            })
            .collect(),
    }
}

fn refuse(reason: String) -> AppError {
    crate::error_lifecycle::LifecycleError::PortableInvalid {
        file: "the full exported set".into(),
        reason,
    }
    .into()
}

fn directories(path: &Path) -> Result<Vec<PathBuf>, AppError> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(refuse("application work inventory is unreadable".into())),
    };
    entries
        .map(|entry| {
            entry
                .map(|e| e.path())
                .map_err(|_| refuse("application work inventory changed or is unreadable".into()))
        })
        .collect()
}

fn related(index: &Index, included: &BTreeSet<String>, key: &str) -> bool {
    let Ok(candidate) = pending::coordinate(key) else {
        return false;
    };
    included
        .iter()
        .filter_map(|key| index.installations.get(key))
        .any(|r| r.package.same_package(&candidate))
}

fn ready(
    settings: &SettingsRoot,
    index: &Index,
    included: &BTreeSet<String>,
    arrangement: bool,
) -> Result<(), AppError> {
    for key in included {
        if index
            .installations
            .get(key)
            .is_some_and(|r| r.pending.is_some())
        {
            return Err(refuse(format!(
                "installation `{key}` has pending work; finish it before full export"
            )));
        }
    }
    let families: Vec<_> = included
        .iter()
        .filter_map(|k| index.installations.get(k))
        .map(|r| {
            r.package
                .directory_name()
                .strip_suffix(&r.package.version)
                .map(str::to_owned)
                .ok_or_else(|| refuse("application work directory identity is inconsistent".into()))
        })
        .collect::<Result<_, _>>()?;
    for path in directories(&settings.app_work_dir())? {
        let Some(name) = path.file_name() else {
            continue;
        };
        if !families
            .iter()
            .any(|family| name.to_string_lossy().starts_with(family))
        {
            continue;
        }
        if name.to_str().is_none() {
            return Err(refuse(
                "relevant application work has a nontext identity".into(),
            ));
        }
        let state = StepsFile::load(&path)
            .map_err(|_| refuse("relevant application work is unreadable or invalid".into()))?
            .ok_or_else(|| {
                refuse("relevant application work has no readable completion state".into())
            })?;
        if path != pending::work(settings, &state.package)
            || !related(index, included, &state.package.key())
        {
            return Err(refuse(
                "relevant application work has a conflicting package identity".into(),
            ));
        }
        if let Some(operation) = &state.operation {
            let checked =
                pending::load(settings, &state.package, Some(&operation.target.operation))
                    .map_err(|_| {
                        refuse(
                            "relevant application work has invalid home or generation binding"
                                .into(),
                        )
                    })?;
            let operation = checked.pending(settings)?;
            if operation.phase != w::ApplicationPendingOperationPhase::Complete
                || operation.publication_record.is_none()
            {
                return Err(refuse(format!(
                    "application `{}` has unfinished or unpublished work; finish it before full export",
                    state.package.key()
                )));
            }
            // Complete operation metadata is authoritative even when an
            // optional step failed or a later removal retired this version.
            // StepOutcome/current index membership alone is not completion.
        } else if !index.installations.contains_key(&state.package.key())
            || state
                .steps
                .iter()
                .any(|s| !matches!(s.outcome, StepOutcome::Done | StepOutcome::Skipped))
        {
            return Err(refuse(format!(
                "legacy work for `{}` has no proven completed installation",
                state.package.key()
            )));
        }
    }
    cohorts_ready(settings, index, included, arrangement)
}

// Isolated concrete cohort reader call: the import owner handles protocol
// admission. Its future v2 reader must preserve this same readiness decision.
fn cohorts_ready(
    settings: &SettingsRoot,
    index: &Index,
    included: &BTreeSet<String>,
    arrangement: bool,
) -> Result<(), AppError> {
    for path in directories(&settings.app_work_dir().join("imports"))? {
        let id = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| refuse("import work identity is unreadable".into()))?;
        let document = import::load(settings, id)?;
        if document.phase() == &w::ApplicationImportPhase::Complete {
            continue;
        }
        let touches_rows = import::effect_keys(settings, &document)?
            .iter()
            .any(|key| related(index, included, key))
            || document
                .current()
                .is_some_and(|r| related(index, included, &r.key));
        let touches_arrangement =
            arrangement && !document.arrangement_done() && document.has_arrangement();
        if touches_rows || touches_arrangement {
            return Err(refuse(
                "an unfinished import cohort affects this snapshot; finish it before full export"
                    .into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "full_export/tests.rs"]
mod tests;
