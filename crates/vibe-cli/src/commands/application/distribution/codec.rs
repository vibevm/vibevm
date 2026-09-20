//! Generated-wire conversion, archive parsing, and semantic validation helpers.

use super::*;
use crate::commands::application::model::PackageIdentity;

pub(super) fn validate_index(
    locator: &ApplicationDistributionDecl,
    index: &DistributionIndex,
) -> Result<()> {
    if index.protocol != INDEX_PROTOCOL || index.release_tag != locator.release_tag {
        bail!("application distribution index protocol or tag is invalid");
    }
    validate_application_identity(&index.application)?;
    let mut targets = BTreeSet::new();
    for target in &index.distributions {
        validate_target(target)?;
        if !targets.insert((
            target.os.as_str(),
            target.arch.as_str(),
            target.libc.as_deref(),
        )) {
            bail!("application distribution index repeats a platform target");
        }
    }
    Ok(())
}

pub(super) fn validate_target(target: &DistributionTarget) -> Result<()> {
    if target.format != "zip"
        || target.size == 0
        || target.size > DISTRIBUTION_BUNDLE_MAX_BYTES
        || !https_url(&target.url)
        || !hex(&target.sha256, 64)
        || !git_oid(&target.source_commit)
        || !tree_hash(&target.source_tree)
        || !portable_token(&target.os)
        || !portable_token(&target.arch)
        || target
            .libc
            .as_deref()
            .is_some_and(|value| !matches!(value, "gnu" | "musl"))
        || (target.os == "linux") != target.libc.is_some()
    {
        bail!("application distribution target is malformed");
    }
    Ok(())
}

pub(super) fn validate_bundle(
    manifest: &BundleManifest,
    target: &DistributionTarget,
    expected: &ApplicationIdentity,
) -> Result<()> {
    let selected = &manifest.application;
    if manifest.protocol != BUNDLE_PROTOCOL
        || selected != expected
        || manifest.os != target.os
        || manifest.arch != target.arch
        || manifest.libc != target.libc
        || manifest.source_commit != target.source_commit
        || manifest.source_tree != target.source_tree
        || manifest.management.runtime != "builtin"
        || portable_path(&manifest.management.entry).is_err()
        || !manifest
            .files
            .iter()
            .any(|file| file.path == manifest.management.entry)
        || manifest.files.is_empty()
        || manifest.launchers.is_empty()
    {
        bail!("application distribution manifest differs from its index or source declaration");
    }
    let mut paths = BTreeSet::new();
    let mut expanded = 0u64;
    for file in &manifest.files {
        portable_path(&file.path)?;
        expanded = expanded
            .checked_add(file.size)
            .ok_or_else(|| anyhow::anyhow!("distribution expanded size overflows"))?;
        if !hex(&file.sha256, 64) || !paths.insert(file.path.to_ascii_lowercase()) {
            bail!("application distribution has an invalid or duplicate file");
        }
    }
    if expanded > DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES {
        bail!("application distribution expanded payload exceeds its limit");
    }
    let commands: BTreeSet<_> = selected.commands.iter().map(String::as_str).collect();
    let mut covered_commands = BTreeSet::new();
    let mut destinations = BTreeSet::new();
    for launcher in &manifest.launchers {
        portable_path(&launcher.path)?;
        portable_path(&launcher.destination)?;
        if launcher.destination.contains('/')
            || !commands.contains(launcher.command.as_str())
            || !launcher_destination_matches(&launcher.destination, &launcher.command)
            || !manifest.files.iter().any(|f| f.path == launcher.path)
            || !destinations.insert(launcher.destination.to_ascii_lowercase())
        {
            bail!("application distribution launcher is invalid or ambiguous");
        }
        covered_commands.insert(launcher.command.as_str());
    }
    if covered_commands != commands {
        bail!("application distribution does not cover every declared command");
    }
    Ok(())
}

fn launcher_destination_matches(destination: &str, command: &str) -> bool {
    destination == command
        || [".cmd", ".ps1", ".sh"]
            .iter()
            .any(|suffix| destination == format!("{command}{suffix}"))
}

fn validate_application_identity(value: &ApplicationIdentity) -> Result<()> {
    validate_identity(
        &value.id,
        (
            &value.package.group,
            &value.package.name,
            &value.package.version,
        ),
        (
            &value.installer_package.group,
            &value.installer_package.name,
            &value.installer_package.version,
        ),
        &value.commands,
    )
}

fn validate_identity(
    id: &str,
    package: (&str, &str, &str),
    installer: (&str, &str, &str),
    commands: &[String],
) -> Result<()> {
    if !portable_token(id) || commands.is_empty() {
        bail!("application distribution identity or commands are invalid");
    }
    for (group, name, version) in [package, installer] {
        vibe_core::Group::parse(group)?;
        vibe_core::PackageName::parse(name)?;
        semver::Version::parse(version)?;
    }
    let mut seen = BTreeSet::new();
    if commands
        .iter()
        .any(|command| !portable_token(command) || !seen.insert(command))
    {
        bail!("application distribution commands are invalid or duplicate");
    }
    Ok(())
}

pub(super) fn distribution_index_from_wire(
    value: index_wire::DistributionIndex,
) -> DistributionIndex {
    DistributionIndex {
        protocol: value.protocol,
        application: index_identity_from_wire(value.application),
        release_tag: value.release_tag,
        distributions: value
            .distributions
            .into_iter()
            .map(|target| DistributionTarget {
                os: target.os,
                arch: target.arch,
                libc: target.libc,
                format: target.format,
                url: target.url,
                sha256: target.sha256,
                size: target.size,
                source_commit: target.source_commit,
                source_tree: target.source_tree,
            })
            .collect(),
    }
}

fn index_identity_from_wire(value: index_wire::ApplicationIdentity) -> ApplicationIdentity {
    ApplicationIdentity {
        id: value.id,
        package: PackageIdentity {
            group: value.package.group,
            name: value.package.name,
            version: value.package.version,
        },
        installer_package: PackageIdentity {
            group: value.installer_package.group,
            name: value.installer_package.name,
            version: value.installer_package.version,
        },
        commands: value.commands,
    }
}

pub(super) fn bundle_manifest_from_wire(value: bundle_wire::BundleManifest) -> BundleManifest {
    BundleManifest {
        protocol: value.protocol,
        application: ApplicationIdentity {
            id: value.application.id,
            package: PackageIdentity {
                group: value.application.package.group,
                name: value.application.package.name,
                version: value.application.package.version,
            },
            installer_package: PackageIdentity {
                group: value.application.installer_package.group,
                name: value.application.installer_package.name,
                version: value.application.installer_package.version,
            },
            commands: value.application.commands,
        },
        os: value.os,
        arch: value.arch,
        libc: value.libc,
        source_commit: value.source_commit,
        source_tree: value.source_tree,
        management: BundleManagement {
            runtime: value.management.runtime,
            entry: value.management.entry,
        },
        launchers: value
            .launchers
            .into_iter()
            .map(|launcher| BundleLauncher {
                command: launcher.command,
                path: launcher.path,
                destination: launcher.destination,
            })
            .collect(),
        files: value
            .files
            .into_iter()
            .map(|file| BundleFile {
                path: file.path,
                sha256: file.sha256,
                size: file.size,
            })
            .collect(),
    }
}

#[cfg(test)]
pub(super) fn bundle_manifest_to_wire(value: &BundleManifest) -> bundle_wire::BundleManifest {
    bundle_wire::BundleManifest {
        protocol: value.protocol.clone(),
        application: bundle_wire::ApplicationIdentity {
            id: value.application.id.clone(),
            package: bundle_wire::PackageIdentity {
                group: value.application.package.group.clone(),
                name: value.application.package.name.clone(),
                version: value.application.package.version.clone(),
            },
            installer_package: bundle_wire::PackageIdentity {
                group: value.application.installer_package.group.clone(),
                name: value.application.installer_package.name.clone(),
                version: value.application.installer_package.version.clone(),
            },
            commands: value.application.commands.clone(),
        },
        os: value.os.clone(),
        arch: value.arch.clone(),
        libc: value.libc.clone(),
        source_commit: value.source_commit.clone(),
        source_tree: value.source_tree.clone(),
        management: bundle_wire::BundleManagement {
            runtime: value.management.runtime.clone(),
            entry: value.management.entry.clone(),
        },
        launchers: value
            .launchers
            .iter()
            .map(|launcher| bundle_wire::BundleLauncher {
                command: launcher.command.clone(),
                path: launcher.path.clone(),
                destination: launcher.destination.clone(),
            })
            .collect(),
        files: value
            .files
            .iter()
            .map(|file| bundle_wire::BundleFile {
                path: file.path.clone(),
                sha256: file.sha256.clone(),
                size: file.size,
            })
            .collect(),
    }
}

pub(super) fn read_entry(
    archive: &mut ZipArchive<File>,
    name: &str,
    maximum: u64,
) -> Result<Vec<u8>> {
    let entry = archive
        .by_name(name)
        .with_context(|| format!("distribution ZIP omits `{name}`"))?;
    if entry.is_dir() || special_mode(entry.unix_mode()) || entry.size() > maximum {
        bail!("distribution manifest entry is invalid");
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        bail!("distribution manifest exceeds its size limit");
    }
    Ok(bytes)
}

pub(super) fn copy_hashed(
    input: &mut impl Read,
    output: &mut impl Write,
    hash: &mut Sha256,
    maximum: u64,
) -> Result<u64> {
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > maximum {
            bail!("distribution file exceeds its declared size");
        }
        hash.update(&buffer[..read]);
        output.write_all(&buffer[..read])?;
    }
    Ok(total)
}

pub(super) fn portable_path(value: &str) -> Result<PathBuf> {
    if value.is_empty()
        || value.len() > DISTRIBUTION_SOURCE_MAX_PATH_BYTES
        || value.contains('\\')
        || value.starts_with('/')
        || value.contains(':')
    {
        bail!("distribution path is not portable");
    }
    let path = PathBuf::from(value);
    let components: Vec<_> = path.components().collect();
    if components.len() > DISTRIBUTION_SOURCE_MAX_DEPTH
        || components.iter().any(|part| match part {
            Component::Normal(value) => value.to_str().is_none_or(is_windows_unsafe_component),
            _ => true,
        })
    {
        bail!("distribution path contains traversal or an unsafe component");
    }
    Ok(path)
}

pub(super) fn special_mode(mode: Option<u32>) -> bool {
    mode.is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000)
}
fn portable_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_'))
}
fn hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn git_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && hex(value, value.len())
}
fn tree_hash(value: &str) -> bool {
    value
        .strip_prefix("sha256-tree/1:")
        .is_some_and(|v| hex(v, 64))
}
fn https_url(value: &str) -> bool {
    value.starts_with("https://")
        && !value.contains(['@', '?', '#', '\\'])
        && !value.bytes().any(|b| b.is_ascii_whitespace())
}

pub fn source_differs(
    target: &DistributionTarget,
    source: Option<&ApplicationSourceObservation>,
) -> bool {
    source.is_some_and(|source| source.source_tree != target.source_tree)
}
