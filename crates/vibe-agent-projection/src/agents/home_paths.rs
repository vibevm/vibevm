//! The INJECTED-HOME skill path helpers — §6.3.1.7's pure path law, as
//! one narrow extension of the closed [`Agent`] surface.
//!
//! > "Pure client paths live in `vibe-agent-projection`. New helpers
//! > accept the injected home and never call ambient directory resolvers.
//! > The older public ambient agent APIs remain compatibility surfaces;
//! > lifecycle providers call only the pure helpers."
//!
//! The two helpers below are the pure twins of the ambient
//! `Agent::skills_root` / `Agent::skill_path` pair in the parent module:
//! the same three user-scope destinations, but the home arrives as a
//! PARAMETER, so a caller that names a temp home cannot reach the
//! operator's real one by construction. Nothing here reads
//! `dirs::home_dir`, `HOME`, `USERPROFILE`, `CODEX_HOME` or
//! `CLAUDE_CONFIG_DIR`; each join is a pure function of its inputs, which
//! is what makes the lifecycle's "a provider never resolves a home"
//! checkable rather than promised.
//!
//! Its own file because the parent module's ambient surfaces and this
//! pure pair answer different questions for different callers — and the
//! deploy lane reads only this one.

use std::path::{Path, PathBuf};

use super::Agent;

impl Agent {
    /// The user-scope skills ROOT for an injected home: `.claude/skills`,
    /// `.agents/skills`, `.config/opencode/skills`.
    ///
    /// `None` is the unsupported-agent answer (Cursor, Claude Desktop
    /// load no filesystem skills), identical to the ambient sibling's —
    /// the vocabulary is one closed set, not two.
    ///
    /// ```
    /// use std::path::Path;
    /// use vibe_agent_projection::agents::Agent;
    ///
    /// let home = Path::new("/tmp/injected-home");
    /// assert_eq!(
    ///     Agent::ClaudeCode.user_skills_root_from_home(home).unwrap(),
    ///     home.join(".claude").join("skills"),
    /// );
    /// assert_eq!(
    ///     Agent::Codex.user_skills_root_from_home(home).unwrap(),
    ///     home.join(".agents").join("skills"),
    /// );
    /// assert_eq!(
    ///     Agent::OpenCode.user_skills_root_from_home(home).unwrap(),
    ///     home.join(".config").join("opencode").join("skills"),
    /// );
    /// // OpenCode's XDG-on-every-OS contract: the same join on any host.
    /// assert!(Agent::Cursor.user_skills_root_from_home(home).is_none());
    /// ```
    #[must_use]
    pub fn user_skills_root_from_home(self, home: &Path) -> Option<PathBuf> {
        if !self.supports_skill() {
            return None;
        }
        match self {
            Agent::ClaudeCode => Some(home.join(".claude").join("skills")),
            Agent::Codex => Some(home.join(".agents").join("skills")),
            // Same XDG-on-every-OS contract as the ambient sibling — see
            // `Agent::config_path`'s comment for the empirical record.
            Agent::OpenCode => Some(home.join(".config").join("opencode").join("skills")),
            Agent::ClaudeCodeDesktop | Agent::Cursor => None,
        }
    }

    /// The user-scope skill ENTRY path (`<root>/<name>/SKILL.md`) for an
    /// injected home.
    ///
    /// Composed over [`Agent::user_skills_root_from_home`] and the two
    /// fixed components every client's skill loader reads, so the
    /// destination a standalone-skill deployment owns and the root the
    /// client discovers can never disagree. `None` for agents with no
    /// filesystem skill loader.
    ///
    /// ```
    /// use std::path::Path;
    /// use vibe_agent_projection::agents::Agent;
    ///
    /// let entry = Agent::ClaudeCode
    ///     .user_skill_entry_from_home(Path::new("/tmp/h"), "demo")
    ///     .unwrap();
    /// assert_eq!(
    ///     entry,
    ///     Path::new("/tmp/h")
    ///         .join(".claude")
    ///         .join("skills")
    ///         .join("demo")
    ///         .join("SKILL.md"),
    /// );
    /// ```
    #[must_use]
    pub fn user_skill_entry_from_home(self, home: &Path, name: &str) -> Option<PathBuf> {
        Some(
            self.user_skills_root_from_home(home)?
                .join(name)
                .join("SKILL.md"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §6.3.1.7's law made executable: the pure helpers answer from the
    /// INJECTED home alone, so a caller that names a temp home gets that
    /// home's paths whatever the operator's real home, `HOME`,
    /// `USERPROFILE` or platform resolver happens to say. The proof is
    /// the assertion set itself: every expected path is spelled out of
    /// the injected value, and any ambient read would name a different
    /// tree.
    #[test]
    fn injected_home_skill_helpers_are_pure_joins_on_every_agent() {
        let injected = Path::new("/tmp/vibevm-test-home");
        for (agent, expected) in [
            (Agent::ClaudeCode, ".claude/skills"),
            (Agent::Codex, ".agents/skills"),
            (Agent::OpenCode, ".config/opencode/skills"),
        ] {
            let root = agent
                .user_skills_root_from_home(injected)
                .unwrap_or_else(|| panic!("{} has a user skills root", agent.as_str()));
            let mut spelled = injected.to_path_buf();
            for segment in expected.split('/') {
                spelled.push(segment);
            }
            assert_eq!(root, spelled, "{}", agent.as_str());
            let entry = agent
                .user_skill_entry_from_home(injected, "demo")
                .expect("a skill-supporting agent takes an entry");
            assert_eq!(entry, spelled.join("demo").join("SKILL.md"));
        }
        for unsupported in [Agent::Cursor, Agent::ClaudeCodeDesktop] {
            assert!(
                unsupported.user_skills_root_from_home(injected).is_none(),
                "{} loads no filesystem skills",
                unsupported.as_str(),
            );
            assert!(
                unsupported
                    .user_skill_entry_from_home(injected, "x")
                    .is_none()
            );
        }
    }
}
