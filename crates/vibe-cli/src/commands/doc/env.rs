//! Composition-root inputs for documentation commands: the ambient
//! values the CLI resolves, and the four readings it takes of its
//! surroundings before any library is called.
//!
//! The readings live here rather than beside the commands that use them
//! because they are the same question the struct above answers — what is
//! around this invocation — and because the `vibe doc` surface itself has
//! a file-length budget to keep. The library discovers none of it: naming
//! every source here is what keeps a documentation build reproducible by
//! inspection.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY");

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use vibe_core::progress::Progress;
use vibe_doc::citations::SpecSources;

/// Ambient values resolved by the CLI and handed to the documentation layer.
#[derive(Debug, Clone, Default)]
pub struct DocEnv {
    /// Invocation-owned observation tree supplied by the CLI composition root.
    pub progress: Progress,
    /// `$VIBE_SETTINGS`, when the operator relocated the settings dir.
    pub settings: Option<OsString>,
    /// The operator's home, for the real `~/.vibe` the tripwire guards.
    pub home: Option<OsString>,
    /// The system temporary directory — where sandboxes go by default.
    pub temp: PathBuf,
    /// The working directory, which is the source tree during a panel run.
    pub cwd: Option<PathBuf>,
    /// The running binary: the default subject of every example.
    pub current_exe: Option<PathBuf>,
    /// The process id, so two runs on one machine cannot share a sandbox.
    pub pid: u32,
    /// `$VIBEVM_INSTALL_ROOT/opt`, else `~/.vibe/opt` — where a downloaded
    /// reader shell is kept.
    pub install_root: Option<PathBuf>,
    /// Has the invocation declared that nobody is at the keyboard?
    pub unattended: bool,
    /// Is the invocation printing a machine document?
    pub json: bool,
}

/// The world a `rule` citation resolves against (PROP-057
/// `##LOCAL-WARMUP`): the checkout the operator is standing in, the
/// packages it authors in-tree, the instances its lock selected, and the
/// machine store `vibe cache add` warms. The library discovers none of
/// them — naming them here is what keeps a documentation build
/// reproducible by inspection.
///
/// Without a working directory there is no checkout, and the store alone
/// answers: that is the local reader's own situation, reading
/// documentation for a project it is not inside.
pub(super) fn spec_sources(
    repo_root: &Option<PathBuf>,
    settings_home: Option<&Path>,
) -> SpecSources {
    let sources = match repo_root {
        Some(root) => {
            let (group, name) = self_coordinate(root);
            SpecSources::for_checkout(root, group.as_deref(), &name)
        }
        None => SpecSources::new(),
    };
    match settings_home {
        Some(home) => sources.with_store(home.join(STORE_DIR)),
        None => sources,
    }
}

/// The machine store's directory under the settings home — the same
/// `<settings>/cache` [`vibe_registry::store::store_root`] resolves, named
/// here because this command hands the path down instead of letting a
/// library read the environment for it.
pub(super) const STORE_DIR: &str = "cache";

/// The checkout's own `[project]` group and name — the coordinate a
/// `spec://` address must carry to reach its authored specs (B-031).
///
/// Read as TOML data: the one question is «what coordinate does this
/// directory answer to», and a strict manifest parse would make it fail
/// over a field it never reads. A directory with no manifest answers to
/// no coordinate, which is a legitimate state — then only the other three
/// sources speak.
pub(super) fn self_coordinate(root: &Path) -> (Option<String>, String) {
    let Ok(text) = std::fs::read_to_string(root.join("vibe.toml")) else {
        return (None, String::new());
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return (None, String::new());
    };
    let read = |table: &str, key: &str| {
        value
            .get(table)
            .and_then(|t| t.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let group = read("project", "group").or_else(|| read("package", "group"));
    let name = read("project", "name")
        .or_else(|| read("package", "name"))
        .unwrap_or_default();
    (group, name)
}

/// The language the project the operator is standing in prefers to read
/// in — `[i18n].preferred`, absent when the project declares none or
/// there is no project at all.
///
/// Read as TOML data for the reason `self_coordinate` is: the one
/// question is what this directory says about itself, and a strict
/// manifest parse would fail over a field this command never looks at.
pub(super) fn preferred_language(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("vibe.toml")).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    value
        .get("i18n")?
        .get("preferred")?
        .as_str()
        .map(str::to_owned)
}

/// The real per-user settings directory the tripwire guards: the
/// relocation variable when it is set, else `<home>/.vibe`.
pub(super) fn settings_home(
    settings: &Option<OsString>,
    home: &Option<OsString>,
) -> Option<PathBuf> {
    if let Some(dir) = settings {
        return Some(PathBuf::from(dir));
    }
    home.as_ref().map(|h| PathBuf::from(h).join(".vibe"))
}
