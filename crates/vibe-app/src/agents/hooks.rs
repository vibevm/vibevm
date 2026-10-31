//! What an operation knows about agent hooks (`PROP-064`): the chosen agent,
//! whether `vibe` itself runs inside a hosting agent, the consent posture,
//! and the caller's callbacks that show a prompt and ask the question.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#consent");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#parking");

use specmark::spec;

use super::SelectedAgent;

/// The prompt of one agent step as the user sees it before consenting.
#[derive(Debug, Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#consent")]
pub struct AgentPrompt<'a> {
    pub step: &'a str,
    pub title: &'a str,
    /// The agent id (`external` for a registration).
    pub agent: &'a str,
    /// The command line with `{prompt}` left as written.
    pub command: &'a [String],
    /// The full prompt after template expansion.
    pub prompt: &'a str,
}

/// What a caller is told about agent steps.
#[derive(Debug, Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#answer")]
pub enum AgentNotice<'a> {
    /// Shown before any agent starts, whether or not a question follows.
    Prompt(AgentPrompt<'a>),
    /// The answer of a finished agent step (`result.json` or the exit).
    Answer {
        step: &'a str,
        ok: bool,
        message: Option<&'a str>,
    },
}

/// The attended consent question: `true` to run the agent.
pub type ConsentPrompt<'a> = &'a dyn Fn(&AgentPrompt<'_>) -> Result<bool, String>;

/// The agent posture of one operation; the default has no agent at all.
#[derive(Clone, Copy, Default)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#selection")]
pub struct AgentHooks<'a> {
    /// The agent `select_agent` chose, if any.
    pub selected: Option<&'a SelectedAgent>,
    /// `vibe` runs inside a hosting agent (`PROP-054#AGENT-HANDSHAKE`):
    /// agent steps are parked for it instead of spawned.
    pub hosted: bool,
    /// `--assume-yes` (implied by `--unattended`).
    pub assume_yes: bool,
    /// Groups whose prompts need no question (`agents.toml`).
    pub trusted_groups: &'a [String],
    /// Prompts and answers are shown here.
    pub notify: Option<&'a dyn Fn(&AgentNotice<'_>)>,
    /// Present only in an attended terminal.
    pub confirm: Option<ConsentPrompt<'a>>,
    /// The exact command a hosting agent re-invokes after answering.
    pub resume_command: Option<&'a str>,
}

impl AgentHooks<'_> {
    /// Whether an agent step can be delivered at all: spawned or parked.
    pub fn available(&self) -> bool {
        self.hosted || self.selected.is_some()
    }

    /// Whether prompts of `group` run without the question.
    pub fn trusts(&self, group: &str) -> bool {
        self.trusted_groups.iter().any(|trusted| trusted == group)
    }
}
