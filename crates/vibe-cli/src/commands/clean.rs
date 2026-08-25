//! `vibe clean` — remove the derived prompt state, Maven-style (PROP-053).
//!
//! Clean deletes exactly what an install writes: the workspace's vibedeps
//! root and the generated boot artifacts (STATIC / INDEX / INLINE, each
//! carrying the generated-by-vibe marker). It never touches the authored
//! surface (`vibevm/vibespecs/**`, `vibevm/vibefacts/`, the instruction
//! files), never `vibe.lock` (the resolution is not derived state — keeping
//! it is what makes `vibe clean install --offline` reproducible), and never
//! the machine cache (our `~/.m2`; PROP-010). `vibe clean install …` chains
//! straight into the install verb after the wipe.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-053#root");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_workspace::Workspace;

use crate::cli::CleanArgs;
use crate::exit_code::InstallError;
use crate::output;

/// The generated boot artifacts clean may remove — fixed names under the
/// boot lane, deleted only when the file really carries the
/// generated-by-vibe marker in its head (a same-named authored file is
/// left alone, loudly).
const GENERATED_BOOT: [&str; 5] = [
    "STATIC.md",
    "STATIC.xml",
    "INDEX.md",
    "INLINE.md",
    "INLINE.xml",
];

/// Run `vibe clean`, then any chained phase.
pub fn run(
    ctx: &output::Context,
    args: CleanArgs,
    prepare_install: impl FnOnce() -> Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    if args.chain.is_some() {
        return super::lifecycle::run_clean(ctx, args, prepare_install, root_offline);
    }
    wipe(ctx, &args.path, args.assume_yes)?;
    Ok(())
}

/// Execute the one clean phase and return the canonical project root.
pub(crate) fn wipe(ctx: &output::Context, path: &Path, assume_yes: bool) -> Result<PathBuf> {
    let project_root = super::install::resolve_project_root(path)?;
    let workspace = Workspace::discover(&project_root)
        .context("discovering the workspace enclosing the project")?;

    let deps_root = workspace
        .root
        .join(vibe_core::layout::current_vibedeps_root());
    let boot_dir = workspace.root.join(vibe_core::layout::current_boot_dir());

    // Plan the removals first, so the confirmation and the report both
    // name what actually leaves.
    let slot_count = count_slots(&deps_root);
    let generated: Vec<PathBuf> = GENERATED_BOOT
        .iter()
        .map(|name| boot_dir.join(name))
        .filter(|p| is_generated_artifact(p))
        .collect();

    if slot_count == 0 && generated.is_empty() {
        ctx.heading("nothing to clean — no dependency slots, no generated boot artifacts");
    } else {
        let approved = if assume_yes || ctx.is_unattended() || ctx.is_json() {
            true
        } else if !console::user_attended() {
            bail!(
                "no TTY available for confirmation; re-run with `--assume-yes` to clean non-interactively"
            );
        } else {
            Confirm::new()
                .with_prompt(format!(
                    "Remove {slot_count} dependency slot(s) and {} generated boot artifact(s)?",
                    generated.len(),
                ))
                .default(false)
                .interact()
                .context("reading user confirmation")?
        };
        if !approved {
            return Err(InstallError::UserDeclined.into());
        }

        if deps_root.exists() {
            std::fs::remove_dir_all(&deps_root)
                .with_context(|| format!("removing `{}`", deps_root.display()))?;
            ctx.heading(&format!(
                "cleaned {slot_count} dependency slot(s) — `{}` removed",
                vibe_core::machine_json_path(&vibe_core::layout::current_vibedeps_root()),
            ));
        }
        for artifact in &generated {
            std::fs::remove_file(artifact)
                .with_context(|| format!("removing `{}`", artifact.display()))?;
            ctx.heading(&format!(
                "cleaned generated `{}`",
                artifact
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ));
        }
    }

    Ok(project_root)
}

/// Top-level slot directories under the vibedeps root (what the report
/// counts). Zero when the root is absent.
fn count_slots(deps_root: &Path) -> usize {
    std::fs::read_dir(deps_root)
        .map(|entries| entries.flatten().filter(|e| e.path().is_dir()).count())
        .unwrap_or(0)
}

/// `true` when the file exists AND its head carries the generated-by-vibe
/// marker — the guard that keeps clean from deleting a same-named authored
/// file (PROP-053 ##CLEAN-REMOVES-DERIVED).
fn is_generated_artifact(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    text.lines()
        .take(3)
        .any(|l| l.to_ascii_lowercase().contains("generated by vibe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_marker_is_required_before_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let generated = dir.path().join("INDEX.md");
        std::fs::write(&generated, "# INDEX.md — generated by vibe, do not edit.\n").unwrap();
        let authored = dir.path().join("STATIC.md");
        std::fs::write(&authored, "# my own notes\n").unwrap();
        assert!(is_generated_artifact(&generated));
        assert!(!is_generated_artifact(&authored));
        assert!(!is_generated_artifact(&dir.path().join("missing.md")));
    }

    #[test]
    fn slot_counting_tolerates_an_absent_root() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(count_slots(&dir.path().join("nope")), 0);
        std::fs::create_dir_all(dir.path().join("a")).unwrap();
        std::fs::create_dir_all(dir.path().join("b")).unwrap();
        assert_eq!(count_slots(dir.path()), 2);
    }
}
