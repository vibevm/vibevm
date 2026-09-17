//! `vibe doc surface` and `vibe doc diff` — the thin surface over the
//! library's pseudo-history of versions (PROP-057 `##PIPE-LIBRARY`,
//! `##OBS-SURFACE-SNAPSHOTS`).
//!
//! Everything with content in it lives in `vibe_doc::surface`. This
//! module turns flags into the library's options, fetches the one half
//! the library refuses to fetch for itself — the documentation
//! obligations, which come from the grounding cell every `vibe facts`
//! verb enters through — and prints.
//!
//! The `now` side of a comparison is behind a flag on purpose. Inside a
//! version the product changes invisibly by design, and a comparison
//! against the working tree answers a question this project otherwise
//! refuses to ask; the norm allows it as a HINT to a full reconciliation
//! and nothing else, so the place where that is decided is a flag an
//! operator has to type.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-SURFACE-SNAPSHOTS");

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use vibe_doc::coverage::Obligation;
use vibe_doc::derived;
use vibe_doc::surface::{self, SurfaceEnv};
use vibe_wire::generated::doc_surface::DocSurface;

use super::{DocEnv, observe};
use crate::cli::{DocDiffArgs, DocSurfaceArgs, ProgressCommonArgs};

/// The word that means «the product this command is», rather than a
/// version somebody declared.
const NOW: &str = "now";

/// `vibe doc surface --record <version>`.
pub fn run_surface(args: DocSurfaceArgs, env: DocEnv) -> Result<()> {
    let surface_env = surface_env(&env, args.binary.clone(), args.timeout);
    let obligations = obligations(&env.cwd, &env.progress)?;
    let recorded = observe::phase(&env.progress, "Recording documentation surface", || {
        surface::record(&args.record, &obligations, &surface_env)
    })?;
    let path = match &args.out {
        Some(dir) => dir.join(format!("{}.json", args.record)),
        None => surface::path_for(&args.path, &args.record),
    };
    observe::phase(&env.progress, "Writing documentation surface", || {
        surface::write(&recorded, &path)
    })?;
    println!(
        "surface {}: {} command(s), {} manifest field(s), {} lock field(s), {} schema(s), \
         {} obligation(s), {} format(s)",
        recorded.version,
        recorded.commands.len(),
        recorded.manifest_fields.len(),
        recorded.lock_fields.len(),
        recorded.schemas.len(),
        recorded.facts.len(),
        recorded.formats.len()
    );
    println!("  written to {}", path.display());
    Ok(())
}

/// `vibe doc diff <old> <new>`.
pub fn run_diff(args: DocDiffArgs, env: DocEnv) -> Result<()> {
    let old = observe::phase(
        &env.progress,
        "Loading previous documentation surface",
        || load(&args.path, &args.from, &args, &env),
    )?;
    let new = observe::phase(&env.progress, "Loading next documentation surface", || {
        load(&args.path, &args.to, &args, &env)
    })?;
    let document = observe::phase(&env.progress, "Comparing documentation surfaces", || {
        compare(&old, &new, &args, &env)
    })?;
    if args.format == "json" {
        print!("{}", vibe_doc::surface::diff::to_json(&document));
    } else {
        print!("{}", vibe_doc::surface::diff::render_md(&document));
    }
    Ok(())
}

/// One side of a comparison: a recorded snapshot, or the running product
/// when the operator asked for `now` and said so.
fn load(package_dir: &Path, version: &str, args: &DocDiffArgs, env: &DocEnv) -> Result<DocSurface> {
    if version == NOW {
        if !args.allow_now {
            bail!(
                "`now` is not a version: inside one, the product changes invisibly by \
                 design, so comparing against the working tree answers a question the \
                 project does not otherwise ask (violates \
                 spec://org.vibevm.core/vibevm/common/PROP-057#OBS-VERSION-CONTRACT; \
                 fix: name a recorded version, or pass --allow-now to take the hint a \
                 full reconciliation may use)"
            );
        }
        let surface_env = surface_env(env, args.binary.clone(), args.timeout);
        let obligations = obligations(&env.cwd, &env.progress)?;
        // Read and not written: a snapshot keyed on `now` would be a
        // snapshot of nothing a month from today.
        return Ok(surface::record(NOW, &obligations, &surface_env)?);
    }
    let path = surface::path_for(package_dir, version);
    if !path.is_file() {
        let known = surface::recorded(package_dir)?;
        bail!(
            "no surface recorded for `{version}` — this package holds {} \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#OBS-SURFACE-SNAPSHOTS; \
             fix: record it with `vibe doc surface --record {version}` against the binary \
             that version names)",
            if known.is_empty() {
                "no snapshots at all".to_owned()
            } else {
                known.join(", ")
            }
        );
    }
    Ok(surface::read(&path)?)
}

/// Place the changes in pages, with the four sources a citation resolves
/// against and the corpus the obligations were observed in.
fn compare(
    old: &DocSurface,
    new: &DocSurface,
    args: &DocDiffArgs,
    env: &DocEnv,
) -> Result<vibe_wire::generated::doc_surface_diff::DocSurfaceDiff> {
    let coordinate = derived::coordinate_of(&args.path)?;
    let settings = super::settings_home(&env.settings, &env.home);
    let sources = super::spec_sources(&env.cwd, settings.as_deref());
    // An obligation's address is relative to the tree it was observed
    // in, which is the checkout this command stands in. Without one the
    // rule half of the placement cannot be resolved, and the honest
    // answer is a refusal rather than a diff missing a third of itself.
    let Some(corpus_root) = env.cwd.clone() else {
        bail!(
            "`vibe doc diff` needs the tree whose specifications state the obligations, \
             and this run has no working directory to read one from (violates \
             spec://org.vibevm.core/vibevm/common/PROP-057#OBS-SURFACE-SNAPSHOTS; \
             fix: run it from the project whose `facts.toml` names the observed corpus)"
        );
    };
    Ok(vibe_doc::surface::diff::diff(
        old,
        new,
        &args.path,
        &coordinate,
        &sources,
        &corpus_root,
    )?)
}

/// What a snapshot needs from the composition root.
fn surface_env(env: &DocEnv, binary: Option<PathBuf>, timeout: u64) -> SurfaceEnv {
    SurfaceEnv {
        binary: binary
            .or_else(|| env.current_exe.clone())
            .unwrap_or_else(|| PathBuf::from("vibe")),
        repo_root: env.cwd.clone().unwrap_or_else(|| PathBuf::from(".")),
        timeout_secs: timeout,
    }
}

/// The documentation obligations of the checkout, from the one cell that
/// owns «which files does this project observe».
///
/// A run outside a checkout records no obligations rather than refusing:
/// the other five halves of a surface are still a surface, and a snapshot
/// taken from a warmed store is a legitimate thing to want. The count is
/// printed, so «no corpus» and «no obligations» do not read the same.
pub(super) fn obligations(
    cwd: &Option<PathBuf>,
    progress: &vibe_core::progress::Progress,
) -> Result<Vec<Obligation>> {
    let Some(root) = cwd.clone() else {
        return Ok(Vec::new());
    };
    let grounded = observe::phase(progress, "Grounding documentation obligations", || {
        crate::commands::progress::grounding::ground(&ProgressCommonArgs {
            path: root,
            campaign: None,
            no_cache: false,
        })
    })?;
    Ok(vibe_doc::coverage::obligations(grounded.docs.iter()))
}

#[cfg(test)]
mod tests;
