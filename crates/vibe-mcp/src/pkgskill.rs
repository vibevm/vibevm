//! Project package-declared skills into coding agents (PROP-018 §2.5, §2.6).
//!
//! `vibe skill install` reads the `[[skill]]` declarations of installed
//! packages (and the project's own nodes) and writes each skill body into
//! every target agent's skill directory, reusing the PROP-015 agent
//! machinery (the [`Agent`] enum and its per-(agent, scope) skill paths).
//! This is the *orthogonal projection* of PROP-018 §2.5 — content travels
//! *out of* the workspace into an agent, the mirror image of subskill
//! delivery into the project tree. Standalone-only (PROP-018 §2.3): no LLM,
//! so it works whether or not an agent is driving vibevm.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill");

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use specmark::spec;
use thiserror::Error;
use vibe_core::machine_json_path;

use crate::agents::{Agent, Scope};

#[path = "pkgskill/exact_path.rs"]
mod exact_path;
#[path = "pkgskill/projection.rs"]
mod projection;
#[path = "pkgskill/receipt.rs"]
mod receipt;

pub use exact_path::EscapedOsPath;
pub use projection::{
    DeclaredSkill, DeclaredSkillFilter, DeclaredSkillProjection, DeclaredSkillProvider,
    PROJECT_SKILL_PREFIX, PROJECT_SKILL_RECONCILE_KEY, PROJECT_SKILL_RECOVER_KEY,
    ProjectSkillBinding, ProjectSkillProviderInput, ProjectSkillTarget, collect_declared_skills,
    collect_project_skill_bindings, lower_project_skill_bindings,
    prepare_declared_skill_projection, probe_project_skill_binding,
    probe_recovered_project_skill_bindings, probe_vanished_project_skill_bindings,
    project_declared_skills_project_scope, project_skill_receipt_exists,
    reconcile_project_skill_binding, reconcile_vanished_project_skill_bindings,
    recover_project_skill_bindings,
};

/// The vibe-skill projection layer's failure surface (PROP-018 §2.5):
/// reading a skill source, writing the projection into an agent's skills
/// directory, or resolving the agent's skills root. One enum for the layer.
///
/// ```
/// use vibe_mcp::pkgskill::PackageSkillError;
/// let e = PackageSkillError::SkillsRoot { detail: "no config dir".into() };
/// assert!(e.to_string().contains("spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill"));
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill")]
pub enum PackageSkillError {
    #[error(
        "reading skill content at `{path}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill; \
          fix: ensure the package's declared skill source and the agent dirs are readable)"
    )]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "writing the projected skill at `{path}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill; \
          fix: ensure the agent's skills directory is writable)"
    )]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "resolving the agent skills root failed: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill; \
          fix: act on the wrapped agent-config error)"
    )]
    SkillsRoot { detail: String },

    #[error(
        "unsafe package-skill path `{path}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill; \
          fix: use a contained normal path with no symlink, junction, or reparse ancestor)"
    )]
    UnsafePath { path: PathBuf, reason: String },

    /// A path or entry name the OS accepts but this projection cannot spell
    /// faithfully. Both fields are already-escaped [`EscapedOsPath`] values,
    /// **never** `PathBuf`: `thiserror` renders a `Path`/`PathBuf` field
    /// through `Path::display`, which substitutes `U+FFFD` for exactly the
    /// units this diagnostic exists to name. Rendering a pre-escaped
    /// `Display` value is the only way the outer error cannot re-lossify it.
    #[error(
        "unportable package-skill path `{path}` (escaped): {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill; \
          fix: rename the entry to an exact-UTF-8 portable name and rerun)"
    )]
    UnportablePath { path: EscapedOsPath, reason: String },
}

/// Per-(skill, agent, scope) outcome of projecting a package skill — the
/// structured record `vibe skill` renders or emits as JSON.
///
/// ```
/// use vibe_mcp::pkgskill::PackageSkillReport;
/// let r = PackageSkillReport {
///     skill: "demo".into(),
///     agent: "claude".into(),
///     scope: "project",
///     path: None,
///     status: "skipped",
///     note: None,
/// };
/// assert_eq!(r.skill, "demo");
/// ```
#[derive(Debug, Clone, Serialize)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill")]
pub struct PackageSkillReport {
    pub skill: String,
    pub agent: String,
    pub scope: &'static str,
    pub path: Option<String>,
    /// `created` / `updated` / `unchanged` / `would-create` /
    /// `would-update` / `skipped` / `removed` / `would-remove` / `absent`.
    pub status: &'static str,
    pub note: Option<String>,
}

/// Project one skill body into one agent + scope (PROP-018 §2.5).
///
/// `source` is the package's declared `[[skill]].path` resolved to an
/// absolute file or directory; its contents are copied into
/// `<agent skills root>/<skill_name>/`. Idempotent: an identical
/// projection is left `unchanged`; a divergent one is replaced wholesale
/// and reported `updated`, so a file the source dropped leaves no stale
/// copy. Agents with no filesystem skill loader (Cursor, Claude Desktop)
/// or no surface for this scope report `skipped`.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill")]
pub fn install_package_skill(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    skill_name: &str,
    source: &Path,
    dry_run: bool,
) -> Result<PackageSkillReport, PackageSkillError> {
    install_package_skill_selecting(agent, scope, project_root, skill_name, source, &[], dry_run)
}

/// Like [`install_package_skill`] but projects only the files matching one
/// of the `include` glob patterns (relative to `source`); an empty `include`
/// projects the whole `source` tree — the §2.6 default (PROP-015 §2.8). Lets
/// a skill pick specific files out of a noisy subtree (e.g. a bridged
/// upstream repo full of unrelated content, PROP-023).
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#skill-include",
    r = 1
)]
pub fn install_package_skill_selecting(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    skill_name: &str,
    source: &Path,
    include: &[String],
    dry_run: bool,
) -> Result<PackageSkillReport, PackageSkillError> {
    if !vibe_core::manifest::SkillDecl::valid_name(skill_name) {
        return Err(PackageSkillError::UnsafePath {
            path: PathBuf::from(skill_name),
            reason: "skill name is not one safe lowercase-kebab component".into(),
        });
    }
    let agent_str = agent.as_str().to_string();
    let scope_str = scope.as_str();

    let Some(root) =
        agent
            .skills_root(scope, project_root)
            .map_err(|e| PackageSkillError::SkillsRoot {
                detail: format!("{e:#}"),
            })?
    else {
        return Ok(skipped(skill_name, agent, scope_str));
    };
    let target = root.join(skill_name);
    let containment_root = match scope {
        Scope::Project => project_root.unwrap_or(root.as_path()),
        Scope::User | Scope::Both => root.as_path(),
    };
    receipt::ensure_no_follow_walk(containment_root, &target, true).map_err(|error| {
        PackageSkillError::UnsafePath {
            path: target.clone(),
            reason: error.to_string(),
        }
    })?;
    let path_str = machine_json_path(&target);

    let source_exists = source
        .try_exists()
        .map_err(|source_error| PackageSkillError::Read {
            path: source.to_path_buf(),
            source: source_error,
        })?;
    if !source_exists {
        return Ok(PackageSkillReport {
            skill: skill_name.to_string(),
            agent: agent_str,
            scope: scope_str,
            path: Some(path_str),
            status: "skipped",
            note: Some(format!("skill source `{}` not found", source.display())),
        });
    }

    let desired = snapshot_source(source, include)?;
    let current = snapshot_dir(&target)?;
    let action = if current.is_none() {
        "created"
    } else if current.as_ref() == Some(&desired) {
        "unchanged"
    } else {
        "updated"
    };

    let status = crate::install::preview_status(action, dry_run);

    if !dry_run && status != "unchanged" {
        // Replace the projection wholesale so the agent dir mirrors the
        // package's skill body exactly. Only the skill's own dir is
        // touched — foreign skill dirs are never read or removed.
        if target
            .try_exists()
            .map_err(|source| PackageSkillError::Write {
                path: target.clone(),
                source,
            })?
        {
            receipt::ensure_no_follow_walk(containment_root, &target, false).map_err(|error| {
                PackageSkillError::UnsafePath {
                    path: target.clone(),
                    reason: error.to_string(),
                }
            })?;
            fs::remove_dir_all(&target).map_err(|source| PackageSkillError::Write {
                path: target.clone(),
                source,
            })?;
        }
        write_snapshot(&target, &desired)?;
    }

    Ok(PackageSkillReport {
        skill: skill_name.to_string(),
        agent: agent_str,
        scope: scope_str,
        path: Some(path_str),
        status,
        note: None,
    })
}

/// Remove a projected skill from one agent + scope — the `vibe skill
/// uninstall` inverse. `removed` when present, `absent` when nothing was
/// there, `skipped` for agents with no skill loader. Only the skill's own
/// `<name>/` dir is touched.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill")]
pub fn uninstall_package_skill(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    skill_name: &str,
    dry_run: bool,
) -> Result<PackageSkillReport, PackageSkillError> {
    if !vibe_core::manifest::SkillDecl::valid_name(skill_name) {
        return Err(PackageSkillError::UnsafePath {
            path: PathBuf::from(skill_name),
            reason: "skill name is not one safe lowercase-kebab component".into(),
        });
    }
    let scope_str = scope.as_str();
    let Some(root) =
        agent
            .skills_root(scope, project_root)
            .map_err(|e| PackageSkillError::SkillsRoot {
                detail: format!("{e:#}"),
            })?
    else {
        return Ok(skipped(skill_name, agent, scope_str));
    };
    let target = root.join(skill_name);
    let containment_root = match scope {
        Scope::Project => project_root.unwrap_or(root.as_path()),
        Scope::User | Scope::Both => root.as_path(),
    };
    receipt::ensure_no_follow_walk(containment_root, &target, true).map_err(|error| {
        PackageSkillError::UnsafePath {
            path: target.clone(),
            reason: error.to_string(),
        }
    })?;
    let path_str = machine_json_path(&target);
    let exists = target
        .try_exists()
        .map_err(|source| PackageSkillError::Write {
            path: target.clone(),
            source,
        })?;
    let status: &'static str = match (exists, dry_run) {
        (false, _) => "absent",
        (true, true) => "would-remove",
        (true, false) => "removed",
    };
    if exists && !dry_run {
        receipt::ensure_no_follow_walk(containment_root, &target, false).map_err(|error| {
            PackageSkillError::UnsafePath {
                path: target.clone(),
                reason: error.to_string(),
            }
        })?;
        fs::remove_dir_all(&target).map_err(|source| PackageSkillError::Write {
            path: target.clone(),
            source,
        })?;
    }
    Ok(PackageSkillReport {
        skill: skill_name.to_string(),
        agent: agent.as_str().to_string(),
        scope: scope_str,
        path: Some(path_str),
        status,
        note: None,
    })
}

fn skipped(skill_name: &str, agent: Agent, scope_str: &'static str) -> PackageSkillReport {
    PackageSkillReport {
        skill: skill_name.to_string(),
        agent: agent.as_str().to_string(),
        scope: scope_str,
        path: None,
        status: "skipped",
        note: Some(format!(
            "agent `{}` has no {scope_str}-scope skill loader",
            agent.as_str()
        )),
    }
}

/// Snapshot a skill body source into a `relpath -> bytes` map. A directory
/// is walked recursively (relpaths forward-slashed); a single file maps to
/// its file name (so a bare `SKILL.md` source lands as `<name>/SKILL.md`).
///
/// The **complete selected set** — after `include` filtering — is judged
/// through the shared portability law before it can be returned to any
/// caller, so neither surface ever stages, publishes, or writes a selection
/// carrying a non-storeable spelling or two spellings of one physical file.
fn snapshot_source(
    source: &Path,
    include: &[String],
) -> Result<BTreeMap<String, Vec<u8>>, PackageSkillError> {
    let mut out = BTreeMap::new();
    let metadata =
        fs::symlink_metadata(source).map_err(|source_error| PackageSkillError::Read {
            path: source.to_path_buf(),
            source: source_error,
        })?;
    if metadata.file_type().is_symlink() || metadata_is_reparse(&metadata) {
        return Err(PackageSkillError::UnsafePath {
            path: source.to_path_buf(),
            reason: "skill source is a symlink/junction/reparse point".into(),
        });
    }
    if metadata.is_dir() {
        collect_dir(source, source, &mut out)?;
        // PROP-015 §2.8: when `include` is set, keep only the files whose
        // relpath matches one of the patterns. Empty `include` keeps the
        // whole tree (the §2.6 default).
        if !include.is_empty() {
            out.retain(|rel, _| include.iter().any(|pat| glob_match(pat, rel)));
        }
    } else if metadata.is_file() {
        let name = match source.file_name() {
            Some(name) => exact_path::exact_utf8_component(name, source)?,
            None => "SKILL.md".to_string(),
        };
        let bytes = fs::read(source).map_err(|err| PackageSkillError::Read {
            path: source.to_path_buf(),
            source: err,
        })?;
        out.insert(name, bytes);
    } else {
        return Err(PackageSkillError::UnsafePath {
            path: source.to_path_buf(),
            reason: "skill source is neither a regular file nor a directory".into(),
        });
    }
    if let Err(fault) = receipt::judge_selection(out.keys().map(String::as_str)) {
        return Err(PackageSkillError::UnsafePath {
            path: source.to_path_buf(),
            reason: format!("selected source file set is not portable: {fault}"),
        });
    }
    Ok(out)
}

/// Snapshot an existing target dir, or `None` when it does not exist.
fn snapshot_dir(dir: &Path) -> Result<Option<BTreeMap<String, Vec<u8>>>, PackageSkillError> {
    let metadata = match fs::symlink_metadata(dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(PackageSkillError::Read {
                path: dir.to_path_buf(),
                source,
            });
        }
    };
    if metadata.file_type().is_symlink() || metadata_is_reparse(&metadata) || !metadata.is_dir() {
        return Err(PackageSkillError::UnsafePath {
            path: dir.to_path_buf(),
            reason: "skill target is not a no-follow directory".into(),
        });
    }
    let mut out = BTreeMap::new();
    collect_dir(dir, dir, &mut out)?;
    Ok(Some(out))
}

fn collect_dir(
    base: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), PackageSkillError> {
    let entries = fs::read_dir(dir).map_err(|source| PackageSkillError::Read {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| PackageSkillError::Read {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|source| PackageSkillError::Read {
            path: path.clone(),
            source,
        })?;
        if metadata.file_type().is_symlink() || metadata_is_reparse(&metadata) {
            return Err(PackageSkillError::UnsafePath {
                path,
                reason: "skill tree contains a symlink/junction/reparse point".into(),
            });
        }
        if metadata.is_dir() {
            collect_dir(base, &path, out)?;
        } else if metadata.is_file() {
            let rel = exact_path::exact_utf8_relative(base, &path)?;
            let bytes = fs::read(&path).map_err(|source| PackageSkillError::Read {
                path: path.clone(),
                source,
            })?;
            // Two entries can only ever land on one key through a lossy
            // rendering, which no longer exists; refuse rather than let one
            // file's bytes silently replace another's.
            if out.insert(rel.clone(), bytes).is_some() {
                return Err(PackageSkillError::UnsafePath {
                    path,
                    reason: format!("two skill tree entries share the relative path `{rel}`"),
                });
            }
        } else {
            return Err(PackageSkillError::UnsafePath {
                path,
                reason: "skill tree contains a non-file, non-directory entry".into(),
            });
        }
    }
    Ok(())
}

/// Match a forward-slash relpath against a restricted glob (PROP-015 §2.8):
/// `*` matches a run of non-`/` chars, `**` matches across `/`, `?` one
/// non-`/` char; everything else is literal, and a trailing `/` selects a
/// whole subtree. Deterministic; filters a skill's projected files.
fn glob_match(pattern: &str, path: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('/') {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    glob_rec(pattern.as_bytes(), path.as_bytes())
}

fn glob_rec(p: &[u8], t: &[u8]) -> bool {
    if p.is_empty() {
        return t.is_empty();
    }
    if let Some(rest) = p.strip_prefix(b"**") {
        // `**` spans path separators; an optional following `/` is folded
        // in so `**/x` also matches a top-level `x`.
        let rest = rest.strip_prefix(b"/").unwrap_or(rest);
        if glob_rec(rest, t) {
            return true;
        }
        for i in 0..t.len() {
            if glob_rec(rest, &t[i + 1..]) {
                return true;
            }
        }
        return false;
    }
    match p[0] {
        b'*' => {
            // A single `*` stays within one path segment.
            if glob_rec(&p[1..], t) {
                return true;
            }
            let mut i = 0;
            while i < t.len() && t[i] != b'/' {
                i += 1;
                if glob_rec(&p[1..], &t[i..]) {
                    return true;
                }
            }
            false
        }
        b'?' => !t.is_empty() && t[0] != b'/' && glob_rec(&p[1..], &t[1..]),
        c => !t.is_empty() && t[0] == c && glob_rec(&p[1..], &t[1..]),
    }
}

fn write_snapshot(
    target_dir: &Path,
    snap: &BTreeMap<String, Vec<u8>>,
) -> Result<(), PackageSkillError> {
    fs::create_dir_all(target_dir).map_err(|source| PackageSkillError::Write {
        path: target_dir.to_path_buf(),
        source,
    })?;
    for (rel, bytes) in snap {
        let dest = target_dir.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|source| PackageSkillError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        fs::write(&dest, bytes).map_err(|source| PackageSkillError::Write {
            path: dest.clone(),
            source,
        })?;
    }
    Ok(())
}

fn metadata_is_reparse(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

#[cfg(test)]
#[path = "pkgskill/tests.rs"]
mod tests;
