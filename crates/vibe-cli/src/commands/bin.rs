//! `vibe bin` — build and dispatch the tools installed packages declare
//! via `[[binary]]` (PROP-025 §§3–4). Artifacts are slot-resident
//! (`vibedeps/<slot>/target/release/…`), so a slot refresh invalidates
//! them for free and content hashes never move (build output is outside
//! the shippable tree, PROP-024 §2.2). Dispatch is the rustup model:
//! `exec` resolves through the CURRENT project's lockfile, so two
//! projects pinning different stack versions run different binaries.
//!
//! Building executes the package's build scripts and proc-macros —
//! arbitrary code — so a build is consent-gated like install hooks
//! (PROP-020): `org.vibevm` is allow-listed; any other group requires
//! `--assume-yes` (v1 has no interactive prompt here — refuse with the
//! recipe instead, the conservative reading).

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-025#dispatch");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{BinaryDecl, Lockfile, Manifest};
use vibe_workspace::Workspace;

/// One declared binary, resolved against its installed slot.
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
    /// The slot-resident release artifact (PROP-025 §3).
    pub fn artifact(&self) -> PathBuf {
        let file = if cfg!(windows) {
            format!("{}.exe", self.decl.name)
        } else {
            self.decl.name.clone()
        };
        self.slot.join("target").join("release").join(file)
    }
}

/// Every `[[binary]]` reachable from the project's lockfile slots.
pub fn collect_binaries(project_root: &Path) -> Result<Vec<DeclaredBinary>> {
    let ws = Workspace::discover(project_root)
        .with_context(|| format!("loading workspace at `{}`", project_root.display()))?;
    let mut out = Vec::new();
    let lock_path = ws.lockfile_path();
    if !lock_path.exists() {
        return Ok(out);
    }
    let lockfile = Lockfile::read(&lock_path)
        .with_context(|| format!("reading lockfile `{}`", lock_path.display()))?;
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

fn find<'a>(bins: &'a [DeclaredBinary], name: &str) -> Result<&'a DeclaredBinary> {
    match bins.iter().find(|b| b.decl.name == name) {
        Some(found) => Ok(found),
        None => {
            let known: Vec<&str> = bins.iter().map(|b| b.decl.name.as_str()).collect();
            bail!(
                "no installed package declares a binary `{name}` \
                 (declared: {known:?}); `vibe bin list` shows the full table"
            )
        }
    }
}

/// The PROP-020-shaped consent gate for a build (PROP-025 §8).
fn consent_to_build(bin: &DeclaredBinary, assume_yes: bool) -> Result<()> {
    if bin.group == "org.vibevm" || assume_yes {
        return Ok(());
    }
    bail!(
        "building `{}` runs `{}`'s build scripts and proc-macros (arbitrary code). \
         The group `{}` is not allow-listed; re-run with --assume-yes to consent \
         (PROP-025 s8, the PROP-020 posture)",
        bin.decl.name,
        bin.package,
        bin.group
    )
}

fn build_one(bin: &DeclaredBinary, assume_yes: bool) -> Result<()> {
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
        .context("spawning cargo (a Rust toolchain is required to build package binaries)")?;
    if !status.success() {
        bail!(
            "bin build: cargo failed for `{}` — the slot builds standalone \
             (PROP-024 s2.4), so this is a real build error, not a topology one",
            bin.decl.name
        );
    }
    if !bin.artifact().exists() {
        bail!(
            "bin build: cargo succeeded but `{}` is missing — the [[binary]] \
             declaration's name must equal the crate's bin target (PROP-025 s2)",
            bin.artifact().display()
        );
    }
    Ok(())
}

/// `vibe bin list`.
pub fn run_list(project_root: &Path) -> Result<()> {
    let bins = collect_binaries(project_root)?;
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
pub fn run_build(project_root: &Path, names: &[String], assume_yes: bool) -> Result<()> {
    let bins = collect_binaries(project_root)?;
    if bins.is_empty() {
        bail!("bin build: no installed package declares a [[binary]]");
    }
    let selected: Vec<&DeclaredBinary> = if names.is_empty() {
        bins.iter().collect()
    } else {
        let mut chosen = Vec::new();
        for name in names {
            chosen.push(find(&bins, name)?);
        }
        chosen
    };
    for bin in selected {
        build_one(bin, assume_yes)?;
    }
    Ok(())
}

/// `vibe bin path <name>` — the artifact path; non-zero when unbuilt.
pub fn run_path(project_root: &Path, name: &str) -> Result<()> {
    let bins = collect_binaries(project_root)?;
    let bin = find(&bins, name)?;
    let artifact = bin.artifact();
    if !artifact.exists() {
        bail!(
            "`{name}` is declared by {} but not built — run `vibe bin build {name}`",
            bin.package
        );
    }
    println!("{}", artifact.display());
    Ok(())
}

/// `vibe bin exec <name> -- <args…>` — build-if-missing, then exec with
/// the exit code passed through.
pub fn run_exec(project_root: &Path, name: &str, args: &[String], assume_yes: bool) -> Result<i32> {
    let bins = collect_binaries(project_root)?;
    let bin = find(&bins, name)?;
    if !bin.artifact().exists() {
        build_one(bin, assume_yes)?;
    }
    let status = std::process::Command::new(bin.artifact())
        .args(args)
        .status()
        .with_context(|| format!("spawning {}", bin.artifact().display()))?;
    Ok(status.code().unwrap_or(1))
}
