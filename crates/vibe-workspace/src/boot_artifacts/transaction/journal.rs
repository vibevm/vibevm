//! Generated journal wire aliases and complete role algebra.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Component, Path};

pub(super) use vibe_wire::generated::boot_artifact_transaction::{
    BootArtifactTransaction as Journal, BootArtifactTransactionMode as JournalMode,
    Entry as JournalEntry, State as RecordedState,
};

use crate::WorkspaceError;
use crate::safe_file;

use super::lock::LOCK_NAME;
use super::{
    JOURNAL_NAME, JOURNAL_SCHEMA, ROLLBACK_JOURNAL_NAME, io_error, journal_path,
    rollback_journal_path,
};

pub(super) const INDEX_TARGET: &str = "INDEX.md";
pub(super) const STATIC_MD_TARGET: &str = "STATIC.md";
pub(super) const STATIC_XML_TARGET: &str = "STATIC.xml";
pub(super) const STAGE_PREFIX: &str = ".vibe-boot-txn-";
pub(super) const STAGE_SUFFIX: &str = ".stage";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StageRole {
    IndexBefore,
    IndexAfter,
    SelectedBefore,
    SelectedAfter,
    StaleBefore,
    CommitIntent,
    RollbackIntent,
    RedirectClaude,
    RedirectAgents,
    RedirectGemini,
}

impl StageRole {
    fn wire(self) -> &'static str {
        match self {
            StageRole::IndexBefore => "index-before",
            StageRole::IndexAfter => "index-after",
            StageRole::SelectedBefore => "selected-before",
            StageRole::SelectedAfter => "selected-after",
            StageRole::StaleBefore => "stale-before",
            StageRole::CommitIntent => "commit-intent",
            StageRole::RollbackIntent => "rollback-intent",
            StageRole::RedirectClaude => "redirect-claude",
            StageRole::RedirectAgents => "redirect-agents",
            StageRole::RedirectGemini => "redirect-gemini",
        }
    }
}

pub(super) fn stage_name(transaction: &str, role: StageRole) -> String {
    format!("{STAGE_PREFIX}{transaction}-{}{STAGE_SUFFIX}", role.wire())
}

pub(super) fn validate_artifact_roles(
    index: &Path,
    selected: &Path,
    stale: &Path,
) -> Result<(), WorkspaceError> {
    if file_name(index)? != INDEX_TARGET {
        return Err(io_error(
            index,
            "INDEX transaction target must be exactly `INDEX.md`",
        ));
    }
    let selected_name = file_name(selected)?;
    let stale_name = file_name(stale)?;
    let valid_pair = matches!(
        (selected_name.as_str(), stale_name.as_str()),
        (STATIC_MD_TARGET, STATIC_XML_TARGET) | (STATIC_XML_TARGET, STATIC_MD_TARGET)
    );
    if !valid_pair {
        return Err(io_error(
            selected,
            "selected/stale targets must be the distinct `STATIC.md`/`STATIC.xml` pair",
        ));
    }
    Ok(())
}

pub(super) fn validate_journal(
    journal: &Journal,
    path: &Path,
    expected_mode: JournalMode,
) -> Result<(), WorkspaceError> {
    if journal.schema != JOURNAL_SCHEMA {
        return Err(io_error(
            path,
            format!("unsupported journal schema {}", journal.schema),
        ));
    }
    if journal.mode != expected_mode {
        return Err(io_error(
            path,
            "journal mode does not match its intent filename",
        ));
    }
    validate_transaction(&journal.transaction, path)?;
    if journal.index.target != INDEX_TARGET {
        return Err(io_error(
            path,
            "journal INDEX role must target exactly `INDEX.md`",
        ));
    }
    let pair = (
        journal.selected.target.as_str(),
        journal.stale.target.as_str(),
    );
    if !matches!(
        pair,
        (STATIC_MD_TARGET, STATIC_XML_TARGET) | (STATIC_XML_TARGET, STATIC_MD_TARGET)
    ) {
        return Err(io_error(
            path,
            "journal selected/stale roles are not the distinct STATIC pair",
        ));
    }

    validate_state(
        &journal.index.before,
        &journal.transaction,
        StageRole::IndexBefore,
        path,
    )?;
    validate_state(
        &journal.index.after,
        &journal.transaction,
        StageRole::IndexAfter,
        path,
    )?;
    if !journal.index.after.present {
        return Err(io_error(path, "journal INDEX-after state must be present"));
    }
    validate_state(
        &journal.selected.before,
        &journal.transaction,
        StageRole::SelectedBefore,
        path,
    )?;
    validate_state(
        &journal.selected.after,
        &journal.transaction,
        StageRole::SelectedAfter,
        path,
    )?;
    validate_state(
        &journal.stale.before,
        &journal.transaction,
        StageRole::StaleBefore,
        path,
    )?;
    if journal.stale.after.present
        || journal.stale.after.digest.is_some()
        || journal.stale.after.staged.is_some()
    {
        return Err(io_error(path, "journal stale-after state must be absent"));
    }

    let stages = recorded_stage_names(journal);
    let unique = stages.iter().collect::<BTreeSet<_>>();
    if unique.len() != stages.len() {
        return Err(io_error(path, "journal stage names must be unique"));
    }
    let reserved = [
        INDEX_TARGET,
        STATIC_MD_TARGET,
        STATIC_XML_TARGET,
        JOURNAL_NAME,
        ROLLBACK_JOURNAL_NAME,
        LOCK_NAME,
        "00-core.xml",
        "90-user.xml",
        "CLAUDE.md",
        "AGENTS.md",
        "GEMINI.md",
    ];
    if stages
        .iter()
        .any(|stage| reserved.contains(&stage.as_str()))
    {
        return Err(io_error(
            path,
            "journal stage aliases an owned or authored file",
        ));
    }
    Ok(())
}

fn validate_state(
    state: &RecordedState,
    transaction: &str,
    role: StageRole,
    path: &Path,
) -> Result<(), WorkspaceError> {
    if !state.present {
        if state.digest.is_none() && state.staged.is_none() {
            return Ok(());
        }
        return Err(io_error(
            path,
            "absent journal state carries payload fields",
        ));
    }
    let digest = state
        .digest
        .as_deref()
        .ok_or_else(|| io_error(path, "present journal state has no digest"))?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(io_error(
            path,
            "journal digest is not 64 hexadecimal characters",
        ));
    }
    let staged = state
        .staged
        .as_deref()
        .ok_or_else(|| io_error(path, "present journal state has no stage"))?;
    if staged != stage_name(transaction, role) {
        return Err(io_error(
            path,
            format!("journal stage `{staged}` does not match its transaction role"),
        ));
    }
    validate_owned_stage_name(staged, Some(transaction), path)
}

pub(super) fn validate_owned_stage_name(
    name: &str,
    transaction: Option<&str>,
    path: &Path,
) -> Result<(), WorkspaceError> {
    validate_local_name(name, path)?;
    let Some(body) = name
        .strip_prefix(STAGE_PREFIX)
        .and_then(|value| value.strip_suffix(STAGE_SUFFIX))
    else {
        return Err(io_error(
            path,
            format!("stage `{name}` is outside the owned namespace"),
        ));
    };
    let valid_role = [
        StageRole::IndexBefore,
        StageRole::IndexAfter,
        StageRole::SelectedBefore,
        StageRole::SelectedAfter,
        StageRole::StaleBefore,
        StageRole::CommitIntent,
        StageRole::RollbackIntent,
        StageRole::RedirectClaude,
        StageRole::RedirectAgents,
        StageRole::RedirectGemini,
    ]
    .iter()
    .any(|role| body.ends_with(&format!("-{}", role.wire())));
    if !valid_role {
        return Err(io_error(path, format!("stage `{name}` has no owned role")));
    }
    let actual_transaction = owned_stage_transaction(name)
        .ok_or_else(|| io_error(path, format!("stage `{name}` has no transaction identity")))?;
    validate_transaction(actual_transaction, path)?;
    if let Some(transaction) = transaction
        && actual_transaction != transaction
    {
        return Err(io_error(
            path,
            format!("stage `{name}` belongs to another transaction"),
        ));
    }
    Ok(())
}

pub(super) fn owned_stage_transaction(name: &str) -> Option<&str> {
    let body = name
        .strip_prefix(STAGE_PREFIX)?
        .strip_suffix(STAGE_SUFFIX)?;
    let role = [
        StageRole::IndexBefore,
        StageRole::IndexAfter,
        StageRole::SelectedBefore,
        StageRole::SelectedAfter,
        StageRole::StaleBefore,
        StageRole::CommitIntent,
        StageRole::RollbackIntent,
        StageRole::RedirectClaude,
        StageRole::RedirectAgents,
        StageRole::RedirectGemini,
    ]
    .iter()
    .find_map(|role| body.strip_suffix(&format!("-{}", role.wire())))?;
    if role.is_empty() { None } else { Some(role) }
}

pub(super) fn validate_twins(
    commit: &Journal,
    rollback: &Journal,
    path: &Path,
) -> Result<(), WorkspaceError> {
    let mut expected = commit.clone();
    expected.mode = JournalMode::Rollback;
    if &expected == rollback {
        Ok(())
    } else {
        Err(io_error(
            path,
            "commit/rollback intents differ in fields other than mode",
        ))
    }
}

pub(super) fn read_journal(
    path: &Path,
    mode: JournalMode,
) -> Result<Option<Journal>, WorkspaceError> {
    safe_file::preflight_absent_or_regular(path).map_err(|error| io_error(path, error))?;
    let mut file = match safe_file::open_existing_read(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error(path, error)),
    };
    let identity = safe_file::identity(&file).map_err(|error| io_error(path, error))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| io_error(path, error))?;
    safe_file::assert_path_identity(path, identity).map_err(|error| io_error(path, error))?;
    let journal: Journal = toml::from_slice(&bytes).map_err(|error| io_error(path, error))?;
    validate_journal(&journal, path, mode)?;
    Ok(Some(journal))
}

pub(super) fn ensure_journal_matches(
    parent: &Path,
    expected: &Journal,
    mode: JournalMode,
) -> Result<(), WorkspaceError> {
    let path = match mode {
        JournalMode::Commit => journal_path(parent),
        JournalMode::Rollback => rollback_journal_path(parent),
    };
    let current = read_journal(&path, mode.clone())?
        .ok_or_else(|| io_error(&path, "active journal disappeared while lock was held"))?;
    if &current == expected {
        Ok(())
    } else {
        Err(io_error(
            &path,
            "active journal changed while the boot lock was held",
        ))
    }
}

pub(super) fn file_name(path: &Path) -> Result<String, WorkspaceError> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io_error(path, "path has no UTF-8 file name"))?;
    validate_local_name(name, path)?;
    Ok(name.to_string())
}

fn validate_transaction(value: &str, path: &Path) -> Result<(), WorkspaceError> {
    if (6..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        Ok(())
    } else {
        Err(io_error(
            path,
            "transaction identity is not 6..64 ASCII alphanumerics",
        ))
    }
}

fn validate_local_name(name: &str, path: &Path) -> Result<(), WorkspaceError> {
    let mut components = Path::new(name).components();
    if matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none() {
        Ok(())
    } else {
        Err(io_error(
            path,
            format!("journal path `{name}` is not a local file name"),
        ))
    }
}

fn recorded_stage_names(journal: &Journal) -> Vec<String> {
    [
        &journal.index.before,
        &journal.index.after,
        &journal.selected.before,
        &journal.selected.after,
        &journal.stale.before,
    ]
    .iter()
    .filter_map(|state| state.staged.clone())
    .collect()
}
