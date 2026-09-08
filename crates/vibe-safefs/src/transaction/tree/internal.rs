pub(super) fn inspect_child_state(parent: &Pinned, name: &str) -> Result<Option<EntryState>> {
    inspect_child_state_mode(parent, name, false)
}

fn inspect_child_state_mode(
    parent: &Pinned,
    name: &str,
    allow_transaction_stage: bool,
) -> Result<Option<EntryState>> {
    ensure_tree_component(name, allow_transaction_stage)?;
    if allow_transaction_stage && is_transaction_stage_name(name) {
        return inspect_transaction_stage_state(parent, name);
    }
    let metadata = match parent.dir.symlink_metadata(name) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(anyhow::Error::new(error).context(format!(
                "inspecting `{}` no-follow",
                parent.join(name).display()
            )));
        }
    };
    if metadata.file_type().is_symlink() {
        bail!(
            "`{}` is a link or reparse point",
            parent.join(name).display()
        );
    }
    if metadata.is_dir() {
        let directory = parent.open_child(name)?;
        return Ok(Some(directory_state(&directory)?));
    }
    if !metadata.is_file() {
        bail!(
            "`{}` is a special filesystem entry",
            parent.join(name).display()
        );
    }
    let view = project_view(parent)?;
    match view.stable_file_state_with_identity(name) {
        Ok(Some((state, identity))) => Ok(Some(EntryState {
            kind: EntryStateKind::File,
            sha256: Some(state.sha256),
            bytes: Some(state.bytes),
            unix_mode: state.unix_mode,
            identity: entry_identity(identity),
        })),
        Ok(None) => Ok(None),
        Err(error) => Err(error.context(format!(
            "inspecting `{}` as a no-follow ordinary entry",
            parent.join(name).display()
        ))),
    }
}

fn directory_state(directory: &Pinned) -> Result<EntryState> {
    Ok(EntryState {
        kind: EntryStateKind::Directory,
        sha256: None,
        bytes: None,
        unix_mode: directory.unix_mode()?,
        identity: entry_identity(directory.identity()?),
    })
}

fn ensure_tree_component(name: &str, allow_transaction_stage: bool) -> Result<()> {
    if allow_transaction_stage && is_transaction_stage_name(name) {
        Ok(())
    } else {
        crate::ensure_safe_component(name)
    }
}

fn is_transaction_stage_name(name: &str) -> bool {
    name.strip_prefix(".vibe-stage-tx-").is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn validate_authorized_transaction_stage_path(path: &str) -> Result<()> {
    if path.is_empty() || path.starts_with('/') || path.ends_with('/') {
        bail!("unsafe journal-authorized transaction-stage path");
    }
    let mut components = path.split('/').collect::<Vec<_>>();
    let name = components
        .pop()
        .ok_or_else(|| anyhow::anyhow!("transaction-stage path has no final component"))?;
    for parent in components {
        crate::ensure_safe_component(parent)?;
    }
    if !is_transaction_stage_name(name) {
        bail!("journal-authorized transaction-stage path has invalid grammar");
    }
    Ok(())
}

fn manifest_has_transaction_stage(manifest: &TreeManifest) -> bool {
    manifest.entries.iter().any(|entry| {
        let name = entry
            .path
            .rsplit_once('/')
            .map_or(entry.path.as_str(), |(_, name)| name);
        is_transaction_stage_name(name)
    })
}

pub(super) fn inspect_transaction_stage_state(
    parent: &Pinned,
    name: &str,
) -> Result<Option<EntryState>> {
    use std::io::{Read, Seek, SeekFrom};

    let mut options = crate::file::cap_options();
    let file = match parent.dir.open_with(name, options.read(true)) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(anyhow::Error::new(error)),
    };
    let mut file = file.into_std();
    let display = parent.join(name);
    crate::file::verify_regular_single_link(&file, &display)?;
    let opening = file.metadata()?;
    let mut read_pass = || -> Result<(u64, String)> {
        file.seek(SeekFrom::Start(0))?;
        let mut hash = Sha256::new();
        let mut bytes = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let used = file.read(&mut buffer)?;
            if used == 0 {
                return Ok((bytes, format!("{:x}", hash.finalize())));
            }
            bytes = bytes
                .checked_add(used as u64)
                .ok_or_else(|| anyhow::anyhow!("transaction stage exceeds u64"))?;
            hash.update(&buffer[..used]);
        }
    };
    let first = read_pass()?;
    let second = read_pass()?;
    let closing = file.metadata()?;
    if first != second || first.0 != opening.len() || first.0 != closing.len() {
        bail!("transaction stage changed during stable inspection");
    }
    let identity = crate::file::identity::file_identity(&file, &display)?;
    Ok(Some(EntryState {
        kind: EntryStateKind::File,
        sha256: Some(first.1),
        bytes: Some(first.0),
        unix_mode: crate::file::unix_mode(&closing),
        identity: entry_identity(identity),
    }))
}

pub(super) fn entry_identity(identity: FileIdentity) -> EntryIdentity {
    EntryIdentity(identity_token(
        b"vibe-safefs-tree-entry-identity-e1\0",
        identity,
    ))
}

fn owned_identity(owner: &str, identity: FileIdentity) -> OwnedDirectoryIdentity {
    let mut hash = Sha256::new();
    hash.update(b"vibe-safefs-owned-directory-e1\0");
    hash.update(owner.as_bytes());
    hash.update(b"\0");
    hash.update(identity.identity_bytes());
    OwnedDirectoryIdentity(format!("sha256:{:x}", hash.finalize()))
}

fn manifest(root: &Pinned) -> Result<TreeManifest> {
    manifest_mode(root, false)
}

fn manifest_mode(root: &Pinned, allow_transaction_stage: bool) -> Result<TreeManifest> {
    let mut first = Vec::new();
    walk(root, "", &mut first, allow_transaction_stage)?;
    first.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    manifest_hook::between(root);
    let mut entries = Vec::new();
    walk(root, "", &mut entries, allow_transaction_stage)?;
    entries.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    if entries != first {
        bail!(
            "tree `{}` changed between its two complete manifest passes",
            root.path().display()
        );
    }
    let digest = manifest_digest(&entries);
    Ok(TreeManifest { digest, entries })
}

fn manifest_digest(entries: &[TreeEntry]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-safefs-tree-manifest-e1\0");
    for entry in entries {
        hash.update(entry.path.as_bytes());
        hash.update(b"\0");
        hash.update(match entry.state.kind {
            EntryStateKind::File => b"file\0".as_slice(),
            EntryStateKind::Directory => b"directory\0".as_slice(),
        });
        hash.update(entry.state.identity.as_str().as_bytes());
        hash.update(b"\0");
        if let Some(digest) = &entry.state.sha256 {
            hash.update(digest.as_bytes());
        }
        hash.update(b"\0");
        if let Some(bytes) = entry.state.bytes {
            hash.update(bytes.to_be_bytes());
        }
        hash.update(b"\0");
        if let Some(mode) = entry.state.unix_mode {
            hash.update(mode.to_be_bytes());
        }
        hash.update(b"\n");
    }
    format!("sha256:{:x}", hash.finalize())
}

fn walk(
    directory: &Pinned,
    prefix: &str,
    entries: &mut Vec<TreeEntry>,
    allow_transaction_stage: bool,
) -> Result<()> {
    let view = project_view(directory)?;
    let mut names = view.child_names(directory)?;
    names.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    for name in &names {
        ensure_tree_component(name, allow_transaction_stage)?;
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let state = inspect_child_state_mode(directory, name, allow_transaction_stage)?
            .ok_or_else(|| anyhow::anyhow!("`{path}` vanished during manifest walk"))?;
        entries.push(TreeEntry {
            path: path.clone(),
            state: state.clone(),
        });
        if state.kind == EntryStateKind::Directory {
            let child = directory.open_child(name)?;
            if directory_state(&child)? != state {
                bail!("directory `{path}` was swapped during manifest walk");
            }
            walk(&child, &path, entries, allow_transaction_stage)?;
            if directory_state(&child)? != state {
                bail!("directory `{path}` changed identity or mode during manifest walk");
            }
        }
    }
    let mut after = view.child_names(directory)?;
    after.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    if after != names {
        bail!(
            "directory `{}` changed membership during manifest walk",
            directory.path().display()
        );
    }
    Ok(())
}

fn holder(project: &Project, relative: &str) -> Result<(Pinned, String)> {
    if relative.is_empty() || relative.starts_with('/') || relative.ends_with('/') {
        bail!("unsafe owned-tree relative path");
    }
    let mut components = relative.split('/').map(str::to_owned).collect::<Vec<_>>();
    let name = components
        .pop()
        .ok_or_else(|| anyhow::anyhow!("owned-tree path has no final component"))?;
    for parent in &components {
        crate::ensure_safe_component(parent)?;
    }
    ensure_tree_component(&name, true)?;
    let parents = components;
    if parents.is_empty() {
        return Ok((project.root_dir()?, name));
    }
    let chain = parents.iter().map(String::as_str).collect::<Vec<_>>();
    Ok((project.dir(&chain, false)?, name))
}

fn manifest_difference(expected: &TreeManifest, actual: &TreeManifest) -> String {
    manifest_entries_difference(&expected.entries, &actual.entries)
}

fn manifest_entries_difference(expected: &[TreeEntry], actual: &[TreeEntry]) -> String {
    let expected_paths = expected
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let actual_paths = actual
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if let Some(extra) = actual_paths.difference(&expected_paths).next() {
        return format!("extra descendant `{extra}`");
    }
    if let Some(missing) = expected_paths.difference(&actual_paths).next() {
        return format!("missing descendant `{missing}`");
    }
    for (wanted, found) in expected.iter().zip(actual) {
        if wanted != found {
            return format!(
                "descendant `{}` changed content, size, mode, kind or identity",
                wanted.path
            );
        }
    }
    "manifest digest changed".to_owned()
}

fn depth(path: &str) -> usize {
    path.bytes().filter(|byte| *byte == b'/').count()
}

fn entry_key(entry: &TreeEntry) -> String {
    let prefix = match entry.state.kind {
        EntryStateKind::File => "file:",
        EntryStateKind::Directory => "directory:",
    };
    format!("{prefix}{}", entry.path)
}

fn cleanup_order(manifest: &TreeManifest) -> Vec<String> {
    let mut files = manifest
        .entries
        .iter()
        .filter(|entry| entry.state.kind == EntryStateKind::File)
        .map(entry_key)
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    let mut directories = manifest
        .entries
        .iter()
        .filter(|entry| entry.state.kind == EntryStateKind::Directory)
        .collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        depth(&right.path)
            .cmp(&depth(&left.path))
            .then_with(|| right.path.as_bytes().cmp(left.path.as_bytes()))
    });
    files.extend(directories.into_iter().map(entry_key));
    files.push("root".to_owned());
    files
}

fn cleanup_intent_token(
    identity: &OwnedDirectoryIdentity,
    manifest: &TreeManifest,
    progress_key: &str,
) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-safefs-cleanup-intent-e1\0");
    hash.update(identity.as_str().as_bytes());
    hash.update(b"\0");
    hash.update(manifest.digest.as_bytes());
    hash.update(b"\0");
    hash.update(progress_key.as_bytes());
    format!("sha256:{:x}", hash.finalize())
}

fn remove_native(
    parent: &Pinned,
    name: &str,
    expected: &EntryState,
) -> std::result::Result<DirectoryDurability, OwnedTreeCleanupError> {
    super::platform::remove_expected(parent, name, expected).map_err(|error| match error {
        super::platform::NativeRemoveError::Changed(detail) => third_error(detail),
        super::platform::NativeRemoveError::Io(error) => {
            OwnedTreeCleanupError::Io(anyhow::Error::new(error))
        }
        #[cfg(not(windows))]
        super::platform::NativeRemoveError::Unsupported => OwnedTreeCleanupError::Unsupported,
    })
}

fn third_error(detail: String) -> OwnedTreeCleanupError {
    OwnedTreeCleanupError::Third { detail }
}

fn classify_manifest_cleanup_error(error: anyhow::Error) -> OwnedTreeCleanupError {
    if error
        .chain()
        .any(|cause| cause.downcast_ref::<std::io::Error>().is_some())
    {
        OwnedTreeCleanupError::Io(error)
    } else {
        third_error(format!(
            "owned descendant set changed or became unsafe: {error:#}"
        ))
    }
}

fn validate_identity_token(token: &str) -> Result<()> {
    let Some(hex) = token.strip_prefix("sha256:") else {
        bail!("identity token must use sha256:<64-lowercase-hex>");
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        bail!("identity token must use sha256:<64-lowercase-hex>");
    }
    Ok(())
}

fn is_raw_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(any(test, feature = "inject-failures"))]
mod tree_hook {
    use std::cell::RefCell;

    type Hook = Box<dyn Fn(&crate::Pinned, &str)>;
    thread_local! {
        static BEFORE: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub fn arm(hook: Option<Hook>) {
        BEFORE.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn before(parent: &crate::Pinned, name: &str) {
        let hook = BEFORE.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(parent, name);
        }
    }
}

#[cfg(not(any(test, feature = "inject-failures")))]
mod tree_hook {
    pub fn before(_: &crate::Pinned, _: &str) {}
}
