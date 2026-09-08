use super::*;

pub(super) fn create_phase_directory(
    workspace: &tx::VerificationWorkspace,
    parent: &str,
    phase: &str,
) -> Result<PhaseDirectory, tx::TransactionError> {
    let project =
        vibe_safefs::Project::open(Path::new(&workspace.intent.display_root)).map_err(|error| {
            tx::TransactionError::Verification(format!(
                "opening journaled verification workspace: {error:#}"
            ))
        })?;
    let actual = project.identity_token().map_err(|error| {
        tx::TransactionError::Verification(format!(
            "identifying journaled verification workspace: {error:#}"
        ))
    })?;
    if actual != workspace.project_identity_token {
        return Err(tx::TransactionError::ThirdState(
            "verification workspace identity differs from the journal".to_owned(),
        ));
    }
    let root = project.root_dir().map_err(|error| {
        tx::TransactionError::Verification(format!("pinning verification workspace: {error:#}"))
    })?;
    let name = match (parent, phase) {
        ("view", "before") => "vb",
        ("view", "after") => "va",
        ("scratch", "before") => "sb",
        ("scratch", "after") => "sa",
        _ => {
            return Err(tx::TransactionError::Verification(
                "unknown verification workspace phase".to_owned(),
            ));
        }
    };
    let (directory, durability) = root
        .create_child_exclusive_journaled(name)
        .map_err(|error| {
            tx::TransactionError::Verification(format!(
                "exclusively creating verification phase directory: {error}"
            ))
        })?;
    if !matches!(
        durability,
        vibe_safefs::DirectoryDurability::Synced
            | vibe_safefs::DirectoryDurability::JournalRecoverable
    ) {
        return Err(tx::TransactionError::Verification(format!(
            "verification phase directory lacks namespace recovery evidence: {durability:?}"
        )));
    }
    Ok(PhaseDirectory {
        path: directory.path().to_path_buf(),
        _capability: directory,
    })
}

pub(super) fn materialize_exact_tree(
    destination_root: &Path,
    source_root: &str,
    expected: &tx::TreeManifest,
) -> Result<(), tx::TransactionError> {
    let source = vibe_safefs::Project::open(Path::new(source_root))
        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?;
    let destination = vibe_safefs::Project::open(destination_root)
        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?;
    for entry in &expected.entries {
        match entry.kind {
            tx::TreeEntryKind::Directory => {
                let components = entry.path.split('/').collect::<Vec<_>>();
                destination
                    .dir(&components, true)
                    .map_err(|error| tx::TransactionError::Verification(error.to_string()))?;
            }
            tx::TreeEntryKind::File => {
                let digest = entry.sha256.as_ref().ok_or_else(|| {
                    tx::TransactionError::Verification("sealed file has no digest".into())
                })?;
                source
                    .copy_stable_file_to_expected(
                        &entry.path,
                        &destination,
                        &entry.path,
                        entry.mode,
                        digest.0.strip_prefix("sha256:").unwrap_or(&digest.0),
                        entry.bytes.unwrap_or(0),
                    )
                    .map_err(|error| {
                        tx::TransactionError::Verification(format!(
                            "materializing `{}`: {error}",
                            entry.path
                        ))
                    })?;
            }
        }
    }
    Ok(())
}

pub(super) fn before_accepted(
    policy: health::BaselinePolicy,
    result: &health::PhaseHealthResult,
) -> bool {
    match policy {
        health::BaselinePolicy::Strict => result.checks.iter().all(|check| match &check.state {
            CheckState::Skipped { .. } | CheckState::Completed(HealthVerdict::Pass) => true,
            CheckState::Completed(HealthVerdict::Structured(value)) => {
                value.status == HealthStatus::Pass
            }
        }),
        health::BaselinePolicy::NoRegression => result.checks.iter().any(|check| {
            matches!(
                check.state,
                CheckState::Completed(HealthVerdict::Structured(_))
            )
        }),
    }
}

pub(super) fn proof_evidence(
    phase: tx::VerificationPhase,
    tree: &tx::TreeManifest,
) -> tx::VerificationEvidence {
    let canonical_evidence =
        format!("tree-proof/e1\nphase={phase:?}\ntree={}\n", tree.digest.0).into_bytes();
    tx::VerificationEvidence {
        accepted: true,
        assurance: tx::Assurance::Full,
        summary: format!("sealed tree proof accepted for {phase:?}"),
        canonical_evidence,
    }
}

pub(super) fn seal(tree: &tx::TreeManifest) -> health::tree::TreeSeal {
    health::tree::TreeSeal {
        tree_digest: tree.digest.0.clone(),
        entries: tree
            .entries
            .iter()
            .map(|entry| health::tree::TreeSealEntry {
                path: entry.path.clone(),
                kind: match entry.kind {
                    tx::TreeEntryKind::File => health::tree::TreeEntryKind::File,
                    tx::TreeEntryKind::Directory => health::tree::TreeEntryKind::Directory,
                },
                sha256: entry.sha256.as_ref().map(|digest| digest.0.clone()),
                bytes: entry.bytes,
                mode: entry.mode,
            })
            .collect(),
    }
}

pub(super) fn observe(root: &Path) -> Result<tx::TreeManifest, tx::TransactionError> {
    let seal = health::tree::observe(root)
        .map_err(|error| tx::TransactionError::Verification(error.to_string()))?;
    Ok(tx::TreeManifest {
        digest: tx::Digest(seal.tree_digest),
        entries: seal
            .entries
            .into_iter()
            .map(|entry| tx::TreeEntry {
                path: entry.path,
                kind: match entry.kind {
                    health::tree::TreeEntryKind::File => tx::TreeEntryKind::File,
                    health::tree::TreeEntryKind::Directory => tx::TreeEntryKind::Directory,
                },
                sha256: entry.sha256.map(tx::Digest),
                bytes: entry.bytes,
                mode: entry.mode,
            })
            .collect(),
    })
}
