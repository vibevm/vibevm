//! The build seam (PROP-019 §2.7): compile a source tree into a `vibe`
//! binary. A crate-internal seam (vibe-cli is a bin crate) so tests drive
//! the pipeline without a real cargo build.

specmark::scope!("spec://vibevm/common/PROP-019#build");

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use specmark::spec;

use super::model::{Profile, VersionId};
use super::store::BINARY_NAME;

/// A selector resolved to a concrete version id and commit (PROP-019 §2.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedVersion {
    pub id: VersionId,
    pub commit: String,
}

/// The product of a successful build: where the binary landed and which
/// toolchain produced it (PROP-019 §2.7).
#[derive(Debug, Clone)]
pub(crate) struct BuildOutput {
    pub binary: PathBuf,
    pub toolchain: String,
}

/// Builds a vibevm source tree into a `vibe` binary (PROP-019 §2.7).
pub(crate) trait Builder {
    fn build(&self, source_root: &Path, target_dir: &Path, profile: Profile)
    -> Result<BuildOutput>;
}

/// The production builder: `cargo build [--release] -p vibe-cli` into a
/// managed `--target-dir`, honouring the tree's `rust-toolchain.toml`
/// (PROP-019 §2.7, §2.8).
#[spec(implements = "spec://vibevm/common/PROP-019#build")]
pub(crate) struct CargoBuilder;

impl Builder for CargoBuilder {
    fn build(
        &self,
        source_root: &Path,
        target_dir: &Path,
        profile: Profile,
    ) -> Result<BuildOutput> {
        // Build into a VVM-managed `--target-dir`, never the source tree's
        // own `target/` — keeps the dev tree clean and, load-bearing on
        // Windows, avoids relinking a `vibe.exe` that is running (PROP-019
        // §2.7, §9.3).
        let mut cmd = Command::new("cargo");
        cmd.current_dir(source_root)
            .args(["build", "-p", "vibe-cli"]);
        if profile == Profile::Release {
            cmd.arg("--release");
        }
        cmd.arg("--target-dir").arg(target_dir);
        let status = cmd
            .status()
            .with_context(|| format!("spawning cargo build in `{}`", source_root.display()))?;
        if !status.success() {
            bail!("cargo build failed (exit {:?})", status.code());
        }
        let binary = target_dir.join(profile.target_subdir()).join(BINARY_NAME);
        if !binary.is_file() {
            bail!(
                "build reported success but `{}` is missing",
                binary.display()
            );
        }
        let toolchain = Command::new("rustc")
            .current_dir(source_root)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        Ok(BuildOutput { binary, toolchain })
    }
}

/// Abbreviate a commit hash for a version label (commits are ASCII hex).
pub(crate) fn short_commit(c: &str) -> String {
    c[..c.len().min(10)].to_string()
}
