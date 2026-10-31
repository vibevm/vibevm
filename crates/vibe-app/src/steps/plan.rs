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
    AgentMode, Condition, FeatureDecl, InstallerOptionDecl, Need, RequirementDecl,
    RequirementScope, ResultDecl, StepDecl, StepKind, StepPhase, result_of, steps_of,
};

use crate::agents::AgentHooks;
use crate::error::{AppError, StepError};
use crate::error_lifecycle::LifecycleError;
use crate::index::PackageCoordinate;
use crate::resolve::ResolvedPackage;
use crate::select::Chooser;
use crate::settings::SettingsRoot;

use super::features::{FeatureRequest, FeatureResolution, Implied, resolve_features, tree_order};
use super::params::{
    LicensePrompt, ParamValue, ParameterPrompt, PlanLicense, admit_license, param_equals,
    param_holds, resolve_parameters,
};
use super::sizes::{downloads, input_local, source_cached};
use super::template::{self, Vars};

/// An attended multi-choice prompt: `prompt`, the labels and which are
/// preselected; returns the chosen indexes or `None` when cancelled.
pub type FeatureChooser<'a> =
    &'a dyn Fn(&str, &[String], &[bool]) -> Result<Option<Vec<usize>>, String>;

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
    pub features: &'a [String],
    pub parameters: &'a BTreeMap<String, String>,
    pub license: Option<&'a str>,
    /// Ask the feature question again with these values preselected
    /// (`modify` in an attended terminal without flags, `PROP-065#modify`).
    pub ask: bool,
}

/// Everything one plan is computed from.
pub(crate) struct PlanInput<'a> {
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
}

/// One step and whether it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#plan")]
pub struct PlannedStep {
    pub step: StepDecl,
    pub run: bool,
    pub reason: Option<String>,
}

/// The resolved view of one option, feature set and parameter values.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#plan")]
pub struct Plan {
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
    let license = admit_license(
        option,
        &key,
        &input.resolved.root,
        input.accept_license,
        input.previous.and_then(|previous| previous.license),
        input.prompts.license.filter(|_| input.attended),
    )?;
    let (profile, resolution) = choose_features(input)?;
    let parameters = resolve_parameters(
        option,
        input.parameters,
        input.previous.map(|previous| previous.parameters),
        input.facts,
        input.prompts.parameter.filter(|_| input.attended),
    )?;
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
        steps.push(PlannedStep { step, run, reason });
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
    })
}

const NO_AGENT: &str = "no agent configured";

/// Resolve one requirement through the caller's resolver.
fn require(
    input: &PlanInput<'_>,
    key: &str,
    requirement: &RequirementDecl,
    default: RequirementScope,
    step: Option<&str>,
) -> Result<PlannedDependency, AppError> {
    let resolver = input
        .requirements
        .ok_or_else(|| LifecycleError::RequirementUnresolved {
            key: key.into(),
            requirement: requirement.package().into(),
            reason: "no package resolver is available in this context".into(),
        })?;
    resolver(requirement.package(), requirement.scope_or(default), step)
}

/// One step's decision: whether it runs, why not, and the requirements
/// it brings when it runs.
type Decision = (bool, Option<String>, Vec<PlannedDependency>);

fn skip(reason: String) -> Result<Decision, AppError> {
    Ok((false, Some(reason), Vec::new()))
}

/// Run or skip one step, or refuse the whole plan. `provided` names the
/// public commands of the option-level requirements.
fn decide(
    input: &PlanInput<'_>,
    step: &StepDecl,
    vars: &Vars<'_>,
    work: &Path,
    key: &str,
    provided: &[String],
) -> Result<Decision, AppError> {
    for condition in &step.when {
        if !holds(input, condition, vars, step)? {
            return skip(format!("when `{condition}` does not hold"));
        }
    }
    for condition in &step.unless {
        if holds(input, condition, vars, step)? {
            return skip(format!("unless `{condition}` holds"));
        }
    }
    if let StepKind::Agent { required, mode, .. } = &step.kind {
        // `PROP-064#selection`: no agent means skip, unless the step is
        // required; `#modes`: interactive needs a terminal unless parked.
        if !input.agent.available() {
            if *required {
                return Err(StepError::AgentRequired {
                    key: key.into(),
                    step: step.id.clone(),
                    reason: step
                        .description
                        .clone()
                        .unwrap_or_else(|| step.title.clone()),
                }
                .into());
            }
            return skip(NO_AGENT.into());
        }
        if *mode == Some(AgentMode::Interactive) && !input.attended && !input.agent.hosted {
            if step.optional {
                return skip("unmet need `terminal`".into());
            }
            return Err(StepError::UnmetNeed {
                key: key.into(),
                step: step.id.clone(),
                need: Need::Terminal.to_string(),
            }
            .into());
        }
    }
    let mut needed = Vec::new();
    for requirement in &step.requires {
        match require(
            input,
            key,
            requirement,
            RequirementScope::Build,
            Some(&step.id),
        ) {
            Ok(dependency) => needed.push(dependency),
            Err(error) if step.optional => {
                return skip(format!("unresolved requirement: {error}"));
            }
            Err(error) => return Err(error),
        }
    }
    let commands: Vec<&str> = provided
        .iter()
        .map(String::as_str)
        .chain(
            needed
                .iter()
                .flat_map(|dependency| dependency.commands.iter().map(String::as_str)),
        )
        .collect();
    for need in &step.needs {
        if need_met(input, need, step, work, &commands) {
            continue;
        }
        if step.optional {
            return skip(format!("unmet need `{need}`"));
        }
        return Err(StepError::UnmetNeed {
            key: key.into(),
            step: step.id.clone(),
            need: need.to_string(),
        }
        .into());
    }
    Ok((true, None, needed))
}

fn holds(
    input: &PlanInput<'_>,
    condition: &Condition,
    vars: &Vars<'_>,
    step: &StepDecl,
) -> Result<bool, AppError> {
    Ok(match condition {
        Condition::Feature(id) => vars.features.iter().any(|chosen| chosen == id),
        Condition::Param(id) => param_holds(input.option, vars.params, id),
        Condition::ParamEquals(test) => {
            param_equals(input.option, vars.params, &test.id, &test.value)
        }
        Condition::CommandPresent(name) => (input.facts.command_present)(name),
        Condition::EnvPresent(name) => (input.facts.env_present)(name),
        Condition::AgentConfigured(value) => input.agent.available() == *value,
        Condition::Offline(value) => input.offline == *value,
        Condition::FilePresent(path) => {
            let expanded = template::expand(path, vars, &step.id, "file_present")?;
            input.resolved.root.join(expanded).exists()
        }
    })
}

/// Whether a need of a running step is met on this host or by the public
/// commands of its requirements (`PROP-065#app-dependencies`).
fn need_met(
    input: &PlanInput<'_>,
    need: &Need,
    step: &StepDecl,
    work: &Path,
    provided: &[&str],
) -> bool {
    match need {
        Need::Command(name) => {
            provided.contains(&name.as_str()) || (input.facts.command_present)(name)
        }
        Need::Agent => input.agent.available(),
        Need::Terminal => input.attended,
        Need::Network if !input.offline => true,
        Need::Network => match &step.kind {
            StepKind::Sources { sources } => sources
                .iter()
                .all(|source| source.url.is_none() || source_cached(input.settings, source)),
            StepKind::Archive { source, sha256, .. } | StepKind::Native { source, sha256, .. } => {
                input_local(work, step, source, sha256)
            }
            _ => false,
        },
    }
}

/// The profile and the resolved feature set: flags (with a profile) as
/// named, else the recorded set of an update, else an attended question
/// (the profiles first, "custom" opening the tree), else the defaults.
fn choose_features(input: &PlanInput<'_>) -> Result<(Option<String>, FeatureResolution), AppError> {
    let option = input.option;
    let request = input.request;
    let find = |id: &str| {
        option
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .ok_or_else(|| StepError::UnknownProfile {
                option: option.id.clone(),
                profile: id.into(),
                declared: option
                    .profiles
                    .iter()
                    .map(|profile| profile.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })
    };
    let mut profile = request.profile.as_deref().map(find).transpose()?;
    let reask = input.previous.is_some_and(|previous| previous.ask);
    let previous = input.previous.map(|previous| previous.features);
    let mut prompted = None;
    let ask = request.is_empty() && (previous.is_none() || reask) && input.attended;
    if let (true, Some(features)) = (ask && !option.features.is_empty(), input.prompts.features) {
        let mut custom = true;
        if let (false, Some(chooser)) = (option.profiles.is_empty(), input.prompts.chooser) {
            let mut ordered: Vec<_> = option.profiles.iter().collect();
            ordered.sort_by_key(|profile| !profile.default);
            let mut labels: Vec<String> = ordered
                .iter()
                .map(|profile| match &profile.description {
                    Some(text) => format!("{} — {text}", profile.name),
                    None => profile.name.clone(),
                })
                .collect();
            labels.push("Custom — choose the features one by one".into());
            let picked = chooser(&format!("Setup type of installer `{}`", option.id), &labels)
                .map_err(|reason| StepError::FeaturePrompt { reason })?
                .ok_or_else(|| StepError::FeaturePrompt {
                    reason: "the selection was cancelled".into(),
                })?;
            if let Some(chosen) = ordered.get(picked) {
                profile = Some(*chosen);
                custom = false;
            }
        }
        if custom {
            let defaults = resolve_features(option, None, &FeatureRequest::default(), None, None)?;
            let order = tree_order(&option.features);
            let labels: Vec<String> = order
                .iter()
                .map(|(depth, feature)| {
                    let indent = "  ".repeat(*depth);
                    match &feature.description {
                        Some(text) => format!("{indent}{} ({}) — {text}", feature.name, feature.id),
                        None => format!("{indent}{} ({})", feature.name, feature.id),
                    }
                })
                .collect();
            let chosen = match previous {
                Some(recorded) if reask => recorded.to_vec(),
                _ => defaults.selected.clone(),
            };
            let preselected: Vec<bool> = order
                .iter()
                .map(|(_, feature)| chosen.contains(&feature.id))
                .collect();
            let picked = features(
                &format!("Select features of installer `{}`", option.id),
                &labels,
                &preselected,
            )
            .map_err(|reason| StepError::FeaturePrompt { reason })?
            .ok_or_else(|| StepError::FeaturePrompt {
                reason: "the selection was cancelled".into(),
            })?;
            prompted = Some(
                picked
                    .iter()
                    .filter_map(|index| order.get(*index).map(|(_, f)| f.id.clone()))
                    .collect::<Vec<_>>(),
            );
        }
    }
    let resolution = resolve_features(option, profile, request, previous, prompted.as_deref())?;
    Ok((profile.map(|profile| profile.id.clone()), resolution))
}
