//! Provider-root Cargo metadata/build adapter for native cdylibs.

use std::path::Path;

use sha2::{Digest, Sha256};
use vibe_workspace::cargo_build::package_cargo_command;

#[cfg(test)]
use crate::mechanism::cargo::message::MetadataTarget;
use crate::mechanism::cargo::message::{
    COMPILER_ARTIFACT, CargoMessage, CargoMetadata, MetadataPackage,
};
use crate::mechanism::error::preview;

use super::path::{VerifiedFile, cargo_reported_file};
use super::provider::ProviderFacts;
use super::{NativeArtifactError, NativePlatform};

const STDERR_TAIL: usize = 2_000;
const CDYLIB: &str = "cdylib";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeToolchain {
    pub(super) cargo: String,
    pub(super) rustc: String,
    pub(super) host: Option<String>,
    pub(super) digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RootCdylib {
    package_id: String,
    package_name: String,
    target_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BuiltCdylib {
    pub(super) file: VerifiedFile,
    pub(super) fresh: bool,
    pub(super) toolchain: NativeToolchain,
}

pub(super) fn build_cdylib(
    provider: &ProviderFacts,
    manifest: &Path,
    provider_root: &Path,
    platform: NativePlatform,
    offline: bool,
) -> Result<BuiltCdylib, NativeArtifactError> {
    let target_dir = provider_root.join("target");
    let metadata_argv = metadata_argv(manifest, offline);
    let metadata_output = run(provider, provider_root, "cargo", &metadata_argv)?;
    let metadata: CargoMetadata =
        serde_json::from_str(&metadata_output).map_err(|error| NativeArtifactError::CargoJson {
            provider: provider.identity.clone(),
            reason: format!("metadata document: {error}"),
        })?;
    let cdylib = select_root_cdylib(provider, manifest, &metadata)?;
    let toolchain = toolchain(provider, provider_root)?;
    let build_argv = build_argv(manifest, &target_dir, offline);
    let build_output = run(provider, provider_root, "cargo", &build_argv)?;
    let messages = parse_messages(provider, &build_output)?;
    let message = select_compiler_artifact(provider, &cdylib, &messages)?;
    let file = select_filename(provider, platform, &target_dir, &message.filenames)?;
    Ok(BuiltCdylib {
        file,
        fresh: message.fresh.unwrap_or(false),
        toolchain,
    })
}

pub(super) fn toolchain(
    provider: &ProviderFacts,
    workdir: &Path,
) -> Result<NativeToolchain, NativeArtifactError> {
    let cargo = run(provider, workdir, "cargo", &["-Vv".to_owned()])?;
    let rustc = run(provider, workdir, "rustc", &["-V".to_owned()])?;
    let cargo_version = first_line(&cargo);
    let rustc_version = first_line(&rustc);
    let host = cargo
        .lines()
        .find_map(|line| line.trim().strip_prefix("host:"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let mut hash = Sha256::new();
    hash.update(b"cargo\0");
    hash.update(cargo_version.as_bytes());
    hash.update(b"\0rustc\0");
    hash.update(rustc_version.as_bytes());
    hash.update(b"\0host\0");
    hash.update(host.as_deref().unwrap_or("").as_bytes());
    Ok(NativeToolchain {
        cargo: cargo_version,
        rustc: rustc_version,
        host,
        digest: format!("{:x}", hash.finalize()),
    })
}

fn metadata_argv(manifest: &Path, offline: bool) -> Vec<String> {
    let mut argv = vec![
        "metadata".to_owned(),
        "--format-version".to_owned(),
        "1".to_owned(),
        "--no-deps".to_owned(),
        "--manifest-path".to_owned(),
        manifest.display().to_string(),
    ];
    if offline {
        argv.push("--offline".to_owned());
    }
    argv
}

fn build_argv(manifest: &Path, target_dir: &Path, offline: bool) -> Vec<String> {
    let mut argv = vec![
        "build".to_owned(),
        "--release".to_owned(),
        "--lib".to_owned(),
        "--message-format=json-render-diagnostics".to_owned(),
        "--target-dir".to_owned(),
        target_dir.display().to_string(),
        "--manifest-path".to_owned(),
        manifest.display().to_string(),
    ];
    if offline {
        argv.push("--offline".to_owned());
    }
    argv
}

fn select_root_cdylib(
    provider: &ProviderFacts,
    manifest: &Path,
    metadata: &CargoMetadata,
) -> Result<RootCdylib, NativeArtifactError> {
    let canonical_manifest =
        manifest
            .canonicalize()
            .map_err(|error| NativeArtifactError::CrateDirectory {
                provider: provider.identity.clone(),
                crate_dir: preview(&manifest.display().to_string()),
                reason: error.to_string(),
            })?;
    let packages = metadata
        .packages
        .iter()
        .filter(|package| manifest_matches(package, &canonical_manifest))
        .collect::<Vec<_>>();
    let [package] = packages.as_slice() else {
        return Err(NativeArtifactError::RootPackage {
            provider: provider.identity.clone(),
            manifest: preview(&manifest.display().to_string()),
            found: packages.len(),
        });
    };
    let targets = package
        .targets
        .iter()
        .filter(|target| has_cdylib(&target.crate_types))
        .collect::<Vec<_>>();
    let [target] = targets.as_slice() else {
        return Err(NativeArtifactError::CdylibTarget {
            provider: provider.identity.clone(),
            package: preview(&package.name),
            found: targets.len(),
        });
    };
    Ok(RootCdylib {
        package_id: package.id.clone(),
        package_name: package.name.clone(),
        target_name: target.name.clone(),
    })
}

fn manifest_matches(package: &MetadataPackage, expected: &Path) -> bool {
    if package.manifest_path.is_empty() {
        return false;
    }
    Path::new(&package.manifest_path)
        .canonicalize()
        .is_ok_and(|candidate| candidate == expected)
}

fn has_cdylib(crate_types: &[String]) -> bool {
    crate_types.iter().any(|kind| kind == CDYLIB)
}

fn parse_messages(
    provider: &ProviderFacts,
    output: &str,
) -> Result<Vec<CargoMessage>, NativeArtifactError> {
    let mut messages = Vec::new();
    for (index, raw) in output.lines().enumerate() {
        let line = raw.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        let message = serde_json::from_str::<CargoMessage>(line).map_err(|error| {
            NativeArtifactError::CargoJson {
                provider: provider.identity.clone(),
                reason: format!(
                    "build stream line {}: {error}; line was `{}`",
                    index + 1,
                    preview(line)
                ),
            }
        })?;
        messages.push(message);
    }
    Ok(messages)
}

fn select_compiler_artifact<'a>(
    provider: &ProviderFacts,
    cdylib: &RootCdylib,
    messages: &'a [CargoMessage],
) -> Result<&'a CargoMessage, NativeArtifactError> {
    let matches = messages
        .iter()
        .filter(|message| {
            message.reason == COMPILER_ARTIFACT
                && message.package_id.as_deref() == Some(cdylib.package_id.as_str())
                && message.target.as_ref().is_some_and(|target| {
                    target.name == cdylib.target_name && has_cdylib(&target.crate_types)
                })
        })
        .collect::<Vec<_>>();
    let [message] = matches.as_slice() else {
        return Err(NativeArtifactError::CompilerArtifact {
            provider: provider.identity.clone(),
            target: format!("{}#{}", cdylib.package_name, cdylib.target_name),
            found: matches.len(),
        });
    };
    Ok(message)
}

fn select_filename(
    provider: &ProviderFacts,
    platform: NativePlatform,
    target_dir: &Path,
    filenames: &[String],
) -> Result<VerifiedFile, NativeArtifactError> {
    let mut matches = Vec::new();
    for filename in filenames
        .iter()
        .filter(|filename| filename.ends_with(platform.suffix()))
    {
        matches.push(cargo_reported_file(
            provider,
            Path::new(filename),
            target_dir,
        )?);
    }
    let [file] = matches.as_slice() else {
        return Err(NativeArtifactError::CdylibFilename {
            provider: provider.identity.clone(),
            suffix: platform.suffix().to_owned(),
            target_root: preview(&target_dir.display().to_string()),
            found: matches.len(),
        });
    };
    Ok(file.clone())
}

fn run(
    provider: &ProviderFacts,
    workdir: &Path,
    program: &str,
    argv: &[String],
) -> Result<String, NativeArtifactError> {
    let output = package_cargo_command(program)
        .args(argv)
        .current_dir(workdir)
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .map_err(|error| NativeArtifactError::Spawn {
            provider: provider.identity.clone(),
            program: format!("{program} {}", argv.join(" ")),
            reason: error.to_string(),
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr
            .chars()
            .rev()
            .take(STDERR_TAIL)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<String>();
        return Err(NativeArtifactError::NonZero {
            provider: provider.identity.clone(),
            program: format!("{program} {}", argv.join(" ")),
            status: output.status.to_string(),
            detail: if detail.trim().is_empty() {
                "no stderr output".to_owned()
            } else {
                detail
            },
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn first_line(value: &str) -> String {
    value.lines().next().unwrap_or("").trim().to_owned()
}

#[cfg(test)]
pub(super) fn select_filename_for_test(
    provider: &ProviderFacts,
    platform: NativePlatform,
    target_dir: &Path,
    filenames: &[String],
) -> Result<VerifiedFile, NativeArtifactError> {
    select_filename(provider, platform, target_dir, filenames)
}

#[cfg(test)]
pub(super) fn select_metadata_for_test(
    provider: &ProviderFacts,
    manifest: &Path,
    metadata: &CargoMetadata,
) -> Result<(String, String), NativeArtifactError> {
    select_root_cdylib(provider, manifest, metadata)
        .map(|target| (target.package_id, target.target_name))
}

#[cfg(test)]
pub(super) fn metadata_target_for_test(name: &str, crate_types: &[&str]) -> MetadataTarget {
    MetadataTarget {
        name: name.to_owned(),
        kind: vec!["lib".to_owned()],
        crate_types: crate_types
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
    }
}
