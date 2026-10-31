//! Choosing, invoking and answering an agent hook (`PROP-064`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#selection");

use specmark::spec;
use thiserror::Error;

/// The agent registry, a hook's consent, its run and its parking.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-064#selection")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#consent")]
pub enum AgentError {
    #[error(
        "agent `{name}` is neither defined in agents.toml nor a built-in preset; known: {known} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-064#selection; fix: name a listed agent, \
         see `vibe agent list` and `vibe agent presets`)"
    )]
    Unknown { name: String, known: String },
    #[error(
        "agent `{name}` is already defined in agents.toml (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; \
         fix: remove it first with `vibe agent remove {name}` or choose another name)"
    )]
    Exists { name: String },
    #[error(
        "`{path}` is not a valid agent registry: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; \
         fix: correct the file, or remove it and select a preset with `vibe agent use PRESET`)"
    )]
    Config { path: String, reason: String },
    #[error(
        "agent `{name}` cannot be used: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; \
         fix: give the definition a command and both argument templates, or name a preset)"
    )]
    Definition { name: String, reason: String },
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
