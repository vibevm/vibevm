//! The planner (`PROP-065#plan`): features, conditions and needs resolved
//! from facts the caller supplies, before anything is downloaded or run.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#plan");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#conditions");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#needs");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#features");

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::{
    Condition, FeatureDecl, InstallerOptionDecl, Need, ResultDecl, StepDecl, StepKind, StepPhase,
    result_of, steps_of,
};
use vibe_wire::generated::application_plan as wire;

use crate::error::{AppError, StepError};
use crate::index::PackageCoordinate;
use crate::installer::source_build::digest_of;
use crate::resolve::ResolvedPackage;
use crate::settings::SettingsRoot;

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
}

fn absent(_: &str) -> bool {
    false
}

impl Default for HostFacts<'static> {
    /// A host on which no command and no variable is present.
    fn default() -> Self {
        Self {
            command_present: &absent,
            env_present: &absent,
        }
    }
}

/// `--feature ID` / `--no-feature ID`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#features")]
pub struct FeatureRequest {
    pub enable: Vec<String>,
    pub disable: Vec<String>,
}

impl FeatureRequest {
    pub fn is_empty(&self) -> bool {
        self.enable.is_empty() && self.disable.is_empty()
    }
}

/// Everything one plan is computed from.
pub(crate) struct PlanInput<'a> {
    pub resolved: &'a ResolvedPackage,
    pub option: &'a InstallerOptionDecl,
    pub request: &'a FeatureRequest,
    /// The features recorded by the installation an update starts from.
    pub previous: Option<&'a [String]>,
    pub settings: &'a SettingsRoot,
    pub facts: HostFacts<'a>,
    pub offline: bool,
    pub attended: bool,
    pub chooser: Option<FeatureChooser<'a>>,
}

/// One step and whether it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#plan")]
pub struct PlannedStep {
    pub step: StepDecl,
    pub run: bool,
    pub reason: Option<String>,
}

/// The resolved view of one option and feature set.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#plan")]
pub struct Plan {
    pub package: PackageCoordinate,
    pub option: String,
    pub platform: String,
    pub arch: String,
    pub result: ResultDecl,
    pub declared: Vec<FeatureDecl>,
    pub features: Vec<String>,
    pub steps: Vec<PlannedStep>,
    pub warnings: Vec<String>,
}

/// The work directory of an installation key.
pub(crate) fn work_dir(settings: &SettingsRoot, package: &PackageCoordinate) -> PathBuf {
    settings.app_work_dir().join(package.directory_name())
}

/// Compute the plan; refuse an unmet need, a required agent step without
/// an agent, or a plan in which nothing produces the declared result.
pub(crate) fn plan(input: &PlanInput<'_>) -> Result<Plan, AppError> {
    let package = &input.resolved.package;
    let key = package.key();
    let features = choose_features(input)?;
    let work = work_dir(input.settings, package);
    let vars = Vars::planning(
        &work,
        &input.resolved.root,
        input.settings.path(),
        &features,
    );
    let mut steps = Vec::new();
    let mut skipped_agents = Vec::new();
    for step in steps_of(input.option) {
        let (run, reason) = decide(input, &step, &vars, &work, &key)?;
        if !run
            && matches!(step.kind, StepKind::Agent { .. })
            && reason.as_deref() == Some(NO_AGENT)
        {
            skipped_agents.push(format!("`{}`", step.id));
        }
        steps.push(PlannedStep { step, run, reason });
    }
    let result = result_of(input.option);
    let produces = steps.iter().any(|planned| {
        planned.run && planned.step.phase == StepPhase::Prepare && planned.step.kind.writes_out()
    });
    if result.is_owned() && !produces {
        return Err(StepError::NoResultStep {
            key,
            option: input.option.id.clone(),
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
    Ok(Plan {
        package: package.clone(),
        option: input.option.id.clone(),
        platform: input.option.platform.clone(),
        arch: input.option.arch.clone(),
        result,
        declared: input.option.features.clone(),
        features,
        steps,
        warnings,
    })
}

const NO_AGENT: &str = "no agent configured";

/// Run or skip one step, or refuse the whole plan.
fn decide(
    input: &PlanInput<'_>,
    step: &StepDecl,
    vars: &Vars<'_>,
    work: &Path,
    key: &str,
) -> Result<(bool, Option<String>), AppError> {
    for condition in &step.when {
        if !holds(input, condition, vars, step)? {
            return Ok((false, Some(format!("when `{condition}` does not hold"))));
        }
    }
    for condition in &step.unless {
        if holds(input, condition, vars, step)? {
            return Ok((false, Some(format!("unless `{condition}` holds"))));
        }
    }
    if let StepKind::Agent { required, .. } = &step.kind {
        // Q1: no agent configuration exists yet (`PROP-064#selection`).
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
        return Ok((false, Some(NO_AGENT.into())));
    }
    for need in &step.needs {
        if need_met(input, need, step, work) {
            continue;
        }
        if step.optional {
            return Ok((false, Some(format!("unmet need `{need}`"))));
        }
        return Err(StepError::UnmetNeed {
            key: key.into(),
            step: step.id.clone(),
            need: need.to_string(),
        }
        .into());
    }
    Ok((true, None))
}

fn holds(
    input: &PlanInput<'_>,
    condition: &Condition,
    vars: &Vars<'_>,
    step: &StepDecl,
) -> Result<bool, AppError> {
    Ok(match condition {
        Condition::Feature(id) => vars.features.iter().any(|chosen| chosen == id),
        Condition::CommandPresent(name) => (input.facts.command_present)(name),
        Condition::EnvPresent(name) => (input.facts.env_present)(name),
        Condition::AgentConfigured(value) => !*value,
        Condition::Offline(value) => input.offline == *value,
        Condition::FilePresent(path) => {
            let expanded = template::expand(path, vars, &step.id, "file_present")?;
            input.resolved.root.join(expanded).exists()
        }
    })
}

/// Whether a need of a running step is met on this host.
fn need_met(input: &PlanInput<'_>, need: &Need, step: &StepDecl, work: &Path) -> bool {
    match need {
        Need::Command(name) => (input.facts.command_present)(name),
        Need::Agent => false,
        Need::Terminal => input.attended,
        Need::Network if !input.offline => true,
        Need::Network => match &step.kind {
            StepKind::Sources { sources } => sources.iter().all(|source| match &source.url {
                None => true,
                Some(url) => {
                    let name = url.rsplit('/').next().unwrap_or_default();
                    let sha = source.sha256.to_ascii_lowercase();
                    let cached = input.settings.app_sources_cache().join(&sha).join(name);
                    digest_of(&cached).is_ok_and(|actual| actual == sha)
                }
            }),
            StepKind::Archive { source, sha256, .. } | StepKind::Native { source, sha256, .. } => {
                !source.contains("://")
                    || digest_of(&super::kinds::input_file(work, step))
                        .is_ok_and(|actual| actual == sha256.to_ascii_lowercase())
            }
            _ => false,
        },
    }
}

/// Flags, then an attended question, then the defaults; an update reuses
/// the recorded set unless flags name features again.
fn choose_features(input: &PlanInput<'_>) -> Result<Vec<String>, AppError> {
    let declared = &input.option.features;
    let known = |id: &String| declared.iter().any(|feature| &feature.id == id);
    for id in input.request.enable.iter().chain(&input.request.disable) {
        if !known(id) {
            return Err(StepError::UnknownFeature {
                option: input.option.id.clone(),
                feature: id.clone(),
                declared: declared
                    .iter()
                    .map(|feature| feature.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            }
            .into());
        }
    }
    let defaults = || -> Vec<bool> { declared.iter().map(|feature| feature.default).collect() };
    let chosen: Vec<bool> = if !input.request.is_empty() {
        declared
            .iter()
            .zip(defaults())
            .map(|(feature, default)| {
                (default || input.request.enable.contains(&feature.id))
                    && !input.request.disable.contains(&feature.id)
            })
            .collect()
    } else if let Some(previous) = input.previous {
        declared
            .iter()
            .map(|feature| previous.contains(&feature.id))
            .collect()
    } else if let (Some(chooser), true, false) =
        (input.chooser, input.attended, declared.is_empty())
    {
        let labels: Vec<String> = declared
            .iter()
            .map(|feature| match &feature.description {
                Some(description) => format!("{} ({}) — {description}", feature.name, feature.id),
                None => format!("{} ({})", feature.name, feature.id),
            })
            .collect();
        let picked = chooser(
            &format!("Select features of installer `{}`", input.option.id),
            &labels,
            &defaults(),
        )
        .map_err(|reason| StepError::FeaturePrompt { reason })?
        .ok_or_else(|| StepError::FeaturePrompt {
            reason: "the selection was cancelled".into(),
        })?;
        (0..declared.len())
            .map(|index| picked.contains(&index))
            .collect()
    } else {
        defaults()
    };
    Ok(declared
        .iter()
        .zip(chosen)
        .filter(|(_, chosen)| *chosen)
        .map(|(feature, _)| feature.id.clone())
        .collect())
}

impl Plan {
    /// The `vibe-app-plan/1` document.
    pub fn to_wire(&self) -> wire::ApplicationPlan {
        wire::ApplicationPlan {
            protocol: "vibe-app-plan/1".into(),
            package: wire::Package {
                group: self.package.group.clone(),
                name: self.package.name.clone(),
                version: self.package.version.clone(),
            },
            option: self.option.clone(),
            platform: self.platform.clone(),
            arch: self.arch.clone(),
            result: if self.result.is_owned() {
                wire::Result::Owned
            } else {
                wire::Result::External
            },
            app_root: self.result.external_root().map(str::to_owned),
            features: self
                .declared
                .iter()
                .map(|feature| wire::Feature {
                    id: feature.id.clone(),
                    name: feature.name.clone(),
                    default: feature.default,
                    chosen: self.features.contains(&feature.id),
                })
                .collect(),
            steps: self
                .steps
                .iter()
                .map(|planned| wire::Step {
                    id: planned.step.id.clone(),
                    title: planned.step.title.clone(),
                    kind: planned.step.kind.name().into(),
                    phase: match planned.step.phase {
                        StepPhase::Prepare => wire::Phase::Prepare,
                        StepPhase::Finish => wire::Phase::Finish,
                    },
                    needs: planned.step.needs.iter().map(ToString::to_string).collect(),
                    optional: planned.step.optional,
                    action: if planned.run {
                        wire::Action::Run
                    } else {
                        wire::Action::Skip
                    },
                    reason: planned.reason.clone(),
                })
                .collect(),
            warnings: self.warnings.clone(),
        }
    }

    /// The plan as text, one line per step.
    ///
    /// ```
    /// # use vibe_app::{Plan, PackageCoordinate};
    /// # use vibe_core::manifest::ResultDecl;
    /// let plan = Plan {
    ///     package: PackageCoordinate { group: "org.example".into(), name: "tool".into(), version: "1.0.0".into() },
    ///     option: "src".into(), platform: "linux".into(), arch: "x86_64".into(),
    ///     result: ResultDecl::owned(), declared: vec![], features: vec![], steps: vec![], warnings: vec![],
    /// };
    /// assert!(plan.render().starts_with("plan for org.example/tool@1.0.0"));
    /// ```
    pub fn render(&self) -> String {
        let result = match self.result.external_root() {
            Some(root) => format!("external `{root}`"),
            None => "owned".into(),
        };
        let mut text = format!(
            "plan for {}: installer `{}` ({}/{}), result {result}\n",
            self.package.key(),
            self.option,
            self.platform,
            self.arch
        );
        if !self.declared.is_empty() {
            let features: Vec<String> = self
                .declared
                .iter()
                .map(|feature| {
                    let mark = if self.features.contains(&feature.id) {
                        "x"
                    } else {
                        " "
                    };
                    format!("[{mark}] {}", feature.id)
                })
                .collect();
            text.push_str(&format!("  features: {}\n", features.join(" ")));
        }
        for (index, planned) in self.steps.iter().enumerate() {
            let step = &planned.step;
            let action = match &planned.reason {
                Some(reason) => format!("skip ({reason})"),
                None => "run".into(),
            };
            let needs = if step.needs.is_empty() {
                String::new()
            } else {
                let needs: Vec<String> = step.needs.iter().map(ToString::to_string).collect();
                format!(" [needs {}]", needs.join(", "))
            };
            let phase = if step.phase == StepPhase::Finish {
                " [finish]"
            } else {
                ""
            };
            text.push_str(&format!(
                "  {}. {} {} ({}): {}{phase}{needs}\n",
                index + 1,
                action,
                step.id,
                step.kind.name(),
                step.title
            ));
        }
        for warning in &self.warnings {
            text.push_str(&format!("warning: {warning}\n"));
        }
        text
    }
}
