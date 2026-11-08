//! Characterization of managed edits and truthful read-only profile observations.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3);
use super::*;
use crate::commands::vvm::env::{EnvPersister, Persisted, RcFilePersister, RegistryValueKind};
use std::fs;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn shared_target_mappings_do_not_claim_login_shell_effectiveness() {
    let root = tempfile::tempdir().unwrap();
    for (shell, relative) in [
        (Shell::Bash, ".bashrc"),
        (Shell::Zsh, ".zshrc"),
        (Shell::Fish, ".config/fish/config.fish"),
        (Shell::Posix, ".profile"),
        (Shell::Pwsh, ".profile"),
    ] {
        let path = shell.rc_path(root.path());
        assert_eq!(path, root.path().join(relative));
        let report = inspect_rc(&path, shell, Path::new("/opt/bin"));
        assert_eq!(report.observation, ObservationStatus::Missing);
        assert_eq!(report.managed_block, BlockStatus::Absent);
        assert_eq!(report.path_entry, PathEntryStatus::Missing);
        assert_eq!(report.login_probe, ProbeStatus::NotRun);
        assert_eq!(report.login_effectiveness, Effectiveness::Unknown);
        assert!(!path.exists(), "inspection creates nothing");
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn edits_preserve_foreign_bom_crlf_prefix_suffix_and_are_idempotent() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(".bashrc");
    let prefix = "\u{feff}# foreign\r\nexport EDITOR=vim\r\n\r\n";
    let suffix = "\r\n# after managed block\r\nforeign=untouched";
    let original = format!(
        "{prefix}{}\r\nexport PATH='/old':$PATH\r\n# retained managed comment\r\n# <<< vibevm (VVM) <<<\r\n{suffix}",
        rc_io::BLOCK_BEGIN
    );
    fs::write(&path, original.as_bytes()).unwrap();
    let persister = RcFilePersister::new(path.clone(), Shell::Bash);
    assert_eq!(
        persister.inspect_path(Path::new("/new/bin")).path_entry,
        PathEntryStatus::Different
    );
    assert_eq!(
        persister.ensure_on_path(Path::new("/new/bin")).unwrap(),
        Persisted::Changed
    );
    persister
        .set_vibevm_home(Path::new("/new/version"))
        .unwrap();
    let once = fs::read(&path).unwrap();
    assert!(once.starts_with(prefix.as_bytes()));
    assert!(once.ends_with(suffix.as_bytes()));
    assert!(
        std::str::from_utf8(&once)
            .unwrap()
            .contains("# retained managed comment\r\n")
    );
    assert_eq!(
        persister.ensure_on_path(Path::new("/new/bin")).unwrap(),
        Persisted::Unchanged
    );
    assert_eq!(
        persister
            .set_vibevm_home(Path::new("/new/version"))
            .unwrap(),
        Persisted::Unchanged
    );
    let report = persister.inspect_path(Path::new("/new/bin"));
    assert_eq!(report.managed_block, BlockStatus::Present);
    assert_eq!(report.path_entry, PathEntryStatus::Correct);
    assert_eq!(fs::read(path).unwrap(), once);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn malformed_markers_and_duplicate_path_assignments_are_not_silently_repaired() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(".bashrc");
    let begin = rc_io::BLOCK_BEGIN;
    let end = "# <<< vibevm (VVM) <<<";
    for text in [
        format!("{begin}\n"),
        format!("{end}\n{begin}\n"),
        format!("{begin}\n{end}\n{begin}\n{end}\n"),
        format!("quoted='{begin}'\n"),
    ] {
        fs::write(&path, &text).unwrap();
        let persister = RcFilePersister::new(path.clone(), Shell::Bash);
        let report = persister.inspect_path(Path::new("/opt/bin"));
        assert_eq!(report.managed_block, BlockStatus::Malformed);
        assert_eq!(report.path_entry, PathEntryStatus::Unknown);
        assert!(persister.ensure_on_path(Path::new("/opt/bin")).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }
    let text =
        format!("{begin}\nexport PATH='/opt/bin':$PATH\nexport PATH='/other':$PATH\n{end}\n");
    fs::write(&path, &text).unwrap();
    let persister = RcFilePersister::new(path.clone(), Shell::Bash);
    assert_eq!(
        persister.inspect_path(Path::new("/opt/bin")).path_entry,
        PathEntryStatus::Ambiguous
    );
    assert!(persister.ensure_on_path(Path::new("/opt/bin")).is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), text);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn nonregular_and_invalid_utf8_are_unavailable_without_mutation() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("directory");
    fs::create_dir(&dir).unwrap();
    assert_eq!(
        inspect_rc(&dir, Shell::Bash, Path::new("/opt/bin")).observation,
        ObservationStatus::Unavailable
    );
    let path = root.path().join(".bashrc");
    fs::write(&path, b"foreign=\xff\n").unwrap();
    let persister = RcFilePersister::new(path.clone(), Shell::Bash);
    assert_eq!(
        persister.inspect_path(Path::new("/opt/bin")).observation,
        ObservationStatus::Unavailable
    );
    assert!(persister.ensure_on_path(Path::new("/opt/bin")).is_err());
    assert_eq!(fs::read(path).unwrap(), b"foreign=\xff\n");
}

#[cfg(unix)]
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn symlink_is_unavailable_and_never_followed_for_inspection_or_repair() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("foreign");
    let path = root.path().join(".bashrc");
    fs::write(&target, b"owner bytes\n").unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();
    let persister = RcFilePersister::new(path.clone(), Shell::Bash);
    assert_eq!(
        persister.inspect_path(Path::new("/opt/bin")).observation,
        ObservationStatus::Unavailable
    );
    assert!(persister.ensure_on_path(Path::new("/opt/bin")).is_err());
    assert!(fs::symlink_metadata(path).unwrap().file_type().is_symlink());
    assert_eq!(fs::read(target).unwrap(), b"owner bytes\n");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn windows_observation_uses_injected_raw_value_without_registry_io() {
    let raw = RawUserVar {
        value: "/managed/bin;/foreign".into(),
        kind: RegistryValueKind::ExpandString,
    };
    let report = inspect_registry(Ok(Some(raw)), Path::new("/managed/bin"));
    assert_eq!(report.target_kind, TargetKind::WindowsRegistry);
    assert_eq!(report.target.as_deref(), Some("HKCU\\Environment\\Path"));
    assert_eq!(report.managed_block, BlockStatus::NotApplicable);
    assert_eq!(report.path_entry, PathEntryStatus::Correct);
    assert_eq!(report.login_effectiveness, Effectiveness::Unknown);
    assert_eq!(
        inspect_registry(Ok(None), Path::new("/managed/bin")).observation,
        ObservationStatus::Missing
    );
    assert_eq!(
        inspect_registry(
            Err(anyhow::anyhow!("injected unreadable value")),
            Path::new("/managed/bin")
        )
        .observation,
        ObservationStatus::Unavailable
    );
}
