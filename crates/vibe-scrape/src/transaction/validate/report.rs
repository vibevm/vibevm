fn validate_verification(value: &Journal) -> Result<(), TransactionError> {
    let mut seen = BTreeSet::new();
    for record in &value.verification {
        digest(&record.evidence_sha256)?;
        if record.evidence_sha256 != bytes_digest(&record.evidence.canonical_evidence) {
            return corrupt("verification evidence digest mismatch");
        }
        if !seen.insert(record.phase) {
            return corrupt("verification phase is recorded more than once");
        }
    }
    let expected = match value.mode {
        TransactionMode::Export => vec![
            VerificationPhase::Before,
            VerificationPhase::FinalResidual,
            VerificationPhase::AfterHealth,
            VerificationPhase::FinalTree,
            VerificationPhase::SourceUnchanged,
        ],
        TransactionMode::InPlace => vec![
            VerificationPhase::Before,
            VerificationPhase::PreContractResidual,
            VerificationPhase::FinalResidual,
            VerificationPhase::AfterHealth,
            VerificationPhase::FinalTree,
        ],
    };
    let actual = value
        .verification
        .iter()
        .map(|record| record.phase)
        .collect::<Vec<_>>();
    if !expected.starts_with(&actual) {
        return corrupt("verification records are not a canonical phase prefix");
    }
    let verified_commit = matches!(
        value.state,
        TransactionState::Verified | TransactionState::CleanupPending
    ) || (value.state == TransactionState::Complete
        && value
            .report
            .as_ref()
            .is_some_and(|report| report.outcome == Outcome::Verified));
    if verified_commit
        && (actual != expected
            || value
                .verification
                .iter()
                .any(|record| !record.evidence.accepted))
    {
        return corrupt(
            "verified transaction lacks the exact complete accepted commit-gating verification set",
        );
    }
    Ok(())
}

fn validate_report(value: &Journal) -> Result<(), TransactionError> {
    let Some(report) = &value.report else {
        if matches!(
            value.state,
            TransactionState::Complete | TransactionState::RollbackFailed
        ) {
            return corrupt("terminal journal has no report");
        }
        return Ok(());
    };
    if report.project_key != value.project_key
        || report.transaction_id != value.transaction_id
        || report.plan_id != value.plan_id
        || report.mode != value.mode
        || report.snapshots != value.snapshots
        || report.verification != value.verification
        || report.actual_mutations != value.actual_mutations
    {
        return corrupt("report identity/evidence differs from journal");
    }
    let expected_before = match &value.execution {
        PreparedMode::Export(plan) => &plan.source_tree.digest,
        PreparedMode::InPlace(plan) => &plan.before_tree.digest,
    };
    if report.before_tree.as_ref() != Some(expected_before) {
        return corrupt("report before tree differs from executable plan");
    }
    let expected_after = match (report.outcome, &value.execution) {
        (Outcome::Refused, PreparedMode::Export(_)) => None,
        (Outcome::Refused, PreparedMode::InPlace(plan)) => Some(&plan.before_tree.digest),
        (Outcome::Verified | Outcome::RolledBack | Outcome::RollbackFailed, _) => {
            value.delivered_tree.as_ref()
        }
    };
    if report.after_tree.as_ref() != expected_after {
        return corrupt("report after tree differs from its outcome direction");
    }
    let expected_assurance = if cfg!(windows) && report.outcome == Outcome::Verified
        || value
            .verification
            .iter()
            .any(|record| record.evidence.assurance == Assurance::Reduced)
    {
        Assurance::Reduced
    } else {
        Assurance::Full
    };
    if report.assurance != expected_assurance {
        return corrupt("report assurance differs from verification evidence");
    }
    if value.state == TransactionState::Complete && report.cleanup != Cleanup::Complete {
        return corrupt("complete journal has a cleanup-pending report");
    }
    let planned = value
        .mutation_progress
        .iter()
        .map(|progress| PlannedMutationEvidence {
            id: progress.id.clone(),
            kind: progress.kind,
        })
        .collect::<Vec<_>>();
    if report.planned_mutations != planned {
        return corrupt("report planned mutations differ from journal");
    }
    match (&value.state, report.outcome) {
        (TransactionState::Verified | TransactionState::CleanupPending, Outcome::Verified)
        | (TransactionState::RollbackFailed, Outcome::RollbackFailed)
        | (TransactionState::RolledBack, Outcome::RolledBack)
        | (TransactionState::Complete, _) => Ok(()),
        _ => corrupt("report outcome is inconsistent with journal state"),
    }
}

fn state_for_mode(mode: TransactionMode, state: &TransactionState) -> Result<(), TransactionError> {
    let legal = match mode {
        TransactionMode::Export => !matches!(
            state,
            TransactionState::BeforePassed
                | TransactionState::Mutating
                | TransactionState::ContractBoundary(_)
        ),
        TransactionMode::InPlace => !matches!(
            state,
            TransactionState::Candidate | TransactionState::PublishedPendingVerify
        ),
    };
    if legal {
        Ok(())
    } else {
        corrupt("journal state is illegal for its mode")
    }
}

fn canonical_tree(tree: &TreeManifest) -> Result<(), TransactionError> {
    digest(&tree.digest)?;
    let mut previous: Option<&str> = None;
    for entry in &tree.entries {
        path(&entry.path)?;
        if is_git(&entry.path) {
            return invalid("a sealed tree contains protected .git metadata");
        }
        if previous.is_some_and(|prior| prior.as_bytes() >= entry.path.as_bytes()) {
            return invalid("tree manifest entries must be unique and byte-sorted");
        }
        previous = Some(&entry.path);
        match entry.kind {
            TreeEntryKind::File if entry.sha256.is_none() || entry.bytes.is_none() => {
                return invalid("manifest file lacks digest or byte count");
            }
            TreeEntryKind::Directory if entry.sha256.is_some() || entry.bytes.is_some() => {
                return invalid("manifest directory carries file evidence");
            }
            _ => {}
        }
        if let Some(value) = &entry.sha256 {
            digest(value)?;
        }
    }
    Ok(())
}

fn state(value: &PathState) -> Result<(), TransactionError> {
    match value {
        PathState::File(file) => file_state(file),
        PathState::Tree(tree) => canonical_subtree(tree),
        PathState::Absent | PathState::EmptyDirectory { .. } => Ok(()),
    }
}

fn canonical_subtree(tree: &SubtreeState) -> Result<(), TransactionError> {
    digest(&tree.digest)?;
    let mut previous: Option<&str> = None;
    for entry in &tree.descendants {
        path(&entry.relative_path)?;
        if previous.is_some_and(|prior| prior.as_bytes() >= entry.relative_path.as_bytes()) {
            return invalid("subtree descendants must be unique and byte-sorted");
        }
        previous = Some(&entry.relative_path);
        match entry.kind {
            TreeEntryKind::File if entry.sha256.is_none() || entry.bytes.is_none() => {
                return invalid("subtree file lacks digest or byte count");
            }
            TreeEntryKind::Directory if entry.sha256.is_some() || entry.bytes.is_some() => {
                return invalid("subtree directory carries file evidence");
            }
            _ => {}
        }
        if let Some(value) = &entry.sha256 {
            digest(value)?;
        }
    }
    Ok(())
}

fn file_state(value: &FileState) -> Result<(), TransactionError> {
    digest(&value.sha256)
}

fn entry_matches_file(entry: &TreeEntry, state: &FileState) -> bool {
    entry.kind == TreeEntryKind::File
        && entry.sha256.as_ref() == Some(&state.sha256)
        && entry.bytes == Some(state.bytes)
        && entry.mode == state.mode
}

fn digest(value: &Digest) -> Result<(), TransactionError> {
    let Some(hex) = value.0.strip_prefix("sha256:") else {
        return invalid("digest must use sha256:<64-lowercase-hex>");
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return invalid("digest must use sha256:<64-lowercase-hex>");
    }
    Ok(())
}

fn transaction_id(value: &TransactionId) -> Result<(), TransactionError> {
    if !(6..=64).contains(&value.0.len())
        || !value.0.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return corrupt("transaction id is outside 6..64 ASCII alphanumerics");
    }
    Ok(())
}

fn path(value: &str) -> Result<(), TransactionError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains(['\\', ':', '\0'])
        || value
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return invalid(format!("non-portable transaction path `{value}`"));
    }
    Ok(())
}

fn token(value: &str, label: &str) -> Result<(), TransactionError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
    {
        return invalid(format!("{label} is not a portable token"));
    }
    Ok(())
}

fn ownership_token(journal: &Journal, role: &str) -> String {
    let material = format!(
        "vibe-scrape-owner-e1\0{}\0{}\0{role}",
        journal.project_key.0, journal.transaction_id.0
    );
    bytes_digest(material.as_bytes()).0
}

fn path_depth(path: &str) -> usize {
    path.split('/').count()
}

fn at_or_below(path: &str, root: &str) -> bool {
    path == root || path.starts_with(&(root.to_owned() + "/"))
}

fn is_git(value: &str) -> bool {
    value == ".git" || value.starts_with(".git/")
}

fn invalid_error(message: impl Into<String>) -> TransactionError {
    TransactionError::InvalidPrepared(message.into())
}

fn invalid<T>(message: impl Into<String>) -> Result<T, TransactionError> {
    Err(invalid_error(message))
}

fn corrupt<T>(message: impl Into<String>) -> Result<T, TransactionError> {
    Err(TransactionError::Store(format!(
        "invalid transaction journal: {}",
        message.into()
    )))
}
