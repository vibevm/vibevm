//! The personal user project's root admission and reading of one narrow
//! settings contract (PROP-074 §2.1).
//!
//! This module owns what both lifecycle halves share and nothing else: the
//! pure derivation `#USER-PROJECT-ROOT-PATH` gives exactly one location,
//! `#USER-PROJECT-ROOT-ADMISSION` admits only an absolute settings value, and
//! the vocabulary of `#USER-PROJECT-ROOT-REPORT` is closed. No filesystem and
//! no environment read happens here — the workspace seam writes and reads,
//! the CLI decides — and no wire type is defined: the domain observation is
//! mapped onto the generated report by the surface.
//!
//! The refusal is a standalone `Error` implementation so it attaches to any
//! `anyhow` chain without adding a central enum arm.
//!
//! ```
//! use std::path::{Path, PathBuf};
//! use vibe_core::user_project::{admit_root, Command, Effects, Publication, Report, State};
//!
//! let root = admit_root(Some(PathBuf::from("/opt/vibe"))).unwrap();
//! assert_eq!(root, PathBuf::from("/opt/vibe/user-project"));
//!
//! let report = Report::success(
//!     Command::Status,
//!     Some(root.clone()),
//!     State::Absent,
//!     Effects::None,
//!     Publication::NotStarted,
//! );
//! assert!(report.is_success());
//! assert!(admit_root(None).is_err());
//! ```

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-ROOT-REPORT");

use specmark::spec;
use std::path::{Path, PathBuf};

use crate::settings::user_project_root_from;

pub mod memory;

/// The report's protocol spelling (`#USER-PROJECT-ROOT-REPORT`).
pub const PROTOCOL: &str = "vibe-user-project/1";

/// Which verb produced an observation.
///
/// ```
/// use vibe_core::user_project::Command;
/// assert_eq!(Command::Init.as_str(), "user:init");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// `vibe user init`.
    Init,
    /// `vibe user status`.
    Status,
}

impl Command {
    /// The wire spelling.
    #[must_use]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-ROOT-REPORT")]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Init => "user:init",
            Self::Status => "user:status",
        }
    }
}

/// What the root is, as far as this operation may tell
/// (`#USER-PROJECT-INITIALIZED-CRITERION`): never a whole-project verdict.
///
/// ```
/// use vibe_core::user_project::State;
/// assert_ne!(State::Absent, State::Unavailable);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// No root, or a truly empty directory. Normal; not a permission to
    /// create.
    Absent,
    /// A `[project]` manifest the ordinary parser accepts. `[workspace]`
    /// beside it is admitted; members are not traversed.
    Initialized,
    /// Anything not admittable: relative settings, a file, a link, residue,
    /// an invalid project.
    Unavailable,
}

/// How far the invocation got (`#USER-PROJECT-ROOT-REPORT-EFFECTS`).
///
/// ```
/// use vibe_core::user_project::Outcome;
/// assert_ne!(Outcome::Created, Outcome::AlreadyInitialized);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Init published a new project.
    Created,
    /// An admitted project was left untouched.
    AlreadyInitialized,
    /// Status answered.
    Observed,
    /// A refusal: nothing was written.
    Refused,
    /// A mutating run that failed after it began.
    Incomplete,
}

/// The surviving owned mutations of **this** invocation — pre-existing
/// residue never counts.
///
/// ```
/// use vibe_core::user_project::Effects;
/// assert_ne!(Effects::Unknown, Effects::None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effects {
    /// No surviving owned mutation is established and none is uncertain.
    None,
    /// Surviving owned changes are established.
    Changed,
    /// Their survival cannot be determined.
    Unknown,
}

/// This invocation's initialization transaction, not the mere existence of a
/// manifest.
///
/// ```
/// use vibe_core::user_project::Publication;
/// assert_ne!(Publication::Committed, Publication::RecoveryRequired);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Publication {
    /// No mutating initialization began.
    NotStarted,
    /// Its effects were proven undone.
    Restored,
    /// Manifest publication succeeded.
    Committed,
    /// Mutation began and neither completion nor full restoration holds.
    RecoveryRequired,
}

/// One observation. The surface maps it onto the generated
/// `user_project_report` wire type; `root` is the admitted absolute path and
/// is `None` exactly when settings are unresolved or relative.
///
/// ```
/// use vibe_core::user_project::{Command, Report};
/// let report = Report::refused(Command::Init, None, "unavailable", "select an absolute root");
/// assert!(!report.is_success());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The verb.
    pub command: Command,
    /// The admitted absolute root.
    pub root: Option<PathBuf>,
    /// The root's state.
    pub state: State,
    /// How far the verb got.
    pub outcome: Outcome,
    /// This invocation's surviving effects.
    pub effects: Effects,
    /// This invocation's publication.
    pub publication: Publication,
    /// The stable refusal reason.
    pub reason: Option<String>,
    /// The actionable next step.
    pub remedy: Option<String>,
}

impl Report {
    /// A finished, successful observation.
    #[must_use]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-ROOT-REPORT")]
    pub fn success(
        command: Command,
        root: Option<PathBuf>,
        state: State,
        effects: Effects,
        publication: Publication,
    ) -> Self {
        let outcome = match (command, state) {
            (Command::Status, _) => Outcome::Observed,
            (Command::Init, State::Initialized) => Outcome::AlreadyInitialized,
            (Command::Init, _) => Outcome::Created,
        };
        Self {
            command,
            root,
            state,
            outcome,
            effects,
            publication,
            reason: None,
            remedy: None,
        }
    }

    /// A refusal before any write.
    #[must_use]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-ROOT-REPORT")]
    pub fn refused(
        command: Command,
        root: Option<PathBuf>,
        reason: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            command,
            root,
            state: State::Unavailable,
            outcome: Outcome::Refused,
            effects: Effects::None,
            publication: Publication::NotStarted,
            reason: Some(reason.into()),
            remedy: Some(remedy.into()),
        }
    }

    /// Turn this report into the failure of a run that began mutating: it
    /// keeps root and state and takes the incomplete outcome.
    #[must_use]
    pub fn refused_with(mut self, reason: impl Into<String>) -> Self {
        self.outcome = Outcome::Incomplete;
        self.reason = Some(reason.into());
        self
    }

    /// Whether the verb finished its job (`ok`).
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self.outcome, Outcome::Created | Outcome::AlreadyInitialized)
            || (matches!(self.outcome, Outcome::Observed)
                && !matches!(self.state, State::Unavailable))
    }
}

/// One typed refusal of the personal project, travelling the error chain to
/// the surface that stamps its report.
///
/// ```
/// use vibe_core::user_project::{Command, Report, UserProjectRefusal};
/// let error = UserProjectRefusal::new(Report::refused(Command::Init, None, "unavailable", "retry"));
/// assert_eq!(error.reason(), "unavailable");
/// ```
#[derive(Debug, Clone)]
pub struct UserProjectRefusal {
    report: Report,
}

impl UserProjectRefusal {
    /// Wrap a refused report. Its `reason`/`remedy` are its display text, so
    /// correcting the report later also corrects what is shown.
    #[must_use]
    pub fn new(report: Report) -> Self {
        Self { report }
    }

    /// The observation this refusal stamps.
    #[must_use]
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// The report, for correction as cleanup learns more.
    pub fn report_mut(&mut self) -> &mut Report {
        &mut self.report
    }

    /// The sentence, for an ordinary error display.
    #[must_use]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-ROOT-REPORT")]
    pub fn reason(&self) -> &str {
        // A refusal always carries one: `refused` sets it.
        self.report
            .reason
            .as_deref()
            .unwrap_or("the user project is unavailable")
    }
}

impl std::fmt::Display for UserProjectRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.report.remedy {
            Some(remedy) if !remedy.is_empty() => write!(f, "{} ({remedy})", self.reason()),
            _ => f.write_str(self.reason()),
        }
    }
}

impl std::error::Error for UserProjectRefusal {}

/// The selected settings value, or the refusal of one that cannot address a
/// personal project.
///
/// Unresolved settings and a relative value both refuse with a `None` root —
/// never a cwd guess — and the refusal keeps a stable reason
/// (`#USER-PROJECT-ROOT-PATH`, `#USER-PROJECT-ROOT-ADMISSION`).
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-ROOT-ADMISSION")]
pub fn selected_settings(settings: Option<PathBuf>) -> Result<PathBuf, UserProjectRefusal> {
    let (reason, remedy) = match settings {
        Some(settings) if settings.is_absolute() => return Ok(settings),
        Some(_) => (
            Unresolved::Relative,
            "set VIBE_SETTINGS to an absolute path, or make the user home absolute",
        ),
        None => (
            Unresolved::Missing,
            "set VIBE_SETTINGS to an absolute path, or make the user home resolvable",
        ),
    };
    Err(UserProjectRefusal::new(wrong_settings(reason, remedy)))
}

/// Why the settings authority yields no root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unresolved {
    /// No settings value at all.
    Missing,
    /// A relative value that a cwd change would re-point.
    Relative,
}

fn wrong_settings(reason: Unresolved, remedy: &str) -> Report {
    let reason = match reason {
        Unresolved::Missing => "the user settings root is unresolved",
        Unresolved::Relative => {
            "the user settings root is relative, so the working directory would select the \
             personal project"
        }
    };
    Report::refused(Command::Status, None, reason, remedy)
}

/// The admitted absolute personal root (`#USER-PROJECT-ROOT-ADMISSION`).
/// Pure: no filesystem, no environment.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-ROOT-ADMISSION")]
pub fn admit_root(settings: Option<PathBuf>) -> Result<PathBuf, UserProjectRefusal> {
    let settings = selected_settings(settings)?;
    if settings
        .components()
        .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(UserProjectRefusal::new(Report::refused(
            Command::Status,
            None,
            "the settings path contains ambiguous parent traversal",
            "select an absolute VIBE_SETTINGS path without '..' components",
        )));
    }
    if settings
        .components()
        .any(|part| matches!(part, std::path::Component::Normal(name) if name.to_str().is_none()))
    {
        return Err(UserProjectRefusal::new(Report::refused(
            Command::Status,
            None,
            "the selected settings path contains an unsupported non-UTF-8 component",
            "select an absolute UTF-8 settings path; no filesystem changes were made",
        )));
    }
    user_project_root_from(Some(settings)).ok_or_else(|| {
        UserProjectRefusal::new(Report::refused(
            Command::Status,
            None,
            "the personal project root could not be derived from the settings root",
            "check the settings root and retry",
        ))
    })
}

/// The wire spelling of an admitted root, in the existing machine-path
/// encoding. `None` stays `None`: an unadmitted root is never the empty
/// string.
#[must_use]
pub fn display(root: Option<&Path>) -> Option<String> {
    root.map(crate::machine_json_path)
}

#[cfg(test)]
#[path = "user_project/tests.rs"]
mod tests;
