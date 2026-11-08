//! Receipt-owned standalone skill reconciliation and removal.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill");

use super::*;

pub(super) fn standalone_receipt_path(skills_root: &Path, skill_name: &str) -> PathBuf {
    skills_root.join(format!(".{skill_name}.vibe-skill-receipt.toml"))
}

pub(super) fn digest_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn standalone_receipt(
    skill_name: &str,
    desired: &BTreeMap<String, Vec<u8>>,
) -> StandaloneSkillReceipt {
    StandaloneSkillReceipt {
        schema: STANDALONE_RECEIPT_SCHEMA,
        provider: None,
        activation: None,
        skill: skill_name.to_string(),
        file: desired
            .iter()
            .map(|(path, bytes)| StandaloneOwnedFile {
                path: path.clone(),
                sha256: digest_bytes(bytes),
            })
            .collect(),
    }
}

pub(super) fn read_standalone_receipt(
    path: &Path,
    skill_name: &str,
) -> Result<Option<StandaloneSkillReceipt>, PackageSkillError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(PackageSkillError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if metadata.file_type().is_symlink() || metadata_is_reparse(&metadata) || !metadata.is_file() {
        return Err(PackageSkillError::UnsafePath {
            path: path.to_path_buf(),
            reason: "standalone ownership receipt is not a no-follow regular file".into(),
        });
    }
    let text = fs::read_to_string(path).map_err(|source| PackageSkillError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let parsed: StandaloneSkillReceipt =
        toml::from_str(&text).map_err(|error| PackageSkillError::UnsafePath {
            path: path.to_path_buf(),
            reason: format!("standalone ownership receipt is malformed: {error}"),
        })?;
    if !matches!(parsed.schema, 1 | 2) || parsed.skill != skill_name {
        return Err(PackageSkillError::UnsafePath {
            path: path.to_path_buf(),
            reason: format!(
                "standalone ownership receipt does not identify schema {STANDALONE_RECEIPT_SCHEMA} skill `{skill_name}`"
            ),
        });
    }
    if parsed.schema == 1 && (parsed.activation.is_some() || parsed.provider.is_some()) {
        return Err(PackageSkillError::UnsafePath {
            path: path.into(),
            reason: "activation metadata requires standalone receipt schema 2".into(),
        });
    }
    if let Some(intent) = &parsed.activation {
        crate::activation::validate(intent).map_err(|e| PackageSkillError::UnsafePath {
            path: path.into(),
            reason: e.to_string(),
        })?;
    }
    receipt::judge_selection(parsed.file.iter().map(|file| file.path.as_str())).map_err(
        |fault| PackageSkillError::UnsafePath {
            path: path.to_path_buf(),
            reason: format!("standalone ownership receipt has an unsafe file set: {fault}"),
        },
    )?;
    if parsed.file.iter().any(|file| {
        file.sha256.len() != 64
            || !file
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }) {
        return Err(PackageSkillError::UnsafePath {
            path: path.to_path_buf(),
            reason: "standalone ownership receipt contains an invalid sha256".into(),
        });
    }
    Ok(Some(parsed))
}

pub(super) fn write_standalone_receipt(
    path: &Path,
    receipt: &StandaloneSkillReceipt,
) -> Result<(), PackageSkillError> {
    let encoded = toml::to_string(receipt).map_err(|error| PackageSkillError::UnsafePath {
        path: path.to_path_buf(),
        reason: format!("cannot encode standalone ownership receipt: {error}"),
    })?;
    let parent = path.parent().ok_or_else(|| PackageSkillError::UnsafePath {
        path: path.into(),
        reason: "receipt lacks parent".into(),
    })?;
    fs::create_dir_all(parent).map_err(|source| PackageSkillError::Write {
        path: parent.into(),
        source,
    })?;
    let project =
        vibe_safefs::Project::open(parent).map_err(|e| PackageSkillError::UnsafePath {
            path: parent.into(),
            reason: e.to_string(),
        })?;
    let name =
        path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| PackageSkillError::UnsafePath {
                path: path.into(),
                reason: "unportable receipt filename".into(),
            })?;
    let directory = project
        .dir(&[], false)
        .map_err(|e| PackageSkillError::UnsafePath {
            path: parent.into(),
            reason: e.to_string(),
        })?;
    project
        .write_atomic_in(&directory, name, encoded.as_bytes())
        .map(|_| ())
        .map_err(|e| PackageSkillError::UnsafePath {
            path: path.into(),
            reason: e.into_report().to_string(),
        })
}

pub(super) fn owned_map(receipt: &StandaloneSkillReceipt) -> BTreeMap<&str, &str> {
    receipt
        .file
        .iter()
        .map(|file| (file.path.as_str(), file.sha256.as_str()))
        .collect()
}

pub(super) fn validate_standalone_ownership(
    target: &Path,
    current: Option<&BTreeMap<String, Vec<u8>>>,
    prior: Option<&StandaloneSkillReceipt>,
    desired: &BTreeMap<String, Vec<u8>>,
) -> Result<(), PackageSkillError> {
    if current.is_some() && prior.is_none() {
        return Err(PackageSkillError::UnsafePath {
            path: target.to_path_buf(),
            reason: "refusing foreign pre-existing skill target without a Vibe ownership receipt"
                .into(),
        });
    }
    let (Some(current), Some(prior)) = (current, prior) else {
        return Ok(());
    };
    let owned = owned_map(prior);
    for (path, expected) in &owned {
        if let Some(bytes) = current.get(*path)
            && digest_bytes(bytes) != *expected
            && desired.get(*path) != Some(bytes)
        {
            return Err(PackageSkillError::UnsafePath {
                path: target.join(path),
                reason: "refusing to overwrite or remove a tampered Vibe-owned skill file".into(),
            });
        }
    }
    for path in desired.keys() {
        if current.contains_key(path) && !owned.contains_key(path.as_str()) {
            return Err(PackageSkillError::UnsafePath {
                path: target.join(path),
                reason: "refusing to overwrite an unowned file inside a Vibe-owned skill target"
                    .into(),
            });
        }
    }
    Ok(())
}

pub(super) fn validate_standalone_uninstall(
    target: &Path,
    current: Option<&BTreeMap<String, Vec<u8>>>,
    prior: Option<&StandaloneSkillReceipt>,
) -> Result<(), PackageSkillError> {
    if current.is_some() && prior.is_none() {
        return Err(PackageSkillError::UnsafePath {
            path: target.to_path_buf(),
            reason: "refusing to uninstall foreign pre-existing skill target without a Vibe ownership receipt"
                .into(),
        });
    }
    let (Some(current), Some(prior)) = (current, prior) else {
        return Ok(());
    };
    for file in &prior.file {
        if let Some(bytes) = current.get(&file.path)
            && digest_bytes(bytes) != file.sha256
        {
            return Err(PackageSkillError::UnsafePath {
                path: target.join(&file.path),
                reason: "refusing to remove a tampered Vibe-owned skill file".into(),
            });
        }
    }
    Ok(())
}

pub(super) fn owned_projection_matches(
    current: Option<&BTreeMap<String, Vec<u8>>>,
    prior: Option<&StandaloneSkillReceipt>,
    desired: &BTreeMap<String, Vec<u8>>,
) -> bool {
    let (Some(current), Some(prior)) = (current, prior) else {
        return false;
    };
    let owned = owned_map(prior);
    desired
        .iter()
        .all(|(path, bytes)| current.get(path) == Some(bytes))
        && owned
            .keys()
            .all(|path| desired.contains_key(*path) || !current.contains_key(*path))
}

pub(super) fn reconcile_standalone_files(
    target: &Path,
    prior: Option<&StandaloneSkillReceipt>,
    desired: &BTreeMap<String, Vec<u8>>,
) -> Result<(), PackageSkillError> {
    if let Some(prior) = prior {
        for file in &prior.file {
            if !desired.contains_key(&file.path) {
                remove_file_if_present(&target.join(&file.path))?;
            }
        }
    }
    write_snapshot(target, desired)
}

pub(super) fn remove_owned_standalone_files(
    target: &Path,
    prior: Option<&StandaloneSkillReceipt>,
) -> Result<(), PackageSkillError> {
    let Some(prior) = prior else {
        return Ok(());
    };
    for file in &prior.file {
        remove_file_if_present(&target.join(&file.path))?;
    }
    remove_empty_dirs(target, target)?;
    Ok(())
}

pub(super) fn remove_file_if_present(path: &Path) -> Result<(), PackageSkillError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(PackageSkillError::Write {
            path: path.to_path_buf(),
            source,
        }),
    }
}

pub(super) fn remove_empty_dirs(root: &Path, dir: &Path) -> Result<bool, PackageSkillError> {
    if !dir.exists() {
        return Ok(true);
    }
    for entry in fs::read_dir(dir).map_err(|source| PackageSkillError::Read {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| PackageSkillError::Read {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|source| PackageSkillError::Read {
            path: path.clone(),
            source,
        })?;
        if metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && !metadata_is_reparse(&metadata)
        {
            remove_empty_dirs(root, &path)?;
        }
    }
    let empty = fs::read_dir(dir)
        .map_err(|source| PackageSkillError::Read {
            path: dir.to_path_buf(),
            source,
        })?
        .next()
        .is_none();
    if empty {
        fs::remove_dir(dir).map_err(|source| PackageSkillError::Write {
            path: dir.to_path_buf(),
            source,
        })?;
    }
    Ok(dir == root && empty)
}
