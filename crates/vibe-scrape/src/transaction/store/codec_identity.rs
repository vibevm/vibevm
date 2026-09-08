fn store_error(message: impl Into<String>) -> TransactionError {
    TransactionError::Store(message.into())
}

fn strict_json_bytes<T: Serialize>(
    value: &T,
    maximum: usize,
    label: &str,
) -> Result<Vec<u8>, TransactionError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| store_error(format!("encoding {label}: {error}")))?;
    if bytes.len() > maximum {
        return Err(store_error(format!(
            "encoded {label} exceeds {maximum} byte bound"
        )));
    }
    Ok(bytes)
}

fn serialize_canonical_plan<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let value = std::str::from_utf8(bytes).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(value)
}

fn deserialize_canonical_plan<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer).map(String::into_bytes)
}

fn strict_json_parse<'a, T: Deserialize<'a>>(
    bytes: &'a [u8],
    label: &str,
) -> Result<T, TransactionError> {
    serde_json::from_slice(bytes)
        .map_err(|error| store_error(format!("invalid strict {label} JSON: {error}")))
}

fn validate_canonical_report_identity(
    journal: &Journal,
    report: &TransactionReport,
    wire: &report_wire::Report,
) -> Result<(), TransactionError> {
    let mode_matches = matches!(
        (report.mode, &wire.mode),
        (TransactionMode::Export, report_wire::ReportMode::Export)
            | (TransactionMode::InPlace, report_wire::ReportMode::InPlace)
    );
    let outcome_matches = matches!(
        (report.outcome, &wire.outcome),
        (Outcome::Verified, report_wire::ReportOutcome::Verified)
            | (Outcome::Refused, report_wire::ReportOutcome::Refused)
            | (Outcome::RolledBack, report_wire::ReportOutcome::RolledBack)
            | (
                Outcome::RollbackFailed,
                report_wire::ReportOutcome::RollbackFailed
            )
    );
    let assurance_matches = matches!(
        (report.assurance, &wire.assurance),
        (Assurance::Full, report_wire::ReportAssurance::Full)
            | (Assurance::Reduced, report_wire::ReportAssurance::Reduced)
    );
    let cleanup_matches = matches!(
        (report.cleanup, &wire.cleanup),
        (Cleanup::Complete, report_wire::ReportCleanup::Complete)
            | (Cleanup::Pending, report_wire::ReportCleanup::Pending)
    );
    let before_tree = report
        .before_tree
        .as_ref()
        .map_or("", |digest| digest.0.as_str());
    if wire.schema != 1
        || !matches!(&wire.command, report_wire::ReportCommand::Scrape)
        || wire.transaction_id != report.transaction_id.0
        || wire.plan_id != report.plan_id.0
        || wire.project_display_root != journal.project_display_root
        || wire.before_tree_digest != before_tree
        || wire.after_tree_digest != report.after_tree.as_ref().map(|digest| digest.0.clone())
        || !mode_matches
        || !outcome_matches
        || !assurance_matches
        || !cleanup_matches
    {
        return Err(store_error(
            "canonical scrape report identity/outcome differs from durable journal evidence",
        ));
    }
    Ok(())
}

fn canonical_report_update_is_legal(old: &report_wire::Report, new: &report_wire::Report) -> bool {
    old.schema == new.schema
        && old.command == new.command
        && old.transaction_id == new.transaction_id
        && old.plan_id == new.plan_id
        && old.project_display_root == new.project_display_root
        && old.mode == new.mode
        && old.outcome == new.outcome
        && old.assurance == new.assurance
        && old.before_tree_digest == new.before_tree_digest
        && old.after_tree_digest == new.after_tree_digest
        && old.deleted_artifacts == new.deleted_artifacts
        && old.dependency_graphs == new.dependency_graphs
        && old.health == new.health
        && old.relocations == new.relocations
        && old.residuals == new.residuals
        && old.rewrites == new.rewrites
        && old.unchanged_files == new.unchanged_files
        && is_prefix(&old.recovery, &new.recovery)
        && is_prefix(&old.rollback, &new.rollback)
        && matches!(
            (&old.cleanup, &new.cleanup),
            (
                report_wire::ReportCleanup::Pending,
                report_wire::ReportCleanup::Pending | report_wire::ReportCleanup::Complete
            )
        )
}

fn validate_project_key(project: &ProjectKey) -> Result<(), TransactionError> {
    let Some(hex) = project.0.strip_prefix("sha256:") else {
        return Err(store_error(
            "project key must use sha256:<64-lowercase-hex>",
        ));
    };
    if !valid_lower_hex_digest(hex) {
        return Err(store_error(
            "project key must use sha256:<64-lowercase-hex>",
        ));
    }
    Ok(())
}

fn project_component(project: &ProjectKey) -> Result<&str, TransactionError> {
    validate_project_key(project)?;
    Ok(&project.0["sha256:".len()..])
}

fn valid_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_transaction_id_text(value: &str) -> bool {
    (6..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn transaction_component(transaction: &TransactionId) -> Result<&str, TransactionError> {
    if !valid_transaction_id_text(&transaction.0) {
        return Err(store_error(
            "transaction id is outside 6..64 ASCII alphanumerics",
        ));
    }
    Ok(&transaction.0)
}

fn transaction_ownership_token(project: &ProjectKey, transaction: &TransactionId) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-scrape-store-owner-e1\0");
    hash.update(project.0.as_bytes());
    hash.update(b"\0");
    hash.update(transaction.0.as_bytes());
    format!("sha256:{:x}", hash.finalize())
}

fn verification_workspace_token(project: &ProjectKey, transaction: &TransactionId) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-scrape-verification-workspace-e1\0");
    hash.update(project.0.as_bytes());
    hash.update(b"\0");
    hash.update(transaction.0.as_bytes());
    format!("sha256:{:x}", hash.finalize())
}

fn journal_intent_sha256(journal: &Journal) -> Result<String, TransactionError> {
    let mut hash = Sha256::new();
    hash.update(b"vibe-scrape-store-journal-intent-e1\0");
    hash_intent_part(&mut hash, &journal.schema.to_be_bytes());
    hash_intent_part(&mut hash, journal.project_key.0.as_bytes());
    hash_intent_part(&mut hash, journal.transaction_id.0.as_bytes());
    hash_intent_part(
        &mut hash,
        &serde_json::to_vec(&journal.mode)
            .map_err(|error| store_error(format!("encoding journal mode intent: {error}")))?,
    );
    hash_intent_part(&mut hash, journal.plan_id.0.as_bytes());
    hash_intent_part(&mut hash, journal.project_display_root.as_bytes());
    hash_intent_part(&mut hash, &journal.canonical_plan);
    hash_intent_part(
        &mut hash,
        &serde_json::to_vec(&journal.verification_workspace).map_err(|error| {
            store_error(format!("encoding verification workspace intent: {error}"))
        })?,
    );
    hash_intent_part(
        &mut hash,
        &serde_json::to_vec(&journal.execution)
            .map_err(|error| store_error(format!("encoding journal execution intent: {error}")))?,
    );
    hash_intent_part(
        &mut hash,
        &serde_json::to_vec(&journal.snapshots)
            .map_err(|error| store_error(format!("encoding journal snapshot intent: {error}")))?,
    );
    Ok(format!("sha256:{:x}", hash.finalize()))
}

fn hash_intent_part(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

fn journal_relative(
    project: &ProjectKey,
    transaction: &TransactionId,
) -> Result<String, TransactionError> {
    Ok(format!(
        "{}/{}/{}/{}",
        TRANSACTIONS_DIRECTORY,
        project_component(project)?,
        transaction_component(transaction)?,
        JOURNAL_FILE
    ))
}

fn snapshot_relative(
    project: &ProjectKey,
    transaction: &TransactionId,
    name: &str,
) -> Result<String, TransactionError> {
    validate_relative(name, "snapshot name")?;
    Ok(format!(
        "{}/{}/{}/{}/{}",
        TRANSACTIONS_DIRECTORY,
        project_component(project)?,
        transaction_component(transaction)?,
        SNAPSHOTS_DIRECTORY,
        name
    ))
}

fn report_relative(transaction: &TransactionId) -> Result<String, TransactionError> {
    Ok(format!(
        "{}/{}.json",
        REPORTS_DIRECTORY,
        transaction_component(transaction)?
    ))
}

fn retirement_name(transaction: &TransactionId) -> Result<String, TransactionError> {
    Ok(format!(
        "{}.retire.json",
        transaction_component(transaction)?
    ))
}

fn retirement_relative(
    project: &ProjectKey,
    transaction: &TransactionId,
) -> Result<String, TransactionError> {
    Ok(format!(
        "{}/{}/{}",
        TRANSACTIONS_DIRECTORY,
        project_component(project)?,
        retirement_name(transaction)?
    ))
}

fn transaction_id_from_retirement_name(name: &str) -> Option<TransactionId> {
    let transaction = name.strip_suffix(".retire.json")?;
    valid_transaction_id_text(transaction).then(|| TransactionId(transaction.to_owned()))
}

fn validate_relative(value: &str, label: &str) -> Result<(), TransactionError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains(['\\', ':', '\0'])
        || value
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(store_error(format!("unsafe {label} `{value}`")));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
    {
        return Err(store_error(format!("non-portable {label} `{value}`")));
    }
    Ok(())
}

fn split_parent(relative: &str) -> Result<(&str, &str), TransactionError> {
    validate_relative(relative, "relative path")?;
    Ok(relative.rsplit_once('/').unwrap_or(("", relative)))
}

fn open_descendant(
    root: &ExternalDirectory,
    relative: &str,
) -> Result<Option<ExternalDirectory>, TransactionError> {
    if relative.is_empty() {
        // ExternalDirectory is intentionally not Clone. Reopening `.` is not
        // admitted, so callers special-case the root parent instead.
        return Err(store_error(
            "internal attempt to reopen an empty directory path",
        ));
    }
    validate_relative(relative, "directory path")?;
    let mut components = relative.split('/');
    let first = components.next().expect("validated nonempty path");
    let Some(mut current) = root
        .open_child(first)
        .map_err(|error| store_error(format!("opening directory `{first}`: {error:#}")))?
    else {
        return Ok(None);
    };
    for component in components {
        let Some(next) = current
            .open_child(component)
            .map_err(|error| store_error(format!("opening directory `{component}`: {error:#}")))?
        else {
            return Ok(None);
        };
        current = next;
    }
    Ok(Some(current))
}

fn require_optional_sync(
    durability: Option<DirectoryDurability>,
    label: &str,
) -> Result<(), TransactionError> {
    if let Some(durability) = durability {
        require_namespace_checkpoint(durability, label)?;
    }
    Ok(())
}

fn require_namespace_checkpoint(
    durability: DirectoryDurability,
    label: &str,
) -> Result<(), TransactionError> {
    if matches!(
        durability,
        DirectoryDurability::Synced | DirectoryDurability::JournalRecoverable
    ) {
        Ok(())
    } else {
        Err(store_error(format!(
            "{label} did not provide usable namespace durability/recovery evidence: {durability:?}"
        )))
    }
}

fn map_owned_create_error(error: OwnedDirectoryCreateError) -> TransactionError {
    match error {
        OwnedDirectoryCreateError::Unsupported => {
            TransactionError::MissingPrimitive(RequiredPrimitive::ExclusivePinnedDirectory)
        }
        other => store_error(format!("creating owned transaction directory: {other}")),
    }
}

fn map_cleanup_error(error: OwnedTreeCleanupError) -> TransactionError {
    match error {
        OwnedTreeCleanupError::Third { detail } => TransactionError::ThirdState(detail),
        OwnedTreeCleanupError::Unsupported => {
            TransactionError::MissingPrimitive(RequiredPrimitive::ExactManifestTreeRemoval)
        }
        OwnedTreeCleanupError::Io(error) => {
            store_error(format!("retiring owned transaction directory: {error:#}"))
        }
    }
}
