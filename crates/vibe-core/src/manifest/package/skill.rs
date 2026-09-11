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

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::manifest::declarant_path::{declarant_path, is_windows_device_name};

use super::embedded_source::valid_embedded_source_name;

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
    /// Optional immutable `[[embedded_source]]` identity. When present,
    /// `path` and `include` select the skill body from that authenticated
    /// source instead of from the package tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
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
    /// Additional immutable upstream resources merged below the local
    /// adapter-owned skill body. The manifest-level validator resolves each
    /// resource's source identity.
    #[serde(default, rename = "resource", skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<SkillResourceDecl>,
}

/// One selected subtree from an authenticated `[[embedded_source]]` merged
/// into an adapter-owned skill at projection time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillResourceDecl {
    pub embedded_source: String,
    /// Path relative to the embedded source root.
    pub path: PathBuf,
    /// Explicit selection below `path`. Remote whole-tree projection is not
    /// an implicit default.
    pub include: Vec<String>,
    /// Destination below the projected skill's `references/` directory.
    pub target: PathBuf,
}

impl SkillDecl {
    /// Validate the Agent Skills identity and declarant-root-relative source
    /// path before either standalone or automatic projection can observe it.
    pub fn validate(&self) -> Result<(), String> {
        if !Self::valid_name(&self.name) {
            return Err(format!(
                "[[skill]] name `{}` must be portable lowercase kebab-case, 1..64 bytes, exactly one normal path component, and not a Windows device name",
                self.name
            ));
        }
        if !valid_declarant_path(&self.path) {
            return Err(format!(
                "[[skill]] `{}` path `{}` must be a non-empty declarant-root-relative normal path with forward slashes and no root, drive prefix, `.`, or `..` component",
                self.name,
                self.path.display()
            ));
        }
        if let Some(source) = &self.source {
            if !valid_embedded_source_name(source) {
                return Err(format!(
                    "[[skill]] `{}` source `{source}` must name one portable [[embedded_source]]",
                    self.name
                ));
            }
            if self.include.is_empty() {
                return Err(format!(
                    "[[skill]] `{}` selects an external source and must declare a non-empty include list",
                    self.name
                ));
            }
            if !self.resources.is_empty() {
                return Err(format!(
                    "[[skill]] `{}` is a direct external skill and cannot also declare [[skill.resource]]; use a local adapter skill for composed resources",
                    self.name
                ));
            }
        }
        for resource in &self.resources {
            resource.validate(&self.name)?;
        }
        Ok(())
    }

    /// Whether `name` is a portable Agent Skills directory identity.
    ///
    /// Kept on the manifest type so receipt readers and filesystem adapters
    /// apply the exact same grammar as authored-manifest validation.
    #[must_use]
    pub fn valid_name(name: &str) -> bool {
        valid_skill_name(name)
    }

    /// The shared Windows device-name table (single source of truth),
    /// reachable through the public manifest surface so receipt containment
    /// delegates instead of maintaining a duplicate.
    #[must_use]
    pub fn is_windows_device_name(component: &str) -> bool {
        is_windows_device_name(component)
    }
}

impl SkillResourceDecl {
    fn validate(&self, skill_name: &str) -> Result<(), String> {
        if !valid_embedded_source_name(&self.embedded_source) {
            return Err(format!(
                "[[skill.resource]] in `{skill_name}` embedded_source `{}` must be a portable source name",
                self.embedded_source
            ));
        }
        if !valid_declarant_path(&self.path) {
            return Err(format!(
                "[[skill.resource]] in `{skill_name}` path `{}` must be a non-empty embedded-source-relative normal path",
                self.path.display()
            ));
        }
        if self.include.is_empty() || self.include.iter().any(|pattern| !valid_include(pattern)) {
            return Err(format!(
                "[[skill.resource]] in `{skill_name}` include must be a non-empty list of safe source-relative glob patterns"
            ));
        }
        if !valid_declarant_path(&self.target) || !is_below_references(&self.target) {
            return Err(format!(
                "[[skill.resource]] in `{skill_name}` target `{}` must be a normal path strictly below `references/`",
                self.target.display()
            ));
        }
        Ok(())
    }
}

fn valid_skill_name(name: &str) -> bool {
    // ASCII first: every later slice is then on a character boundary, so a
    // non-ASCII spelling such as `éé` refuses instead of panicking.
    if !name.is_ascii() {
        return false;
    }
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    if is_windows_device_name(name) {
        return false;
    }
    let mut segment_len = 0usize;
    for byte in bytes {
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            segment_len += 1;
        } else if *byte == b'-' && segment_len > 0 {
            segment_len = 0;
        } else {
            return false;
        }
    }
    segment_len > 0
}

fn valid_declarant_path(path: &Path) -> bool {
    declarant_path(path).is_ok()
}

fn valid_include(pattern: &str) -> bool {
    pattern.trim() == pattern
        && !pattern.is_empty()
        && !pattern.starts_with('/')
        && !pattern.chars().any(|ch| matches!(ch, '\\' | ':'))
        && pattern
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn is_below_references(path: &Path) -> bool {
    let mut components = path.components();
    matches!(
        (components.next(), components.next()),
        (
            Some(std::path::Component::Normal(first)),
            Some(std::path::Component::Normal(_))
        ) if first == "references"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decl(name: &str, path: &str) -> SkillDecl {
        SkillDecl {
            name: name.into(),
            source: None,
            path: path.into(),
            description: None,
            agents: Vec::new(),
            include: Vec::new(),
            resources: Vec::new(),
        }
    }

    #[test]
    fn agent_skill_name_and_source_path_are_closed() {
        assert!(decl("review-code", "skills/review-code").validate().is_ok());
        for name in [
            "",
            "Review",
            "review_code",
            "-review",
            "review-",
            "a--b",
            "..",
            "a/b",
            "con",
            "nul",
            "com1",
            "lpt9",
        ] {
            assert!(decl(name, "skills/review").validate().is_err(), "{name}");
        }
        for path in [
            "",
            ".",
            "..",
            "../outside",
            "/outside",
            "C:/outside",
            "//server/share",
            "skills\\escape",
            "skills//escape",
            "skills/./escape",
            "skills/escape/",
            "skills/file:stream",
            "skills/COM1",
            "skills/aux",
            "skills/trailing.",
            "skills/trailing ",
            // Extension-bearing device aliases judge the basename stem.
            "skills/CON.txt",
            "skills/NUL.md",
            "skills/COM1.json",
            "skills/LPT9.log",
            // Console/clock devices and superscript port spellings.
            "skills/CONIN$",
            "skills/CONOUT$",
            "skills/CLOCK$",
            "skills/COM¹",
            "skills/LPT²/body",
        ] {
            assert!(decl("review", path).validate().is_err(), "{path}");
        }
    }

    #[test]
    fn device_name_table_judges_stems_extensions_and_superscripts() {
        use super::is_windows_device_name;
        for device in [
            "con",
            "CON",
            "PRN",
            "aux",
            "nul",
            "com1",
            "LPT9",
            "CON.txt",
            "NUL.md",
            "COM1.json",
            "LPT9.log",
            "con.in.txt",
            "CONIN$",
            "conout$",
            "CLOCK$",
            "clock$.tmp",
            "COM¹",
            "com²",
            "LPT³.cfg",
        ] {
            assert!(is_windows_device_name(device), "{device}");
            assert!(SkillDecl::is_windows_device_name(device), "{device}");
        }
        for ordinary in [
            "context",
            "console",
            "component",
            "com",
            "lpt",
            "com10",
            "com0",
            "lpt0",
            "confidence.md",
            "com¹⁰",
        ] {
            assert!(!is_windows_device_name(ordinary), "{ordinary}");
        }
    }

    #[test]
    fn non_ascii_skill_names_refuse_without_panicking() {
        for name in ["éé", "demo\u{0301}", "スキル", "demo❤"] {
            let result = std::panic::catch_unwind(|| decl(name, "skills/x").validate());
            assert!(matches!(result, Ok(Err(_))), "{name:?}");
        }
    }

    #[test]
    fn external_skill_and_adapter_resource_boundaries_are_closed() {
        let mut direct = decl("review", "skills/review");
        direct.source = Some("upstream".into());
        assert!(direct.validate().unwrap_err().contains("non-empty include"));
        direct.include = vec!["**/*".into()];
        direct.validate().unwrap();

        let mut adapter = decl("review", "skills/review");
        adapter.resources.push(SkillResourceDecl {
            embedded_source: "upstream".into(),
            path: "docs".into(),
            include: vec!["**/*.md".into()],
            target: "references/upstream".into(),
        });
        adapter.validate().unwrap();
        adapter.resources[0].target = "SKILL.md".into();
        assert!(
            adapter
                .validate()
                .unwrap_err()
                .contains("below `references/`")
        );
    }
}
