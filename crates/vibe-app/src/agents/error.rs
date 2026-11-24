//! Choosing, invoking and answering an agent hook (`PROP-064`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#selection");

use specmark::spec;
use thiserror::Error;

/// Exact launcher-spelling refusal under `PROP-064#presets`, not an alias.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-064#presets")]
pub(super) const CLAUDE_PRESET_HINT: &str =
    "preset claude is not built in · did you mean claude-code?";
/// Exact launcher-spelling refusal under `PROP-064#presets`, not an alias.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-064#presets")]
pub(super) const QWEN_PRESET_HINT: &str = "preset qwen is not built in · did you mean qwen-code?";

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-064#selection")]
fn unknown_headline(name: &str, known: &str) -> String {
    if known.is_empty() {
        format!("agent {name} is not defined in agents.toml")
    } else {
        format!(
            "agent `{name}` is neither defined in agents.toml nor a built-in preset; known: {known}"
        )
    }
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition")]
fn definition_headline(name: &str, reason: &str) -> String {
    match reason {
        CLAUDE_PRESET_HINT | QWEN_PRESET_HINT => reason.to_owned(),
        _ => format!("agent `{name}` cannot be used: {reason}"),
    }
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-064#presets")]
fn definition_fix(reason: &str) -> &'static str {
    match reason {
        CLAUDE_PRESET_HINT => "name preset `claude-code`",
        QWEN_PRESET_HINT => "name preset `qwen-code`",
        _ => "give the definition a command and both argument templates, or name a preset",
    }
}

// Internal operation markers are stripped before displaying the cause. Only
// these exact prefixes select a repair; arbitrary external reasons stay neutral.
pub(super) const CONFIG_LOAD: &str = "loading agent registry: ";
pub(super) const CONFIG_SERIALIZE: &str = "serializing agent registry: ";
pub(super) const CONFIG_WRITE: &str = "writing agent registry: ";

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition")]
fn config_parts(reason: &str) -> (&'static str, &'static str, &str) {
    if let Some(cause) = reason.strip_prefix(CONFIG_SERIALIZE) {
        (
            "could not serialize agent registry",
            "check that the registry fields can be serialized",
            cause,
        )
    } else if let Some(cause) = reason.strip_prefix(CONFIG_WRITE) {
        (
            "could not write agent registry",
            "check write permissions and that the target can be atomically replaced",
            cause,
        )
    } else if let Some(cause) = reason.strip_prefix(CONFIG_LOAD) {
        (
            "not a valid agent registry",
            "correct the TOML syntax and field types in the file",
            cause,
        )
    } else {
        (
            "agent registry operation failed",
            "check the reported cause and registry settings",
            reason,
        )
    }
}

/// The agent registry, a hook's consent, its run and its parking.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-064#selection")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#consent")]
pub enum AgentError {
    #[error(
        "{headline} (violates spec://org.vibevm.core/vibevm/common/PROP-064#selection; fix: {fix}){cause}",
        headline = unknown_headline(.name, .known),
        fix = if .known.is_empty() {
            "`vibe agent list` or `vibe agent presets`"
        } else {
            "name a listed agent, see `vibe agent list` and `vibe agent presets`"
        },
        cause = if .known.is_empty() { ": no agent is defined there" } else { "" }
    )]
    Unknown { name: String, known: String },
    #[error(
        "agent `{name}` is already defined in agents.toml (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; \
         fix: remove it first with `vibe agent remove {name}` or choose another name)"
    )]
    Exists { name: String },
    #[error(
        "{headline}: `{path}` (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; fix: {fix}): {cause}",
        headline = config_parts(.reason).0,
        fix = config_parts(.reason).1,
        cause = config_parts(.reason).2
    )]
    Config { path: String, reason: String },
    #[error(
        "{headline} (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; fix: {fix})",
        headline = definition_headline(.name, .reason),
        fix = definition_fix(.reason)
    )]
    Definition { name: String, reason: String },
    #[error(
        "`{file}` carries a trust entry vibe cannot read: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust; \
         fix: correct the entry — each one is a group (`org.example`) or a subtree \
         (`org.example.*`) in `trusted_groups` / `untrusted_groups`)"
    )]
    Trust { file: String, reason: String },
    #[error(
        "the external agent registration is invalid: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-064#external-registration; \
         fix: set VIBE_AGENT_COMMAND to an executable and VIBE_AGENT_ARGS to a JSON array of strings)"
    )]
    Registration { reason: String },
    #[error(
        "agent step `{step}` of `{key}` needs consent before agent `{agent}` runs its prompt: {reason}; nothing was run \
         (violates spec://org.vibevm.core/vibevm/common/PROP-064#consent; fix: review the prompt shown above and rerun \
         with --assume-yes, or list the package group in trusted_groups of agents.toml)"
    )]
    ConsentRequired {
        key: String,
        step: String,
        agent: String,
        reason: String,
    },
    #[error(
        "agent `{agent}` failed step `{step}`: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-064#answer; \
         fix: read the transcript and result in `{run}`, correct the cause and rerun with --resume)"
    )]
    Failed {
        step: String,
        agent: String,
        reason: String,
        run: String,
    },
    #[error(
        "agent step `{}` of `{}` is parked for the hosting agent: perform the task in {}, write result.json \
         into `{}` and run `{}` (spec://org.vibevm.core/vibevm/common/PROP-064#parking; the work directory `{}` is kept)",
        .0.step, .0.key, .0.task_files.join(", "), .0.run, .0.resume, .0.work
    )]
    Parked(Box<ParkedStep>),
    #[error(
        "{action} `{path}` failed: {source} (violates spec://org.vibevm.core/vibevm/common/PROP-064#envelope; \
         fix: check permissions and free space)"
    )]
    Io {
        action: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// A step handed to the hosting agent (`PROP-064#parking`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#parking")]
pub struct ParkedStep {
    pub key: String,
    pub step: String,
    /// The run identifier the outbox directory carries.
    pub run_id: String,
    /// The step's run directory, where `result.json` is expected.
    pub run: String,
    /// Every parked task file of this run.
    pub task_files: Vec<String>,
    /// The command the hosting agent re-invokes after answering.
    pub resume: String,
    /// The kept work directory.
    pub work: String,
}

pub(crate) fn io(
    action: &'static str,
    path: &std::path::Path,
    source: std::io::Error,
) -> AgentError {
    AgentError::Io {
        action,
        path: path.display().to_string(),
        source,
    }
}
