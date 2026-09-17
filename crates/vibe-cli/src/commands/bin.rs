//! `vibe bin` — build and dispatch the tools installed packages declare
//! via `[[binary]]` (PROP-025 §§3–4). The resolution/build cell lives in
//! `vibe_workspace::bins` (shared with the tcg oracle registry,
//! PROP-026 §4 — one implementation, two consumers); this file is the
//! CLI's thin verbs over it.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#dispatch");

use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_workspace::bins::{build_binary, collect_binaries, find_binary};

use crate::output;

/// `vibe bin list`.
pub fn run_list(ctx: &output::Context, project_root: &Path) -> Result<()> {
    let inventory = ctx.progress().task("Reading declared package binaries");
    let bins = match collect_binaries(project_root) {
        Ok(bins) => bins,
        Err(error) => {
            inventory.fail(error.to_string());
            return Err(error.into());
        }
    };
    inventory.set_progress(bins.len() as u64, Some(bins.len() as u64), "binaries");
    inventory.finish();
    if bins.is_empty() {
        eprintln!("bin list: no installed package declares a [[binary]].");
        return Ok(());
    }
    for bin in &bins {
        let state = if bin.artifact().exists() {
            "built"
        } else {
            "not built"
        };
        println!(
            "{}\t{}\t{}\t{}",
            bin.decl.name,
            bin.package,
            state,
            bin.decl.description.as_deref().unwrap_or("")
        );
    }
    eprintln!(
        "{} binar{} declared.",
        bins.len(),
        if bins.len() == 1 { "y" } else { "ies" }
    );
    Ok(())
}

/// `vibe bin build [<names>…]`.
pub fn run_build(
    ctx: &output::Context,
    project_root: &Path,
    names: &[String],
    assume_yes: bool,
) -> Result<()> {
    let discovery = ctx.progress().task("Discovering package binaries");
    let bins = match collect_binaries(project_root) {
        Ok(bins) => bins,
        Err(error) => {
            discovery.fail(error.to_string());
            return Err(error.into());
        }
    };
    discovery.set_progress(bins.len() as u64, Some(bins.len() as u64), "binaries");
    discovery.finish();
    if bins.is_empty() {
        bail!("bin build: no installed package declares a [[binary]]");
    }
    let selected: Vec<_> = if names.is_empty() {
        bins.iter().collect()
    } else {
        let mut chosen = Vec::new();
        for name in names {
            chosen.push(find_binary(&bins, name)?);
        }
        chosen
    };
    let builds = ctx.progress().task("Building package binaries");
    builds.set_progress(0, Some(selected.len() as u64), "binaries");
    for (index, bin) in selected.iter().enumerate() {
        let component = builds
            .progress()
            .task(format!("Building {}", bin.decl.name));
        component.detail(format!("package: {}", bin.package));
        // `bin build` is always rendered as plain line progress, so Cargo's
        // inherited diagnostics and the liveness heartbeat can coexist
        // without cursor control corrupting either stream.
        let result = build_binary(bin, assume_yes);
        match result {
            Ok(()) => component.finish(),
            Err(error) => {
                component.fail(error.to_string());
                builds.fail("binary build failed");
                return Err(error.into());
            }
        }
        builds.set_progress((index + 1) as u64, Some(selected.len() as u64), "binaries");
    }
    builds.finish();
    Ok(())
}

/// `vibe bin path <name>` — the artifact path; non-zero when unbuilt.
pub fn run_path(ctx: &output::Context, project_root: &Path, name: &str) -> Result<()> {
    let lookup = ctx.progress().task(format!("Resolving binary {name}"));
    let bins = match collect_binaries(project_root) {
        Ok(bins) => bins,
        Err(error) => {
            lookup.fail(error.to_string());
            return Err(error.into());
        }
    };
    let bin = match find_binary(&bins, name) {
        Ok(bin) => bin,
        Err(error) => {
            lookup.fail(error.to_string());
            return Err(error.into());
        }
    };
    let artifact = bin.artifact();
    if !artifact.exists() {
        lookup.fail("binary is not built");
        bail!(
            "`{name}` is declared by {} but not built — run `vibe bin build {name}`",
            bin.package
        );
    }
    lookup.finish();
    println!("{}", artifact.display());
    Ok(())
}

/// The `app`-kind boundary of `vibe bin exec` (PROP-057
/// `##KIND-APP-VS-TOOL`).
///
/// The line between `tool` and `app` is mechanical: a `tool` lives in a
/// project and runs through this very command by the lock file, while an
/// `app` runs in no consumer project at all — it is built and deployed on
/// its own. So dispatching an `app`'s binary here is not a missing feature
/// to be added later; it is the one operation the kind says does not
/// exist, and the refusal says which kind drew the line.
///
/// The kind is read from the slot manifest rather than carried on
/// [`DeclaredBinary`](vibe_workspace::bins::DeclaredBinary): the
/// declaration is already addressed by its slot, and only this one verb
/// asks the question.
fn refuse_app_dispatch(bin: &vibe_workspace::bins::DeclaredBinary, name: &str) -> Result<()> {
    let manifest_path = bin.slot.join(vibe_core::manifest::Manifest::FILENAME);
    let Ok(manifest) = vibe_core::manifest::Manifest::read(&manifest_path) else {
        // An unreadable slot manifest is not this verb's diagnosis to
        // make: `vibe check` owns it, and dispatch stays on the path it
        // took before the kind existed.
        return Ok(());
    };
    if manifest.package.map(|p| p.kind) == Some(vibe_core::PackageKind::App) {
        bail!(
            "`{name}` is declared by {} and an `app` package runs nowhere in a consumer \
             project — it is built and deployed on its own \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#KIND-APP-VS-TOOL; \
              fix: run the app through its own deploy profile, or declare the binary in a \
              `tool` package if a project really does dispatch it)",
            bin.package
        );
    }
    Ok(())
}

/// `vibe bin exec <name> -- <args…>` — build-if-missing, then exec with
/// the exit code passed through.
pub fn run_exec(
    ctx: &output::Context,
    project_root: &Path,
    name: &str,
    args: &[String],
    assume_yes: bool,
) -> Result<i32> {
    let preparation = ctx.progress().task(format!("Preparing binary {name}"));
    let bins = match collect_binaries(project_root) {
        Ok(bins) => bins,
        Err(error) => {
            preparation.fail(error.to_string());
            return Err(error.into());
        }
    };
    let bin = match find_binary(&bins, name) {
        Ok(bin) => bin,
        Err(error) => {
            preparation.fail(error.to_string());
            return Err(error.into());
        }
    };
    if let Err(error) = refuse_app_dispatch(bin, name) {
        preparation.fail(error.to_string());
        return Err(error);
    }
    if !bin.artifact().exists() {
        preparation.detail(format!("building package: {}", bin.package));
        if let Err(error) = build_binary(bin, assume_yes) {
            preparation.fail(error.to_string());
            return Err(error.into());
        }
    }
    preparation.finish();
    let status = ctx.suspend_progress(|| {
        std::process::Command::new(bin.artifact())
            .args(args)
            .status()
            .with_context(|| format!("spawning {}", bin.artifact().display()))
    })?;
    Ok(status.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use vibe_core::manifest::BinaryDecl;
    use vibe_workspace::bins::DeclaredBinary;

    use super::refuse_app_dispatch;

    /// One slot on disk carrying a manifest of the given kind, plus the
    /// declaration that addresses it.
    fn declared(kind: &str) -> (tempfile::TempDir, DeclaredBinary) {
        let dir = tempfile::tempdir().expect("tempdir");
        let slot = dir.path().join("slot");
        fs::create_dir_all(&slot).expect("slot");
        fs::write(
            slot.join("vibe.toml"),
            format!(
                "[package]\ngroup = \"org.vibevm.doc\"\nname = \"web\"\nkind = \"{kind}\"\n\
                 version = \"0.1.0\"\n"
            ),
        )
        .expect("slot manifest");
        let bin = DeclaredBinary {
            decl: BinaryDecl {
                name: "site".to_string(),
                crate_dir: PathBuf::from("crates/site"),
                description: None,
            },
            package: "org.vibevm.doc/web".to_string(),
            group: "org.vibevm.doc".to_string(),
            vibedeps_root: dir.path().to_path_buf(),
            slot,
        };
        (dir, bin)
    }

    /// An `app` runs in no consumer project, so the one verb that would
    /// run it there refuses and says which rule drew the line
    /// (PROP-057 `##KIND-APP-VS-TOOL`).
    #[test]
    fn an_app_package_is_never_dispatched_by_bin_exec() {
        let (_dir, bin) = declared("app");
        let error = refuse_app_dispatch(&bin, "site").expect_err("an app never dispatches");
        let message = error.to_string();
        assert!(
            message.contains("org.vibevm.doc/web"),
            "the refusal names the declaring package: {message}"
        );
        assert!(
            message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#KIND-APP-VS-TOOL"),
            "the refusal is navigable back to the rule: {message}"
        );
    }

    /// Every other kind dispatches as it always did — the boundary is
    /// `app` against `tool`, not a new gate on `vibe bin exec`.
    ///
    /// The loop names the kinds whose manifest is complete with
    /// `[package]` alone; `mcp` and `doc` each owe their own tables, so
    /// their slot manifests would fail to READ here and take the
    /// unreadable-manifest path instead of the one under test.
    #[test]
    fn every_other_kind_still_dispatches() {
        for kind in ["flow", "feat", "stack", "tool", "lang"] {
            let (_dir, bin) = declared(kind);
            refuse_app_dispatch(&bin, "site")
                .unwrap_or_else(|e| panic!("`{kind}` must still dispatch: {e}"));
        }
    }
}
