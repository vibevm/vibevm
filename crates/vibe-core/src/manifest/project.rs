//! `vibe.toml` — the project manifest.
//!
//! Schema: `VIBEVM-SPEC.md` §7.5.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::{read_toml, write_toml};

/// Top-level `vibe.toml` structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    pub project: ProjectSection,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<ActiveSection>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm: Option<LlmSection>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<RegistrySection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectSection {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub authors: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmSection {
    pub default_provider: String,
    pub default_model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrySection {
    /// `git+ssh://…`, `git+https://…`, or `file://…` for a local directory.
    pub url: String,
    #[serde(default = "default_ref", skip_serializing_if = "is_default_ref")]
    pub r#ref: String,
}

fn default_ref() -> String {
    "main".into()
}

fn is_default_ref(r: &String) -> bool {
    r == "main"
}

impl ProjectManifest {
    pub const FILENAME: &'static str = "vibe.toml";

    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        read_toml(path)
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        write_toml(path, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_roundtrip() {
        let raw = r#"
[project]
name = "demo"
version = "0.0.1"
"#;
        let m: ProjectManifest = toml::from_str(raw).unwrap();
        assert_eq!(m.project.name, "demo");
        assert_eq!(m.project.version, "0.0.1");
        assert!(m.active.is_none());
    }

    #[test]
    fn full_roundtrip() {
        let raw = r#"
[project]
name = "my-telegram-client"
version = "0.0.1"
authors = ["Oleg <oleg@example.com>"]

[active]
stack = "rust-cli"

[llm]
default_provider = "anthropic"
default_model = "claude-sonnet-4-7"
api_key_env = "ANTHROPIC_API_KEY"

[registry]
url = "git@gitverse.ru:anarchic/vibespecs.git"
ref = "main"
"#;
        let m: ProjectManifest = toml::from_str(raw).unwrap();
        assert_eq!(m.project.authors, vec!["Oleg <oleg@example.com>"]);
        assert_eq!(m.active.as_ref().unwrap().stack.as_deref(), Some("rust-cli"));
        assert_eq!(m.llm.as_ref().unwrap().default_provider, "anthropic");
        assert_eq!(
            m.registry.as_ref().unwrap().url,
            "git@gitverse.ru:anarchic/vibespecs.git"
        );

        // Roundtrip through TOML.
        let rendered = toml::to_string_pretty(&m).unwrap();
        let back: ProjectManifest = toml::from_str(&rendered).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn rejects_unknown_fields() {
        let raw = r#"
[project]
name = "demo"
version = "0.0.1"
mystery_field = true
"#;
        assert!(toml::from_str::<ProjectManifest>(raw).is_err());
    }

    #[test]
    fn read_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vibe.toml");
        std::fs::write(
            &path,
            r#"[project]
name = "disk-demo"
version = "0.1.0"
"#,
        )
        .unwrap();
        let m = ProjectManifest::read(&path).unwrap();
        assert_eq!(m.project.name, "disk-demo");
    }
}
