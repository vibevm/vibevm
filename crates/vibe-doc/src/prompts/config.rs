//! `[doc.prompts]` — what the prompt check runs, and against what
//! (PROP-057 `##STYLE-PROMPT-FIRST`).
//!
//! The table lives in `<package>/prompts.toml`, beside the fixtures'
//! `examples/deferred.toml`, and NOT in `vibe.toml`: the manifest is a
//! closed, strictly parsed contract shared by every package kind, and
//! teaching it a documentation-only table would be a wire change with a
//! codegen and a schema behind it. The table keeps the name the norm
//! gives it, which is what an author looks for.
//!
//! ## Why a prompt's fixture is declared here and not on the page
//!
//! PROP-045 §7 closes a `<prompt>` to one attribute, `id`. A `fixture`
//! attribute would be a sixth element of a closed vocabulary — a change
//! to the norm and to the pivot. So the mapping «prompt → the state it
//! starts from» lives beside the runner that needs it, keyed by the same
//! `<page>#<id>` a report prints.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-PROMPT-FIRST");

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{DocError, Result};

/// The file a documentation package declares its prompt runner in.
pub const PROMPTS_FILE: &str = "prompts.toml";

/// How long an agent may work on one prompt before it is killed. Agents
/// are slow and a documented task is a real task, so the default is
/// generous — but it is bounded, because a hung child must fail a run
/// and never own it.
pub const DEFAULT_TIMEOUT_SECS: u64 = 900;

/// The fixture a prompt runs in when nothing names another: an empty
/// folder, which is where most task pages start the reader.
pub const DEFAULT_FIXTURE: &str = "empty";

/// `prompts.toml` as written.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptsFile {
    /// Declaration schema. `1` is the only version that exists.
    pub schema: u32,
    #[serde(default)]
    pub doc: DocTable,
}

/// The `[doc]` table, whose one member is `prompts`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocTable {
    #[serde(default)]
    pub prompts: Config,
}

/// `[doc.prompts]`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The command the prompt text is handed to. It is a PROGRAM and its
    /// arguments, split the way a documented command line is split — not
    /// a shell line: a pipeline or a redirect here would make the check
    /// depend on whichever shell the machine happens to have.
    ///
    /// Absent is legal: the central session passes `--runner` instead,
    /// which is how one corpus is run through two agents for neutrality.
    #[serde(default)]
    pub runner: Option<String>,
    /// Seconds one prompt may take.
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// How many prompts a run takes when the caller names no number.
    /// `0` is all of them.
    #[serde(default)]
    pub sample: usize,
    /// The fixture every prompt starts from unless its own row names
    /// another.
    #[serde(default = "default_fixture")]
    pub fixture: String,
    /// The skill to project into the sandbox project before the runner
    /// is called. Absent means the runner brings its own, which is the
    /// truth for a real agent: the skill is part of ITS environment, not
    /// of the project the prompt works in.
    #[serde(default)]
    pub skill: Option<String>,
    /// Per-prompt rows, in the shape `examples/deferred.toml` already
    /// uses: page, id, and what this one needs.
    #[serde(default, rename = "prompt")]
    pub prompts: Vec<PromptDecl>,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

fn default_fixture() -> String {
    DEFAULT_FIXTURE.to_owned()
}

/// One prompt's own row.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptDecl {
    /// The page's address inside the package, e.g. `start/first-project.xml`.
    pub page: String,
    /// The prompt's `id` on that page.
    pub id: String,
    /// The fixture this prompt starts from.
    #[serde(default)]
    pub fixture: Option<String>,
    /// Why this prompt does not run yet, when it does not. A declared
    /// skip says what is missing; a silent one says nothing.
    #[serde(default)]
    pub skip: Option<String>,
}

impl Config {
    /// The fixture one prompt starts from.
    pub fn fixture_of(&self, page: &str, id: &str) -> &str {
        self.row(page, id)
            .and_then(|r| r.fixture.as_deref())
            .unwrap_or(&self.fixture)
    }

    /// Why one prompt does not run, when a row says so.
    pub fn skip_of(&self, page: &str, id: &str) -> Option<&str> {
        self.row(page, id).and_then(|r| r.skip.as_deref())
    }

    fn row(&self, page: &str, id: &str) -> Option<&PromptDecl> {
        self.prompts.iter().find(|r| r.page == page && r.id == id)
    }
}

/// Where a package declares its prompt runner.
pub fn config_path(package_dir: &Path) -> PathBuf {
    package_dir.join(PROMPTS_FILE)
}

/// Read a package's `[doc.prompts]`.
///
/// A package with no `prompts.toml` is refused rather than passed: the
/// check would otherwise print «0 prompts run» for a manual full of
/// prompts, which is the same number as «every prompt passed».
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// std::fs::write(
///     dir.path().join("prompts.toml"),
///     "schema = 1\n\n[doc.prompts]\nrunner = \"agent --quiet\"\nsample = 3\n",
/// )
/// .unwrap();
///
/// let config = vibe_doc::prompts::config::read(dir.path()).unwrap();
/// assert_eq!(config.runner.as_deref(), Some("agent --quiet"));
/// assert_eq!(config.sample, 3);
/// assert_eq!(config.fixture, "empty");
/// ```
pub fn read(package_dir: &Path) -> Result<Config> {
    let path = config_path(package_dir);
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::Prompt {
        message: format!("`{}` cannot be read: {e}", path.display()),
    })?;
    let parsed: PromptsFile = toml::from_str(&text).map_err(|e| DocError::Prompt {
        message: format!("`{}` does not parse: {e}", path.display()),
    })?;
    if parsed.schema != 1 {
        return Err(DocError::Prompt {
            message: format!(
                "`{}` declares schema {} — this check reads schema 1 and refuses to guess \
                 at another",
                path.display(),
                parsed.schema
            ),
        });
    }
    Ok(parsed.doc.prompts)
}

#[cfg(test)]
mod tests;
