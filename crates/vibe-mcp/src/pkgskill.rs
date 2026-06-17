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

specmark::scope!("spec://vibevm/common/PROP-018#vibe-skill");

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use specmark::spec;
use thiserror::Error;

use crate::agents::{Agent, Scope};

/// The vibe-skill projection layer's failure surface (PROP-018 §2.5):
/// reading a skill source, writing the projection into an agent's skills
/// directory, or resolving the agent's skills root. One enum for the layer.
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/common/PROP-018#vibe-skill")]
pub enum PackageSkillError {
    #[error(
        "reading skill content at `{path}` failed: {source} \
         (violates spec://vibevm/common/PROP-018#vibe-skill; \
          fix: ensure the package's declared skill source and the agent dirs are readable)"
    )]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "writing the projected skill at `{path}` failed: {source} \
         (violates spec://vibevm/common/PROP-018#vibe-skill; \
          fix: ensure the agent's skills directory is writable)"
    )]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "resolving the agent skills root failed: {detail} \
         (violates spec://vibevm/common/PROP-018#vibe-skill; \
          fix: act on the wrapped agent-config error)"
    )]
    SkillsRoot { detail: String },
}

/// Per-(skill, agent, scope) outcome of projecting a package skill — the
/// structured record `vibe skill` renders or emits as JSON.
#[derive(Debug, Clone, Serialize)]
#[spec(implements = "spec://vibevm/common/PROP-018#vibe-skill")]
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
#[spec(implements = "spec://vibevm/common/PROP-018#vibe-skill")]
pub fn install_package_skill(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    skill_name: &str,
    source: &Path,
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
    let path_str = target.display().to_string().replace('\\', "/");

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

    let desired = snapshot_source(source)?;
    let current = snapshot_dir(&target)?;
    let action = if current.is_none() {
        "created"
    } else if current.as_ref() == Some(&desired) {
        "unchanged"
    } else {
        "updated"
    };

    let status: &'static str = match (action, dry_run) {
        ("unchanged", _) => "unchanged",
        ("created", true) => "would-create",
        ("updated", true) => "would-update",
        (s, _) => s,
    };

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
#[spec(implements = "spec://vibevm/common/PROP-018#vibe-skill")]
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
    let path_str = target.display().to_string().replace('\\', "/");
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
fn snapshot_source(source: &Path) -> Result<BTreeMap<String, Vec<u8>>, PackageSkillError> {
    let mut out = BTreeMap::new();
    if source.is_dir() {
        collect_dir(source, source, &mut out)?;
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
mod tests {
    use super::*;
    use specmark::verifies;

    /// Build a package skill body on disk: `<dir>/skills/<name>/SKILL.md`
    /// plus an asset, returning the skill-body dir.
    fn make_skill_body(root: &Path, body: &str) -> std::path::PathBuf {
        let dir = root.join("skills").join("demo");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), body).unwrap();
        fs::write(dir.join("ref.md"), "asset").unwrap();
        dir
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-018#vibe-skill", r = 3)]
    fn projects_dir_skill_and_is_idempotent() {
        let pkg = tempfile::tempdir().unwrap();
        let proj = tempfile::tempdir().unwrap();
        let body = make_skill_body(pkg.path(), "the skill");

        let r = install_package_skill(
            Agent::ClaudeCode,
            Scope::Project,
            Some(proj.path()),
            "demo",
            &body,
            false,
        )
        .unwrap();
        assert_eq!(r.status, "created");
        let landed = proj
            .path()
            .join(".claude")
            .join("skills")
            .join("demo")
            .join("SKILL.md");
        assert!(landed.is_file());
        assert_eq!(fs::read_to_string(&landed).unwrap(), "the skill");
        assert!(proj.path().join(".claude/skills/demo/ref.md").is_file());

        // Second run with identical bytes → unchanged.
        let r2 = install_package_skill(
            Agent::ClaudeCode,
            Scope::Project,
            Some(proj.path()),
            "demo",
            &body,
            false,
        )
        .unwrap();
        assert_eq!(r2.status, "unchanged");
    }

    #[test]
    fn updates_when_body_diverges_and_drops_stale_files() {
        let pkg = tempfile::tempdir().unwrap();
        let proj = tempfile::tempdir().unwrap();
        let body = make_skill_body(pkg.path(), "v1");
        install_package_skill(
            Agent::OpenCode,
            Scope::Project,
            Some(proj.path()),
            "demo",
            &body,
            false,
        )
        .unwrap();

        // Drop the asset and change the body → the projection must follow.
        fs::remove_file(body.join("ref.md")).unwrap();
        fs::write(body.join("SKILL.md"), "v2").unwrap();
        let r = install_package_skill(
            Agent::OpenCode,
            Scope::Project,
            Some(proj.path()),
            "demo",
            &body,
            false,
        )
        .unwrap();
        assert_eq!(r.status, "updated");
        let base = proj.path().join(".opencode").join("skills").join("demo");
        assert_eq!(fs::read_to_string(base.join("SKILL.md")).unwrap(), "v2");
        assert!(!base.join("ref.md").exists(), "stale file must be dropped");
    }

    #[test]
    fn single_file_source_lands_under_skill_dir() {
        let pkg = tempfile::tempdir().unwrap();
        let proj = tempfile::tempdir().unwrap();
        let file = pkg.path().join("SKILL.md");
        fs::write(&file, "single").unwrap();
        let r = install_package_skill(
            Agent::Codex,
            Scope::Project,
            Some(proj.path()),
            "solo",
            &file,
            false,
        )
        .unwrap();
        assert_eq!(r.status, "created");
        assert_eq!(
            fs::read_to_string(proj.path().join(".agents/skills/solo/SKILL.md")).unwrap(),
            "single"
        );
    }

    #[test]
    fn skipped_for_skill_unsupported_agent() {
        let proj = tempfile::tempdir().unwrap();
        let file = proj.path().join("SKILL.md");
        fs::write(&file, "x").unwrap();
        // Cursor is JSON-config-only — no filesystem skill loader.
        let r = install_package_skill(
            Agent::Cursor,
            Scope::Project,
            Some(proj.path()),
            "k",
            &file,
            false,
        )
        .unwrap();
        assert_eq!(r.status, "skipped");
        assert!(r.path.is_none());
    }

    #[test]
    fn dry_run_writes_nothing() {
        let pkg = tempfile::tempdir().unwrap();
        let proj = tempfile::tempdir().unwrap();
        let body = make_skill_body(pkg.path(), "x");
        let r = install_package_skill(
            Agent::ClaudeCode,
            Scope::Project,
            Some(proj.path()),
            "demo",
            &body,
            true,
        )
        .unwrap();
        assert_eq!(r.status, "would-create");
        assert!(!proj.path().join(".claude").exists());
    }

    #[test]
    fn uninstall_removes_then_reports_absent() {
        let pkg = tempfile::tempdir().unwrap();
        let proj = tempfile::tempdir().unwrap();
        let body = make_skill_body(pkg.path(), "x");
        install_package_skill(
            Agent::ClaudeCode,
            Scope::Project,
            Some(proj.path()),
            "demo",
            &body,
            false,
        )
        .unwrap();
        let r = uninstall_package_skill(
            Agent::ClaudeCode,
            Scope::Project,
            Some(proj.path()),
            "demo",
            false,
        )
        .unwrap();
        assert_eq!(r.status, "removed");
        assert!(!proj.path().join(".claude/skills/demo").exists());
        let r2 = uninstall_package_skill(
            Agent::ClaudeCode,
            Scope::Project,
            Some(proj.path()),
            "demo",
            false,
        )
        .unwrap();
        assert_eq!(r2.status, "absent");
    }
}
