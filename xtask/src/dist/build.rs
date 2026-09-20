//! Native bundle construction from a clean committed source snapshot.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use semver::Version;
use serde::Deserialize;
use vibe_publish::{
    BundleDistributionManifest, DISTRIBUTION_MANIFEST_FILENAME, DISTRIBUTION_MANIFEST_MAX_BYTES,
    DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, DistributionAsset, DistributionComponent,
    DistributionComponentName, DistributionSourceArchive, PlatformDistributionFragment,
    SUPPORTED_DISTRIBUTION_TARGETS, sha256_digest,
};

use super::archive::{ArchiveEntry, read_zip_bounded, write_zip};
use super::scrub_release_credentials;
use super::snapshot::{self, GitIdentity, SourceSnapshot};

mod output;
use output::{run_visible, write_output};

const COMPONENT_VIBE: &str = "vibe";
const COMPONENT_INDEX: &str = "vibe-index";

#[derive(Debug, Clone)]
pub(crate) struct BuildOptions {
    pub(crate) target: Option<String>,
    pub(crate) out_dir: PathBuf,
    pub(crate) checks: bool,
    pub(crate) tests: bool,
    pub(crate) self_check: bool,
}

pub(crate) fn run(repo_root: &Path, options: &BuildOptions) -> Result<()> {
    let target = resolve_target(options.target.as_deref())?;
    let identity = snapshot::committed_identity(repo_root, true)?;
    let version = workspace_version_at_commit(repo_root, &identity)?;
    validate_expected_version(&version)?;
    let out_dir = absolute_out_dir(repo_root, &options.out_dir);
    let cargo_target_root = out_dir.join(".cargo-target");
    let release_target_dir = cargo_target_root.join("release").join(&target);
    fs::create_dir_all(&release_target_dir).with_context(|| {
        format!(
            "creating persistent release Cargo target `{}`",
            release_target_dir.display()
        )
    })?;

    if options.checks || options.tests || options.self_check {
        let gate_target_dir = cargo_target_root.join("gates").join(&target);
        fs::create_dir_all(&gate_target_dir).with_context(|| {
            format!(
                "creating isolated gate Cargo target `{}`",
                gate_target_dir.display()
            )
        })?;
        let gate_snapshot = snapshot::materialise_pinned(repo_root, &identity)?;
        if options.checks {
            run_cargo_gate(&gate_snapshot, &gate_target_dir, &target, "check")?;
        }
        if options.tests {
            run_cargo_gate(&gate_snapshot, &gate_target_dir, &target, "test")?;
        }
        if options.self_check {
            run_self_check(&gate_snapshot, &gate_target_dir)?;
        }
    }

    // Optional gates may legitimately create ignored files or otherwise alter
    // their checkout. Release compilation always starts from a fresh expansion
    // of the already pinned commit and never reuses that disposable tree.
    let snapshot = snapshot::materialise_pinned(repo_root, &identity)?;
    let binaries = build_binaries(&snapshot, &release_target_dir, &target)?;
    let (asset_bytes, bootstrap_bytes, fragment) =
        assemble_bundle(&snapshot, &version, &target, &binaries)?;
    let asset_name = bundle_asset_name(&version, &target);
    let bootstrap_name = bootstrap_asset_name(&target);
    let fragment_name = fragment_asset_name(&version, &target);
    let asset_path = out_dir.join(asset_name);
    let bootstrap_path = out_dir.join(bootstrap_name);
    let fragment_path = out_dir.join(fragment_name);
    write_output(&asset_path, &asset_bytes)?;
    write_output(&bootstrap_path, &bootstrap_bytes)?;
    write_output(&fragment_path, &fragment.to_json_bytes()?)?;
    println!("dist build: {}", asset_path.display());
    println!("dist build: {}", bootstrap_path.display());
    println!("dist build: {}", fragment_path.display());
    Ok(())
}

pub(crate) fn workspace_version_at_commit(
    repo_root: &Path,
    identity: &GitIdentity,
) -> Result<Version> {
    let bytes = snapshot::read_commit_file(repo_root, &identity.commit, "Cargo.toml")?;
    let text = String::from_utf8(bytes).context("committed Cargo.toml is not UTF-8")?;
    parse_workspace_version(&text).context("parsing committed workspace Cargo.toml")
}

fn parse_workspace_version(text: &str) -> Result<Version> {
    let document: toml::Value = toml::from_str(text).context("invalid TOML")?;
    let raw = document
        .get("workspace")
        .and_then(|value| value.get("package"))
        .and_then(|value| value.get("version"))
        .and_then(toml::Value::as_str)
        .context("Cargo.toml has no string [workspace.package].version")?;
    Version::parse(raw).with_context(|| format!("workspace version `{raw}` is not SemVer"))
}

pub(crate) fn validate_expected_version(actual: &Version) -> Result<()> {
    let Ok(raw) = std::env::var("VIBEVM_DIST_VERSION") else {
        return Ok(());
    };
    let expected = Version::parse(raw.trim())
        .with_context(|| format!("VIBEVM_DIST_VERSION `{raw}` is not a semantic version"))?;
    if &expected != actual {
        bail!(
            "VIBEVM_DIST_VERSION requires `{expected}`, but this workspace/fragment is \
             `{actual}`; refusing before any release upload"
        );
    }
    Ok(())
}

pub(crate) fn bundle_asset_name(version: &Version, target: &str) -> String {
    format!("vibevm-{version}-{target}.zip")
}

pub(crate) fn fragment_asset_name(version: &Version, target: &str) -> String {
    format!("vibevm-{version}-{target}.fragment.json")
}

pub(crate) fn bootstrap_asset_name(target: &str) -> String {
    let suffix = if target == "x86_64-pc-windows-msvc" {
        ".exe"
    } else {
        ""
    };
    format!("vibe-bootstrap-{target}{suffix}")
}

pub(crate) const fn aggregate_asset_name() -> &'static str {
    vibe_publish::release_manifest::DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME
}

pub(crate) fn absolute_out_dir(repo_root: &Path, out_dir: &Path) -> PathBuf {
    if out_dir.is_absolute() {
        out_dir.to_path_buf()
    } else {
        repo_root.join(out_dir)
    }
}

fn resolve_target(requested: Option<&str>) -> Result<String> {
    let native = native_target()?;
    let selected = requested.unwrap_or(native);
    if !SUPPORTED_DISTRIBUTION_TARGETS.contains(&selected) {
        bail!(
            "unsupported distribution target `{selected}`; expected one of: {}",
            SUPPORTED_DISTRIBUTION_TARGETS.join(", ")
        );
    }
    let compatible = match (std::env::consts::OS, std::env::consts::ARCH, selected) {
        ("windows", "x86_64", "x86_64-pc-windows-msvc")
        | ("linux", "x86_64", "x86_64-unknown-linux-musl")
        | ("linux", "x86_64", "x86_64-unknown-linux-gnu")
        | ("macos", "x86_64", "x86_64-apple-darwin")
        | ("macos", "aarch64", "aarch64-apple-darwin") => true,
        _ => false,
    };
    if !compatible {
        bail!(
            "distribution target `{selected}` does not match this native host (`{native}`); run \
             the wrapper on the target's own operating system and architecture"
        );
    }
    Ok(selected.to_string())
}

fn native_target() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        ("linux", "x86_64") if cfg!(target_env = "musl") => Ok("x86_64-unknown-linux-musl"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        (os, arch) => bail!(
            "unsupported distribution host OS/architecture `{os}/{arch}`; supported native \
             targets are: {}",
            SUPPORTED_DISTRIBUTION_TARGETS.join(", ")
        ),
    }
}

fn run_cargo_gate(
    snapshot: &SourceSnapshot,
    target_dir: &Path,
    target: &str,
    verb: &str,
) -> Result<()> {
    let mut command = cargo_command(snapshot, target_dir);
    command
        .arg(verb)
        .args(["--locked", "--workspace", "--target", target]);
    if verb == "check" {
        command.arg("--all-targets");
    }
    run_visible(&mut command, &format!("cargo {verb}"))
}

fn run_self_check(snapshot: &SourceSnapshot, target_dir: &Path) -> Result<()> {
    initialise_scratch_git(&snapshot.root)?;
    let mut command = Command::new("bash");
    command
        .arg("tools/self-check.sh")
        .current_dir(&snapshot.root)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("SOURCE_DATE_EPOCH", &snapshot.identity.source_date_epoch);
    scrub_release_credentials(&mut command);
    run_visible(&mut command, "tools/self-check.sh")?;
    let mut status = Command::new("git");
    status
        .args(["status", "--porcelain=v1", "--untracked-files=normal"])
        .current_dir(&snapshot.root);
    scrub_release_credentials(&mut status);
    let output = status
        .output()
        .context("checking the post-self-check scratch tree")?;
    if !output.status.success() {
        bail!(
            "post-self-check `git status` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    if !output.stdout.is_empty() {
        bail!(
            "--self-check changed the clean source snapshot; refusing to build bytes outside \
             the pinned `git archive`:\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    Ok(())
}

fn initialise_scratch_git(root: &Path) -> Result<()> {
    for args in [
        vec!["init", "--quiet", "--initial-branch=main"],
        vec!["config", "core.autocrlf", "false"],
        vec!["config", "user.name", "vibevm distribution check"],
        vec!["config", "user.email", "distribution@vibevm.invalid"],
        vec!["add", "-A"],
        vec!["commit", "--quiet", "-m", "distribution source snapshot"],
    ] {
        let mut command = Command::new("git");
        command.args(&args).current_dir(root);
        scrub_release_credentials(&mut command);
        let output = command
            .output()
            .with_context(|| format!("spawning scratch `git {}`", args.join(" ")))?;
        if !output.status.success() {
            bail!(
                "scratch `git {}` failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
    }
    Ok(())
}

fn build_binaries(
    snapshot: &SourceSnapshot,
    target_dir: &Path,
    target: &str,
) -> Result<BTreeMap<String, PathBuf>> {
    let mut command = cargo_command(snapshot, target_dir);
    command.args([
        "build",
        "--locked",
        "--release",
        "--target",
        target,
        "--message-format=json-render-diagnostics",
        "-p",
        "vibe-cli",
        "-p",
        "vibe-index",
        "--bins",
    ]);
    let output = command
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .context("spawning release Cargo build")?;
    if !output.status.success() {
        bail!(
            "release Cargo build failed (exit {:?}); compiler diagnostics were emitted above",
            output.status.code()
        );
    }
    select_binaries(&output.stdout)
}

fn cargo_command(snapshot: &SourceSnapshot, target_dir: &Path) -> Command {
    let mut rustflags = vec![
        "--remap-path-prefix".to_string(),
        format!("{}=/vibevm", snapshot.root.display()),
    ];
    if cfg!(windows) {
        rustflags.extend(["-C".to_string(), "target-feature=+crt-static".to_string()]);
    }
    let encoded = rustflags.join("\u{1f}");
    let mut command = Command::new("cargo");
    command
        .current_dir(&snapshot.root)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("CARGO_INCREMENTAL", "0")
        .env("SOURCE_DATE_EPOCH", &snapshot.identity.source_date_epoch)
        .env("CARGO_ENCODED_RUSTFLAGS", encoded)
        .env_remove("RUSTFLAGS");
    scrub_release_credentials(&mut command);
    command
}

#[derive(Deserialize)]
struct CargoMessage {
    reason: String,
    #[serde(default)]
    target: Option<CargoTarget>,
    #[serde(default)]
    executable: Option<PathBuf>,
}

#[derive(Deserialize)]
struct CargoTarget {
    name: String,
    kind: Vec<String>,
}

fn select_binaries(stdout: &[u8]) -> Result<BTreeMap<String, PathBuf>> {
    let mut selected = BTreeMap::new();
    for line in stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let Ok(message) = serde_json::from_slice::<CargoMessage>(line) else {
            continue;
        };
        if message.reason != "compiler-artifact" {
            continue;
        }
        let Some(target) = message.target else {
            continue;
        };
        let Some(executable) = message.executable else {
            continue;
        };
        if !target.kind.iter().any(|kind| kind == "bin")
            || !matches!(target.name.as_str(), COMPONENT_VIBE | COMPONENT_INDEX)
        {
            continue;
        }
        if let Some(previous) = selected.insert(target.name.clone(), executable.clone())
            && previous != executable
        {
            bail!(
                "Cargo reported two executable paths for `{}`: `{}` and `{}`",
                target.name,
                previous.display(),
                executable.display()
            );
        }
    }
    for required in [COMPONENT_VIBE, COMPONENT_INDEX] {
        let path = selected
            .get(required)
            .with_context(|| format!("Cargo emitted no compiler-artifact for `{required}`"))?;
        if !path.is_file() {
            bail!(
                "Cargo reported `{required}` at `{}`, but no file exists there",
                path.display()
            );
        }
    }
    Ok(selected)
}

fn assemble_bundle(
    snapshot: &SourceSnapshot,
    version: &Version,
    target: &str,
    binaries: &BTreeMap<String, PathBuf>,
) -> Result<(Vec<u8>, Vec<u8>, PlatformDistributionFragment)> {
    let windows = target == "x86_64-pc-windows-msvc";
    let names = if windows {
        [
            (COMPONENT_VIBE, "vibe.exe"),
            (COMPONENT_INDEX, "vibe-index.exe"),
        ]
    } else {
        [(COMPONENT_VIBE, "vibe"), (COMPONENT_INDEX, "vibe-index")]
    };
    let mut entries = Vec::new();
    let mut components = Vec::new();
    let mut bootstrap_bytes = None;
    for (component, bundle_name) in names {
        let source = binaries
            .get(component)
            .with_context(|| format!("built component `{component}` is absent"))?;
        verify_version(source, component, version)?;
        let bytes = fs::read(source)
            .with_context(|| format!("reading built component `{}`", source.display()))?;
        let name = if component == COMPONENT_VIBE {
            DistributionComponentName::Vibe
        } else {
            DistributionComponentName::VibeIndex
        };
        components.push(DistributionComponent {
            name,
            path: bundle_name.to_string(),
            size: bytes.len() as u64,
            digest: sha256_digest(&bytes),
        });
        if component == COMPONENT_VIBE {
            bootstrap_bytes = Some(bytes.clone());
        }
        entries.push(ArchiveEntry {
            name: bundle_name.to_string(),
            bytes,
            mode: 0o755,
        });
    }
    let source_archive = DistributionSourceArchive {
        path: DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.to_string(),
        size: snapshot.source_zip.len() as u64,
        digest: sha256_digest(&snapshot.source_zip),
        tree_oid: snapshot.identity.tree.clone(),
    };
    entries.push(ArchiveEntry {
        name: DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.to_string(),
        bytes: snapshot.source_zip.clone(),
        mode: 0o644,
    });
    for path in ["LICENSE.md", "README.md"] {
        entries.push(ArchiveEntry {
            name: path.to_string(),
            bytes: fs::read(snapshot.root.join(path))
                .with_context(|| format!("reading canonical bundle asset `{path}`"))?,
            mode: 0o644,
        });
    }
    let manifest = BundleDistributionManifest::new(
        version.clone(),
        snapshot.identity.commit.clone(),
        target.to_string(),
        components,
        source_archive,
    )?;
    entries.push(ArchiveEntry {
        name: DISTRIBUTION_MANIFEST_FILENAME.to_string(),
        bytes: manifest.to_json_bytes()?,
        mode: 0o644,
    });
    let asset_bytes = write_zip(&entries).context("writing deterministic distribution ZIP")?;
    verify_bundle_bytes(&asset_bytes, &manifest)?;
    let asset = DistributionAsset {
        name: bundle_asset_name(version, target),
        size: asset_bytes.len() as u64,
        digest: sha256_digest(&asset_bytes),
    };
    let bootstrap_bytes = bootstrap_bytes.context("built bundle has no vibe bootstrap bytes")?;
    let bootstrap = DistributionAsset {
        name: bootstrap_asset_name(target),
        size: bootstrap_bytes.len() as u64,
        digest: sha256_digest(&bootstrap_bytes),
    };
    let fragment = PlatformDistributionFragment::new(asset, bootstrap, manifest)?;
    Ok((asset_bytes, bootstrap_bytes, fragment))
}

pub(crate) fn verify_bundle_bytes(
    bytes: &[u8],
    expected: &BundleDistributionManifest,
) -> Result<()> {
    verified_bundle_entries(bytes, expected).map(drop)
}

pub(crate) fn verified_bundle_entries(
    bytes: &[u8],
    expected: &BundleDistributionManifest,
) -> Result<Vec<ArchiveEntry>> {
    let mut limits = expected
        .components
        .iter()
        .map(|component| (component.path.clone(), component.size))
        .collect::<BTreeMap<_, _>>();
    limits.insert(
        expected.source_archive.path.clone(),
        expected.source_archive.size,
    );
    for path in [DISTRIBUTION_MANIFEST_FILENAME, "LICENSE.md", "README.md"] {
        limits.insert(path.to_string(), DISTRIBUTION_MANIFEST_MAX_BYTES);
    }
    let entries =
        read_zip_bounded(bytes, &limits).context("reading bounded completed distribution ZIP")?;
    let by_name = entries
        .iter()
        .map(|entry| (entry.name.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let manifest_entry = by_name
        .get(DISTRIBUTION_MANIFEST_FILENAME)
        .context("distribution ZIP lacks DISTRIBUTION.json")?;
    let embedded = BundleDistributionManifest::from_json_slice(&manifest_entry.bytes)?;
    if &embedded != expected {
        bail!("embedded DISTRIBUTION.json differs from the platform fragment's bundle manifest");
    }
    for component in &expected.components {
        let entry = by_name
            .get(component.path.as_str())
            .with_context(|| format!("distribution ZIP lacks `{}`", component.path))?;
        if entry.bytes.len() as u64 != component.size
            || sha256_digest(&entry.bytes) != component.digest
        {
            bail!(
                "distribution component `{}` failed size/SHA-256 verification",
                component.path
            );
        }
    }
    let source = by_name
        .get(expected.source_archive.path.as_str())
        .context("distribution ZIP lacks vibevm-source.zip")?;
    if source.bytes.len() as u64 != expected.source_archive.size
        || sha256_digest(&source.bytes) != expected.source_archive.digest
    {
        bail!("vibevm-source.zip failed size/SHA-256 verification");
    }
    let expected_names = expected
        .components
        .iter()
        .map(|component| component.path.as_str())
        .chain([
            expected.source_archive.path.as_str(),
            DISTRIBUTION_MANIFEST_FILENAME,
            "LICENSE.md",
            "README.md",
        ])
        .collect::<BTreeSet<_>>();
    let actual_names = by_name.keys().copied().collect::<BTreeSet<_>>();
    if actual_names != expected_names {
        bail!(
            "distribution ZIP file set differs from the closed bundle contract (expected \
             {expected_names:?}, got {actual_names:?})"
        );
    }
    Ok(entries)
}

fn verify_version(path: &Path, component: &str, version: &Version) -> Result<()> {
    let mut command = Command::new(path);
    command.arg("--version");
    scrub_release_credentials(&mut command);
    let output = command
        .output()
        .with_context(|| format!("running `{component} --version` at `{}`", path.display()))?;
    if !output.status.success() {
        bail!(
            "`{component} --version` failed with exit {:?}",
            output.status.code()
        );
    }
    let actual = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let expected = format!("{component} {version}");
    if actual != expected {
        bail!("`{component} --version` returned `{actual}`; expected `{expected}`");
    }
    Ok(())
}

#[cfg(test)]
#[path = "build/tests.rs"]
mod tests;
