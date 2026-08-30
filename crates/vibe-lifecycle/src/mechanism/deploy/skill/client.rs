//! The closed standalone-skill client vocabulary — §6.3.0.5's three
//! destination rows, and everything that differs between them.
//!
//! The projection family landed the lesson first (§6.3.0's R8-CLIENTS-
//! PACKAGE ratification 2: "Three provider identities share one closed
//! implementation"): what differs between `deploy:claude-skill`,
//! `deploy:codex-skill` and `deploy:opencode-skill` is DATA — a skills
//! root, an agent — so it lives here and the adapter in the parent cell is
//! written once. Which client a target gets is decided by the SELECTED
//! PROVIDER's handler name, never by a config member: §6.3.0.2's own
//! reason, a stringly `client = "claude"` would put routing inside the
//! very table the routing law sits above.
//!
//! The skills roots are the §6.3 commissioning matrix's user-scope rows,
//! and the ONLY place the provider learns them from is the pure
//! injected-home helper on the closed [`Agent`] surface — §6.3.1.7's
//! "lifecycle providers call only the pure helpers". The forward-slashed
//! spellings beside them are for RESOURCE IDENTITIES (which are strings,
//! never paths), not for joining to a home.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use vibe_agent_projection::agents::Agent;

use crate::mechanism::{
    BUILTIN_CLAUDE_SKILL_PIN, BUILTIN_CODEX_SKILL_PIN, BUILTIN_OPENCODE_SKILL_PIN,
};

/// The entry document every client's skill loader reads.
pub(crate) const ENTRY_DOCUMENT: &str = "SKILL.md";

/// One commissioning client, as a standalone-skill destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillClient {
    Claude,
    Codex,
    OpenCode,
}

impl SkillClient {
    /// All three clients, in the registry's own row order — the closed
    /// vocabulary as one value, for the suites that walk it.
    #[cfg(test)]
    pub(crate) const ALL: [Self; 3] = [Self::Claude, Self::Codex, Self::OpenCode];

    /// The client's own word, as plan summaries, evidence and the
    /// fingerprint's domain separation spell it.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
        }
    }

    /// The reserved provider identity this client's skill row answers
    /// under — the registry's own `#claude-skill` / `#codex-skill` /
    /// `#opencode-skill` spellings, so a receipt records the exact row
    /// that reconciled the destination.
    pub(crate) const fn pin(self) -> &'static str {
        match self {
            Self::Claude => BUILTIN_CLAUDE_SKILL_PIN,
            Self::Codex => BUILTIN_CODEX_SKILL_PIN,
            Self::OpenCode => BUILTIN_OPENCODE_SKILL_PIN,
        }
    }

    /// The closed agent whose pure injected-home helper owns this client's
    /// user skills root — §6.3.1.7's single source of the destination
    /// path. The provider never joins a home itself.
    pub(crate) const fn agent(self) -> Agent {
        match self {
            Self::Claude => Agent::ClaudeCode,
            Self::Codex => Agent::Codex,
            Self::OpenCode => Agent::OpenCode,
        }
    }

    /// The home-relative, forward-slashed skills root of this client —
    /// `.claude/skills`, `.agents/skills`, `.config/opencode/skills`.
    ///
    /// Used for RESOURCE IDENTITIES (`home:.claude/skills/…`), which are
    /// strings the receipts and intents carry; the absolute destination is
    /// a different question and is answered only by the pure helper, so
    /// the two can never disagree about the root they name.
    pub(crate) const fn skills_relative(self) -> &'static str {
        match self {
            Self::Claude => ".claude/skills",
            Self::Codex => ".agents/skills",
            Self::OpenCode => ".config/opencode/skills",
        }
    }
}
