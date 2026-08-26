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

#[path = "pkgskill/projection.rs"]
mod projection;

pub use projection::{
    DeclaredSkill, DeclaredSkillFilter, DeclaredSkillProjection, collect_declared_skills,
    prepare_declared_skill_projection, project_declared_skills_project_scope,
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
    let path_str = machine_json_path(&target);

    if !source.exists() {
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
        if target.exists() {
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
    let path_str = machine_json_path(&target);
    let exists = target.exists();
    let status: &'static str = match (exists, dry_run) {
        (false, _) => "absent",
        (true, true) => "would-remove",
        (true, false) => "removed",
    };
    if exists && !dry_run {
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
fn snapshot_source(
    source: &Path,
    include: &[String],
) -> Result<BTreeMap<String, Vec<u8>>, PackageSkillError> {
    let mut out = BTreeMap::new();
    if source.is_dir() {
        collect_dir(source, source, &mut out)?;
        // PROP-015 §2.8: when `include` is set, keep only the files whose
        // relpath matches one of the patterns. Empty `include` keeps the
        // whole tree (the §2.6 default).
        if !include.is_empty() {
            out.retain(|rel, _| include.iter().any(|pat| glob_match(pat, rel)));
        }
    } else {
        let name = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "SKILL.md".to_string());
        let bytes = fs::read(source).map_err(|err| PackageSkillError::Read {
            path: source.to_path_buf(),
            source: err,
        })?;
        out.insert(name, bytes);
    }
    Ok(out)
}

/// Snapshot an existing target dir, or `None` when it does not exist.
fn snapshot_dir(dir: &Path) -> Result<Option<BTreeMap<String, Vec<u8>>>, PackageSkillError> {
    if !dir.exists() {
        return Ok(None);
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
        if path.is_dir() {
            collect_dir(base, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&path).map_err(|source| PackageSkillError::Read {
                path: path.clone(),
                source,
            })?;
            out.insert(rel, bytes);
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

#[cfg(test)]
#[path = "pkgskill/tests.rs"]
mod tests;
