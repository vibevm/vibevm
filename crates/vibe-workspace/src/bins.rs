//! Declared-binary resolution and slot builds (PROP-025 §§3–4) — the
//! shared cell behind `vibe bin` AND the tcg oracle registry
//! (PROP-026 §4): lockfile → slot → `[[binary]]` declarations →
//! slot-resident artifact, plus the consent-gated release build.
//! Extracted from vibe-cli so the CLI and any tool host resolve
//! through ONE implementation — dispatch-invariant logic must not
//! exist twice.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-025#dispatch");

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::{BinaryDecl, Lockfile, Manifest};

use crate::Workspace;

/// This cell's failure surface (one thiserror enum per layer; every
/// message cites its violated REQ and a fix surface).
///
/// ```
/// use vibe_workspace::bins::BinsError;
/// let e = BinsError::UnknownBinary { name: "x".into(), known: vec![] };
/// assert!(e.to_string().contains("vibe bin list"));
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://vibevm/modules/vibe-workspace/PROP-025#dispatch")]
pub enum BinsError {
    #[error(
        "violates spec://vibevm/modules/vibe-workspace/PROP-025#dispatch: \
         loading workspace at `{path}`: {detail}; fix surface: run inside a \
         vibevm project (a `vibe.toml` above the cwd)"
    )]
    Workspace { path: PathBuf, detail: String },

    #[error(
        "violates spec://vibevm/modules/vibe-workspace/PROP-025#dispatch: \
         reading lockfile `{path}`: {detail}; fix surface: re-run \
         `vibe install` to regenerate it"
    )]
    Lockfile { path: PathBuf, detail: String },

    #[error(
        "violates spec://vibevm/modules/vibe-workspace/PROP-025#dispatch: \
         no installed package declares a binary `{name}` (declared: \
         {known:?}); fix surface: `vibe bin list` shows the full table"
    )]
    UnknownBinary { name: String, known: Vec<String> },

    #[error(
        "violates spec://vibevm/modules/vibe-workspace/PROP-025#security: \
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
        "violates spec://vibevm/modules/vibe-workspace/PROP-025#build: \
         spawning cargo for `{name}`: {detail}; fix surface: install a Rust \
         toolchain — package binaries build with the consumer's cargo"
    )]
    CargoSpawn { name: String, detail: String },

    #[error(
        "violates spec://vibevm/modules/vibe-workspace/PROP-025#build: cargo \
         failed for `{name}`; fix surface: the slot builds standalone \
         (PROP-024 s2.4) — read cargo's own error, this is a real build \
         error, not a topology one"
    )]
    BuildFailed { name: String },

    #[error(
        "violates spec://vibevm/modules/vibe-workspace/PROP-025#manifest: \
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
    /// Absolute slot directory.
    pub slot: PathBuf,
}

impl DeclaredBinary {
    /// The bare artifact filename — `<name>.exe` on Windows, `<name>`
    /// elsewhere.
    fn artifact_file(&self) -> String {
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
    ///     slot: std::path::PathBuf::from("vibedeps/stack-typescript-ai-native-lang/0.4.0"),
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
    ///     slot: std::path::PathBuf::from("vibedeps/stack-typescript-ai-native-lang/0.4.0"),
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
        let slot = ws.vibedeps_slot(pkg.kind, &pkg.name, &pkg.version);
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
        let slot = ws.vibedeps_slot(pkg.kind, &pkg.name, &pkg.version);
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
                    slot: slot.clone(),
                },
                version: pkg.version.to_string(),
            });
        }
    }
    out.sort_by(|a, b| a.decl.name.cmp(&b.decl.name));
    Ok(out)
}

/// The PROP-020-shaped consent gate for a build (PROP-025 §8):
/// `org.vibevm` is allow-listed; anything else needs explicit consent —
/// there is no prompt at this layer, callers refuse with the recipe.
pub fn consent_to_build(bin: &DeclaredBinary, assume_yes: bool) -> Result<(), BinsError> {
    if bin.group == "org.vibevm" || assume_yes {
        return Ok(());
    }
    Err(BinsError::ConsentRequired {
        name: bin.decl.name.clone(),
        package: bin.package.clone(),
        group: bin.group.clone(),
    })
}

/// Consent-gated `cargo build --release` of one declared binary in its
/// slot; verifies the artifact landed where dispatch will look.
pub fn build_binary(bin: &DeclaredBinary, assume_yes: bool) -> Result<(), BinsError> {
    consent_to_build(bin, assume_yes)?;
    eprintln!(
        "bin build: `{}` ({}) — cargo build --release in {}",
        bin.decl.name,
        bin.package,
        bin.slot.display()
    );
    let status = std::process::Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(bin.slot.join("Cargo.toml"))
        .arg("--bin")
        .arg(&bin.decl.name)
        .status()
        .map_err(|e| BinsError::CargoSpawn {
            name: bin.decl.name.clone(),
            detail: e.to_string(),
        })?;
    if !status.success() {
        return Err(BinsError::BuildFailed {
            name: bin.decl.name.clone(),
        });
    }
    if !bin.release_artifact().exists() {
        return Err(BinsError::ArtifactMissing {
            artifact: bin.release_artifact(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCK: &str = r#"
[meta]
generated_by = "vibe-test"
generated_at = "2026-07-07T00:00:00Z"
schema_version = 5

[[package]]
kind = "stack"
group = "org.vibevm"
name = "typescript-ai-native-lang"
version = "0.4.0"
registry = "vibespecs"
source_url = "file://packages"
source_ref = "v0.4.0"
content_hash = "sha256:deadbeef"
files_written = []
"#;

    fn fixture_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
        )
        .expect("vibe.toml");
        std::fs::write(dir.path().join("vibe.lock"), LOCK).expect("vibe.lock");
        let slot = dir
            .path()
            .join("vibedeps")
            .join("stack-typescript-ai-native-lang")
            .join("0.4.0");
        std::fs::create_dir_all(&slot).expect("slot");
        std::fs::write(
            slot.join("vibe.toml"),
            r#"[package]
name = "typescript-ai-native-lang"
group = "org.vibevm"
kind = "stack"
version = "0.4.0"
authors = ["x"]
license = "EULA"
description = "fixture"
keywords = []

[[binary]]
name = "typescript-ai-native-tcg"
crate = "crates/typescript-ai-native-tcg"
"#,
        )
        .expect("slot manifest");
        dir
    }

    #[test]
    fn collect_walks_lockfile_slots_and_sorts() {
        let dir = fixture_project();
        let bins = collect_binaries(dir.path()).expect("collect");
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].decl.name, "typescript-ai-native-tcg");
        assert_eq!(bins[0].group, "org.vibevm");
        // No build on disk in the fixture slot → dispatch resolves to the
        // release path (the stable fallback).
        assert!(bins[0].artifact().to_string_lossy().contains("release"));
    }

    #[test]
    fn artifact_prefers_debug_over_release() {
        let dir = tempfile::tempdir().expect("tempdir");
        let bin = DeclaredBinary {
            decl: vibe_core::manifest::BinaryDecl {
                name: "typescript-ai-native-tcg".into(),
                crate_dir: "crates/typescript-ai-native-tcg".into(),
                description: None,
            },
            package: "org.vibevm/typescript-ai-native-lang".into(),
            group: "org.vibevm".into(),
            slot: dir.path().to_path_buf(),
        };
        // Nothing built yet → dispatch falls back to the release path.
        assert_eq!(bin.artifact(), bin.release_artifact());
        // A plain `cargo build` (debug) in the slot wins over release.
        let debug = bin.debug_artifact();
        std::fs::create_dir_all(debug.parent().expect("debug parent")).expect("debug dir");
        std::fs::write(&debug, b"stub").expect("debug artifact");
        assert_eq!(bin.artifact(), debug);
        // Debug still wins even once a release build also exists.
        let release = bin.release_artifact();
        std::fs::create_dir_all(release.parent().expect("release parent")).expect("release dir");
        std::fs::write(&release, b"stub").expect("release artifact");
        assert_eq!(bin.artifact(), bin.debug_artifact());
    }

    #[test]
    fn missing_lockfile_is_an_empty_set() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
        )
        .expect("vibe.toml");
        assert!(collect_binaries(dir.path()).expect("collect").is_empty());
    }

    #[test]
    fn unknown_binary_names_the_known_set() {
        let dir = fixture_project();
        let bins = collect_binaries(dir.path()).expect("collect");
        let err = find_binary(&bins, "nope").expect_err("unknown");
        let msg = err.to_string();
        assert!(
            msg.contains("nope") && msg.contains("typescript-ai-native-tcg"),
            "{msg}"
        );
    }

    #[test]
    fn consent_allowlists_org_vibevm_and_refuses_with_recipe() {
        let dir = fixture_project();
        let bins = collect_binaries(dir.path()).expect("collect");
        assert!(consent_to_build(&bins[0], false).is_ok(), "allow-listed");

        let mut foreign = bins[0].clone();
        foreign.group = "com.example".to_string();
        foreign.package = "com.example/thing".to_string();
        let err = consent_to_build(&foreign, false).expect_err("refused");
        assert!(err.to_string().contains("--assume-yes"), "{err}");
        assert!(consent_to_build(&foreign, true).is_ok(), "explicit consent");
    }
}
