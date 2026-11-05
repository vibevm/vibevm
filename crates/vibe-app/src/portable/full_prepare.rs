//! Public preparation for the existing finite full-import owner, before effects.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION");
use super::{ImportSecrets, document};
use crate::{AppError, OperationContext, ResolvedPackage, RunMode};
use std::collections::BTreeMap;
use vibe_core::manifest::{InstallerOptionDecl, StepKind, steps_of};
use vibe_wire::generated::{
    application::{e1::import_input::ImportTargetInput, e2::apps},
    shared as w,
};

#[derive(Debug, Clone)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION")]
/// Only fully classified public rows can construct this private cohort input.
pub struct PreparedFullImport {
    pub(crate) state: w::ApplicationFullImport,
    pub(crate) target: Option<w::ApplicationImportTarget>,
}
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
pub struct FullImportCandidate<'a> {
    pub application: &'a w::PortableApplication,
    pub resolved: &'a ResolvedPackage,
    pub option: &'a InstallerOptionDecl,
}
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT")]
pub struct FullImportPreparation<'a> {
    pub document: &'a apps::AppsManifest,
    pub candidates: &'a [FullImportCandidate<'a>],
    pub target: &'a ImportTargetInput,
    pub secrets: &'a ImportSecrets,
    pub context: &'a OperationContext<'a>,
    pub skip_installed: bool,
    pub accept_license: bool,
}
fn invalid(reason: impl Into<String>) -> AppError {
    document::invalid("full import", &reason.into())
}
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT")]
pub fn prepare_full_import(
    request: FullImportPreparation<'_>,
) -> Result<PreparedFullImport, AppError> {
    document::validate_full(request.document).map_err(invalid)?;
    if request.context.settings.is_project() {
        return Err(invalid("portable import targets only the user home"));
    }
    if request.target.protocol != "vibe-app-import-input/1" {
        return Err(invalid("unsupported target input protocol"));
    }
    let keys = document::active_keys(&request.document.roots, request.document.application.iter());
    if request
        .target
        .applications
        .keys()
        .any(|k| !keys.contains(k.as_str()))
        || request.secrets.keys().any(|k| !keys.contains(k))
    {
        return Err(invalid(
            "target or secret input is unused: its application is outside the selected roots and required closure",
        ));
    }
    let mut bindings = BTreeMap::new();
    let mut secret_requirements = BTreeMap::new();
    for candidate in request.candidates {
        let row = candidate.application;
        let option = candidate.option;
        if !keys.contains(&row.key)
            || request
                .document
                .application
                .iter()
                .find(|r| r.key == row.key)
                != Some(row)
            || candidate.resolved.package.key() != row.key
            || !candidate.resolved.options.contains(option)
            || bindings.contains_key(&row.key)
        {
            return Err(invalid(
                "candidate does not identify one exact unique portable application",
            ));
        }
        let target = request
            .target
            .applications
            .get(&row.key)
            .cloned()
            .unwrap_or(w::PortableTargetInput {
                path_bindings: BTreeMap::new(),
                installer: None,
                review_definition: None,
                configuration: None,
            });
        if target.installer.as_ref().is_some_and(|id| id != &option.id) {
            return Err(invalid(
                "selected target installer differs from target input",
            ));
        }
        if (row.installer != option.id
            || row.platform != option.platform
            || row.arch != option.arch)
            && target.installer.is_none()
        {
            return Err(invalid(format!(
                "application `{}` needs an explicit target installer choice",
                row.key
            )));
        }
        let parameters = option
            .configuration
            .as_ref()
            .map(|d| d.parameters.as_slice())
            .unwrap_or(&option.parameters);
        let ids = request
            .secrets
            .for_key(&row.key)
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if ids
            .iter()
            .any(|id| !parameters.iter().any(|p| &p.id == id && p.secret))
        {
            return Err(invalid(format!(
                "secret input for `{}` names an unknown or nonsecret parameter",
                row.key
            )));
        }
        if !ids.is_empty() {
            secret_requirements.insert(row.key.clone(), ids);
        }
        let configuration_definition = match (&row.configuration, &option.configuration) {
            (w::PortableConfigurationState::Configured(source), Some(definition)) => {
                let evaluated = crate::configuration::transfer::prepare(
                    crate::configuration::transfer::TransferRequest {
                        definition,
                        history: &source.history,
                        target: &target,
                        ephemeral: &[],
                        facts: request.context.facts,
                        prompt: None,
                    },
                )
                .map_err(invalid)?;
                // Public target decisions must be settled before this table becomes durable.
                if !evaluated.record.evaluation.problems.is_empty()
                    || !evaluated
                        .record
                        .evaluation
                        .questions
                        .iter()
                        .all(|q| match q {
                            w::ConfigurationQuestion::SecretResupply(_) => true,
                            w::ConfigurationQuestion::Parameter(p) => p.secret,
                            _ => false,
                        })
                {
                    return Err(crate::configuration::ConfigurationError::Unresolved {
                        option: format!("{}/{}", row.key, option.id),
                        evaluation: Box::new(evaluated.record.evaluation),
                    }
                    .into());
                }
                Some(evaluated.record.evaluation.definition)
            }
            (w::PortableConfigurationState::Configured(_), None) => {
                return Err(invalid(
                    "a legacy target installer cannot represent transported common intent",
                ));
            }
            (w::PortableConfigurationState::LegacyEffective(source), None) => {
                super::legacy_target_public(&row.key, source, &target, option)?;
                None
            }
            (w::PortableConfigurationState::LegacyEffective(_), Some(_)) => {
                return Err(invalid(
                    "legacy effective history needs deliberate ordinary reconfiguration before common transfer",
                ));
            }
        };
        bindings.insert(
            row.key.clone(),
            w::ApplicationPortableBinding {
                application: row.clone(),
                source: candidate.resolved.source.clone(),
                installer: option.id.clone(),
                platform: option.platform.clone(),
                arch: option.arch.clone(),
                definition: super::continuation::definition(candidate.resolved, option)?,
                configuration_definition,
                target_input: target,
                agent_modes: steps_of(option)
                    .into_iter()
                    .filter_map(|s| match s.kind {
                        StepKind::Agent { mode, .. } => Some((
                            s.id,
                            match crate::steps::agent_mode(mode, request.context.chooser.is_some())
                            {
                                RunMode::Interactive => w::ApplicationAgentMode::Interactive,
                                RunMode::Unattended => w::ApplicationAgentMode::Unattended,
                            },
                        )),
                        _ => None,
                    })
                    .collect(),
            },
        );
    }
    if bindings.len() != keys.len() {
        return Err(invalid(
            "every carried application must be classified before full import",
        ));
    }
    Ok(PreparedFullImport {
        target: None,
        state: w::ApplicationFullImport {
            completed_bindings: Default::default(),
            roots: request.document.roots.clone(),
            bindings,
            cursor: 0,
            current: None,
            results: vec![],
            skip_installed: request.skip_installed,
            accept_license: request.accept_license,
            collections: request
                .document
                .collection
                .iter()
                .map(|c| w::ApplicationPendingCollection {
                    name: c.name.clone(),
                    launchers: c.launchers.clone(),
                })
                .collect(),
            pins: request
                .document
                .pin
                .iter()
                .map(|p| w::ApplicationPendingPin {
                    target: match p.target {
                        apps::PinTarget::Taskbar => w::ApplicationPendingPinTarget::Taskbar,
                        apps::PinTarget::Favorites => w::ApplicationPendingPinTarget::Favorites,
                    },
                    launchers: p.launchers.clone(),
                })
                .collect(),
            launchers: request
                .document
                .launcher
                .iter()
                .map(|l| w::ApplicationPendingLauncher {
                    key: l.key.clone(),
                    name: l.name.clone(),
                    icon: l.icon.clone(),
                    hidden: Some(l.hidden),
                })
                .collect(),
            arrangement_done: false,
            waiting: None,
            secret_requirements,
        },
    })
}
impl PreparedFullImport {
    pub fn results(&self) -> &[w::ApplicationPendingImportResult] {
        &self.state.results
    }
    pub fn current(&self) -> Option<&w::ApplicationPendingTarget> {
        self.state.current.as_ref()
    }
}
