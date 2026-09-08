use super::*;

pub(crate) fn verify_asset(
    asset: &AssetIdentity,
    identity_project: &Project,
) -> Result<File, HealthError> {
    let path = Path::new(&asset.display_path);
    let mut file = open_identity_locked(path).map_err(|error| {
        HealthError::Execution(format!(
            "opening sealed health asset `{}`: {error}",
            path.display()
        ))
    })?;
    verify_named_asset(asset, identity_project)?;
    // Read the held handle as a second digest pass. On Windows its share mode
    // prevents replacement/write until the launch and final-name recheck end.
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let used = file.read(&mut buffer).map_err(|error| {
            HealthError::Execution(format!("hashing held asset `{}`: {error}", path.display()))
        })?;
        if used == 0 {
            break;
        }
        hash.update(&buffer[..used]);
    }
    if format!("sha256:{:x}", hash.finalize()) != asset.sha256 {
        return Err(HealthError::Execution(format!(
            "held asset `{}` digest changed",
            path.display()
        )));
    }
    Ok(file)
}

pub(super) fn recheck_asset(
    asset: &AssetIdentity,
    identity_project: &Project,
) -> Result<(), HealthError> {
    verify_named_asset(asset, identity_project)
}

fn verify_named_asset(
    asset: &AssetIdentity,
    identity_project: &Project,
) -> Result<(), HealthError> {
    let path = Path::new(&asset.display_path);
    let pinned = Project::pin_absolute_file(path).map_err(|error| {
        HealthError::Execution(format!(
            "pinning named health asset `{}`: {error:#}",
            path.display()
        ))
    })?;
    let snapshot = pinned
        .read_snapshot_bounded(identity_project, 64 * 1024 * 1024)
        .map_err(|error| {
            HealthError::Execution(format!(
                "rechecking named health asset `{}`: {error:#}",
                path.display()
            ))
        })?;
    if snapshot.size != asset.bytes
        || format!("sha256:{}", snapshot.sha256) != asset.sha256
        || snapshot.unix_mode != asset.mode
    {
        return Err(HealthError::Execution(format!(
            "named health asset `{}` changed bytes, size, or mode",
            path.display()
        )));
    }
    let planned = asset.live_identity.ok_or_else(|| {
        HealthError::Unsupported(format!(
            "asset `{}` has no live opaque FileIdentity for exact launch",
            asset.id
        ))
    })?;
    if snapshot.identity != planned {
        return Err(HealthError::Execution(format!(
            "named health asset `{}` changed opaque filesystem identity",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn open_identity_locked(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt as _;

    // FILE_SHARE_READ only: no compatible writer/deleter/renamer can acquire
    // the name until the held launch epoch ends.
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0x0000_0001)
        .open(path)
}

#[cfg(not(windows))]
fn open_identity_locked(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

pub(super) fn materialize_arg(
    arg: &ExpandedArg,
    assets: &[AssetIdentity],
    bundle: Option<&CustomBundle>,
    scratch: &str,
) -> Result<String, HealthError> {
    match arg {
        ExpandedArg::Value(value) => Ok(value.clone()),
        ExpandedArg::AssetPath(id) => assets
            .iter()
            .find(|asset| &asset.id == id)
            .map(|asset| asset.display_path.clone())
            .ok_or_else(|| {
                HealthError::Execution(format!("argv refers to absent sealed asset `{id}`"))
            }),
        ExpandedArg::BundlePath(path) => {
            let bundle = bundle.ok_or_else(|| {
                HealthError::Execution("bundle argv has no sealed custom bundle".to_owned())
            })?;
            let entry = bundle
                .entries
                .iter()
                .find(|entry| entry.path == *path)
                .ok_or_else(|| {
                    HealthError::Execution(format!("bundle member `{path}` is absent"))
                })?;
            if entry.kind != BundleEntryKind::File {
                return Err(HealthError::Execution(format!(
                    "bundle argv member `{path}` is not a regular file"
                )));
            }
            Ok(bundle_target(scratch, path).display().to_string())
        }
    }
}

pub(super) fn materialize_bundle(bundle: &CustomBundle, scratch: &str) -> Result<(), HealthError> {
    let root = Path::new(scratch).join("verifier-bundle");
    std::fs::create_dir_all(&root).map_err(|error| {
        HealthError::Execution(format!("creating verifier bundle root: {error}"))
    })?;
    for entry in &bundle.entries {
        let target = bundle_target(scratch, &entry.path);
        match entry.kind {
            BundleEntryKind::Directory => {
                std::fs::create_dir_all(&target).map_err(|error| {
                    HealthError::Execution(format!(
                        "materializing verifier bundle directory `{}`: {error}",
                        entry.path
                    ))
                })?;
            }
            BundleEntryKind::File => {
                let bytes = entry.content.as_ref().ok_or_else(|| {
                    HealthError::Execution(format!(
                        "bundle file `{}` has no sealed bytes",
                        entry.path
                    ))
                })?;
                if entry.bytes != Some(bytes.len() as u64)
                    || entry.sha256.as_deref()
                        != Some(format!("sha256:{:x}", Sha256::digest(bytes)).as_str())
                {
                    return Err(HealthError::Execution(format!(
                        "bundle file `{}` differs from its sealed size/digest",
                        entry.path
                    )));
                }
                let parent = target.parent().ok_or_else(|| {
                    HealthError::Execution("bundle target has no parent".to_owned())
                })?;
                std::fs::create_dir_all(parent).map_err(|error| {
                    HealthError::Execution(format!(
                        "creating verifier bundle parent for `{}`: {error}",
                        entry.path
                    ))
                })?;
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&target)
                {
                    Ok(mut file) => {
                        use std::io::Write as _;
                        file.write_all(bytes)
                            .and_then(|()| file.sync_all())
                            .map_err(|error| {
                                HealthError::Execution(format!(
                                    "materializing verifier bundle `{}`: {error}",
                                    entry.path
                                ))
                            })?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        let existing = std::fs::read(&target).map_err(|read| {
                            HealthError::Execution(format!(
                                "re-reading verifier bundle `{}`: {read}",
                                entry.path
                            ))
                        })?;
                        if existing != *bytes {
                            return Err(HealthError::Execution(format!(
                                "verifier bundle `{}` already exists with different bytes",
                                entry.path
                            )));
                        }
                    }
                    Err(error) => {
                        return Err(HealthError::Execution(format!(
                            "creating verifier bundle `{}`: {error}",
                            entry.path
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

fn bundle_target(scratch: &str, path: &str) -> PathBuf {
    Path::new(scratch)
        .join("verifier-bundle")
        .join(path.replace('/', std::path::MAIN_SEPARATOR_STR))
}

pub(super) fn validate_isolated_roots(root: &str, protected: &str) -> Result<(), HealthError> {
    let root = Path::new(root);
    let protected = Path::new(protected);
    if !root.is_absolute() || !protected.is_absolute() || root == protected {
        return Err(HealthError::Unsupported(
            "local health requires distinct absolute phase-view and protected roots".to_owned(),
        ));
    }
    if root.starts_with(protected) || protected.starts_with(root) {
        return Err(HealthError::Unsupported(
            "local health phase-view and protected roots must be disjoint".to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn validate_command_cwd(phase_root: &str, cwd: &str) -> Result<(), HealthError> {
    let phase_root = Path::new(phase_root);
    let cwd = Path::new(cwd);
    if !phase_root.is_absolute() || !cwd.is_absolute() || !cwd.starts_with(phase_root) {
        return Err(HealthError::Preparation(
            "health command cwd is not contained by the exact phase root".to_owned(),
        ));
    }
    let relative = cwd.strip_prefix(phase_root).map_err(|error| {
        HealthError::Preparation(format!("deriving health cwd from phase root: {error}"))
    })?;
    if relative.components().any(|component| {
        !matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    }) {
        return Err(HealthError::Preparation(
            "health command cwd contains a non-portable component".to_owned(),
        ));
    }
    let project = Project::open(phase_root).map_err(|error| {
        HealthError::Preparation(format!("opening exact phase root for cwd proof: {error:#}"))
    })?;
    let portable = relative
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !portable.is_empty() {
        project.dir(&portable, false).map_err(|error| {
            HealthError::Preparation(format!("proving nested health cwd no-follow: {error:#}"))
        })?;
    }
    Ok(())
}

fn prove_tree(root: &str, expected: &TreeSeal) -> Result<(), HealthError> {
    let observed = observe_tree(root)?;
    let differences = expected.compare(&observed);
    if differences.is_empty() {
        Ok(())
    } else {
        Err(HealthError::Tree(format!(
            "protected tree differs from its seal: {differences:?}"
        )))
    }
}

pub(super) fn prove_phase_trees(
    root: &str,
    protected: &str,
    expected: &TreeSeal,
) -> Result<(), HealthError> {
    prove_tree(protected, expected)?;
    if root != protected {
        prove_tree(root, expected)?;
    }
    Ok(())
}

pub(super) fn observe_tree(root: &str) -> Result<TreeSeal, HealthError> {
    let project = Project::open(Path::new(root)).map_err(|error| {
        HealthError::Tree(format!("opening protected tree `{root}`: {error:#}"))
    })?;
    let inventory = crate::inventory::collect(&project)
        .map_err(|error| HealthError::Tree(error.to_string()))?;
    Ok(TreeSeal::from_inventory(&inventory))
}
