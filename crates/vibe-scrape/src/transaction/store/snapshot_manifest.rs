fn validate_snapshot_bounds(records: &[SnapshotRecord]) -> Result<(), TransactionError> {
    let mut total = 0u64;
    for record in records {
        validate_snapshot_record_bound(record)?;
        total = total
            .checked_add(record.bytes)
            .ok_or_else(|| store_error("snapshot byte total overflow"))?;
    }
    if total > MAX_SNAPSHOT_TOTAL_BYTES {
        return Err(store_error(format!(
            "snapshot set exceeds {} byte bound",
            MAX_SNAPSHOT_TOTAL_BYTES
        )));
    }
    Ok(())
}

fn validate_snapshot_record_bound(record: &SnapshotRecord) -> Result<(), TransactionError> {
    validate_relative(&record.name, "snapshot name")?;
    if record.bytes > MAX_SNAPSHOT_BYTES as u64 {
        return Err(store_error(format!(
            "snapshot `{}` exceeds {} byte bound",
            record.name, MAX_SNAPSHOT_BYTES
        )));
    }
    Ok(())
}

fn verify_snapshot_bytes(record: &SnapshotRecord, bytes: &[u8]) -> Result<(), TransactionError> {
    if bytes.len() as u64 != record.bytes {
        return Err(store_error(format!(
            "snapshot `{}` byte count differs from journal",
            record.name
        )));
    }
    let digest = sha256_bytes(bytes);
    if digest != record.sha256.0 {
        return Err(store_error(format!(
            "snapshot `{}` digest differs from journal",
            record.name
        )));
    }
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("sha256:{:x}", hash.finalize())
}

fn snapshot_directories(files: &BTreeSet<String>) -> BTreeSet<String> {
    let mut directories = BTreeSet::new();
    for file in files {
        let mut prefix = String::new();
        let mut components = file.split('/').peekable();
        while let Some(component) = components.next() {
            if components.peek().is_none() {
                break;
            }
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(component);
            directories.insert(prefix.clone());
        }
    }
    directories
}

fn collect_external_entries(
    directory: &ExternalDirectory,
    prefix: &str,
    output: &mut Vec<(String, EntryStateKind)>,
    depth: usize,
) -> Result<(), TransactionError> {
    if depth > 128 {
        return Err(store_error("external transaction tree exceeds depth bound"));
    }
    for name in directory
        .child_names_bounded(MAX_DIRECTORY_CHILDREN)
        .map_err(|error| store_error(format!("enumerating external transaction tree: {error:#}")))?
    {
        validate_relative(&name, "external transaction component")?;
        if output.len() >= MAX_TREE_ENTRIES {
            return Err(store_error("external transaction tree exceeds entry bound"));
        }
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let state = directory
            .inspect_child_state(&name)
            .map_err(|error| store_error(format!("inspecting `{path}`: {error:#}")))?
            .ok_or_else(|| store_error(format!("external entry `{path}` vanished")))?;
        output.push((path.clone(), state.kind));
        if state.kind == EntryStateKind::Directory {
            let child = directory
                .open_child(&name)
                .map_err(|error| store_error(format!("opening `{path}`: {error:#}")))?
                .ok_or_else(|| store_error(format!("directory `{path}` vanished")))?;
            collect_external_entries(&child, &path, output, depth + 1)?;
        }
    }
    Ok(())
}

fn observe_safefs_manifest(
    directory: &ExternalDirectory,
) -> Result<SafefsTreeManifest, TransactionError> {
    let mut entries = Vec::new();
    collect_safefs_manifest_entries(directory, "", &mut entries, 0)?;
    entries.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    let digest = safefs_manifest_digest(&entries);
    Ok(SafefsTreeManifest { digest, entries })
}

fn validate_workspace_owner(
    directory: &ExternalDirectory,
    owner: &OwnerWire,
    journal: &Journal,
) -> Result<(), TransactionError> {
    let expected = journal
        .verification_workspace
        .as_ref()
        .ok_or_else(|| store_error("transaction journal has no verification-workspace intent"))?;
    let state = directory
        .inspect_child_state(&expected.name)
        .map_err(|error| store_error(format!("inspecting verification workspace: {error:#}")))?
        .ok_or_else(|| store_error("verification workspace is absent"))?;
    if state.kind != EntryStateKind::Directory
        || state.identity.as_str() != owner.workspace_entry_identity
    {
        return Err(store_error(
            "verification workspace entry identity differs from its owner seal",
        ));
    }
    let child = directory
        .open_child(&expected.name)
        .map_err(|error| store_error(format!("opening verification workspace: {error:#}")))?
        .ok_or_else(|| store_error("verification workspace vanished"))?;
    if child.path().display().to_string() != expected.display_root {
        return Err(store_error(
            "verification workspace display root differs from journal intent",
        ));
    }
    let project = Project::open(child.path()).map_err(|error| {
        store_error(format!(
            "opening verification workspace identity: {error:#}"
        ))
    })?;
    if project.identity_token().map_err(|error| {
        store_error(format!(
            "reading verification workspace identity: {error:#}"
        ))
    })? != owner.workspace_project_identity
    {
        return Err(store_error(
            "verification workspace project identity differs from owner seal",
        ));
    }
    Ok(())
}

fn validate_transaction_home_shape(
    directory: &ExternalDirectory,
    journal: &Journal,
    owner_wire: &OwnerWire,
) -> Result<(), TransactionError> {
    let mut owner = false;
    let mut journal_file = false;
    let mut verification = false;
    for name in directory
        .child_names_bounded(MAX_DIRECTORY_CHILDREN)
        .map_err(|error| store_error(format!("enumerating transaction root: {error:#}")))?
    {
        let state = directory
            .inspect_child_state(&name)
            .map_err(|error| {
                store_error(format!(
                    "inspecting transaction-root entry `{name}`: {error:#}"
                ))
            })?
            .ok_or_else(|| store_error(format!("transaction-root entry `{name}` vanished")))?;
        match (name.as_str(), state.kind) {
            (OWNER_FILE, EntryStateKind::File) => owner = true,
            (JOURNAL_FILE, EntryStateKind::File) => journal_file = true,
            (SNAPSHOTS_DIRECTORY, EntryStateKind::Directory) => {}
            (VERIFICATION_DIRECTORY, EntryStateKind::Directory) => {
                verification = true;
            }
            _ => {
                return Err(store_error(format!(
                    "unexpected entry `{name}` in transaction root"
                )));
            }
        }
    }
    if !owner || !journal_file || !verification {
        return Err(store_error(
            "transaction root lacks its exact owner/journal/workspace authority",
        ));
    }
    validate_workspace_owner(directory, owner_wire, journal)?;
    Ok(())
}

fn collect_safefs_manifest_entries(
    directory: &ExternalDirectory,
    prefix: &str,
    output: &mut Vec<SafefsTreeEntry>,
    depth: usize,
) -> Result<(), TransactionError> {
    if depth > 128 {
        return Err(store_error("transaction manifest exceeds depth bound"));
    }
    for name in directory
        .child_names_bounded(MAX_DIRECTORY_CHILDREN)
        .map_err(|error| store_error(format!("enumerating transaction manifest: {error:#}")))?
    {
        validate_relative(&name, "transaction manifest component")?;
        if output.len() >= MAX_TREE_ENTRIES {
            return Err(store_error("transaction manifest exceeds entry bound"));
        }
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let state = directory
            .inspect_child_state(&name)
            .map_err(|error| store_error(format!("inspecting manifest entry `{path}`: {error:#}")))?
            .ok_or_else(|| store_error(format!("manifest entry `{path}` vanished")))?;
        output.push(SafefsTreeEntry {
            path: path.clone(),
            state: state.clone(),
        });
        if state.kind == EntryStateKind::Directory {
            let child = directory
                .open_child(&name)
                .map_err(|error| {
                    store_error(format!("opening manifest directory `{path}`: {error:#}"))
                })?
                .ok_or_else(|| store_error(format!("manifest directory `{path}` vanished")))?;
            collect_safefs_manifest_entries(&child, &path, output, depth + 1)?;
        }
    }
    Ok(())
}

fn safefs_manifest_digest(entries: &[SafefsTreeEntry]) -> String {
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

fn validate_safefs_manifest(manifest: &SafefsTreeManifest) -> Result<(), TransactionError> {
    let mut previous: Option<&str> = None;
    for entry in &manifest.entries {
        validate_relative(&entry.path, "retirement manifest path")?;
        if previous.is_some_and(|value| value.as_bytes() >= entry.path.as_bytes()) {
            return Err(store_error(
                "retirement manifest paths are not unique and byte-sorted",
            ));
        }
        previous = Some(&entry.path);
        match entry.state.kind {
            EntryStateKind::File if entry.state.sha256.is_none() || entry.state.bytes.is_none() => {
                return Err(store_error("retirement manifest file lacks content state"));
            }
            EntryStateKind::Directory
                if entry.state.sha256.is_some() || entry.state.bytes.is_some() =>
            {
                return Err(store_error(
                    "retirement manifest directory carries file content state",
                ));
            }
            _ => {}
        }
    }
    if safefs_manifest_digest(&manifest.entries) != manifest.digest {
        return Err(store_error("retirement manifest digest mismatch"));
    }
    Ok(())
}

fn validate_retirement_identity(
    wire: &RetirementWire,
    project: &ProjectKey,
    transaction: &TransactionId,
) -> Result<(), TransactionError> {
    if wire.schema != 2
        || wire.project_key != project.0
        || wire.transaction_id != transaction.0
        || !wire
            .stable_report_sha256
            .strip_prefix("sha256:")
            .is_some_and(valid_lower_hex_digest)
        || wire.ownership_token != transaction_ownership_token(project, transaction)
    {
        return Err(store_error(
            "retirement checkpoint identity differs from selected transaction",
        ));
    }
    OwnedDirectoryIdentity::from_token(&wire.directory_identity).map_err(|error| {
        store_error(format!("invalid retirement directory identity: {error:#}"))
    })?;
    let manifest: SafefsTreeManifest = wire.manifest.clone().try_into()?;
    validate_safefs_manifest(&manifest)?;
    let order = safefs_cleanup_order(&manifest);
    if wire.completed.len() > order.len()
        || wire
            .completed
            .iter()
            .zip(&order)
            .any(|(actual, expected)| actual != expected)
    {
        return Err(store_error(
            "retirement completion list is not the canonical manifest prefix",
        ));
    }
    if wire.tree_removed {
        if wire.active.is_some() || wire.completed != order {
            return Err(store_error(
                "removed retirement tree has incomplete or active progress",
            ));
        }
        return Ok(());
    }
    if let Some(active) = &wire.active {
        if wire.completed.len() >= order.len() || active.progress_key != order[wire.completed.len()]
        {
            return Err(store_error(
                "retirement active intent is not the exact next manifest entry",
            ));
        }
        if active.root {
            if active.progress_key != "root"
                || active.path != transaction.0
                || !matches!(active.expected.kind, SafefsEntryKindWire::Directory)
            {
                return Err(store_error("retirement root intent has invalid shape"));
            }
        } else {
            let expected = manifest
                .entries
                .iter()
                .find(|entry| safefs_entry_key(entry) == active.progress_key)
                .ok_or_else(|| store_error("retirement active entry is absent from manifest"))?;
            if active.path != expected.path {
                return Err(store_error(
                    "retirement active path differs from manifest entry",
                ));
            }
        }
    }
    Ok(())
}

fn safefs_entry_key(entry: &SafefsTreeEntry) -> String {
    format!(
        "{}:{}",
        match entry.state.kind {
            EntryStateKind::File => "file",
            EntryStateKind::Directory => "directory",
        },
        entry.path
    )
}

fn safefs_cleanup_order(manifest: &SafefsTreeManifest) -> Vec<String> {
    let mut files = manifest
        .entries
        .iter()
        .filter(|entry| entry.state.kind == EntryStateKind::File)
        .map(safefs_entry_key)
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    let mut directories = manifest
        .entries
        .iter()
        .filter(|entry| entry.state.kind == EntryStateKind::Directory)
        .collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        left.path
            .split('/')
            .count()
            .cmp(&right.path.split('/').count())
            .reverse()
            .then_with(|| right.path.as_bytes().cmp(left.path.as_bytes()))
    });
    files.extend(directories.into_iter().map(safefs_entry_key));
    files.push("root".to_owned());
    files
}
