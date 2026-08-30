//! Native prebuilt/source artifact resolution and provider-root Cargo builds.
//!
//! This module deliberately stops at a verified file and a durable artifact
//! record. It does not load a library, attach lifecycle dispatch, reorder the
//! registry, or build lazily during invocation.

mod cargo;
mod error;
mod path;
mod platform;
mod provider;
mod record;
mod witness;

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::{ExtensionHandler, MechanismKey, MechanismRoutes};
use vibe_extension_registry::{MechanismRegistry, resolve_mechanism};

use crate::{ExtensionRegistryRow, MechanismRegistryRow};

pub use error::NativeArtifactError;
pub use platform::NativePlatform;

use cargo::{build_cdylib, toolchain};
use path::{VerifiedFile, prebuilt_file, relative_spelling, source_crate};
use provider::{ProviderFacts, ProviderHome, facts};
use record::{
    SourceRecordExpectation, SourceRecordInputs, record_path, revalidate_source_record,
    write_source_record,
};
use witness::{config_witness, record_id, source_witness};

/// Inputs supplied by the future build-fence wiring.
///
/// `candidates` is borrowed and already ordered/controlled by the extension
/// registry. This atom neither collects nor sorts it. Non-native rows are
/// ignored; native source groups retain first occurrence order.
#[derive(Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#BUILD-PHASE-OWNS-IT")]
pub struct NativeBuildExecution<'a> {
    pub candidates: &'a [&'a ExtensionRegistryRow],
    pub selected_project_root: &'a Path,
    pub registry: &'a MechanismRegistry,
    pub routes: &'a MechanismRoutes,
    pub platform: NativePlatform,
    pub offline: bool,
    pub created_at: &'a str,
}

/// One provider/crate group that Cargo actually built.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
pub struct NativeBuildOutcome {
    pub record_id: String,
    pub provider: String,
    pub crate_dir: String,
    pub path_absolute: String,
    pub path_relative: String,
    pub digest: String,
    pub bytes: u64,
    /// Cargo's compiler-artifact freshness bit.
    pub fresh: bool,
    pub record: String,
}

/// Whether a resolved file came directly from package content or from a
/// revalidated source-build record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PREBUILT-CLOSED")]
pub enum NativeArtifactOrigin {
    Prebuilt,
    SourceRecord,
}

/// An absolute file path that passed the current platform and containment
/// laws, plus its portable provider-relative identity and current digest.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE")]
pub struct ResolvedNativeArtifact {
    pub extension: String,
    pub provider: String,
    pub path_absolute: String,
    pub path_relative: String,
    pub digest: String,
    pub bytes: u64,
    pub origin: NativeArtifactOrigin,
    pub record: Option<String>,
}

#[derive(Debug)]
struct SourceGroup<'a> {
    provider: ProviderFacts,
    crate_dir: PathBuf,
    crate_wire: String,
    rows: Vec<&'a ExtensionRegistryRow>,
}

/// Validate all current-platform prebuilts and build every source fallback
/// once per exact `(provider identity, crate_dir)` group.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#BUILD-PHASE-OWNS-IT")]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
pub fn build_native_sources(
    execution: &NativeBuildExecution<'_>,
) -> Result<Vec<NativeBuildOutcome>, NativeArtifactError> {
    let groups = source_groups(execution.candidates, execution.platform, true)?;
    if groups.is_empty() {
        return Ok(Vec::new());
    }
    let build_provider = select_build_provider(execution)?;
    let mut outcomes = Vec::with_capacity(groups.len());
    for group in groups {
        prepare_dependency_ignore(&group.provider)?;
        let (provider_root, manifest) = source_crate(&group.provider, &group.crate_dir)?;
        let config = config_witness(&group.rows);
        let id = record_id(
            &group.provider.identity,
            &group.crate_wire,
            execution.platform,
        );
        let built = build_cdylib(
            &group.provider,
            &manifest,
            &provider_root,
            execution.platform,
            execution.offline,
        )?;
        // Cargo metadata may materialise a lockfile in an authored host. Bind
        // the record to the source tree that exists after the scheduled Cargo
        // call, not to the preflight tree that Cargo was allowed to complete.
        let source = source_witness(&group.provider)?;
        let record = write_source_record(
            &SourceRecordInputs {
                selected_project_root: execution.selected_project_root,
                provider: &group.provider,
                crate_dir: &group.crate_wire,
                platform: execution.platform,
                record_id: &id,
                build_provider: &build_provider,
                source_witness: &source,
                config_witness: &config,
                created_at: execution.created_at,
            },
            &built,
        )?;
        outcomes.push(NativeBuildOutcome {
            record_id: id,
            provider: group.provider.identity,
            crate_dir: group.crate_wire,
            path_absolute: slash(&built.file.absolute),
            path_relative: built.file.relative,
            digest: built.file.digest,
            bytes: built.file.bytes,
            fresh: built.fresh,
            record,
        });
    }
    Ok(outcomes)
}

/// Resolve one already-retained native row for later lifecycle wiring.
///
/// A declared current-platform prebuilt is verified directly. A source
/// fallback never scans `target/`: it revalidates the stable artifact record,
/// including current source/config/toolchain/platform and file bytes.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PREBUILT-CLOSED")]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE")]
pub fn resolve_native_artifact(
    execution: &NativeBuildExecution<'_>,
    row: &ExtensionRegistryRow,
) -> Result<ResolvedNativeArtifact, NativeArtifactError> {
    let ExtensionHandler::Native {
        crate_dir,
        prebuilt,
    } = &row.declaration().handler
    else {
        return Err(NativeArtifactError::NoCurrentArtifact {
            extension: row.key().to_string(),
            platform: execution.platform.key().to_owned(),
            declared: "row is not a native handler".to_owned(),
        });
    };
    let provider = facts(row);
    if let Some(path) = prebuilt
        .as_ref()
        .and_then(|entries| entries.get(execution.platform.key()))
    {
        let file = prebuilt_file(&row.key().to_string(), &provider, path, execution.platform)?;
        return Ok(resolved(
            row,
            provider.identity,
            file,
            NativeArtifactOrigin::Prebuilt,
            None,
        ));
    }
    let Some(crate_dir) = crate_dir else {
        return Err(no_current(row, prebuilt.as_ref(), execution.platform));
    };
    let crate_wire =
        relative_spelling(crate_dir).map_err(|reason| NativeArtifactError::CrateDirectory {
            provider: provider.identity.clone(),
            crate_dir: slash(crate_dir),
            reason,
        })?;
    let rows = source_group_rows(
        execution.candidates,
        &provider.identity,
        &crate_wire,
        execution.platform,
    )?;
    let source = source_witness(&provider)?;
    let config = config_witness(&rows);
    let id = record_id(&provider.identity, &crate_wire, execution.platform);
    let build_provider = select_build_provider(execution)?;
    let (provider_root, _) = source_crate(&provider, crate_dir)?;
    let current_toolchain = toolchain(&provider, &provider_root)?;
    let file = revalidate_source_record(&SourceRecordExpectation {
        selected_project_root: execution.selected_project_root,
        provider: &provider,
        platform: execution.platform,
        record_id: &id,
        build_provider: &build_provider,
        source_witness: &source,
        config_witness: &config,
        toolchain: &current_toolchain,
    })?;
    Ok(resolved(
        row,
        provider.identity,
        file,
        NativeArtifactOrigin::SourceRecord,
        Some(record_path(&id)),
    ))
}

fn source_groups<'a>(
    candidates: &'a [&'a ExtensionRegistryRow],
    platform: NativePlatform,
    validate_prebuilt: bool,
) -> Result<Vec<SourceGroup<'a>>, NativeArtifactError> {
    let mut groups: Vec<SourceGroup<'a>> = Vec::new();
    for row in candidates {
        let ExtensionHandler::Native {
            crate_dir,
            prebuilt,
        } = &row.declaration().handler
        else {
            continue;
        };
        let provider = facts(row);
        if let Some(path) = prebuilt
            .as_ref()
            .and_then(|entries| entries.get(platform.key()))
        {
            if validate_prebuilt {
                prebuilt_file(&row.key().to_string(), &provider, path, platform)?;
            }
            continue;
        }
        let Some(crate_dir) = crate_dir else {
            return Err(no_current(row, prebuilt.as_ref(), platform));
        };
        let crate_wire =
            relative_spelling(crate_dir).map_err(|reason| NativeArtifactError::CrateDirectory {
                provider: provider.identity.clone(),
                crate_dir: slash(crate_dir),
                reason,
            })?;
        if let Some(group) = groups.iter_mut().find(|group| {
            group.provider.identity == provider.identity && group.crate_wire == crate_wire
        }) {
            group.rows.push(*row);
        } else {
            groups.push(SourceGroup {
                provider,
                crate_dir: crate_dir.clone(),
                crate_wire,
                rows: vec![*row],
            });
        }
    }
    Ok(groups)
}

fn source_group_rows<'a>(
    candidates: &'a [&'a ExtensionRegistryRow],
    provider: &str,
    crate_wire: &str,
    platform: NativePlatform,
) -> Result<Vec<&'a ExtensionRegistryRow>, NativeArtifactError> {
    let groups = source_groups(candidates, platform, false)?;
    groups
        .into_iter()
        .find(|group| group.provider.identity == provider && group.crate_wire == crate_wire)
        .map(|group| group.rows)
        .ok_or_else(|| NativeArtifactError::SourceState {
            record: record_path(&record_id(provider, crate_wire, platform)),
            reason: "source group is not present in the supplied candidate epoch".to_owned(),
        })
}

fn select_build_provider(
    execution: &NativeBuildExecution<'_>,
) -> Result<String, NativeArtifactError> {
    let key = "build:cargo".parse::<MechanismKey>().map_err(|error| {
        NativeArtifactError::MechanismSelection {
            reason: format!("engine-owned key is invalid: {error}"),
        }
    })?;
    let selection =
        resolve_mechanism(execution.registry, &key, None, execution.routes).map_err(|error| {
            NativeArtifactError::MechanismSelection {
                reason: error.to_string(),
            }
        })?;
    admit_builtin(selection.row())
}

fn admit_builtin(row: &MechanismRegistryRow) -> Result<String, NativeArtifactError> {
    let provider = row.pin().to_string();
    if !row.is_builtin() {
        return Err(NativeArtifactError::TransportNotLanded {
            provider,
            kind: row.handler().kind().to_owned(),
        });
    }
    match row.handler() {
        ExtensionHandler::Builtin { name } if name == "cargo" => Ok(provider),
        ExtensionHandler::Builtin { name } => Err(NativeArtifactError::UnknownBuiltin {
            provider,
            name: name.clone(),
        }),
        handler => Err(NativeArtifactError::UnknownBuiltin {
            provider,
            name: handler.kind().to_owned(),
        }),
    }
}

fn prepare_dependency_ignore(provider: &ProviderFacts) -> Result<(), NativeArtifactError> {
    if provider.home == ProviderHome::Host {
        return Ok(());
    }
    let root = provider
        .root()
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| NativeArtifactError::BuildIgnore {
            provider: provider.identity.clone(),
            root: slash(provider.root()),
            reason: "slot has no `<dependency-root>/<coordinate>/<version>` ancestry".to_owned(),
        })?;
    if root.file_name().and_then(|name| name.to_str())
        != Some(vibe_workspace::vibedeps::VIBEDEPS_DIR)
    {
        return Err(NativeArtifactError::BuildIgnore {
            provider: provider.identity.clone(),
            root: slash(root),
            reason: format!(
                "dependency root must end in `{}`",
                vibe_workspace::vibedeps::VIBEDEPS_DIR
            ),
        });
    }
    vibe_workspace::vibedeps::ensure_build_output_ignores(root).map_err(|error| {
        NativeArtifactError::BuildIgnore {
            provider: provider.identity.clone(),
            root: slash(root),
            reason: error.to_string(),
        }
    })?;
    Ok(())
}

fn no_current(
    row: &ExtensionRegistryRow,
    prebuilt: Option<&std::collections::BTreeMap<String, PathBuf>>,
    platform: NativePlatform,
) -> NativeArtifactError {
    let mut keys = prebuilt
        .into_iter()
        .flat_map(|entries| entries.keys())
        .take(8)
        .cloned()
        .collect::<Vec<_>>();
    if keys.is_empty() {
        keys.push("none".to_owned());
    }
    NativeArtifactError::NoCurrentArtifact {
        extension: row.key().to_string(),
        platform: platform.key().to_owned(),
        declared: keys.join(", "),
    }
}

fn resolved(
    row: &ExtensionRegistryRow,
    provider: String,
    file: VerifiedFile,
    origin: NativeArtifactOrigin,
    record: Option<String>,
) -> ResolvedNativeArtifact {
    ResolvedNativeArtifact {
        extension: row.key().to_string(),
        provider,
        path_absolute: slash(&file.absolute),
        path_relative: file.relative,
        digest: file.digest,
        bytes: file.bytes,
        origin,
        record,
    }
}

fn slash(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
