use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use tempfile::TempDir;

use super::*;

const TX: &str = "ABCDEF12";
const NEGATIVE_ROLE_CORPUS: &str = include_str!("fixtures/negative-roles.toml");

fn present(parent: &Path, role: StageRole, bytes: &[u8]) -> RecordedState {
    let name = stage_name(TX, role);
    fs::write(parent.join(&name), bytes).unwrap();
    RecordedState {
        present: true,
        digest: Some(bytes_digest(bytes)),
        staged: Some(name),
    }
}

fn valid_commit(parent: &Path) -> Journal {
    fs::create_dir_all(parent).unwrap();
    fs::write(parent.join("INDEX.md"), b"OLD-INDEX").unwrap();
    fs::write(parent.join("STATIC.md"), b"OLD-STATIC").unwrap();
    Journal {
        schema: JOURNAL_SCHEMA,
        transaction: TX.to_string(),
        mode: JournalMode::Commit,
        index: JournalEntry {
            target: "INDEX.md".to_string(),
            before: present(parent, StageRole::IndexBefore, b"OLD-INDEX"),
            after: present(parent, StageRole::IndexAfter, b"NEW-INDEX"),
        },
        selected: JournalEntry {
            target: "STATIC.xml".to_string(),
            before: absent_state(),
            after: present(parent, StageRole::SelectedAfter, b"NEW-STATIC"),
        },
        stale: JournalEntry {
            target: "STATIC.md".to_string(),
            before: present(parent, StageRole::StaleBefore, b"OLD-STATIC"),
            after: absent_state(),
        },
    }
}

fn persist_journal(parent: &Path, journal: &Journal, rollback: bool) {
    let path = if rollback {
        rollback_journal_path(parent)
    } else {
        journal_path(parent)
    };
    fs::write(path, toml::to_string(journal).unwrap()).unwrap();
}

#[test]
fn role_algebra_rejects_authored_targets_and_stage_aliases_before_cleanup() {
    for mutation in 0..5 {
        let temp = TempDir::new().unwrap();
        let authored = temp.path().join("90-user.xml");
        let core = temp.path().join("00-core.xml");
        fs::write(&authored, b"USER").unwrap();
        fs::write(&core, b"CORE").unwrap();
        let mut journal = valid_commit(temp.path());
        match mutation {
            0 => journal.index.target = "00-core.xml".to_string(),
            1 => journal.selected.target = "90-user.xml".to_string(),
            2 => journal.index.after.staged = Some("90-user.xml".to_string()),
            3 => journal.selected.after.staged = journal.index.after.staged.clone(),
            4 => journal.index.after = absent_state(),
            _ => unreachable!(),
        }
        persist_journal(temp.path(), &journal, false);
        assert!(recover_pending(temp.path()).is_err(), "mutation {mutation}");
        assert_eq!(fs::read(&authored).unwrap(), b"USER");
        assert_eq!(fs::read(&core).unwrap(), b"CORE");
        assert_eq!(
            fs::read(temp.path().join("INDEX.md")).unwrap(),
            b"OLD-INDEX"
        );
    }
}

#[test]
fn durable_negative_role_corpus_stays_red() {
    let corpus: toml::Value = toml::from_str(NEGATIVE_ROLE_CORPUS).unwrap();
    let cases = corpus["case"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let mutation = case["mutation"].as_str().unwrap();
        let value = case["value"].as_str().unwrap();
        let temp = TempDir::new().unwrap();
        let mut commit = valid_commit(temp.path());
        let mut rollback = commit.clone();
        rollback.mode = JournalMode::Rollback;
        let result = match mutation {
            "index-target" => {
                commit.index.target = value.to_string();
                validate_journal(&commit, Path::new(JOURNAL_NAME), JournalMode::Commit)
            }
            "selected-target" => {
                commit.selected.target = value.to_string();
                validate_journal(&commit, Path::new(JOURNAL_NAME), JournalMode::Commit)
            }
            "index-after-stage" => {
                commit.index.after.staged = Some(value.to_string());
                validate_journal(&commit, Path::new(JOURNAL_NAME), JournalMode::Commit)
            }
            "duplicate-selected-after-stage" => {
                assert_eq!(value, "index-after");
                commit.selected.after.staged = commit.index.after.staged.clone();
                validate_journal(&commit, Path::new(JOURNAL_NAME), JournalMode::Commit)
            }
            "rollback-index-after-digest" => {
                rollback.index.after.digest = Some(value.to_string());
                validate_twins(&commit, &rollback, Path::new(ROLLBACK_JOURNAL_NAME))
            }
            "index-after-absent" => {
                assert_eq!(value, "absent");
                commit.index.after = absent_state();
                validate_journal(&commit, Path::new(JOURNAL_NAME), JournalMode::Commit)
            }
            other => panic!("unknown negative role mutation {other}"),
        };
        assert!(result.is_err(), "negative case `{id}` became green");
    }
}

#[test]
fn mode_filename_and_complete_twin_equality_are_mandatory() {
    let temp = TempDir::new().unwrap();
    let mut commit = valid_commit(temp.path());
    commit.mode = JournalMode::Rollback;
    persist_journal(temp.path(), &commit, false);
    assert!(recover_pending(temp.path()).is_err());

    let temp = TempDir::new().unwrap();
    let commit = valid_commit(temp.path());
    let mut rollback = commit.clone();
    rollback.mode = JournalMode::Rollback;
    rollback.index.after.digest = Some("f".repeat(64));
    let orphan = temp
        .path()
        .join(stage_name("ORPHAN12", StageRole::IndexAfter));
    fs::write(&orphan, b"AGED-ORPHAN").unwrap();
    fs::File::options()
        .write(true)
        .open(&orphan)
        .unwrap()
        .set_times(
            fs::FileTimes::new()
                .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000)),
        )
        .unwrap();
    persist_journal(temp.path(), &commit, false);
    persist_journal(temp.path(), &rollback, true);
    let error = recover_pending(temp.path()).unwrap_err().to_string();
    assert!(
        error.contains("differ in fields other than mode"),
        "{error}"
    );
    assert_eq!(fs::read(orphan).unwrap(), b"AGED-ORPHAN");
}

#[test]
fn hardlinked_journal_stage_index_and_static_are_refused_without_alias_mutation() {
    for role in ["journal", "stage", "index", "static"] {
        let temp = TempDir::new().unwrap();
        let journal = valid_commit(temp.path());
        persist_journal(temp.path(), &journal, false);
        let target = match role {
            "journal" => journal_path(temp.path()),
            "stage" => temp
                .path()
                .join(journal.index.after.staged.as_ref().unwrap()),
            "index" => temp.path().join("INDEX.md"),
            "static" => temp.path().join("STATIC.md"),
            _ => unreachable!(),
        };
        let alias = temp.path().join(format!("{role}-alias"));
        fs::hard_link(&target, &alias).unwrap();
        let before = fs::read(&alias).unwrap();
        let error = recover_pending(temp.path()).unwrap_err().to_string();
        assert!(error.contains("hardlinks"), "{role}: {error}");
        assert_eq!(fs::read(&alias).unwrap(), before, "{role}");
    }
}

#[test]
fn lock_hardlink_and_nonregular_lock_are_refused() {
    let temp = TempDir::new().unwrap();
    let lock = temp.path().join(lock::LOCK_NAME);
    fs::write(&lock, b"").unwrap();
    fs::hard_link(&lock, temp.path().join("lock-alias")).unwrap();
    assert!(
        recover_pending(temp.path())
            .unwrap_err()
            .to_string()
            .contains("hardlinks")
    );

    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join(lock::LOCK_NAME)).unwrap();
    assert!(
        recover_pending(temp.path())
            .unwrap_err()
            .to_string()
            .contains("non-regular")
    );
}

#[test]
fn nonregular_journal_stage_index_and_static_are_refused() {
    for role in ["journal", "stage", "index", "static"] {
        let temp = TempDir::new().unwrap();
        let journal = valid_commit(temp.path());
        persist_journal(temp.path(), &journal, false);
        let target = match role {
            "journal" => journal_path(temp.path()),
            "stage" => temp
                .path()
                .join(journal.index.after.staged.as_ref().unwrap()),
            "index" => temp.path().join("INDEX.md"),
            "static" => temp.path().join("STATIC.md"),
            _ => unreachable!(),
        };
        fs::remove_file(&target).unwrap();
        fs::create_dir(&target).unwrap();
        let error = recover_pending(temp.path()).unwrap_err().to_string();
        assert!(error.contains("non-regular"), "{role}: {error}");
    }
}

#[test]
fn symbolic_journal_stage_index_and_static_are_refused_when_supported() {
    for role in ["journal", "stage", "index", "static"] {
        let temp = TempDir::new().unwrap();
        let journal = valid_commit(temp.path());
        persist_journal(temp.path(), &journal, false);
        let target = match role {
            "journal" => journal_path(temp.path()),
            "stage" => temp
                .path()
                .join(journal.index.after.staged.as_ref().unwrap()),
            "index" => temp.path().join("INDEX.md"),
            "static" => temp.path().join("STATIC.md"),
            _ => unreachable!(),
        };
        let referent = temp.path().join(format!("{role}-referent"));
        fs::write(&referent, fs::read(&target).unwrap()).unwrap();
        fs::remove_file(&target).unwrap();
        if let Err(error) = make_symlink(&referent, &target) {
            eprintln!("symlink unavailable; skipping symbolic matrix: {error}");
            return;
        }
        let error = recover_pending(temp.path()).unwrap_err().to_string();
        assert!(
            error.contains("symbolic-link") || error.contains("reparse-point"),
            "{role}: {error}"
        );
    }
}

#[cfg(windows)]
#[test]
fn junction_reparse_points_at_every_file_role_are_refused() {
    for role in ["lock", "journal", "stage", "index", "static"] {
        let temp = TempDir::new().unwrap();
        let journal = valid_commit(temp.path());
        persist_journal(temp.path(), &journal, false);
        let link = match role {
            "lock" => temp.path().join(lock::LOCK_NAME),
            "journal" => journal_path(temp.path()),
            "stage" => temp
                .path()
                .join(journal.index.after.staged.as_ref().unwrap()),
            "index" => temp.path().join("INDEX.md"),
            "static" => temp.path().join("STATIC.md"),
            _ => unreachable!(),
        };
        if fs::symlink_metadata(&link).is_ok() {
            fs::remove_file(&link).unwrap();
        }
        let target = temp.path().join(format!("{role}-junction-target"));
        fs::create_dir(&target).unwrap();
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .status()
            .unwrap();
        assert!(status.success());
        let error = recover_pending(temp.path()).unwrap_err().to_string();
        assert!(
            error.contains("reparse-point") || error.contains("non-regular"),
            "{role}: {error}"
        );
    }
}

#[test]
fn dangling_stale_link_is_not_treated_as_absent() {
    let temp = TempDir::new().unwrap();
    let index = temp.path().join("INDEX.md");
    let selected = temp.path().join("STATIC.xml");
    let stale = temp.path().join("STATIC.md");
    fs::write(&index, b"OLD-INDEX").unwrap();
    if let Err(error) = make_symlink(&temp.path().join("missing"), &stale) {
        eprintln!("symlink unavailable; skipping: {error}");
        return;
    }
    let result = write_with_faults(
        ArtifactWrite {
            index_path: &index,
            index_bytes: b"NEW-INDEX",
            static_path: &selected,
            static_bytes: Some(b"NEW-STATIC"),
            stale_path: &stale,
        },
        &NoFault,
    );
    assert!(result.unwrap_err().to_string().contains("symbolic-link"));
    assert!(fs::symlink_metadata(&stale).is_ok());
}

#[test]
fn valid_owned_orphans_age_out_but_authored_and_malformed_names_survive() {
    let temp = TempDir::new().unwrap();
    let orphan = stage_name("ORPHAN12", StageRole::IndexAfter);
    fs::write(temp.path().join(&orphan), b"ORPHAN").unwrap();
    fs::write(temp.path().join("90-user.xml"), b"USER").unwrap();
    let malformed = format!("{STAGE_PREFIX}not-owned{STAGE_SUFFIX}");
    fs::write(temp.path().join(&malformed), b"AUTHORED").unwrap();
    let future = SystemTime::now() + Duration::from_secs(2 * 60 * 60);
    sweep_orphan_stages_at(temp.path(), &Default::default(), future).unwrap();
    assert!(fs::symlink_metadata(temp.path().join(orphan)).is_err());
    assert_eq!(fs::read(temp.path().join("90-user.xml")).unwrap(), b"USER");
    assert_eq!(fs::read(temp.path().join(malformed)).unwrap(), b"AUTHORED");
}

#[test]
fn recent_or_current_transaction_stages_are_not_orphan_swept() {
    let temp = TempDir::new().unwrap();
    let recent = stage_name("RECENT12", StageRole::IndexAfter);
    let current = stage_name("CURRENT1", StageRole::SelectedAfter);
    fs::write(temp.path().join(&recent), b"RECENT").unwrap();
    fs::write(temp.path().join(&current), b"CURRENT").unwrap();
    let current_set = std::collections::BTreeSet::from(["CURRENT1".to_string()]);
    sweep_orphan_stages_at(temp.path(), &current_set, SystemTime::now()).unwrap();
    assert_eq!(fs::read(temp.path().join(recent)).unwrap(), b"RECENT");
    assert_eq!(fs::read(temp.path().join(current)).unwrap(), b"CURRENT");
}

#[test]
fn hardlinked_owned_orphan_is_refused_not_deleted() {
    let temp = TempDir::new().unwrap();
    let orphan = stage_name("ORPHAN12", StageRole::IndexAfter);
    let path = temp.path().join(&orphan);
    let alias = temp.path().join("alias");
    fs::write(&path, b"ORPHAN").unwrap();
    fs::hard_link(&path, &alias).unwrap();
    let future = SystemTime::now() + Duration::from_secs(2 * 60 * 60);
    assert!(sweep_orphan_stages_at(temp.path(), &Default::default(), future).is_err());
    assert_eq!(fs::read(alias).unwrap(), b"ORPHAN");
}

#[test]
fn hardlinked_selector_is_refused_and_the_alias_survives() {
    let temp = TempDir::new().unwrap();
    let selector = temp.path().join("CLAUDE.md");
    fs::write(&selector, b"OLD-SELECTOR").unwrap();
    let alias = temp.path().join("claude-alias");
    fs::hard_link(&selector, &alias).unwrap();
    let error = replace_selector(&selector, b"NEW-SELECTOR", "SAFETY1")
        .unwrap_err()
        .to_string();
    assert!(error.contains("hardlinks"), "{error}");
    assert_eq!(fs::read(&alias).unwrap(), b"OLD-SELECTOR");
    assert_eq!(fs::read(&selector).unwrap(), b"OLD-SELECTOR");
}

#[test]
fn nonregular_selector_is_refused() {
    let temp = TempDir::new().unwrap();
    let selector = temp.path().join("CLAUDE.md");
    fs::create_dir(&selector).unwrap();
    let error = replace_selector(&selector, b"NEW-SELECTOR", "SAFETY1")
        .unwrap_err()
        .to_string();
    assert!(error.contains("non-regular"), "{error}");
    assert!(fs::symlink_metadata(&selector).unwrap().is_dir());
}

#[test]
fn symbolic_selector_is_refused_when_supported() {
    let temp = TempDir::new().unwrap();
    let referent = temp.path().join("claude-referent");
    fs::write(&referent, b"OLD-SELECTOR").unwrap();
    let selector = temp.path().join("CLAUDE.md");
    if let Err(error) = make_symlink(&referent, &selector) {
        eprintln!("symlink unavailable; skipping symbolic selector matrix: {error}");
        return;
    }
    let error = replace_selector(&selector, b"NEW-SELECTOR", "SAFETY1")
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("symbolic-link") || error.contains("reparse-point"),
        "{error}"
    );
    assert!(fs::symlink_metadata(&selector).is_ok());
    assert_eq!(fs::read(&referent).unwrap(), b"OLD-SELECTOR");
}

#[cfg(windows)]
#[test]
fn junction_selector_is_refused() {
    let temp = TempDir::new().unwrap();
    let selector = temp.path().join("CLAUDE.md");
    let target = temp.path().join("claude-junction-target");
    fs::create_dir(&target).unwrap();
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&selector)
        .arg(&target)
        .status()
        .unwrap();
    assert!(status.success());
    let error = replace_selector(&selector, b"NEW-SELECTOR", "SAFETY1")
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("reparse-point") || error.contains("non-regular"),
        "{error}"
    );
    assert!(fs::symlink_metadata(&selector).is_ok());
    assert!(fs::symlink_metadata(&target).unwrap().is_dir());
}

#[cfg(unix)]
fn make_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn make_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, link)
}
