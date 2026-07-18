//! Packaging the vibeterm Electron app into a relocatable directory for the VVM
//! install pipeline (PROP-019 §2.7 amendment). A crate-internal seam so tests
//! drive the pipeline without npm/electron — they inject [`SkipPackager`] (or a
//! fake) and assert the dist set, never running the real packaging step.

specmark::scope!("spec://vibevm/common/PROP-019#build");

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use specmark::spec;

use super::tools::has_tool;
use crate::output;

/// The product of a vibeterm packaging step: the staged, relocatable app dir
/// (the electron binary at its root, `resources/app/` inside — produced by
/// `apps/vibeterm/scripts/package.mjs`).
pub(crate) struct VibetermOutput {
    pub dir: PathBuf,
}

/// Packages the vibeterm app from a source tree (PROP-019 §2.7 amendment).
/// `Ok(None)` means the step was gracefully skipped — a Rust-only dev box
/// (npm/electron unavailable), or a tree that carries no `apps/vibeterm`. The
/// instance still installs; `vibe term` then names the missing setup step
/// rather than hanging.
pub(crate) trait VibetermPackager {
    fn package(&self, source_root: &Path, staging_root: &Path) -> Result<Option<VibetermOutput>>;
}

/// The production packager: drives `apps/vibeterm/scripts/package.mjs`, which
/// encapsulates the npm 11 / node-pty-ABI / electron-packager dance. Packaging
/// is per-OS — it runs on the target host (see PROP-019 §2.7).
pub(crate) struct NpmPackager<'a> {
    ctx: &'a output::Context,
}

impl<'a> NpmPackager<'a> {
    pub(crate) fn new(ctx: &'a output::Context) -> Self {
        Self { ctx }
    }
}

impl VibetermPackager for NpmPackager<'_> {
    #[spec(implements = "spec://vibevm/common/PROP-019#build")]
    fn package(&self, source_root: &Path, staging_root: &Path) -> Result<Option<VibetermOutput>> {
        let app = source_root.join("apps").join("vibeterm");
        if !app.join("package.json").is_file() {
            // A tree without apps/vibeterm (an old tag, a partial checkout). Say
            // so — a silent skip here is exactly what makes a later `vibe term`
            // failure mysterious.
            self.ctx.summary(
                "vibeterm not packaged (no apps/vibeterm in the source tree) — \
                 `vibe term` will name the setup step",
            );
            return Ok(None);
        }
        // The packaging script needs node + npm on PATH. Absent → graceful skip;
        // the instance installs without vibeterm (vibe term then errors clearly).
        if !has_tool("npm") || !has_tool("node") {
            self.ctx.summary(
                "vibeterm not packaged (npm/node unavailable) — \
                 `vibe term` will name the setup step",
            );
            return Ok(None);
        }
        std::fs::create_dir_all(staging_root)
            .with_context(|| format!("creating vibeterm staging `{}`", staging_root.display()))?;
        self.ctx.step(&format!(
            "packaging vibeterm into {}",
            staging_root.display()
        ));
        let script = app.join("scripts").join("package.mjs");
        let status = Command::new("node")
            .arg(&script)
            .arg("--out")
            .arg(staging_root)
            .current_dir(&app)
            .status()
            .with_context(|| format!("spawning vibeterm packaging ({})", script.display()))?;
        if !status.success() {
            bail!("vibeterm packaging failed (exit {:?})", status.code());
        }
        // package.mjs leaves exactly one `vibeterm-<plat>-<arch>` child; resolve
        // it so the in-instance rels are `vibeterm/…` (not a nested platform dir).
        let children: Vec<PathBuf> = std::fs::read_dir(staging_root)
            .with_context(|| format!("reading vibeterm staging `{}`", staging_root.display()))?
            .filter_map(Result::ok)
            .filter_map(|e| {
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let is_pkg = e.file_name().to_string_lossy().starts_with("vibeterm-");
                (is_dir && is_pkg).then(|| e.path())
            })
            .collect();
        if children.len() != 1 {
            bail!(
                "vibeterm packaging produced {} `vibeterm-*` dirs under `{}` (expected 1)",
                children.len(),
                staging_root.display()
            );
        }
        // `children.len() == 1` here (checked above); take it without `unwrap`
        // (the conform `no-unwrap-in-domain` ban).
        let dir = match children.into_iter().next() {
            Some(d) => d,
            None => bail!("vibeterm packaging produced no `vibeterm-*` dir"),
        };
        Ok(Some(VibetermOutput { dir }))
    }
}

/// A no-op packager for tests: always skips vibeterm (the dist set reduces to
/// the binary alone, matching the pre-vibeterm-pipeline behaviour).
#[cfg(test)]
pub(crate) struct SkipPackager;

#[cfg(test)]
impl VibetermPackager for SkipPackager {
    fn package(&self, _source_root: &Path, _staging_root: &Path) -> Result<Option<VibetermOutput>> {
        Ok(None)
    }
}
