//! The planner (`PROP-065#plan`): license, features, parameters,
//! conditions, needs and download sizes resolved from facts the caller
//! supplies, before anything is downloaded or run.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#plan");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#conditions");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#needs");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#features");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::{
    FeatureDecl, InstallerOptionDecl, RequirementScope, ResultDecl, StepDecl, StepKind, StepPhase,
    result_of, steps_of,
};

use crate::agents::AgentHooks;
use crate::error::{AppError, StepError};
use crate::index::PackageCoordinate;
use crate::resolve::ResolvedPackage;
use crate::select::{Choice, Chooser};
use crate::settings::SettingsRoot;

use super::features::{FeatureRequest, Implied};
use super::params::{
    LicensePrompt, ParamValue, ParameterPrompt, PlanLicense, admit_license, resolve_parameters,
};
use super::sizes::downloads;
mod decision;
use super::template::{self, Vars};
use decision::{NO_AGENT, decide, effective_agent_mode, require};

#[path = "plan/remap_note.rs"]
mod remap_note;
use remap_note::unremappable_network;

mod prompt;
mod selection;
use selection::choose_features;

/// An attended multi-choice prompt: `prompt`, the rows and which are
/// preselected; returns the chosen indexes or `None` when cancelled.
pub type FeatureChooser<'a> =
    &'a dyn Fn(&str, &[Choice], &[bool]) -> Result<Option<Vec<usize>>, String>;

/// Host facts the caller looks up; `vibe-app` reads no environment.
#[derive(Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
pub struct HostFacts<'a> {
    /// Whether a command name is found on PATH.
    pub command_present: &'a dyn Fn(&str) -> bool,
    /// Whether an environment variable is set.
    pub env_present: &'a dyn Fn(&str) -> bool,
    /// The absolute form of a `path` parameter value (`PROP-065#parameters`).
    pub resolve_path: &'a dyn Fn(&str) -> PathBuf,
}

fn absent(_: &str) -> bool {
    false
}

fn as_given(path: &str) -> PathBuf {
    PathBuf::from(path)
}

impl Default for HostFacts<'static> {
    /// A host on which no command and no variable is present and every
    /// path is taken as given.
    fn default() -> Self {
        Self {
            command_present: &absent,
            env_present: &absent,
            resolve_path: &as_given,
        }
    }
}

/// The attended questions of a plan; all absent unattended.
#[derive(Clone, Copy, Default)]
pub(crate) struct Prompts<'a> {
    /// The profile question (profiles first, then "custom").
    pub chooser: Option<Chooser<'a>>,
    pub features: Option<FeatureChooser<'a>>,
    pub parameter: Option<ParameterPrompt<'a>>,
    pub license: Option<LicensePrompt<'a>>,
}

/// Resolves one requirement (`group/name@req`, its effective scope, the
/// requiring step) against the home's index first, then the registries
/// (`PROP-065#app-dependencies`).
pub(crate) type RequirementResolver<'a> =
    &'a dyn Fn(&str, RequirementScope, Option<&str>) -> Result<PlannedDependency, AppError>;

/// One required application package of a plan (`PROP-065#app-dependencies`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies")]
pub struct PlannedDependency {
    /// The authored `group/name@requirement`.
    pub package: String,
    pub scope: RequirementScope,
    /// The requiring step; `None` for an option-level requirement.
    pub step: Option<String>,
    /// The installation key that satisfies it.
    pub key: String,
    /// An installed version satisfies it and is used as is; otherwise it
    /// is installed before the step that needs it.
    pub installed: bool,
    /// Whether the installation is (or will be) hidden.
    pub hidden: bool,
    /// The public commands it puts on the requiring step's PATH.
    pub commands: Vec<String>,
    /// The installer option that provides it (recorded, or chosen for this
    /// host when it is installed first).
    pub option: Option<String>,
    /// The resolved package to install, when it is not installed.
    pub resolved: Option<Box<ResolvedPackage>>,
}

/// What the installation an update starts from recorded.
#[derive(Clone, Copy)]
pub(crate) struct Previous<'a> {
    pub secret_origins:
        &'a BTreeMap<String, vibe_wire::generated::shared::ConfigurationParameterOrigin>,
    pub configuration: Option<&'a crate::configuration::ConfigurationRecord>,
    pub features: &'a [String],
    pub parameters: &'a BTreeMap<String, String>,
    pub license: Option<&'a str>,
    /// Ask the feature question again with these values preselected
    /// (`modify` in an attended terminal without flags, `PROP-065#modify`).
    pub ask: bool,
}

/// Everything one plan is computed from.
pub(crate) struct PlanInput<'a> {
    pub agent_modes: Option<&'a BTreeMap<String, crate::RunMode>>,
    pub configuration: Option<&'a crate::configuration::ConfigurationSubmission>,
    pub resolved: &'a ResolvedPackage,
    pub option: &'a InstallerOptionDecl,
    pub request: &'a FeatureRequest,
    /// `--set ID=VALUE`.
    pub parameters: &'a [(String, String)],
    pub accept_license: bool,
    pub previous: Option<Previous<'a>>,
    pub settings: &'a SettingsRoot,
    pub facts: HostFacts<'a>,
    pub offline: bool,
    pub attended: bool,
    pub prompts: Prompts<'a>,
    /// The chosen agent and whether `vibe` runs inside a hosting agent
    /// (`PROP-064#selection`, `#parking`).
    pub agent: AgentHooks<'a>,
    /// Resolves `requires` entries (`PROP-065#app-dependencies`).
    pub requirements: Option<RequirementResolver<'a>>,
    /// Set when this plan is a requirement of another package's plan.
    pub required_by: Option<RequiredBy>,
}

/// Why a plan is being computed for a package the user did not name: the
/// requiring package and the step that named it
/// (`PROP-065#app-dependencies`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies")]
pub struct RequiredBy {
    /// The installation key of the package whose plan named the requirement.
    pub package: String,
    /// The requiring step; `None` for an option-level requirement.
    pub step: Option<String>,
}

/// One step and whether it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#plan")]
pub struct PlannedStep {
    /// Effective delivery mode, retained independently of a later caller's terminal.
    pub agent_mode: Option<crate::RunMode>,
    pub step: StepDecl,
    /// The validated literal prompt for a running agent; never reread at execution.
    /// This internal plan data is not part of the authored step or wire document.
    pub prepared_prompt: Option<String>,
    pub run: bool,
    pub reason: Option<String>,
    /// What a reader has to know about a step that WILL run
    /// (`PROP-066#not-covered`): today only the marking of a `recipe`
    /// step that declares `needs = "network"` while the active remap
    /// policy is `unmatched = "deny"`. What such a script downloads on
    /// its own is outside vibe's fetches and outside the rows, so the
    /// reader sees it before the build starts rather than after it fails.
    pub note: Option<String>,
}

/// The resolved view of one option, feature set and parameter values.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#plan")]
pub struct Plan {
    /// Legacy secret origins captured by this operation; common records own their own metadata.
    pub secret_origins:
        BTreeMap<String, vibe_wire::generated::shared::ConfigurationParameterOrigin>,
    /// Value-free presence of explicitly provided secret answers, distinct from authored defaults.
    pub secret_provided: Vec<String>,
    pub configuration: Option<crate::configuration::ConfigurationRecord>,
    pub package: PackageCoordinate,
    pub option: String,
    pub platform: String,
    pub arch: String,
    pub result: ResultDecl,
    pub declared: Vec<FeatureDecl>,
    pub profile: Option<String>,
    pub features: Vec<String>,
    pub implied: Vec<Implied>,
    pub parameters: Vec<ParamValue>,
    pub license: Option<PlanLicense>,
    /// Declared bytes of the inputs that will be fetched (`PROP-065#sizes`).
    pub download_bytes: u64,
    /// Inputs that will be fetched without a declared size.
    pub unknown_sizes: u32,
    /// Required application packages in plan order: option-level first,
    /// then each running step's (`PROP-065#app-dependencies`).
    pub dependencies: Vec<PlannedDependency>,
    pub steps: Vec<PlannedStep>,
    pub warnings: Vec<String>,
    /// Set when this plan belongs to a package another plan requires, so
    /// the text says whose it is instead of appearing unexplained between
    /// the owner's plan and its first step (`PROP-065#app-dependencies`).
    /// The `vibe-app-plan/1` document is unchanged: a requirement is
    /// already a `dependency` entry of the requiring plan.
    pub required_by: Option<RequiredBy>,
}

impl Plan {
    /// Actual declared feature IDs used by the execution environment adapter.
    pub(crate) fn feature_ids(&self) -> Vec<&str> {
        match &self.configuration {
            Some(c) => c
                .definition
                .features
                .iter()
                .map(|f| f.id.as_str())
                .collect(),
            None => self.declared.iter().map(|f| f.id.as_str()).collect(),
        }
    }
}

/// The work directory of an installation key.
pub(crate) fn work_dir(settings: &SettingsRoot, package: &PackageCoordinate) -> PathBuf {
    settings.app_work_dir().join(package.directory_name())
}

/// Compute the plan; refuse a missing license, an invalid feature choice
/// or parameter, an unmet need, a required agent step without an agent, an
/// interactive agent step without a terminal, or a plan in which nothing
/// produces the declared result.
pub(crate) fn plan(input: &PlanInput<'_>) -> Result<Plan, AppError> {
    let package = &input.resolved.package;
    let key = package.key();
    let option = input.option;
    option
        .validate_step_environment(
            &vibe_core::manifest::FieldPath::root().table("installer_options"),
        )
        .map_err(|error| crate::error::ResolveError::Manifest {
            package: key.clone(),
            reason: format!("{}: {error}", error.field),
        })?;
    let license = admit_license(
        option,
        &key,
        &input.resolved.root,
        input.accept_license,
        input.previous.and_then(|previous| previous.license),
        input.prompts.license.filter(|_| input.attended),
    )?;
    let (profile, resolution, parameters, configuration, secret_provided, secret_origins) =
        if option.configuration.is_some() {
            let common = crate::configuration::plan::prepare(input)?;
            let secret_provided = common
                .record
                .evaluation
                .parameters
                .iter()
                .filter_map(|p| match p {
                    vibe_wire::generated::shared::ConfigurationParameterResult::Secret(p)
                        if p.provided =>
                    {
                        Some(p.id.clone())
                    }
                    _ => None,
                })
                .collect();
            let features = common
                .record
                .evaluation
                .resolved
                .as_ref()
                .map(|r| r.features.selected.clone())
                .unwrap_or_default();
            (
                common.record.evaluation.input.intent.profile.clone(),
                super::features::FeatureResolution {
                    selected: features,
                    implied: vec![],
                },
                common.parameters,
                Some(common.record),
                secret_provided,
                BTreeMap::new(),
            )
        } else {
            if input.configuration.is_some() {
                return Err(crate::configuration::ConfigurationError::Invalid {
                    option: option.id.clone(),
                    reason: "this installer has no common configuration definition".into(),
                }
                .into());
            }
            let (profile, resolution) = choose_features(input)?;
            let parameters = resolve_parameters(
                option,
                input.parameters,
                input.previous.map(|previous| previous.parameters),
                input.facts,
                input.prompts.parameter.filter(|_| input.attended),
                input.previous.map(|p| p.secret_origins),
            )?;
            (
                profile,
                resolution,
                parameters.values,
                None,
                parameters.supplied_secrets,
                parameters.secret_origins,
            )
        };
    let work = work_dir(input.settings, package);
    let vars = Vars::planning(
        &work,
        &input.resolved.root,
        input.settings.path(),
        &resolution.selected,
        &parameters,
    );
    let mut steps = Vec::new();
    let mut skipped_agents = Vec::new();
    let mut dependencies = option
        .requires
        .iter()
        .map(|requirement| require(input, &key, requirement, RequirementScope::Runtime, None))
        .collect::<Result<Vec<_>, _>>()?;
    let provided: Vec<String> = dependencies
        .iter()
        .flat_map(|dependency| dependency.commands.iter().cloned())
        .collect();
    for step in steps_of(option) {
        let (run, reason, needed) = decide(input, &step, &vars, &work, &key, &provided)?;
        if run {
            dependencies.extend(needed);
        }
        if !run
            && matches!(step.kind, StepKind::Agent { .. })
            && reason.as_deref() == Some(NO_AGENT)
        {
            skipped_agents.push(format!("`{}`", step.id));
        }
        let note = unremappable_network(run, &step);
        let prepared_prompt = prompt::prepare(option, &step, run, &input.resolved.root, &vars)?;
        let agent_mode = match &step.kind {
            StepKind::Agent { mode, .. } => Some(effective_agent_mode(input, &step.id, *mode)),
            _ => None,
        };
        steps.push(PlannedStep {
            agent_mode,
            step,
            prepared_prompt,
            run,
            reason,
            note,
        });
    }
    let result = result_of(option);
    let produces = steps.iter().any(|planned| {
        planned.run && planned.step.phase == StepPhase::Prepare && planned.step.kind.writes_out()
    });
    if result.is_owned() && !produces {
        return Err(StepError::NoResultStep {
            key,
            option: option.id.clone(),
        }
        .into());
    }
    let warnings = if skipped_agents.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "no agent is configured, so agent step(s) {} will be skipped; provide one with --agent NAME, \
             the VIBE_AGENT_COMMAND registration, or `default` in agents.toml",
            skipped_agents.join(", ")
        )]
    };
    let fetched = downloads(
        steps
            .iter()
            .filter(|planned| planned.run)
            .map(|planned| &planned.step),
        input.settings,
        &work,
    );
    Ok(Plan {
        secret_provided,
        secret_origins,
        configuration,
        package: package.clone(),
        option: option.id.clone(),
        platform: option.platform.clone(),
        arch: option.arch.clone(),
        result,
        declared: option.features.clone(),
        profile,
        features: resolution.selected,
        implied: resolution.implied,
        parameters,
        license,
        download_bytes: fetched.bytes,
        unknown_sizes: fetched.unknown,
        dependencies,
        steps,
        warnings,
        required_by: input.required_by.clone(),
    })
}
