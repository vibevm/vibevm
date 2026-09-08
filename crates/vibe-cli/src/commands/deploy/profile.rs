//! Deploy-profile selection — §7.0.5's "ONCE, in the command layer that
//! owns flags", as a pure function.
//!
//! > "explicit `--profile`, else the manifest's `default_profile`, else
//! > the exactly-one rule, else a typed refusal naming the defined
//! > profiles. Environment and secrets never choose."
//!
//! Three properties make that sentence real rather than decorative, and
//! all three are visible in the signature below:
//!
//! 1. it takes the manifest's `[deploy]` section and the flag, and NOTHING
//!    else — no environment, no settings, no filesystem. A rule that
//!    cannot read an environment variable cannot be influenced by one;
//! 2. it is the ONE implementation. `vibe deploy`, `vibe deploy --plan`
//!    and `vibe undeploy` all call it, so the three surfaces cannot
//!    disagree about which profile a project means;
//! 3. what it returns travels as DATA
//!    ([`vibe_lifecycle::DeploySelection`]) into the dispatch. The engine
//!    receives a name and an ordered target list and has no way to
//!    re-derive either.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use anyhow::{Result, bail};
use specmark::spec;
use vibe_core::manifest::{DeploySection, TargetApplicability, TargetOs};
use vibe_lifecycle::DeploySelection;

/// Forward commands project by one injected OS; explicit inverse does not.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ProfileMode {
    Forward(TargetOs),
    Inverse,
}

/// The active engine selection plus the complete authored plan decisions.
#[derive(Debug)]
pub(crate) struct ProfileResolution {
    selection: DeploySelection,
    decisions: Vec<TargetApplicability>,
    observed_os: Option<TargetOs>,
}

impl ProfileResolution {
    pub(crate) fn into_selection(self) -> DeploySelection {
        self.selection
    }

    pub(crate) fn selection(&self) -> &DeploySelection {
        &self.selection
    }

    pub(crate) fn decisions(&self) -> &[TargetApplicability] {
        &self.decisions
    }

    pub(crate) fn observed_os(&self) -> Option<TargetOs> {
        self.observed_os
    }
}

impl std::ops::Deref for ProfileResolution {
    type Target = DeploySelection;

    fn deref(&self) -> &Self::Target {
        &self.selection
    }
}

/// Resolve the profile this invocation deploys.
///
/// `Ok(None)` means the project declares no deployable surface at all —
/// no `[deploy]` section, or an empty one — and the run is the historical
/// no-op. It is deliberately NOT a refusal: `vibe deploy` has always been
/// the ninth phase verb on projects that deploy nothing, and turning that
/// into an error would make the legality rule a rule about the verb
/// rather than about choosing among profiles.
///
/// Every other unresolvable case IS a refusal, and each names the
/// profiles this project defines.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub(crate) fn resolve_profile(
    deploy: Option<&DeploySection>,
    requested: Option<&str>,
    mode: ProfileMode,
) -> Result<Option<ProfileResolution>> {
    let Some(section) = deploy.filter(|section| !section.is_empty()) else {
        if let Some(name) = requested {
            bail!(
                "`--profile {name}` was requested, but this project declares no deploy profiles \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
                 fix: declare `[deploy.profiles.{name}]` with its `targets`, or drop the flag)"
            );
        }
        return Ok(None);
    };
    let defined = defined(section);
    if let Some(name) = requested {
        let Some(_profile) = section.profiles.get(name) else {
            bail!(
                "`--profile {name}` names no profile this project defines; defined: {defined} \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
                 fix: name one of the defined profiles)"
            );
        };
        return Ok(Some(selection(section, name, mode)?));
    }
    if let Some(name) = &section.default_profile {
        let Some(_profile) = section.profiles.get(name) else {
            // A validated manifest cannot reach this; a programmatically
            // built section can, and it refuses rather than deploying
            // nothing under a name nobody defined.
            bail!(
                "the manifest's `[deploy] default_profile = \"{name}\"` names no defined \
                 profile; defined: {defined} \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; \
                 fix: correct `default_profile`)"
            );
        };
        return Ok(Some(selection(section, name, mode)?));
    }
    // The exactly-one rule, and nothing beyond it: with two profiles and
    // no declared default, the project has not said which one it means,
    // and no environment variable is allowed to say it for the operator.
    let mut profiles = section.profiles.iter();
    match (profiles.next(), profiles.next()) {
        (Some((name, _)), None) => Ok(Some(selection(section, name, mode)?)),
        _ => bail!(
            "`vibe deploy` needs a profile: this project declares no `[deploy] default_profile` \
             and defines {} profiles; defined: {defined} \
             (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: \
             pass `--profile <name>`, or declare `[deploy] default_profile` — an environment \
             variable never chooses a profile)",
            section.profiles.len(),
        ),
    }
}

/// One profile row as the resolved selection.
fn selection(section: &DeploySection, name: &str, mode: ProfileMode) -> Result<ProfileResolution> {
    let (targets, decisions, observed_os) = match mode {
        ProfileMode::Forward(os) => {
            let projected = section.project_profile(name, os)?;
            (
                projected.active_targets().to_vec(),
                projected.decisions().to_vec(),
                Some(os),
            )
        }
        ProfileMode::Inverse => (section.inverse_profile_targets(name)?, Vec::new(), None),
    };
    Ok(ProfileResolution {
        selection: DeploySelection {
            profile: name.to_owned(),
            targets,
        },
        decisions,
        observed_os,
    })
}

/// The profiles this project defines, for a refusal that names them.
fn defined(section: &DeploySection) -> String {
    if section.profiles.is_empty() {
        return "none".to_owned();
    }
    section
        .profiles
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}
