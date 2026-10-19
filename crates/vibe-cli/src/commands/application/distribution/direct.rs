//! Direct installer artifacts, verified before publication or execution.

use super::*;
use vibe_core::manifest::ArchiveRunnerDecl;

#[derive(Debug, Clone, Copy)]
pub enum DirectArchiveFormat {
    Zip,
    TarGz,
}

/// Fetch a native installer into a new staged regular file and verify its digest.
/// The caller chooses the extension needed by the platform installer.
pub fn verify_installer_file(
    source: &str,
    sha256: &str,
    staging: &Path,
    offline: bool,
    progress: &Progress,
) -> Result<PathBuf> {
    valid_digest(sha256)?;
    if staging.exists() {
        bail!("installer staging destination already exists");
    }
    let parent = staging
        .parent()
        .ok_or_else(|| anyhow::anyhow!("installer staging has no parent"))?;
    let temporary = tempfile::NamedTempFile::new_in(parent)?;
    let task = progress.task("Fetching application installer");
    let mut input: Box<dyn Read> = if source.starts_with("https://") {
        if offline {
            bail!("offline install requires a local installer source");
        }
        let url = reqwest::Url::parse(source)?;
        if !url.username().is_empty() || url.password().is_some() {
            bail!("installer URL must not contain credentials");
        }
        Box::new(
            reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(120))
                .redirect(reqwest::redirect::Policy::custom(|attempt| {
                    let url = attempt.url();
                    if attempt.previous().len() >= 5
                        || url.scheme() != "https"
                        || !url.username().is_empty()
                        || url.password().is_some()
                    {
                        attempt.error(
                            "installer redirect left the bounded credential-free HTTPS scope",
                        )
                    } else {
                        attempt.follow()
                    }
                }))
                .build()?
                .get(url)
                .send()
                .context("fetching application installer")?
                .error_for_status()?,
        )
    } else {
        if source.contains("://") {
            bail!("installer source must be HTTPS or a local file");
        }
        let path = Path::new(source);
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            bail!("installer source is not a regular file");
        }
        Box::new(File::open(path)?)
    };
    let mut output = temporary.reopen()?;
    let mut hash = Sha256::new();
    let count = copy_hashed(
        &mut input,
        &mut output,
        &mut hash,
        DISTRIBUTION_BUNDLE_MAX_BYTES,
    )?;
    if count == 0 || format!("{:x}", hash.finalize()) != sha256.to_ascii_lowercase() {
        bail!("application installer SHA-256 differs from declaration");
    }
    output.sync_all()?;
    drop(output);
    temporary.persist_noclobber(staging)?;
    task.finish();
    Ok(staging.to_path_buf())
}

#[allow(clippy::too_many_arguments)]
pub fn verify_direct_archive(
    source: &str,
    format: DirectArchiveFormat,
    sha256: &str,
    payload_root: Option<&Path>,
    runners: &[ArchiveRunnerDecl],
    selected: &ApplicationIdentity,
    staging: &Path,
    offline: bool,
    progress: &Progress,
    provider_local: bool,
) -> Result<VerifiedDistribution> {
    if staging.exists() {
        bail!("archive staging destination already exists");
    }
    if cfg!(windows) && !provider_local && !runners.is_empty() {
        bail!("direct archive Windows runner requires a literal-argument native launcher");
    }
    let parent = staging
        .parent()
        .ok_or_else(|| anyhow::anyhow!("archive staging has no parent"))?;
    let scratch = tempfile::tempdir_in(parent)?;
    let archive_path = verify_installer_file(
        source,
        sha256,
        &scratch.path().join("artifact"),
        offline,
        progress,
    )?;
    let extracted = scratch.path().join("extracted");
    fs::create_dir(&extracted)?;
    let mut files = Vec::new();
    let mut seen = BTreeSet::new();
    let mut components = std::collections::BTreeMap::new();
    let mut expanded = 0u64;
    match format {
        DirectArchiveFormat::Zip => {
            let mut archive = ZipArchive::new(File::open(&archive_path)?)?;
            if archive.len() as u64 > DISTRIBUTION_SOURCE_MAX_FILES {
                bail!("direct ZIP contains too many entries");
            }
            for index in 0..archive.len() {
                let mut entry = archive.by_index(index)?;
                let name = entry.name().to_owned();
                let directory = entry.is_dir();
                let path = checked_entry(&name, directory, &mut seen, &mut components)?;
                let mode = entry.unix_mode();
                if directory {
                    if mode.is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o040000) {
                        bail!("direct ZIP contains a special directory");
                    }
                    continue;
                }
                if special_mode(mode) {
                    bail!("direct ZIP contains a link or special entry");
                }
                expanded = checked_expanded(expanded, entry.size())?;
                let size = entry.size();
                write_entry(
                    &mut entry,
                    &extracted.join(path),
                    size,
                    mode,
                    &name,
                    &mut files,
                )?;
            }
        }
        DirectArchiveFormat::TarGz => {
            let decoder = flate2::read::GzDecoder::new(File::open(&archive_path)?);
            let mut archive = tar::Archive::new(decoder);
            for (index, item) in archive.entries()?.enumerate() {
                if index as u64 >= DISTRIBUTION_SOURCE_MAX_FILES {
                    bail!("direct tar.gz contains too many entries");
                }
                let mut entry = item?;
                let name = entry
                    .path()?
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("tar.gz path is not UTF-8"))?
                    .to_owned();
                let kind = entry.header().entry_type();
                if !kind.is_file() && !kind.is_dir() {
                    bail!("direct tar.gz contains a link or special entry");
                }
                let path = checked_entry(&name, kind.is_dir(), &mut seen, &mut components)?;
                if kind.is_dir() {
                    continue;
                }
                expanded = checked_expanded(expanded, entry.size())?;
                let size = entry.size();
                let mode = entry.header().mode().ok();
                write_entry(
                    &mut entry,
                    &extracted.join(path),
                    size,
                    mode,
                    &name,
                    &mut files,
                )?;
            }
        }
    }
    if files.is_empty() {
        bail!("direct archive has no files");
    }
    let selected_root = match payload_root {
        Some(value) => extracted.join(portable_path(
            value
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("payload root is not UTF-8"))?,
        )?),
        None => extracted.clone(),
    };
    if !selected_root.is_dir() {
        bail!("declared archive payload root does not exist");
    }
    let relative_root = selected_root.strip_prefix(&extracted)?;
    let files: Vec<_> = files
        .into_iter()
        .filter_map(|mut file: BundleFile| {
            let relative = Path::new(&file.path).strip_prefix(relative_root).ok()?;
            if relative.as_os_str().is_empty() {
                return None;
            }
            file.path = relative.to_string_lossy().replace('\\', "/");
            Some(file)
        })
        .collect();
    if files.is_empty() {
        bail!("declared archive payload root has no files");
    }
    let mut launchers = Vec::new();
    let mut destinations = BTreeSet::new();
    let mut logical_names = BTreeSet::new();
    let mut covered = BTreeSet::new();
    for runner in runners {
        let path = runner
            .path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("archive runner path is not UTF-8"))?;
        portable_path(path)?;
        if !files.iter().any(|file| file.path == path) {
            bail!("archive runner is absent from selected payload");
        }
        portable_path(&runner.name)?;
        if runner.name.contains('/') {
            bail!("archive runner name must be one portable component");
        }
        let command = if provider_local {
            let logical = logical_runner_name(&runner.name)?;
            if !logical_names.insert(logical.clone()) {
                bail!("direct archive repeats a logical runner name");
            }
            logical
        } else {
            selected
                .commands
                .iter()
                .find(|command| super::super::model::launcher_kind(&runner.name, command).is_some())
                .ok_or_else(|| anyhow::anyhow!("archive runner name does not match a command"))?
                .clone()
        };
        if !destinations.insert(runner.name.to_ascii_lowercase()) {
            bail!("direct archive repeats a runner destination");
        }
        if !provider_local
            && super::super::model::launcher_kind(&runner.name, &command)
                .is_some_and(|kind| kind.covers_command_on(std::env::consts::OS))
        {
            covered.insert(command.clone());
        }
        launchers.push(BundleLauncher {
            command,
            path: path.into(),
            destination: runner.name.clone(),
        });
    }
    if !provider_local && covered.len() != selected.commands.len() {
        bail!("direct archive lacks a runnable public runner for a declared command");
    }
    fs::rename(&selected_root, staging).context("staging selected archive payload")?;
    let entry = files[0].path.clone();
    Ok(VerifiedDistribution {
        manifest: BundleManifest {
            protocol: BUNDLE_PROTOCOL.into(),
            application: selected.clone(),
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            libc: current_libc().map(str::to_owned),
            source_commit: String::new(),
            source_tree: String::new(),
            management: BundleManagement {
                runtime: "builtin".into(),
                entry,
            },
            launchers,
            files,
        },
        payload_root: staging.to_path_buf(),
        asset_sha256: sha256.to_ascii_lowercase(),
        direct_archive: true,
    })
}

fn logical_runner_name(name: &str) -> Result<String> {
    let logical = [".exe", ".cmd", ".bat", ".ps1", ".sh"]
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .unwrap_or(name);
    if logical.is_empty()
        || logical.len() > 64
        || !logical.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        bail!("archive runner has no portable logical command name");
    }
    Ok(logical.to_owned())
}

fn valid_digest(value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("installer SHA-256 must be 64 hexadecimal digits");
    }
    Ok(())
}

fn checked_entry(
    name: &str,
    directory: bool,
    seen: &mut BTreeSet<String>,
    components: &mut std::collections::BTreeMap<String, String>,
) -> Result<PathBuf> {
    let name = if directory {
        name.trim_end_matches('/')
    } else {
        name
    };
    let path = portable_path(name)?;
    if !seen.insert(name.to_ascii_lowercase()) {
        bail!("archive contains duplicate or case-conflicting entries");
    }
    let mut prefix = String::new();
    for component in path.components() {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(&component.as_os_str().to_string_lossy());
        if components
            .insert(prefix.to_ascii_lowercase(), prefix.clone())
            .is_some_and(|previous| previous != prefix)
        {
            bail!("archive contains case-conflicting path components");
        }
    }
    Ok(path)
}

fn checked_expanded(current: u64, size: u64) -> Result<u64> {
    let total = current
        .checked_add(size)
        .ok_or_else(|| anyhow::anyhow!("archive expanded size overflow"))?;
    if total > DISTRIBUTION_SOURCE_EXPANDED_MAX_BYTES {
        bail!("archive expanded payload exceeds limit");
    }
    Ok(total)
}

fn write_entry(
    input: &mut impl Read,
    destination: &Path,
    size: u64,
    mode: Option<u32>,
    name: &str,
    files: &mut Vec<BundleFile>,
) -> Result<()> {
    fs::create_dir_all(
        destination
            .parent()
            .ok_or_else(|| anyhow::anyhow!("archive destination has no parent"))?,
    )?;
    let mut output = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)?;
    let mut hash = Sha256::new();
    if copy_hashed(input, &mut output, &mut hash, size)? != size {
        bail!("archive entry size changed during extraction");
    }
    output.sync_all()?;
    apply_zip_permissions(destination, mode)?;
    files.push(BundleFile {
        path: name.into(),
        sha256: format!("{:x}", hash.finalize()),
        size,
    });
    Ok(())
}
