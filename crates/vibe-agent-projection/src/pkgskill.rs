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
use sha2::{Digest, Sha256};
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

#[path = "pkgskill/activation.rs"]
mod activation;
#[path = "pkgskill/snapshot.rs"]
mod snapshot;
pub use activation::{
    SkillActivationStatus, SkillActivationTarget, admit_managed_skill_activation,
    change_managed_skill_activation, install_managed_skill_snapshot, managed_skill_status,
    register_builtin_skill_snapshot, retain_managed_skill_activation,
};

#[path = "pkgskill/standalone.rs"]
mod standalone;
use snapshot::*;
use standalone::*;

pub use exact_path::EscapedOsPath;
pub use projection::{
    DeclaredSkill, DeclaredSkillFilter, DeclaredSkillProjection, DeclaredSkillProvider,
    PROJECT_SKILL_PREFIX, PROJECT_SKILL_RECONCILE_KEY, PROJECT_SKILL_RECOVER_KEY,
    ProjectSkillBinding, ProjectSkillProviderInput, ProjectSkillTarget, collect_declared_skills,
    collect_project_skill_bindings, declared_skill_activation_status, lower_project_skill_bindings,
    prepare_declared_skill_activation, prepare_declared_skill_projection,
    probe_project_skill_binding, probe_recovered_project_skill_bindings,
    probe_vanished_project_skill_bindings, project_declared_skills_project_scope,
    project_skill_receipt_exists, reconcile_project_skill_binding,
    reconcile_vanished_project_skill_bindings, recover_project_skill_bindings,
};

/// The vibe-skill projection layer's failure surface (PROP-018 §2.5):
/// reading a skill source, writing the projection into an agent's skills
/// directory, or resolving the agent's skills root. One enum for the layer.
///
/// ```
/// use vibe_agent_projection::pkgskill::PackageSkillError;
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

    #[error(
        "invalid projected skill metadata at `{path}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill; \
          fix: make the SKILL.md frontmatter name match the manifest-declared skill name)"
    )]
    InvalidMetadata { path: PathBuf, reason: String },
}

const STANDALONE_RECEIPT_SCHEMA: u32 = 1;

use vibe_wire::generated::standalone_skill_receipt::{
    OwnedFile as StandaloneOwnedFile, StandaloneSkillReceipt,
};

/// Per-(skill, agent, scope) outcome of projecting a package skill — the
/// structured record `vibe skill` renders or emits as JSON.
///
/// ```
/// use vibe_agent_projection::pkgskill::PackageSkillReport;
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
/// projection is left `unchanged`; a divergent owned projection is reconciled
/// and reported `updated`, so a file the source dropped leaves no stale owned
/// copy while unrelated neighbor files survive. Agents with no filesystem
/// skill loader (Cursor, Claude Desktop) or no surface for this scope report
/// `skipped`.
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
            path: Some(machine_json_path(&target)),
            status: "skipped",
            note: Some(format!("skill source `{}` not found", source.display())),
        });
    }

    let desired = snapshot_source(source, include)?;
    validate_skill_frontmatter(skill_name, source, &desired)?;
    install_managed_skill_snapshot(
        agent,
        scope,
        project_root,
        None,
        skill_name,
        &desired,
        dry_run,
    )
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
    let receipt_path = standalone_receipt_path(&root, skill_name);
    let prior = read_standalone_receipt(&receipt_path, skill_name)?;
    let current = snapshot_dir(&target)?;
    validate_standalone_uninstall(&target, current.as_ref(), prior.as_ref())?;
    let installed = exists || prior.is_some();
    let status: &'static str = match (installed, dry_run) {
        (false, _) => "absent",
        (true, true) => "would-remove",
        (true, false) => "removed",
    };
    if installed && !dry_run {
        receipt::ensure_no_follow_walk(containment_root, &target, true).map_err(|error| {
            PackageSkillError::UnsafePath {
                path: target.clone(),
                reason: error.to_string(),
            }
        })?;
        remove_owned_standalone_files(&target, prior.as_ref())?;
        remove_file_if_present(&receipt_path)?;
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

pub(crate) fn validate_skill_frontmatter(
    skill_name: &str,
    source: &Path,
    selected: &BTreeMap<String, Vec<u8>>,
) -> Result<(), PackageSkillError> {
    let Some(bytes) = selected.get("SKILL.md") else {
        return Ok(());
    };
    let skill_path = if source.is_file() {
        source.to_path_buf()
    } else {
        source.join("SKILL.md")
    };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(());
    };
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return Ok(());
    }
    let mut declared_name: Option<String> = None;
    let mut closed = false;
    for line in lines {
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        if line.starts_with(char::is_whitespace) || line.trim_start().starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim() != "name" {
            continue;
        }
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .or_else(|| {
                value
                    .strip_prefix('\'')
                    .and_then(|value| value.strip_suffix('\''))
            })
            .unwrap_or_else(|| {
                value
                    .split_once(" #")
                    .map_or(value, |(name, _)| name.trim())
            });
        if declared_name.replace(value.to_string()).is_some() {
            return Err(PackageSkillError::InvalidMetadata {
                path: skill_path,
                reason: "frontmatter contains more than one top-level `name`".into(),
            });
        }
    }
    if !closed {
        return if declared_name.is_some() {
            Err(PackageSkillError::InvalidMetadata {
                path: skill_path,
                reason: "frontmatter opening delimiter has no closing `---`".into(),
            })
        } else {
            Ok(())
        };
    }
    if let Some(declared_name) = declared_name
        && declared_name != skill_name
    {
        return Err(PackageSkillError::InvalidMetadata {
            path: skill_path,
            reason: format!(
                "frontmatter name `{declared_name}` does not match manifest-declared skill name `{skill_name}`"
            ),
        });
    }
    Ok(())
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
