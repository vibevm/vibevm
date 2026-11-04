//! Classified finite legacy import input. It is not a portable rich configuration format.
use crate::{AppError, ResolvedPackage, RunMode};
use sha2::{Digest, Sha256};
use std::path::Path;
use vibe_core::manifest::{InstallerOptionDecl, StepKind, steps_of};
use vibe_wire::generated::{apps_manifest as wire, shared as w};

/// A classified import cohort; only this module can construct its persisted input.
#[derive(Debug, Clone, PartialEq, Eq)]
#[specmark::spec(
    documents = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub struct PreparedImport {
    pub(crate) state: w::ApplicationPendingImport,
}
/// One actual selected row supplied by the source/selection owner.
#[specmark::spec(
    documents = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub struct ImportCandidate<'a> {
    pub row: &'a wire::Application,
    pub resolved: &'a ResolvedPackage,
    pub option: &'a InstallerOptionDecl,
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#portable-set")]
pub fn prepare_import(
    set: &wire::AppsManifest,
    candidates: &[ImportCandidate<'_>],
    skip_installed: bool,
    accept_license: bool,
    registry: Option<&Path>,
    attended: bool,
) -> Result<PreparedImport, AppError> {
    let mut rows = Vec::new();
    for candidate in candidates {
        let option = candidate.option;
        let row = candidate.row;
        if option.configuration.is_some() {
            return Err(
                crate::configuration::ConfigurationError::UnsupportedFormat {
                    format: super::APPS_PROTOCOL,
                    operation: "import",
                    package: row.key.clone(),
                }
                .into(),
            );
        }
        if let Some(id) = row
            .parameters
            .keys()
            .find(|id| option.parameters.iter().any(|p| &p.id == *id && p.secret))
        {
            return Err(crate::configuration::ConfigurationError::Invalid {
                option: format!("{}/{}", row.key, option.id),
                reason: format!("portable parameter `{id}` is secret; supply it ephemerally"),
            }
            .into());
        }
        let parameters = row
            .parameters
            .iter()
            .filter(|(id, _)| option.parameters.iter().any(|p| &p.id == *id && !p.secret))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect();
        let features = row
            .features
            .iter()
            .filter(|id| option.features.iter().any(|f| &f.id == *id))
            .cloned()
            .collect();
        rows.push(w::ApplicationPendingImportRow {
            key: row.key.clone(),
            installer: option.id.clone(),
            features,
            parameters,
            license_accepted: matches!(row.license, Some(wire::ApplicationLicense::Accepted)),
            held: row.held,
            hidden: row.hidden,
            mark: w::ApplicationPendingImportRowMark::Manual,
            source: candidate.resolved.source.clone(),
            platform: option.platform.clone(),
            arch: option.arch.clone(),
            definition: definition(candidate.resolved, option)?,
            agent_modes: steps_of(option)
                .into_iter()
                .filter_map(|s| match s.kind {
                    StepKind::Agent { mode, .. } => Some((
                        s.id,
                        match crate::steps::agent_mode(mode, attended) {
                            RunMode::Interactive => w::ApplicationAgentMode::Interactive,
                            RunMode::Unattended => w::ApplicationAgentMode::Unattended,
                        },
                    )),
                    _ => None,
                })
                .collect(),
        });
    }
    let registry = registry
        .map(|p| {
            p.canonicalize()
                .map_err(|e| crate::ResolveError::Registry {
                    spelling: "import registry".into(),
                    reason: e.to_string(),
                })
                .and_then(|p| {
                    p.to_str()
                        .map(str::to_owned)
                        .ok_or_else(|| crate::ResolveError::Registry {
                            spelling: "import registry".into(),
                            reason: "registry path is not lossless UTF-8".into(),
                        })
                })
        })
        .transpose()?;
    Ok(PreparedImport {
        state: w::ApplicationPendingImport {
            rows,
            cursor: 0,
            current: None,
            results: vec![],
            auto: set
                .application
                .iter()
                .filter(|r| r.mark == wire::ApplicationMark::Auto)
                .map(|r| r.key.clone())
                .collect(),
            skip_installed,
            accept_license,
            collections: set
                .collection
                .iter()
                .map(|c| w::ApplicationPendingCollection {
                    name: c.name.clone(),
                    launchers: c.launchers.clone(),
                })
                .collect(),
            pins: set
                .pin
                .iter()
                .map(|p| w::ApplicationPendingPin {
                    target: match p.target {
                        wire::PinTarget::Taskbar => w::ApplicationPendingPinTarget::Taskbar,
                        wire::PinTarget::Favorites => w::ApplicationPendingPinTarget::Favorites,
                    },
                    launchers: p.launchers.clone(),
                })
                .collect(),
            launchers: set
                .launcher
                .iter()
                .map(|l| w::ApplicationPendingLauncher {
                    key: l.key.clone(),
                    name: l.name.clone(),
                    hidden: Some(l.hidden),
                    icon: l.icon.clone(),
                })
                .collect(),
            arrangement_done: false,
            registry,
        },
    })
}
impl PreparedImport {
    pub fn rows(&self) -> &[w::ApplicationPendingImportRow] {
        &self.state.rows
    }
    pub fn cursor(&self) -> usize {
        self.state.cursor as usize
    }
    pub fn results(&self) -> &[w::ApplicationPendingImportResult] {
        &self.state.results
    }
    /// Record a source/selection refusal discovered before any row execution.
    /// Execution dispositions and attempt identities belong to the cohort owner.
    pub fn note_refusal(&mut self, key: String, code: String, message: String) {
        self.state.results.push(w::ApplicationPendingImportResult {
            key,
            action: w::ApplicationPendingImportResultAction::Refused,
            option: None,
            code: Some(code),
            message,
            target: None,
            row_index: None,
        });
    }
    pub fn current(&self) -> Option<&w::ApplicationPendingTarget> {
        self.state.current.as_ref()
    }
    pub fn automatic(&self) -> &[String] {
        &self.state.auto
    }
    pub(crate) fn retained(state: w::ApplicationPendingImport) -> Self {
        Self { state }
    }
}
pub(crate) fn definition(
    resolved: &ResolvedPackage,
    option: &InstallerOptionDecl,
) -> Result<String, AppError> {
    let bytes = serde_json::to_vec(&(&resolved.application, option))
        .map_err(|_| invalid("cannot encode selected legacy import declaration"))?;
    let mut digest = Sha256::new();
    digest.update(b"vibevm.app.legacy-import-definition/v1\0");
    digest.update(bytes);
    Ok(format!(
        "legacy-import-definition/1:sha256:{:x}",
        digest.finalize()
    ))
}
pub(crate) fn invalid(reason: &str) -> AppError {
    crate::LifecycleError::PortableInvalid {
        file: "retained import".into(),
        reason: reason.into(),
    }
    .into()
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#portable-set")]
pub fn import_row(row: &w::ApplicationPendingImportRow) -> wire::Application {
    wire::Application {
        key: row.key.clone(),
        installer: row.installer.clone(),
        features: row.features.clone(),
        parameters: row.parameters.clone(),
        license: row
            .license_accepted
            .then_some(wire::ApplicationLicense::Accepted),
        mark: wire::ApplicationMark::Manual,
        held: row.held,
        hidden: row.hidden,
    }
}

pub(crate) fn arrangement(
    state: &w::ApplicationPendingImport,
) -> (Vec<wire::Collection>, Vec<wire::Pin>, Vec<wire::Launcher>) {
    (
        state
            .collections
            .iter()
            .map(|c| wire::Collection {
                name: c.name.clone(),
                launchers: c.launchers.clone(),
            })
            .collect(),
        state
            .pins
            .iter()
            .map(|p| wire::Pin {
                target: match p.target {
                    w::ApplicationPendingPinTarget::Taskbar => wire::PinTarget::Taskbar,
                    w::ApplicationPendingPinTarget::Favorites => wire::PinTarget::Favorites,
                },
                launchers: p.launchers.clone(),
            })
            .collect(),
        state
            .launchers
            .iter()
            .map(|l| wire::Launcher {
                key: l.key.clone(),
                name: l.name.clone(),
                hidden: l.hidden.unwrap_or(false),
                icon: l.icon.clone(),
            })
            .collect(),
    )
}
