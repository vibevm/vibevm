//! Path helpers, display utils, report rendering, and templates for
//! `vibe init`. Split out of `mod.rs` to keep the main module under the
//! 600-line file budget.

use super::*;
use crate::cli::InitArgs;
use crate::output;
use std::path::Path;

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
