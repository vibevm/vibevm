//! `[[skill]]` — the agent-installable skills a package declares
//! (PROP-018 §2.4). A package of *any* kind may ship skills for coding
//! agents; this section names which of its files constitute each skill and,
//! optionally, which agents to project it into.
//!
//! A skill is **not** a fifth package kind (the four kinds stay closed,
//! `package_ref.rs`) and **not** a subskill: subskill *delivery* (PROP-003
//! §2.5) materialises content into the project tree, whereas a skill is
//! projected *out of* the workspace, into a coding agent's own skill
//! directory (PROP-018 §2.5), by `vibe skill install`. The `[[mcp]]` sibling
//! (a bundled MCP server a package ships for agents) is reserved but not yet
//! wired (PROP-018 §6).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-018#skill-decl");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// `[[skill]]` — one agent-installable skill a package ships (PROP-018 §2.4).
///
/// `name` becomes the skill's directory name inside each target agent
/// (`.<agent>/skills/<name>/…`, the paths PROP-015 §2.6 resolves). `path`
/// is the file or directory, relative to the package root, whose contents
/// are the skill body. `agents`, when non-empty, restricts projection to
/// those agent ids (e.g. `"claude"`, `"opencode"`, `"codex"`); empty means
/// every skill-supporting agent. Agent ids are validated downstream in
/// `vibe-mcp`, which owns the agent vocabulary — `vibe-core` keeps them as
/// opaque strings so the manifest layer stays free of the agent enum.
///
/// ```
/// use vibe_core::manifest::SkillDecl;
///
/// let s: SkillDecl = toml::from_str(r#"
///     name = "vim"
///     path = "skills/vim"
///     description = "Drive vim from an agent"
///     agents = ["claude", "opencode"]
///     include = ["SKILL.md", "references/**/*.md"]
/// "#).unwrap();
/// assert_eq!(s.name, "vim");
/// assert_eq!(s.path.to_str(), Some("skills/vim"));
/// assert_eq!(s.agents.len(), 2);
/// assert_eq!(s.include.len(), 2);
///
/// // `description`, `agents`, `include` are optional; a bare skill targets
/// // every skill-supporting agent and projects its whole `path` tree.
/// let bare: SkillDecl =
///     toml::from_str(r#"name = "q"
/// path = "q/SKILL.md""#).unwrap();
/// assert!(bare.description.is_none());
/// assert!(bare.agents.is_empty());
/// assert!(bare.include.is_empty()); // empty → whole path tree (§2.6)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillDecl {
    /// The skill id — becomes its directory name inside each agent
    /// (`.<agent>/skills/<name>/…`).
    pub name: String,
    /// File or directory (relative to the package root) whose contents are
    /// the skill body projected into the agent.
    pub path: PathBuf,
    /// Optional human description, surfaced by `vibe skill list`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Agent ids to project into; empty means every skill-supporting agent
    /// (PROP-015 §2.6 — `claude` / `opencode` / `codex`). Validated in
    /// `vibe-mcp`, not here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
    /// Glob patterns (relative to `path`) selecting which files to project
    /// (PROP-015 §2.8 `#skill-include`). Empty → the whole `path` tree, the
    /// §2.6 default. Lets a skill pick specific files out of a noisy subtree
    /// (e.g. a bridged upstream repo full of unrelated content). Matching is
    /// performed downstream in `vibe-mcp`; `vibe-core` keeps the patterns as
    /// opaque strings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,
}
