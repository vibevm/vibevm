//! Path helpers, display utils, report rendering, file-ensurance, and
//! templates for `vibe init`.

use super::*;
use crate::cli::InitArgs;
use crate::output;
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;
use vibe_core::manifest::{Lockfile, Manifest, ProjectSection, RegistrySection};

pub(super) fn resolve_name(args: &InitArgs, path: &Path) -> Result<String> {
    if let Some(n) = &args.name {
        return Ok(n.clone());
    }
    let basename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");
    Ok(basename.to_string())
}

pub(super) fn relative_to_root(root: &Path, full: &Path) -> String {
    let stripped = full.strip_prefix(root).unwrap_or(full);
    display_pathbuf(stripped)
}

pub(super) fn display_pathbuf(p: &Path) -> String {
    // Display with forward slashes — consistent across macOS/Linux/Windows.
    let s = p.display().to_string();
    s.replace('\\', "/")
}

/// Canonicalize and strip Windows UNC (`\\?\`) prefix where present.
pub(super) fn canonical_no_unc(path: &Path) -> Result<std::path::PathBuf> {
    let canon = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    Ok(strip_unc(canon))
}

#[cfg(windows)]
pub(super) fn strip_unc(p: std::path::PathBuf) -> std::path::PathBuf {
    let s = p.as_os_str().to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        std::path::PathBuf::from(rest)
    } else {
        p
    }
}

#[cfg(not(windows))]
pub(super) fn strip_unc(p: std::path::PathBuf) -> std::path::PathBuf {
    p
}

/// Re-export for sibling command modules.
pub(crate) fn strip_unc_public(p: std::path::PathBuf) -> std::path::PathBuf {
    strip_unc(p)
}

/// Prefer the user-supplied display (e.g. `.`) if it still points at the
/// canonical path; otherwise fall back to the canonical (UNC-stripped) form.
pub(super) fn normalize_display(requested: &Path, canonical: &Path) -> String {
    let requested_matches = requested
        .canonicalize()
        .map(|c| strip_unc(c) == *canonical)
        .unwrap_or(false);
    if requested_matches {
        display_pathbuf(requested)
    } else {
        display_pathbuf(canonical)
    }
}

pub(crate) fn current_timestamp_utc() -> String {
    vibe_core::timestamp::now_utc()
}

pub(super) fn report(
    ctx: &output::Context,
    name: &str,
    display_root: &str,
    outcomes: &[Outcome],
) -> Result<()> {
    let created = outcomes
        .iter()
        .filter(|o| o.action == Action::Created)
        .count();
    let kept = outcomes.iter().filter(|o| o.action == Action::Kept).count();

    if ctx.is_json() {
        use vibe_wire::generated::init_report::{
            InitReport, Outcome as WireOutcome, OutcomeAction,
        };
        let payload = InitReport {
            ok: true,
            command: "init".to_string(),
            project: name.to_string(),
            path: display_root.to_string(),
            created: u32::try_from(created).unwrap_or(u32::MAX),
            kept: u32::try_from(kept).unwrap_or(u32::MAX),
            outcomes: outcomes
                .iter()
                .map(|o| WireOutcome {
                    path: o.path.clone(),
                    action: match o.action {
                        Action::Created => OutcomeAction::Created,
                        Action::Kept => OutcomeAction::Kept,
                    },
                    reason: o.reason.to_string(),
                })
                .collect(),
        };
        ctx.emit_json(&payload)?;
        return Ok(());
    }

    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe init: {created} created, {kept} kept in `{display_root}`"
        ));
        return Ok(());
    }

    println!();
    ctx.summary(&format!(
        "Done. Project `{name}`: {created} file{} created, {kept} kept.",
        if created == 1 { "" } else { "s" }
    ));
    println!();
    println!("Next:");
    println!("  • edit spec/boot/00-core.md and spec/common/ as your project takes shape");
    println!("  • install packages with `vibe install <kind>:<name>` (e.g. flow:wal)");
    Ok(())
}

// ==== Templates ============================================================
//
// The project-facing content `vibe init` writes lives as data under
// `crates/vibe-cli/templates/` and is pulled in with `include_str!`: code
// renders, data carries the prose. `.gitattributes` pins the tree to LF, so
// the bytes `include_str!` embeds are the bytes the e2e tests expect.

pub(super) const BOOT_90_USER_TEMPLATE: &str = include_str!("../../../templates/boot-90-user.md");

pub(super) fn boot_00_core_template(project_name: &str) -> String {
    include_str!("../../../templates/boot-00-core.md").replace("{project_name}", project_name)
}

pub(super) const ROOT_GITIGNORE_TEMPLATE: &str = include_str!("../../../templates/root-gitignore");
pub(super) fn generate_boot_artifacts(ctx: &output::Context, path: &Path) -> Result<Vec<Outcome>> {
    const ARTIFACTS: [&str; 4] = ["spec/boot/INDEX.md", "CLAUDE.md", "AGENTS.md", "GEMINI.md"];
    let preexisting: Vec<bool> = ARTIFACTS.iter().map(|f| path.join(f).exists()).collect();

    let workspace = vibe_workspace::Workspace::load(path)
        .with_context(|| "loading the new project to generate its boot artifacts")?;
    vibe_workspace::install::regenerate_boot(&workspace)
        .with_context(|| "generating the boot artifacts")?;

    let mut outcomes = Vec::with_capacity(ARTIFACTS.len());
    for (artifact, &existed) in ARTIFACTS.iter().zip(&preexisting) {
        if existed {
            ctx.skipped(artifact, "regenerated");
        } else {
            ctx.created(artifact);
        }
        outcomes.push(Outcome {
            path: (*artifact).to_string(),
            action: if existed {
                Action::Kept
            } else {
                Action::Created
            },
            reason: "boot artifact",
        });
    }
    Ok(outcomes)
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct Outcome {
    pub(super) path: String,
    pub(super) action: Action,
    pub(super) reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Action {
    Created,
    Kept,
}

pub(super) fn ensure_file(
    ctx: &output::Context,
    root: &Path,
    path: &Path,
    content: &str,
    reason: &'static str,
) -> Result<Outcome> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let rel = relative_to_root(root, path);
    if path.exists() {
        ctx.skipped(&rel, "already exists");
        return Ok(Outcome {
            path: rel,
            action: Action::Kept,
            reason,
        });
    }
    fs::write(path, content).with_context(|| format!("writing `{}`", path.display()))?;
    ctx.created(&rel);
    Ok(Outcome {
        path: rel,
        action: Action::Created,
        reason,
    })
}

pub(super) fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("creating `{}`", path.display()))
}

/// Ensure the project-root `.gitignore` protects the L3 personal-settings file
/// (PROP-040 §9 `#gitignore-autogen`): `/.vibe/settings.local.toml` must never
/// be accidentally committed. Idempotent — if the file already ignores the local
/// settings basename (any line naming `settings.local.toml`), this is a no-op;
/// otherwise it appends a small section, preserving all existing content.
/// Returns `Some(Outcome)` only when it appended, `None` when already covered or
/// no root `.gitignore` is present (the step-5 `ensure_file` owns creation).
pub(super) fn ensure_local_settings_gitignored(root: &Path) -> Result<Option<Outcome>> {
    let path = root.join(".gitignore");
    let existing = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => return Ok(None),
    };
    // Marker: any line that ignores the local settings file. Match the
    // distinctive basename so an operator's own spelling still counts.
    if existing
        .lines()
        .any(|line| line.contains("settings.local.toml"))
    {
        return Ok(None);
    }
    let mut content = existing;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(
        "\n# vibevm personal settings (L3 — never committed; PROP-040 §9 #gitignore-autogen)\n\
         /.vibe/settings.local.toml\n\
         /.vibe/*.local.toml\n",
    );
    fs::write(&path, &content)
        .with_context(|| format!("appending L3 settings entry to `{}`", path.display()))?;
    Ok(Some(Outcome {
        path: relative_to_root(root, &path),
        action: Action::Created,
        reason: "gitignore: L3 personal settings",
    }))
}

pub(super) fn ensure_project_manifest(
    ctx: &output::Context,
    root: &Path,
    name: &str,
    stack: Option<&str>,
    registries: Vec<RegistrySection>,
    authors: &[String],
) -> Result<Outcome> {
    let path = root.join(Manifest::FILENAME);
    let rel = relative_to_root(root, &path);
    if path.exists() {
        ctx.skipped(&rel, "already exists");
        return Ok(Outcome {
            path: rel,
            action: Action::Kept,
            reason: "project manifest",
        });
    }

    let manifest = Manifest {
        project: Some(ProjectSection {
            name: name.to_string(),
            version: "0.0.1".to_string(),
            authors: authors.to_vec(),
        }),
        active: stack.map(|s| ActiveSection {
            stack: Some(s.to_string()),
        }),
        registries,
        ..Default::default()
    };

    manifest.write(&path)?;
    ctx.created(&rel);
    Ok(Outcome {
        path: rel,
        action: Action::Created,
        reason: "project manifest",
    })
}

pub(super) fn ensure_empty_lockfile(ctx: &output::Context, root: &Path) -> Result<Outcome> {
    let path = root.join(Lockfile::FILENAME);
    let rel = relative_to_root(root, &path);
    if path.exists() {
        ctx.skipped(&rel, "already exists");
        return Ok(Outcome {
            path: rel,
            action: Action::Kept,
            reason: "lockfile",
        });
    }
    let lockfile = Lockfile::empty(
        format!("vibe {}", env!("CARGO_PKG_VERSION")),
        current_timestamp_utc(),
    );
    lockfile.write(&path)?;
    ctx.created(&rel);
    Ok(Outcome {
        path: rel,
        action: Action::Created,
        reason: "lockfile",
    })
}
