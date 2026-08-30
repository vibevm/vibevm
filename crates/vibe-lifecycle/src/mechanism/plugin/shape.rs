//! §6.2's fixed directory shape, enforced over the plugin SOURCE tree
//! before a single byte is staged.
//!
//! > "It contains root `plugin.json`, fixed `skills/<name>/SKILL.md`,
//! > optional `mcp.json`, and only valid reverse-domain client-extension
//! > directories. It enforces containment across symlinks, junctions and
//! > reparse points."
//!
//! Each clause is a check here, and the containment one is the reason the
//! walk is hand-written rather than a directory iterator with a filter:
//! every entry is proved not to be a link BEFORE it is descended into or
//! read, so a junction pointing outside the workspace refuses instead of
//! being packaged.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::path::{Path, PathBuf};

use crate::mechanism::MechanismError;
use crate::mechanism::contain::{FileFault, prove_directory};
use crate::mechanism::error::preview;

use super::config::is_reverse_domain;

/// The plugin manifest every Agent Plugin 1.0 directory has at its root.
pub(crate) const PLUGIN_MANIFEST: &str = "plugin.json";

/// The optional MCP server declaration at the same root.
pub(crate) const MCP_MANIFEST: &str = "mcp.json";

/// The fixed skills directory.
pub(crate) const SKILLS_DIR: &str = "skills";

/// The fixed entry document of one packaged skill.
pub(crate) const SKILL_ENTRY: &str = "SKILL.md";

/// One source file the provider will stage, with its tree-relative
/// identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceFile {
    pub(crate) relative: String,
    pub(crate) absolute: PathBuf,
}

/// One validated plugin source tree.
///
/// `Eq` is deliberately absent: the validated MCP servers are a JSON value,
/// and JSON numbers have no total equality. `PartialEq` is what the tree
/// really has, and claiming more would be a derive nobody could honour.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PluginSource {
    pub(crate) files: Vec<SourceFile>,
    /// The `name`/`version` members of `plugin.json`.
    pub(crate) identity: super::manifest::PluginIdentity,
    /// The packaged skill names, in sorted order.
    pub(crate) skills: Vec<String>,
    /// The declared MCP servers, when the tree carries an `mcp.json` — the
    /// document `validate_mcp_manifest` already judged.
    pub(crate) mcp: Option<super::manifest::McpServers>,
}

impl PluginSource {
    /// One tree-relative file of this source, when the tree carries it.
    ///
    /// The projections address the canonical tree by NAME (`plugin.json`,
    /// `mcp.json`, everything under `skills/`) rather than re-walking it,
    /// so the validated census stays the one census.
    pub(crate) fn file(&self, relative: &str) -> Option<&SourceFile> {
        self.files.iter().find(|file| file.relative == relative)
    }
}

/// Read and validate one plugin source tree.
pub(crate) fn read_source(
    target: &str,
    project_root: &Path,
    source: &str,
) -> Result<PluginSource, MechanismError> {
    let root = crate::mechanism::contain::join_relative(project_root, source);
    prove_directory(&root).map_err(|fault| MechanismError::SourceMissing {
        target: target.to_owned(),
        provider: crate::mechanism::BUILTIN_AGENT_PLUGIN_PIN.to_owned(),
        path: source.to_owned(),
        reason: fault.reason(),
    })?;
    let mut files: Vec<SourceFile> = Vec::new();
    let mut skills: Vec<String> = Vec::new();
    let mut has_mcp = false;
    let mut has_manifest = false;
    for (name, entry) in listing(target, source, &root, "")? {
        match entry {
            Entry::File(path) if name == PLUGIN_MANIFEST => {
                has_manifest = true;
                files.push(SourceFile {
                    relative: name,
                    absolute: path,
                });
            }
            Entry::File(path) if name == MCP_MANIFEST => {
                has_mcp = true;
                files.push(SourceFile {
                    relative: name,
                    absolute: path,
                });
            }
            Entry::File(_) => {
                return Err(shape(
                    target,
                    &name,
                    "the only files an Agent Plugin root admits are `plugin.json` and the \
                     optional `mcp.json`",
                ));
            }
            Entry::Directory(path) if name == SKILLS_DIR => {
                skills = read_skills(target, source, &path, &mut files)?;
            }
            Entry::Directory(path) if is_reverse_domain(&name) => {
                descend(target, source, &path, &name, &mut files)?;
            }
            Entry::Directory(_) => {
                return Err(shape(
                    target,
                    &name,
                    "a directory beside `skills` is a client extension and must be named in \
                     reverse-domain form",
                ));
            }
        }
    }
    if !has_manifest {
        return Err(shape(
            target,
            PLUGIN_MANIFEST,
            "an Agent Plugin 1.0 directory declares itself in a root `plugin.json`",
        ));
    }
    let identity = super::manifest::validate_plugin_manifest(target, &root)?;
    let mcp = if has_mcp {
        Some(super::manifest::validate_mcp_manifest(target, &root)?)
    } else {
        None
    };
    files.sort_by(|left, right| left.relative.cmp(&right.relative));
    Ok(PluginSource {
        files,
        identity,
        skills,
        mcp,
    })
}

/// `skills/` — one directory per skill, each with its `SKILL.md`.
fn read_skills(
    target: &str,
    source: &str,
    root: &Path,
    files: &mut Vec<SourceFile>,
) -> Result<Vec<String>, MechanismError> {
    let mut skills = Vec::new();
    for (name, entry) in listing(target, source, root, SKILLS_DIR)? {
        let path = match entry {
            Entry::Directory(path) => path,
            Entry::File(_) => {
                return Err(shape(
                    target,
                    &format!("{SKILLS_DIR}/{name}"),
                    "`skills/` holds one directory per skill and no loose files",
                ));
            }
        };
        let entry_document = path.join(SKILL_ENTRY);
        if crate::mechanism::contain::prove_regular_file(&entry_document).is_err() {
            return Err(shape(
                target,
                &format!("{SKILLS_DIR}/{name}/{SKILL_ENTRY}"),
                "a packaged skill's entry document is fixed at `skills/<name>/SKILL.md`",
            ));
        }
        descend(
            target,
            source,
            &path,
            &format!("{SKILLS_DIR}/{name}"),
            files,
        )?;
        skills.push(name);
    }
    skills.sort();
    Ok(skills)
}

/// Collect every regular file below one directory, proving containment at
/// each step.
fn descend(
    target: &str,
    source: &str,
    directory: &Path,
    prefix: &str,
    files: &mut Vec<SourceFile>,
) -> Result<(), MechanismError> {
    for (name, entry) in listing(target, source, directory, prefix)? {
        let relative = format!("{prefix}/{name}");
        match entry {
            Entry::File(path) => files.push(SourceFile {
                relative,
                absolute: path,
            }),
            Entry::Directory(path) => descend(target, source, &path, &relative, files)?,
        }
    }
    Ok(())
}

/// One directory entry, already proved to be a real file or a real
/// directory rather than a link.
enum Entry {
    File(PathBuf),
    Directory(PathBuf),
}

/// One directory's entries in name order, each proved non-link.
fn listing(
    target: &str,
    source: &str,
    directory: &Path,
    prefix: &str,
) -> Result<Vec<(String, Entry)>, MechanismError> {
    let refuse = |relative: &str, reason: String| MechanismError::PluginShape {
        target: target.to_owned(),
        entry: preview(relative),
        reason,
    };
    let here = if prefix.is_empty() {
        source.to_owned()
    } else {
        format!("{source}/{prefix}")
    };
    let listing = std::fs::read_dir(directory).map_err(|error| refuse(&here, error.to_string()))?;
    let mut entries: Vec<(String, Entry)> = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|error| refuse(&here, error.to_string()))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return Err(refuse(
                &here,
                "it holds an entry whose name is not valid UTF-8".to_owned(),
            ));
        };
        let name = name.to_owned();
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| refuse(&relative, error.to_string()))?;
        if metadata.file_type().is_symlink() {
            return Err(refuse(&relative, FileFault::Link.reason()));
        }
        if metadata.is_dir() {
            entries.push((name, Entry::Directory(path)));
        } else if metadata.is_file() {
            entries.push((name, Entry::File(path)));
        } else {
            return Err(refuse(&relative, FileFault::NotRegular.reason()));
        }
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(entries)
}

/// One §6.2 shape refusal.
fn shape(target: &str, entry: &str, reason: &str) -> MechanismError {
    MechanismError::PluginShape {
        target: target.to_owned(),
        entry: preview(entry),
        reason: reason.to_owned(),
    }
}
