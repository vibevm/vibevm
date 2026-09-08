enum HeldParent<'a> {
    Root(&'a Pinned),
    Child(Pinned),
}

impl std::ops::Deref for HeldParent<'_> {
    type Target = Pinned;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Root(root) => root,
            Self::Child(child) => child,
        }
    }
}

fn holder<'a>(
    base: &'a Pinned,
    relative: &str,
    create_parents: bool,
) -> Result<Option<(HeldParent<'a>, String)>, TransactionError> {
    let (parents, name) = vibe_safefs::split_relative(relative)
        .map_err(fs_error("splitting capability-relative path"))?;
    let mut parents = parents.into_iter();
    let Some(first) = parents.next() else {
        return Ok(Some((HeldParent::Root(base), name)));
    };
    let mut directory = match base.open_child_checked(&first) {
        Ok(Some(child)) => child,
        Ok(None) if create_parents => create_parent(base, &first, relative)?,
        Ok(None) => return Ok(None),
        Err(error) => {
            return Err(TransactionError::ThirdState(format!(
                "parent `{first}` for `{relative}` is unsafe: {error:#}"
            )));
        }
    };
    for component in parents {
        match directory.open_child_checked(&component) {
            Ok(Some(child)) => directory = child,
            Ok(None) if create_parents => {
                directory = create_parent(&directory, &component, relative)?;
            }
            Ok(None) => return Ok(None),
            Err(error) => {
                return Err(TransactionError::ThirdState(format!(
                    "parent `{component}` for `{relative}` is unsafe: {error:#}"
                )));
            }
        }
    }
    Ok(Some((HeldParent::Child(directory), name)))
}

fn create_parent(
    parent: &Pinned,
    component: &str,
    relative: &str,
) -> Result<Pinned, TransactionError> {
    match parent.create_child_exclusive(component) {
        Ok(child) => {
            require_namespace_checkpoint(
                parent.sync_directory(),
                &format!("parent created for `{relative}`"),
            )?;
            Ok(child)
        }
        Err(vibe_safefs::ExclusiveChildError::NotCreated(error)) => {
            Err(TransactionError::ThirdState(format!(
                "parent `{component}` for `{relative}` was raced: {error:#}"
            )))
        }
        Err(vibe_safefs::ExclusiveChildError::CreatedNotReopened { path, source }) => {
            Err(TransactionError::ThirdState(format!(
                "created parent `{}` could not be reopened: {source:#}",
                path.display()
            )))
        }
    }
}

fn require_absent(root: &Pinned, relative: &str) -> Result<(), TransactionError> {
    let Some((parent, name)) = holder(root, relative, false)? else {
        return Ok(());
    };
    match parent.inspect_child_state(&name) {
        Ok(None) => Ok(()),
        Ok(Some(_)) => Err(TransactionError::ThirdState(format!(
            "destination `{relative}` is occupied"
        ))),
        Err(error) => Err(TransactionError::ThirdState(format!(
            "destination `{relative}` cannot be inspected: {error:#}"
        ))),
    }
}

fn create_directory_exact(
    project: &SafefsProject,
    root: &Pinned,
    relative: &str,
    mode: Option<u32>,
) -> Result<(), TransactionError> {
    let state = PathState::EmptyDirectory { mode };
    if state_matches(project, root, relative, &state)? {
        return Ok(());
    }
    let (parent, name) = holder(root, relative, true)?
        .ok_or_else(|| TransactionError::Filesystem(format!("parent of `{relative}` is absent")))?;
    if parent
        .inspect_child_state(&name)
        .map_err(fs_error("inspecting directory destination"))?
        .is_some()
    {
        return Err(TransactionError::ThirdState(format!(
            "directory destination `{relative}` is occupied"
        )));
    }
    let (child, durability) = parent
        .create_child_exclusive_journaled(&name)
        .map_err(|error| {
            TransactionError::ThirdState(format!(
                "exclusive directory creation for `{relative}` failed: {error}"
            ))
        })?;
    require_namespace_checkpoint(durability, &format!("parent of `{relative}`"))?;
    if child
        .unix_mode()
        .map_err(fs_error("reading created directory mode"))?
        != mode
    {
        return Err(TransactionError::ThirdState(format!(
            "created directory `{relative}` has an unexpected mode"
        )));
    }
    Ok(())
}

fn remove_empty_directory_exact(
    project: &SafefsProject,
    root: &Pinned,
    relative: &str,
    mode: Option<u32>,
) -> Result<(), TransactionError> {
    if !state_matches(project, root, relative, &PathState::EmptyDirectory { mode })? {
        return Err(TransactionError::ThirdState(format!(
            "directory `{relative}` is not the sealed empty state"
        )));
    }
    let (parent, name) = holder(root, relative, false)?.ok_or_else(|| {
        TransactionError::ThirdState(format!("directory `{relative}` disappeared"))
    })?;
    let actual = parent
        .inspect_child_state(&name)
        .map_err(fs_error("sealing empty directory for removal"))?
        .ok_or_else(|| {
            TransactionError::ThirdState(format!("directory `{relative}` disappeared"))
        })?;
    let durability = parent
        .remove_child_expected(&name, &actual)
        .map_err(map_cleanup)?;
    require_namespace_checkpoint(durability, &format!("parent of `{relative}`"))
}

fn remove_file_exact(
    project: &SafefsProject,
    root: &Pinned,
    relative: &str,
    expected: &FileState,
) -> Result<(), TransactionError> {
    if !state_matches(project, root, relative, &PathState::File(expected.clone()))? {
        return Err(TransactionError::ThirdState(format!(
            "file `{relative}` is not its sealed state"
        )));
    }
    let (parent, name) = holder(root, relative, false)?
        .ok_or_else(|| TransactionError::ThirdState(format!("file `{relative}` disappeared")))?;
    let actual = parent
        .inspect_child_state(&name)
        .map_err(fs_error("sealing file for removal"))?
        .ok_or_else(|| TransactionError::ThirdState(format!("file `{relative}` disappeared")))?;
    let durability = parent
        .remove_child_expected(&name, &actual)
        .map_err(map_cleanup)?;
    require_namespace_checkpoint(durability, &format!("parent of `{relative}`"))
}

#[allow(clippy::too_many_arguments)]
fn rename_exact(
    project: &SafefsProject,
    source_root: &Pinned,
    source_path: &str,
    destination_root: &Pinned,
    destination_path: &str,
    expected: &PathState,
    create_destination_parents: bool,
) -> Result<(), TransactionError> {
    if !state_matches(project, source_root, source_path, expected)? {
        return Err(TransactionError::ThirdState(format!(
            "rename source `{source_path}` is not its sealed state"
        )));
    }
    let (source_parent, source_name) = holder(source_root, source_path, false)?
        .ok_or_else(|| TransactionError::ThirdState(format!("`{source_path}` disappeared")))?;
    let (destination_parent, destination_name) = holder(
        destination_root,
        destination_path,
        create_destination_parents,
    )?
    .ok_or_else(|| {
        TransactionError::ThirdState(format!("parent of `{destination_path}` is absent"))
    })?;
    if destination_parent
        .inspect_child_state(&destination_name)
        .map_err(fs_error("inspecting rename destination"))?
        .is_some()
    {
        return Err(TransactionError::ThirdState(format!(
            "rename destination `{destination_path}` is occupied"
        )));
    }
    let source_state = source_parent
        .inspect_child_state(&source_name)
        .map_err(fs_error("sealing rename source"))?
        .ok_or_else(|| TransactionError::ThirdState(format!("`{source_path}` disappeared")))?;
    let durability = source_parent
        .rename_child_noreplace_to_durable(
            &destination_parent,
            &source_name,
            &destination_name,
            &source_state,
        )
        .map_err(map_rename)?;
    require_namespace_checkpoint(
        durability,
        &format!("journaled rename `{source_path}` -> `{destination_path}`"),
    )
}

fn read_sealed_file(
    project: &SafefsProject,
    relative: &str,
    expected: &FileState,
) -> Result<Vec<u8>, TransactionError> {
    let root = project
        .root_dir()
        .map_err(fs_error("pinning source project"))?;
    read_sealed_file_at(project, &root, relative, expected)
}

fn read_sealed_file_at(
    project: &SafefsProject,
    root: &Pinned,
    relative: &str,
    expected: &FileState,
) -> Result<Vec<u8>, TransactionError> {
    let expected_state = PathState::File(expected.clone());
    if !state_matches(project, root, relative, &expected_state)? {
        return Err(TransactionError::ThirdState(format!(
            "source file `{relative}` differs from its sealed before state"
        )));
    }
    let bytes = project
        .read_file_in(root, relative)
        .map_err(fs_error("reading sealed source file"))?
        .ok_or_else(|| TransactionError::ThirdState(format!("`{relative}` disappeared")))?;
    if digest_bytes(&bytes) != expected.sha256 || bytes.len() as u64 != expected.bytes {
        return Err(TransactionError::ThirdState(format!(
            "source file `{relative}` changed while it was copied"
        )));
    }
    if !state_matches(project, root, relative, &expected_state)? {
        return Err(TransactionError::ThirdState(format!(
            "source file `{relative}` changed during its copy"
        )));
    }
    Ok(bytes)
}

fn state_matches(
    project: &SafefsProject,
    root: &Pinned,
    relative: &str,
    expected: &PathState,
) -> Result<bool, TransactionError> {
    let Some((parent, name)) = holder(root, relative, false)? else {
        return Ok(*expected == PathState::Absent);
    };
    let actual = parent.inspect_child_state(&name).map_err(|error| {
        TransactionError::ThirdState(format!(
            "observing `{relative}` no-follow failed: {error:#}"
        ))
    })?;
    match expected {
        PathState::Absent => Ok(actual.is_none()),
        PathState::File(file) => Ok(actual.is_some_and(|state| {
            state.kind == vibe_safefs::EntryStateKind::File
                && state.sha256.as_deref().map(model_digest).as_ref() == Some(&file.sha256)
                && state.bytes == Some(file.bytes)
                && state.unix_mode == file.mode
        })),
        PathState::EmptyDirectory { mode } => {
            let Some(state) = actual else {
                return Ok(false);
            };
            if state.kind != vibe_safefs::EntryStateKind::Directory || state.unix_mode != *mode {
                return Ok(false);
            }
            let child = parent
                .open_child(&name)
                .map_err(fs_error("opening expected empty directory"))?;
            Ok(project
                .child_names(&child)
                .map_err(fs_error("enumerating expected empty directory"))?
                .is_empty())
        }
        PathState::Tree(tree) => {
            let Some(state) = actual else {
                return Ok(false);
            };
            if state.kind != vibe_safefs::EntryStateKind::Directory
                || state.unix_mode != tree.root_mode
            {
                return Ok(false);
            }
            let child = parent
                .open_child(&name)
                .map_err(fs_error("opening expected subtree"))?;
            let mut descendants = Vec::new();
            collect_subtree(project, &child, "", &mut descendants)?;
            descendants.sort_by(|left, right| {
                left.relative_path
                    .as_bytes()
                    .cmp(right.relative_path.as_bytes())
            });
            Ok(descendants == tree.descendants)
        }
    }
}

fn collect_subtree(
    project: &SafefsProject,
    directory: &Pinned,
    prefix: &str,
    answer: &mut Vec<super::SubtreeEntry>,
) -> Result<(), TransactionError> {
    let mut names = project
        .child_names(directory)
        .map_err(fs_error("enumerating sealed subtree"))?;
    names.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    for name in names {
        let state = directory
            .inspect_child_state(&name)
            .map_err(fs_error("inspecting sealed subtree entry"))?
            .ok_or_else(|| {
                TransactionError::ThirdState("subtree entry vanished during observation".to_owned())
            })?;
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        answer.push(super::SubtreeEntry {
            relative_path: path.clone(),
            kind: match state.kind {
                vibe_safefs::EntryStateKind::File => TreeEntryKind::File,
                vibe_safefs::EntryStateKind::Directory => TreeEntryKind::Directory,
            },
            sha256: state.sha256.as_deref().map(model_digest),
            bytes: state.bytes,
            mode: state.unix_mode,
        });
        if state.kind == vibe_safefs::EntryStateKind::Directory {
            let child = directory
                .open_child(&name)
                .map_err(fs_error("opening sealed subtree directory"))?;
            collect_subtree(project, &child, &path, answer)?;
        }
    }
    Ok(())
}

fn model_tree_at(project: &SafefsProject, root: &Pinned) -> Result<TreeManifest, TransactionError> {
    let mut observed = Vec::new();
    collect_subtree(project, root, "", &mut observed)?;
    Ok(transaction_manifest(
        observed
            .into_iter()
            .map(|entry| TreeEntry {
                path: entry.relative_path,
                kind: entry.kind,
                sha256: entry.sha256,
                bytes: entry.bytes,
                mode: entry.mode,
            })
            .collect(),
    ))
}
