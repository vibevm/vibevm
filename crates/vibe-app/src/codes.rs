//! Stable error codes (`PROP-065#error-codes`): every error variant of this
//! crate has one snake_case code, its class and a one-line meaning, declared
//! once here. The match each table generates is exhaustive, so a new variant
//! without a code does not compile; `error_codes` is the table a CLI lists
//! and maps to exit statuses (`PROP-065#exit-codes`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#error-codes");

use specmark::spec;

use crate::agents::AgentError;
use crate::error::{
    AppError, InstallerError, LaunchError, ResolveError, StateError, StepError, StorageError,
};

/// Whether an error refused the request before any effect, or a started
/// operation failed (`PROP-065#exit-codes`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#exit-codes")]
pub enum ErrorClass {
    /// Refused before anything changed: usage, planning, needs, consent,
    /// license, parameters, a held lock.
    Refusal,
    /// A started operation failed or was interrupted; resumable when a
    /// work directory is kept.
    Failure,
    /// The run is parked for a hosting agent (`PROP-064#parking`); the
    /// same command with `--resume` continues it.
    Delegated,
}

/// One row of the error-code table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#error-codes")]
pub struct ErrorCode {
    pub code: &'static str,
    /// The error enum the code belongs to.
    pub family: &'static str,
    pub meaning: &'static str,
    pub class: ErrorClass,
}

use ErrorClass::{Delegated, Failure, Refusal};

macro_rules! codes {
    ($table:ident, $family:ident [$($delegate:ident),*] {
        $($variant:ident => $code:literal, $class:ident, $meaning:literal;)*
    }) => {
        impl $family {
            /// The stable snake_case code of this error (`PROP-065#error-codes`).
            pub fn code(&self) -> &'static str {
                match self {
                    $(Self::$delegate(inner) => inner.code(),)*
                    $(Self::$variant { .. } => $code,)*
                }
            }
        }

        const $table: &[ErrorCode] = &[$(ErrorCode {
            code: $code,
            family: stringify!($family),
            meaning: $meaning,
            class: $class,
        },)*];
    };
}

codes!(STATE, StateError [] {
    Io => "state_io", Failure, "reading or writing the settings root failed";
    Locked => "locked", Refusal, "another vibe command holds the application mutation lock";
    Index => "index_invalid", Failure, "the application index is unreadable";
    Storage => "storage_config_invalid", Failure, "storage.toml is invalid";
    UnknownLibrary => "unknown_library", Refusal, "the named storage library is not configured";
    LibraryUnavailable => "library_unavailable", Refusal, "the storage library is not attached";
});

codes!(STORAGE, StorageError [] {
    RelativePath => "library_path_relative", Refusal, "a library path is not absolute";
    InvalidId => "library_id_invalid", Refusal, "a library id is not a usable portable token";
    AlreadyListed => "library_already_listed", Refusal, "the directory is already a configured library";
    NotEmpty => "library_not_empty", Refusal, "a new library needs a new or empty directory";
    NotALibrary => "not_a_library", Refusal, "the directory holds no library marker";
    DetachDefault => "detach_default", Refusal, "the default library cannot be detached";
    NotPortable => "not_portable", Refusal, "the installation cannot move between libraries";
    SameLibrary => "same_library", Refusal, "the installation already lives in that library";
    DestinationOccupied => "destination_occupied", Refusal, "an unowned directory occupies the destination";
    Io => "storage_io", Failure, "moving or editing a library failed";
});

codes!(RESOLVE, ResolveError [] {
    Coordinate => "coordinate_invalid", Refusal, "the package coordinate is malformed";
    Registry => "registry_failed", Failure, "resolving the package from its registry failed";
    Manifest => "manifest_invalid", Refusal, "the package is not a usable application package";
    NotGloballyInstallable => "not_globally_installable", Refusal, "the package declares no installer options";
    SelectionRequired => "selection_required", Refusal, "a platform, architecture or installer must be named";
    NoOption => "no_option", Refusal, "no installer option matches the selection";
    Cancelled => "selection_cancelled", Refusal, "the attended selection was cancelled";
    Prompt => "prompt_failed", Refusal, "the attended selection could not be shown";
});

codes!(INSTALLER, InstallerError [] {
    Spawn => "spawn_failed", Failure, "a child process could not be started";
    Exited => "child_exited", Failure, "a child process exited unsuccessfully";
    Fetch => "fetch_failed", Failure, "an installer source could not be fetched";
    Digest => "digest_mismatch", Failure, "an installer source does not match its sha256";
    Offline => "offline", Failure, "an installer source needs the network under --offline";
    Archive => "archive_invalid", Failure, "an archive is unsafe or malformed";
    WindowsOnly => "windows_only", Failure, "a native installer runs only on Windows";
    Interpreter => "interpreter_unavailable", Failure, "the selected interpreter does not exist on this host";
    SourceLayout => "source_layout", Failure, "a source input cannot be laid out";
    Io => "installer_io", Failure, "writing installer files failed";
    Oversize => "size_exceeded", Failure, "an input is larger than its declared size";
    Interrupted => "child_interrupted", Failure, "a child process was interrupted by a signal";
});

codes!(STEP, StepError [] {
    UnknownFeature => "unknown_feature", Refusal, "a named feature is not declared";
    FeaturePrompt => "feature_prompt_failed", Refusal, "the attended feature question failed or was cancelled";
    UnknownProfile => "unknown_profile", Refusal, "a named profile is not declared";
    FeatureConflict => "feature_conflict", Refusal, "the feature choice contradicts the feature tree";
    ParameterInvalid => "parameter_invalid", Refusal, "a parameter value is missing or invalid";
    LicenseRequired => "license_required", Refusal, "the license was not accepted";
    LicenseUnreadable => "license_unreadable", Refusal, "the declared license file cannot be read";
    UnmetNeed => "need_unmet", Refusal, "a running step needs what this host lacks";
    AgentRequired => "agent_missing", Refusal, "a required agent step has no agent";
    NoResultStep => "plan_refused", Refusal, "no running step produces the declared result";
    Template => "template_invalid", Refusal, "a step template cannot be expanded";
    Copy => "copy_unsafe", Failure, "a copy step would leave the package or the payload";
    Failed => "step_failed", Failure, "a step failed; the work directory is kept for --resume";
    EmptyResult => "empty_result", Failure, "the steps produced no payload";
    FinishFailed => "finish_failed", Failure, "a finish step failed after the installation was recorded";
    Interrupted => "interrupted", Failure, "the installation was interrupted; the work directory is kept for --resume";
    NothingToResume => "nothing_to_resume", Refusal, "--resume found no kept work directory";
    ResumeMismatch => "resume_mismatch", Refusal, "the kept work directory belongs to another choice";
    UnknownStep => "unknown_step", Refusal, "--from-step names no step";
});

codes!(AGENT, AgentError [] {
    Unknown => "agent_unknown", Refusal, "the named agent is neither defined nor a preset";
    Exists => "agent_exists", Refusal, "an agent definition of that name already exists";
    Config => "agents_config_invalid", Refusal, "agents.toml is invalid";
    Definition => "agent_definition_invalid", Refusal, "an agent definition lacks a command or a template";
    Registration => "agent_registration_invalid", Refusal, "VIBE_AGENT_COMMAND / VIBE_AGENT_ARGS are malformed";
    ConsentRequired => "consent_required", Refusal, "an agent prompt was not consented to";
    Failed => "agent_failed", Failure, "the agent exited unsuccessfully or answered fail";
    Parked => "delegated", Delegated, "the run is parked for the hosting agent; rerun with --resume";
    Io => "agent_io", Failure, "writing the agent envelope failed";
});

codes!(LAUNCH, LaunchError [] {
    UnknownLauncher => "unknown_launcher", Refusal, "the launcher id is not declared";
    Unavailable => "launcher_unavailable", Refusal, "the launcher cannot run here";
    Spawn => "launcher_spawn_failed", Failure, "the launched program could not be started";
});

codes!(APP, AppError [State, Storage, Resolve, Installer, Launch, Step, Agent] {
    NotInstalled => "not_installed", Refusal, "the application is not installed";
    AmbiguousInstallation => "ambiguous_installation", Refusal, "several installed versions match";
    ForceUnsupported => "force_unsupported", Refusal, "--force has no authored variant to run";
    LibraryForExternal => "library_for_external", Refusal, "--library applies only to owned results";
    ForeignDirectory => "foreign_directory", Refusal, "an unowned directory occupies the payload path";
    Hook => "hook_failed", Failure, "a pre_install or post_install hook failed";
    MissingUpdate => "missing_update", Refusal, "the installation declares no update command";
    SlotChanged => "slot_changed", Refusal, "the update would change the application slot";
    Unavailable => "unavailable", Refusal, "the installation's library is unavailable";
    MissingAppRoot => "missing_app_root", Failure, "the installer did not produce its app_root";
    CommandReserved => "command_reserved", Refusal, "a public command belongs to another slot";
    UnknownSlot => "unknown_slot", Refusal, "the slot has no installed provider";
    NotProvider => "not_provider", Refusal, "the key does not provide the slot";
    ProxyImage => "proxy_image_missing", Refusal, "the PATH proxy image is missing";
    SessionChoice => "session_choice_invalid", Refusal, "a session choice is malformed";
});

/// Every code of the application subsystem, grouped by enum.
///
/// ```
/// let codes = vibe_app::error_codes();
/// assert!(codes.iter().any(|row| row.code == "need_unmet"));
/// ```
pub fn error_codes() -> Vec<ErrorCode> {
    [STATE, STORAGE, RESOLVE, INSTALLER, STEP, AGENT, LAUNCH, APP].concat()
}

impl AppError {
    /// Whether this error refused the request before any effect.
    pub fn class(&self) -> ErrorClass {
        let code = self.code();
        error_codes()
            .into_iter()
            .find(|row| row.code == code)
            .map_or(Failure, |row| row.class)
    }

    /// The step responsible, when one is.
    pub fn step(&self) -> Option<&str> {
        match self {
            Self::Step(
                StepError::Failed { step, .. }
                | StepError::FinishFailed { step, .. }
                | StepError::UnmetNeed { step, .. }
                | StepError::AgentRequired { step, .. }
                | StepError::Template { step, .. }
                | StepError::Copy { step, .. },
            ) => Some(step),
            Self::Step(StepError::Interrupted { step, .. }) => step.as_deref(),
            Self::Agent(
                AgentError::ConsentRequired { step, .. } | AgentError::Failed { step, .. },
            ) => Some(step),
            Self::Agent(AgentError::Parked(parked)) => Some(&parked.step),
            _ => None,
        }
    }

    /// The kept work directory, when one is.
    pub fn work(&self) -> Option<&str> {
        match self {
            Self::Step(
                StepError::Failed { work, .. }
                | StepError::Interrupted { work, .. }
                | StepError::NothingToResume { work, .. }
                | StepError::ResumeMismatch { work, .. },
            ) => Some(work),
            Self::Agent(AgentError::Parked(parked)) => Some(&parked.work),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_code_is_unique_snake_case_and_explained() {
        let codes = error_codes();
        let mut seen = BTreeSet::new();
        for row in &codes {
            assert!(seen.insert(row.code), "duplicated code `{}`", row.code);
            assert!(
                row.code
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'_'),
                "`{}` is not snake_case",
                row.code
            );
            assert!(!row.meaning.is_empty() && row.family.ends_with("Error"));
        }
        for named in [
            "not_installed",
            "selection_required",
            "locked",
            "need_unmet",
            "plan_refused",
            "license_required",
            "parameter_invalid",
            "step_failed",
            "agent_missing",
            "consent_required",
            "unavailable",
            "interrupted",
        ] {
            assert!(seen.contains(named), "PROP-065#error-codes names `{named}`");
        }
    }

    #[test]
    fn delegated_variants_report_the_inner_code_and_class() {
        let locked: AppError = StateError::Locked { path: "x".into() }.into();
        assert_eq!((locked.code(), locked.class()), ("locked", Refusal));
        let failed: AppError = StepError::Failed {
            key: "k".into(),
            step: "build".into(),
            kind: "recipe",
            work: "/w".into(),
            source: Box::new(AppError::UnknownSlot { slot: "s".into() }),
        }
        .into();
        assert_eq!((failed.code(), failed.class()), ("step_failed", Failure));
        assert_eq!((failed.step(), failed.work()), (Some("build"), Some("/w")));
        let interrupted: AppError = StepError::Interrupted {
            key: "k".into(),
            during: "between steps".into(),
            step: None,
            work: "/w".into(),
        }
        .into();
        assert_eq!(interrupted.step(), None);
        assert_eq!(interrupted.work(), Some("/w"));
    }
}
