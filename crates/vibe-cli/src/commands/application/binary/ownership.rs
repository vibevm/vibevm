//! Validated built-in application ownership and generation helpers.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#binary-ownership");

use super::*;

/// The public slot containing either a legacy direct host or a versioned
/// provider host at slot/group/name/version.
pub(super) fn slot_root_for_host(host: &Path) -> Result<PathBuf> {
    let parent = host
        .parent()
        .ok_or_else(|| anyhow::anyhow!("application host has no parent"))?;
    if parent.file_name().is_some_and(|name| name == "apps") {
        return Ok(host.to_path_buf());
    }
    let group = parent
        .parent()
        .ok_or_else(|| anyhow::anyhow!("application host has no group"))?;
    let slot = group
        .parent()
        .ok_or_else(|| anyhow::anyhow!("application host has no slot"))?;
    if slot
        .parent()
        .is_none_or(|apps| apps.file_name().is_none_or(|name| name != "apps"))
    {
        bail!("application host is outside the slot installation layout");
    }
    Ok(slot.to_path_buf())
}

pub(super) fn verify_generation(root: &Path, manifest: &BundleManifest) -> Result<()> {
    for file in &manifest.files {
        let path = root.join(&file.path);
        let mut ancestor = path.parent();
        while let Some(directory) = ancestor {
            if directory == root {
                break;
            }
            let metadata = fs::symlink_metadata(directory)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                bail!("cached application generation contains an unsafe parent");
            }
            ancestor = directory.parent();
        }
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() != file.size
            || hash_file(&path)? != file.sha256
        {
            bail!("cached application generation differs from its verified manifest");
        }
    }
    Ok(())
}

pub(super) fn ensure_owned_host(settings_root: &Path, host_root: &Path) -> Result<PathBuf> {
    fs::create_dir_all(settings_root)?;
    let settings = crate::commands::init::strip_unc_public(fs::canonicalize(settings_root)?);
    if host_root.exists() {
        let host = crate::commands::init::strip_unc_public(fs::canonicalize(host_root)?);
        if !host.starts_with(&settings) || host == settings {
            bail!("application host escapes settings root");
        }
        return Ok(host);
    }
    if !host_root.starts_with(&settings) {
        bail!("application host escapes settings root");
    }
    fs::create_dir_all(host_root)?;
    let host = crate::commands::init::strip_unc_public(fs::canonicalize(host_root)?);
    if !host.starts_with(&settings) || host == settings {
        bail!("application host escapes settings root");
    }
    Ok(host)
}

pub(super) fn read_management(entry: &ManagementEntry) -> Result<BinaryManagement> {
    let receipt_metadata = fs::symlink_metadata(&entry.entry)?;
    if receipt_metadata.file_type().is_symlink() || !receipt_metadata.is_file() {
        bail!("binary application management receipt is not a regular file");
    }
    let raw = fs::read(&entry.entry).context("reading binary application management receipt")?;
    let value: BinaryManagement =
        serde_json::from_slice(&raw).context("parsing binary application management receipt")?;
    if value.protocol != MANAGEMENT_PROTOCOL {
        bail!("binary application management protocol is unsupported");
    }
    let management_dir = entry
        .entry
        .parent()
        .ok_or_else(|| anyhow::anyhow!("binary management entry has no parent"))?;
    let host = management_dir
        .parent()
        .ok_or_else(|| anyhow::anyhow!("binary management entry has no host"))?;
    let direct_apps = host
        .parent()
        .filter(|path| path.file_name().is_some_and(|name| name == "apps"));
    let (apps, slot) = if let Some(apps) = direct_apps {
        (apps, host)
    } else {
        let package_name = host
            .parent()
            .ok_or_else(|| anyhow::anyhow!("binary host has no package name parent"))?;
        let group = package_name
            .parent()
            .ok_or_else(|| anyhow::anyhow!("binary host has no package group parent"))?;
        let slot = group
            .parent()
            .ok_or_else(|| anyhow::anyhow!("binary host has no slot parent"))?;
        let apps = slot
            .parent()
            .filter(|path| path.file_name().is_some_and(|name| name == "apps"))
            .ok_or_else(|| {
                anyhow::anyhow!("binary management host is outside settings applications")
            })?;
        for directory in [slot, group, package_name] {
            let metadata = fs::symlink_metadata(directory)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                bail!("binary application package path contains a link or non-directory");
            }
        }
        (apps, slot)
    };
    let opt = apps
        .parent()
        .filter(|path| path.file_name().is_some_and(|name| name == "opt"))
        .ok_or_else(|| anyhow::anyhow!("binary management host is outside settings opt"))?;
    let settings = opt
        .parent()
        .ok_or_else(|| anyhow::anyhow!("binary management host has no settings root"))?;
    let generations = host.join("generations");
    for directory in [opt, apps, host, management_dir, generations.as_path()] {
        let metadata = fs::symlink_metadata(directory)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!("binary application management path contains a link or non-directory");
        }
    }
    if entry
        .entry
        .file_name()
        .is_none_or(|name| name != "binary.json")
        || management_dir
            .file_name()
            .is_none_or(|name| name != "management")
        || slot.file_name().and_then(|name| name.to_str()) != Some(value.application_id.as_str())
        || value.payload_root.parent() != Some(&host.join("generations"))
        || value.bundle_entry.parent().is_none()
        || !value.bundle_entry.starts_with(&value.payload_root)
        || value
            .bundle_entry
            .strip_prefix(&value.payload_root)
            .is_ok_and(|relative| {
                relative
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
            })
    {
        bail!("binary application management receipt escapes its owned host");
    }
    let payload_metadata = fs::symlink_metadata(&value.payload_root)?;
    if payload_metadata.file_type().is_symlink() || !payload_metadata.is_dir() {
        bail!("binary application payload root is not an owned directory");
    }
    let bundle_metadata = fs::symlink_metadata(&value.bundle_entry)
        .context("reading binary bundle management entry")?;
    if bundle_metadata.file_type().is_symlink() || !bundle_metadata.is_file() {
        bail!("binary bundle management entry is not a regular owned file");
    }
    let mut ancestor = value.bundle_entry.parent();
    while let Some(directory) = ancestor {
        if directory == value.payload_root {
            break;
        }
        let metadata = fs::symlink_metadata(directory)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!("binary bundle management entry has an unsafe parent");
        }
        ancestor = directory.parent();
    }
    let launcher_root = settings.join("opt").join("bin");
    let launcher_metadata = fs::symlink_metadata(&launcher_root)?;
    if launcher_metadata.file_type().is_symlink() || !launcher_metadata.is_dir() {
        bail!("binary application launcher root is not an owned directory");
    }
    let mut destinations = std::collections::BTreeSet::new();
    for launcher in &value.launchers {
        if launcher.destination.parent() != Some(launcher_root.as_path())
            || launcher
                .destination
                .file_name()
                .and_then(|name| name.to_str())
                .is_none_or(|name| name.is_empty() || name.contains(['/', '\\']))
            || launcher.sha256.len() != 64
            || !launcher
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || !destinations.insert(launcher.destination.clone())
        {
            bail!("binary application management receipt has an invalid launcher destination");
        }
    }
    Ok(value)
}

pub(super) fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

pub(super) fn hash_file(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("application-owned path is not a regular file");
    }
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
