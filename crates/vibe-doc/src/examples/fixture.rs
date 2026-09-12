//! The fixture declaration — `examples/<fixture>/example.toml` inside the
//! documentation package (PROP-057 `##PIPE-EXAMPLE-RUNNER`).
//!
//! A fixture is a hermetic project state plus the rules the comparison of
//! its examples may use. Both halves are DECLARED, never inferred: a
//! reader of the fixture must be able to see which differences the runner
//! is allowed to ignore, because that is the difference between a
//! normalisation and a loosened comparison (PROP-045
//! `##ROW-DOCVOCAB-EXAMPLE-CHECK` — a divergence is red).
//!
//! A fixture is a RECIPE, not a checked-in tree of thousands of files: it
//! names the fixture it grows out of and the commands that turn that state
//! into this one, exactly as the campaign's fixture table writes them. The
//! few files a recipe cannot produce by running the product — a manifest a
//! page asks the reader to hand-write — live in the fixture's own `tree/`
//! and are laid over the parent's state.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{DocError, Result};

/// Where a documentation package keeps its fixtures, relative to its root.
pub const FIXTURES_DIR: &str = "examples";

/// The fixture recipe's own file name.
pub const FIXTURE_FILE: &str = "example.toml";

/// The package-wide list of examples that are not captured from a debug
/// build, with the reason each one waits (X-026).
pub const DEFERRED_FILE: &str = "deferred.toml";

/// The overlay directory inside a fixture whose contents are copied onto
/// the sandbox after the parent state and before the build steps.
pub const OVERLAY_DIR: &str = "tree";

/// The sandbox directory that becomes `VIBE_SETTINGS` — the whole
/// per-user `~/.vibe`, relocated.
pub const HOME_DIR: &str = "home";

/// The sandbox directory holding the local registry copy the fixtures
/// resolve through (X-025), addressed from a fixture's files as
/// `${REGISTRY}`.
pub const REGISTRY_DIR: &str = "registry";

/// `example.toml` as written.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureFile {
    /// Declaration schema. `1` is the only version that exists; an
    /// unknown number is refused rather than guessed at.
    pub schema: u32,
    #[serde(default)]
    pub fixture: FixtureDecl,
    #[serde(default)]
    pub normalize: NormalizeDecl,
    /// `--json` document `command` → the JTD schema it must satisfy,
    /// resolved against the repository root. A document with no entry
    /// here is reported as UNCHECKED, never as passed.
    #[serde(default)]
    pub jtd: BTreeMap<String, String>,
}

/// How the fixture's state is reached.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureDecl {
    /// The fixture this one grows out of. `None` makes it a root fixture
    /// whose whole state is its own overlay.
    #[serde(default)]
    pub from: Option<String>,
    /// The working directory of this fixture's examples, relative to the
    /// sandbox root. Defaults to `work`.
    #[serde(default)]
    pub cwd: Option<String>,
    /// `true` runs the examples in the SOURCE TREE itself instead of a
    /// sandbox — the one fixture (`host`) whose subject is the checkout of
    /// vibe. Read-only commands only; the tripwire guards the rest.
    #[serde(default)]
    pub in_source_tree: bool,
    /// The commands that turn the parent's state into this one, in order.
    #[serde(default)]
    pub step: Vec<StepDecl>,
}

/// One build step of a fixture recipe. Exactly one of the four verbs is
/// set: a step either runs a command or edits the tree, never both.
///
/// The three file verbs exist because the campaign's own fixture table
/// needs them: a page tells the reader to copy a project, to add a table
/// to a manifest, or to change one setting in a generated file, and a
/// recipe that could only run commands would have to hand-write the
/// generated file instead — which is how a fixture stops testing the
/// product and starts testing a transcription of it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepDecl {
    /// A command line, in the same closed program set the examples use.
    /// `${REGISTRY}` and `${SANDBOX}` are substituted before it runs.
    #[serde(default)]
    pub run: Option<String>,
    /// Copy one directory of the sandbox to another, retargeted.
    #[serde(default)]
    pub copy: Option<CopyDecl>,
    /// Append text to a file the product generated.
    #[serde(default)]
    pub append: Option<AppendDecl>,
    /// Replace every occurrence of one literal in a file with another.
    #[serde(default)]
    pub edit: Option<EditDecl>,
    /// Working directory for a `run` step, relative to the sandbox root.
    /// Defaults to the fixture's own `cwd`.
    #[serde(default)]
    pub cwd: Option<String>,
}

/// `copy = { from = …, to = … }` — both relative to the sandbox root.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyDecl {
    pub from: String,
    pub to: String,
}

/// `append = { path = …, text = … }` — the text a page asks the reader to
/// add to a file, added verbatim with a leading newline.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppendDecl {
    pub path: String,
    pub text: String,
}

/// `edit = { path = …, from = …, to = … }` — a literal, not a pattern: a
/// fixture changes a known setting, it does not rewrite a file by shape.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditDecl {
    pub path: String,
    pub from: String,
    pub to: String,
}

/// Which differences the comparison of this fixture's examples may
/// ignore. Every one of them is a named class, and every name is visible
/// in the fixture file: there are no match templates, and nothing here is
/// inferred from the output.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizeDecl {
    /// `\r\n` → `\n`, on both sides. A golden read on Windows and a
    /// stream captured from a Windows child disagree about nothing else.
    #[serde(default = "yes")]
    pub crlf: bool,
    /// Strip ANSI escape sequences.
    #[serde(default = "yes")]
    pub ansi: bool,
    /// Replace the sandbox, the repository and the real user home with
    /// `<TMP>`, `<REPO>` and `<HOME>` — most specific path first, in the
    /// native, forward-slash and JSON-escaped spellings alike.
    #[serde(default = "yes")]
    pub paths: bool,
    /// `\` → `/`, AFTER the path replacements (a replacement that ran
    /// second would never find the native spelling).
    #[serde(default = "yes")]
    pub slashes: bool,
    /// `vibe.exe` → `vibe` in usage lines: the reader types `vibe`.
    #[serde(default = "yes")]
    pub exe_name: bool,
    /// `vibe <semver>` → `vibe <VERSION>`. Package versions are content
    /// and are never touched.
    #[serde(default = "yes")]
    pub product_version: bool,
    /// Drop trailing whitespace on every line and the trailing blank
    /// lines of the stream.
    #[serde(default = "yes")]
    pub trailing_space: bool,
    /// Line FORMS whose maximal consecutive runs are sorted, for output
    /// whose order the product does not promise. A form, never a global
    /// sort: the rest of the stream keeps its order.
    #[serde(default)]
    pub sort_blocks: Vec<String>,
    /// The fixture's own replacements, applied after the named classes.
    #[serde(default)]
    pub replace: Vec<ReplaceDecl>,
}

impl Default for NormalizeDecl {
    fn default() -> NormalizeDecl {
        NormalizeDecl {
            crlf: true,
            ansi: true,
            paths: true,
            slashes: true,
            exe_name: true,
            product_version: true,
            trailing_space: true,
            sort_blocks: Vec::new(),
            replace: Vec::new(),
        }
    }
}

fn yes() -> bool {
    true
}

/// One local replacement rule: a line form and what it becomes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaceDecl {
    /// A regular expression, matched in multi-line mode so `^` and `$`
    /// address one line.
    pub pattern: String,
    /// The replacement text; `$1` and friends address capture groups.
    pub with: String,
}

/// `deferred.toml` — the examples that do not run yet, each with the
/// reason and the event that will capture it. An example listed here is
/// SKIPPED with its reason: it never turns a panel red, and it never
/// pretends that an empty golden asserts silence (X-026).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeferredFile {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub deferred: Vec<DeferredDecl>,
}

/// One deferred example.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeferredDecl {
    /// The page's address inside the package, e.g. `start/install-vibe.xml`.
    pub page: String,
    /// The example's `id` on that page.
    pub id: String,
    /// What event captures it. `release` is X-026's status — the output
    /// comes from a release distribution, never from a debug build;
    /// `blocked` waits on a product fix or a later atom.
    pub captured: Captured,
    /// Why, in one sentence, for the person reading the skip line.
    pub reason: String,
}

/// The event that will capture a deferred example.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Captured {
    /// Captured from a release distribution at publication (X-026).
    Release,
    /// Waiting on a product fix or on an atom that has not landed.
    Blocked,
}

impl Captured {
    /// The word a skip line uses.
    pub fn as_str(self) -> &'static str {
        match self {
            Captured::Release => "release",
            Captured::Blocked => "blocked",
        }
    }
}

/// Read one fixture recipe.
pub fn read_fixture(package_dir: &Path, name: &str) -> Result<FixtureFile> {
    let path = fixture_dir(package_dir, name).join(FIXTURE_FILE);
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::Fixture {
        fixture: name.to_owned(),
        message: format!("cannot read `{}`: {e}", path.display()),
    })?;
    let parsed: FixtureFile = toml::from_str(&text).map_err(|e| DocError::Fixture {
        fixture: name.to_owned(),
        message: format!("`{}` does not parse: {e}", path.display()),
    })?;
    if parsed.schema != 1 {
        return Err(DocError::Fixture {
            fixture: name.to_owned(),
            message: format!(
                "declares schema {} — this runner reads schema 1 and refuses to guess at another",
                parsed.schema
            ),
        });
    }
    Ok(parsed)
}

/// Where a fixture lives.
pub fn fixture_dir(package_dir: &Path, name: &str) -> PathBuf {
    package_dir.join(FIXTURES_DIR).join(name)
}

/// Read the package's deferred list. Its absence means «nothing is
/// deferred», which is the state a healthy package converges to.
pub fn read_deferred(package_dir: &Path) -> Result<Vec<DeferredDecl>> {
    let path = package_dir.join(FIXTURES_DIR).join(DEFERRED_FILE);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::io("reading", &path, e))?;
    let parsed: DeferredFile = toml::from_str(&text).map_err(|e| DocError::Fixture {
        fixture: DEFERRED_FILE.to_owned(),
        message: format!("`{}` does not parse: {e}", path.display()),
    })?;
    Ok(parsed.deferred)
}

/// The chain of fixtures that must be built to reach `name`, parent
/// first. A cycle is refused by name rather than hung on.
pub fn ancestry(package_dir: &Path, name: &str) -> Result<Vec<(String, FixtureFile)>> {
    let mut chain: Vec<(String, FixtureFile)> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut cursor = Some(name.to_owned());
    while let Some(current) = cursor {
        if seen.contains(&current) {
            return Err(DocError::Fixture {
                fixture: name.to_owned(),
                message: format!("`{current}` is its own ancestor — the recipe is a cycle"),
            });
        }
        seen.push(current.clone());
        let decl = read_fixture(package_dir, &current)?;
        cursor = decl.fixture.from.clone();
        chain.push((current, decl));
    }
    chain.reverse();
    Ok(chain)
}

#[cfg(test)]
mod tests;
