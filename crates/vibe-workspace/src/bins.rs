//! Declared-binary resolution and slot builds (PROP-025 §§3–4) — the
//! shared cell behind `vibe bin` AND the tcg oracle registry
//! (PROP-026 §4): lockfile → slot → `[[binary]]` declarations →
//! slot-resident artifact, plus the consent-gated release build.
//! Extracted from vibe-cli so the CLI and any tool host resolve
//! through ONE implementation — dispatch-invariant logic must not
//! exist twice.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#dispatch");

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::{BinaryDecl, Lockfile, Manifest};

use crate::Workspace;

mod build;
mod provider;

#[cfg(test)]
use build::prepare_build_output_ignores;
pub use build::{
    BinaryProviderHome, BuildAuthorization, BuildOutput, build_binary, build_binary_authorized,
    build_binary_authorized_with_output, consent_to_build,
};
pub use provider::{find_binary_in_authored_package_root, find_binary_in_provider_slot};

/// This cell's failure surface (one thiserror enum per layer; every
/// message cites its violated REQ and a fix surface).
///
/// ```
/// use vibe_workspace::bins::BinsError;
/// let e = BinsError::UnknownBinary { name: "x".into(), known: vec![] };
/// assert!(e.to_string().contains("vibe bin list"));
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#dispatch")]
pub enum BinsError {
    #[error(
        "violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#dispatch: \
         loading workspace at `{path}`: {detail}; fix surface: run inside a \
         vibevm project (a `vibe.toml` above the cwd)"
    )]
    Workspace { path: PathBuf, detail: String },

    #[error(
        "violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#dispatch: \
         reading lockfile `{path}`: {detail}; fix surface: re-run \
         `vibe install` to regenerate it"
    )]
    Lockfile { path: PathBuf, detail: String },

    #[error(
        "violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#dispatch: \
         no installed package declares a binary `{name}` (declared: \
         {known:?}); fix surface: `vibe bin list` shows the full table"
    )]
    UnknownBinary { name: String, known: Vec<String> },

    #[error(
        "violates spec://org.vibevm.core/vibevm/common/PROP-054#H-BINARY: \
         reading binary provider manifest `{path}`: {detail}; fix surface: \
         reinstall the exact providing package slot"
    )]
    ProviderManifest { path: PathBuf, detail: String },

    #[error(
        "violates spec://org.vibevm.core/vibevm/common/PROP-054#H-BINARY: \
         binary provider slot `{slot}` contains `{actual}`, expected `{expected}`; \
         fix surface: reinstall the provider so its slot and manifest coordinate agree"
    )]
    ProviderMismatch {
        slot: PathBuf,
        expected: String,
        actual: String,
    },

    #[error(
        "violates spec://org.vibevm.core/vibevm/common/PROP-054#H-BINARY: \
         provider `{package}` declares no binary `{name}` (declared: {known:?}); \
         fix surface: name a [[binary]] from that exact provider manifest"
    )]
    UnknownProviderBinary {
        package: String,
        name: String,
        known: Vec<String>,
    },

    #[error(
        "violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#security: \
         building `{name}` runs `{package}`'s build scripts and proc-macros \
         (arbitrary code) and the group `{group}` is not allow-listed; fix \
         surface: consent explicitly — `vibe bin build {name} --assume-yes` \
         (the PROP-020 posture)"
    )]
    ConsentRequired {
        name: String,
        package: String,
        group: String,
    },

    #[error(
        "violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build: \
         declared binary `{name}` has slot `{slot}` outside the authoritative \
         dependency root `{root}`: {reason}; fix surface: re-run `vibe install` so the \
         lockfile and materialised slot use the canonical layout"
    )]
    MalformedSlot {
        name: String,
        slot: PathBuf,
        root: PathBuf,
        reason: String,
    },

    #[error(
        "violates spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD: \
         preparing build-output ignores for `{name}` at `{root}`: {detail}; \
         fix surface: make the dependency root writable and ensure `.gitignore` \
         is an independent regular file with exactly one hardlink"
    )]
    BuildOutputIgnore {
        name: String,
        root: PathBuf,
        detail: String,
    },

    #[error(
        "violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build: \
         spawning cargo for `{name}`: {detail}; fix surface: install a Rust \
         toolchain — package binaries build with the consumer's cargo"
    )]
    CargoSpawn { name: String, detail: String },

    #[error(
        "violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build: cargo \
         failed for `{name}`; fix surface: the slot builds standalone \
         (PROP-024 s2.4) — read cargo's own error, this is a real build \
         error, not a topology one"
    )]
    BuildFailed { name: String },

    #[error(
        "violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#manifest: \
         cargo succeeded but `{artifact}` is missing; fix surface: the \
         [[binary]] declaration's name must equal the crate's bin target"
    )]
    ArtifactMissing { artifact: PathBuf },
}

/// One declared binary, resolved against its installed slot.
#[derive(Debug, Clone)]
pub struct DeclaredBinary {
    pub decl: BinaryDecl,
    /// `<group>/<name>` of the declaring package.
    pub package: String,
    /// The declaring package's group (consent allow-listing).
    pub group: String,
    /// Authoritative dependency root that owns `slot`.
    pub vibedeps_root: PathBuf,
    /// Absolute slot directory.
    pub slot: PathBuf,
}

impl DeclaredBinary {
    /// The bare artifact filename — `<name>.exe` on Windows, `<name>`
    /// elsewhere.
    pub(super) fn artifact_file(&self) -> String {
        if cfg!(windows) {
            format!("{}.exe", self.decl.name)
        } else {
            self.decl.name.clone()
        }
    }

    /// The slot-resident **release** artifact (PROP-025 §3) — where a
    /// consent-gated `cargo build --release` lands, and the stable
    /// fallback dispatch uses when no debug build is present.
    ///
    /// ```
    /// # use vibe_workspace::bins::DeclaredBinary;
    /// # use vibe_core::manifest::BinaryDecl;
    /// let bin = DeclaredBinary {
    ///     decl: BinaryDecl {
    ///         name: "typescript-ai-native-tcg".into(),
    ///         crate_dir: "crates/typescript-ai-native-tcg".into(),
    ///         description: None,
    ///     },
    ///     package: "org.vibevm/typescript-ai-native-lang".into(),
    ///     group: "org.vibevm".into(),
    ///     vibedeps_root: vibe_core::layout::current_vibedeps_root(),
    ///     slot: vibe_core::layout::current_vibedeps_root()
    ///         .join("org.vibevm.typescript-ai-native-lang/0.4.0"),
    /// };
    /// assert!(bin.release_artifact().to_string_lossy().contains("release"));
    /// ```
    pub fn release_artifact(&self) -> PathBuf {
        self.slot
            .join("target")
            .join("release")
            .join(self.artifact_file())
    }

    /// The slot-resident **debug** artifact — where a plain `cargo build`
    /// (no `--release`) in the slot lands. Preferred over the release
    /// build when it exists on disk (see [`Self::artifact`]).
    pub fn debug_artifact(&self) -> PathBuf {
        self.slot
            .join("target")
            .join("debug")
            .join(self.artifact_file())
    }

    /// The artifact dispatch should launch: the **debug** build when one
    /// exists in the slot, otherwise the **release** build. Debug wins so
    /// an iterating developer who has run a plain `cargo build` in the
    /// slot gets that fresh binary without a `--release` rebuild; release
    /// is the stable fallback (and the only artifact `vibe` builds
    /// itself). Both MCP-server registration and `vibe bin exec` resolve
    /// through here, so the launched binary stays consistent across them.
    ///
    /// ```
    /// # use vibe_workspace::bins::DeclaredBinary;
    /// # use vibe_core::manifest::BinaryDecl;
    /// let bin = DeclaredBinary {
    ///     decl: BinaryDecl {
    ///         name: "typescript-ai-native-tcg".into(),
    ///         crate_dir: "crates/typescript-ai-native-tcg".into(),
    ///         description: None,
    ///     },
    ///     package: "org.vibevm/typescript-ai-native-lang".into(),
    ///     group: "org.vibevm".into(),
    ///     vibedeps_root: vibe_core::layout::current_vibedeps_root(),
    ///     slot: vibe_core::layout::current_vibedeps_root()
    ///         .join("org.vibevm.typescript-ai-native-lang/0.4.0"),
    /// };
    /// // No build on disk at this synthetic slot → falls back to release.
    /// let artifact = bin.artifact();
    /// assert!(artifact.starts_with(&bin.slot));
    /// assert!(artifact.to_string_lossy().contains("release"));
    /// ```
    pub fn artifact(&self) -> PathBuf {
        let debug = self.debug_artifact();
        if debug.exists() {
            debug
        } else {
            self.release_artifact()
        }
    }
}

/// Every `[[binary]]` reachable from the project's lockfile slots,
/// sorted by name. A missing lockfile is an empty set, not an error
/// (a fresh project has nothing installed yet).
pub fn collect_binaries(project_root: &Path) -> Result<Vec<DeclaredBinary>, BinsError> {
    let ws = Workspace::discover(project_root).map_err(|e| BinsError::Workspace {
        path: project_root.to_path_buf(),
        detail: e.to_string(),
    })?;
    let vibedeps_root = ws.vibedeps_root();
    let mut out = Vec::new();
    let lock_path = ws.lockfile_path();
    if !lock_path.exists() {
        return Ok(out);
    }
    let lockfile = Lockfile::read(&lock_path).map_err(|e| BinsError::Lockfile {
        path: lock_path.clone(),
        detail: e.to_string(),
    })?;
    for pkg in &lockfile.packages {
        let slot = ws.vibedeps_slot(&pkg.group, &pkg.name, &pkg.version);
        let manifest_path = slot.join(Manifest::FILENAME);
        if !manifest_path.exists() {
            continue;
        }
        let Ok(manifest) = Manifest::read(&manifest_path) else {
            continue;
        };
        for decl in &manifest.binaries {
            out.push(DeclaredBinary {
                decl: decl.clone(),
                package: format!("{}/{}", pkg.group, pkg.name),
                group: pkg.group.to_string(),
                vibedeps_root: vibedeps_root.clone(),
                slot: slot.clone(),
            });
        }
    }
    out.sort_by(|a, b| a.decl.name.cmp(&b.decl.name));
    Ok(out)
}

/// Find one declared binary by its PATH-facing name.
pub fn find_binary<'a>(
    bins: &'a [DeclaredBinary],
    name: &str,
) -> Result<&'a DeclaredBinary, BinsError> {
    bins.iter()
        .find(|b| b.decl.name == name)
        .ok_or_else(|| BinsError::UnknownBinary {
            name: name.to_string(),
            known: bins.iter().map(|b| b.decl.name.clone()).collect(),
        })
}

/// One `[[mcp_server]]` reachable from the project's lockfile slots
/// (PROP-027 §2.4): the declaration plus the resolved `[[binary]]` that
/// serves it — the artifact path, consent group, and slot all come from
/// the binary half.
#[derive(Debug, Clone)]
pub struct DeclaredMcpServer {
    /// The `[[mcp_server]]` table as declared in the slot manifest.
    pub decl: vibe_core::manifest::McpServerDecl,
    /// The `[[binary]]` the declaration references, fully resolved.
    pub binary: DeclaredBinary,
    /// The declaring package's version (registration reports carry it).
    pub version: String,
}

/// Every `[[mcp_server]]` reachable from the project's lockfile slots,
/// sorted by server name. Only `mcp`-kind packages may declare them
/// (`Manifest::validate`), so this is exactly the registerable set; a
/// missing lockfile is an empty set. A declaration whose binary does
/// not resolve is skipped here — the manifest validator refuses such
/// packages at install time, so a slot cannot normally carry one.
pub fn collect_mcp_servers(project_root: &Path) -> Result<Vec<DeclaredMcpServer>, BinsError> {
    let ws = Workspace::discover(project_root).map_err(|e| BinsError::Workspace {
        path: project_root.to_path_buf(),
        detail: e.to_string(),
    })?;
    let vibedeps_root = ws.vibedeps_root();
    let mut out = Vec::new();
    let lock_path = ws.lockfile_path();
    if !lock_path.exists() {
        return Ok(out);
    }
    let lockfile = Lockfile::read(&lock_path).map_err(|e| BinsError::Lockfile {
        path: lock_path.clone(),
        detail: e.to_string(),
    })?;
    for pkg in &lockfile.packages {
        let slot = ws.vibedeps_slot(&pkg.group, &pkg.name, &pkg.version);
        let manifest_path = slot.join(Manifest::FILENAME);
        if !manifest_path.exists() {
            continue;
        }
        let Ok(manifest) = Manifest::read(&manifest_path) else {
            continue;
        };
        for decl in &manifest.mcp_servers {
            let Some(bin_decl) = manifest.binaries.iter().find(|b| b.name == decl.binary) else {
                continue;
            };
            out.push(DeclaredMcpServer {
                decl: decl.clone(),
                binary: DeclaredBinary {
                    decl: bin_decl.clone(),
                    package: format!("{}/{}", pkg.group, pkg.name),
                    group: pkg.group.to_string(),
                    vibedeps_root: vibedeps_root.clone(),
                    slot: slot.clone(),
                },
                version: pkg.version.to_string(),
            });
        }
    }
    out.sort_by(|a, b| a.decl.name.cmp(&b.decl.name));
    Ok(out)
}

#[cfg(test)]
#[path = "bins/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "bins/tests_build_ignore.rs"]
mod tests_build_ignore;
