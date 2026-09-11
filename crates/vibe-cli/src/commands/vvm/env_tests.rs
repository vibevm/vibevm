use super::*;
use crate::commands::vvm::store::{BINARY_NAME, INDEX_BINARY_NAME};
use specmark::verifies;
use std::io::{self, Write};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#activation", r = 2)]
fn shell_detect_and_export_line() {
    assert_eq!(Shell::detect(Some("/usr/bin/zsh")), Shell::Zsh);
    assert_eq!(Shell::detect(Some("/bin/bash")), Shell::Bash);
    assert_eq!(Shell::detect(Some("/usr/local/bin/fish")), Shell::Fish);
    let home = Path::new("/opt/vibevm/versions/branch/main");
    assert!(
        Shell::Bash
            .export_line(home)
            .starts_with("export VIBEVM_SHELL_HOME=")
    );
    assert!(
        Shell::Fish
            .export_line(home)
            .starts_with("set -gx VIBEVM_SHELL_HOME")
    );
    assert!(
        Shell::Pwsh
            .export_line(home)
            .starts_with("$env:VIBEVM_SHELL_HOME")
    );
}

#[test]
fn shell_exports_quote_hostile_paths_as_literals() {
    let hostile = Path::new("/tmp/a b'\"$()`tick");
    assert_eq!(
        Shell::Bash.export_line(hostile),
        "export VIBEVM_SHELL_HOME='/tmp/a b'\"'\"'\"$()`tick'; export VIBEVM_HOME='/tmp/a b'\"'\"'\"$()`tick'"
    );
    assert_eq!(
        Shell::Fish.export_line(hostile),
        "set -gx VIBEVM_SHELL_HOME '/tmp/a b\\'\"$()`tick'; set -gx VIBEVM_HOME '/tmp/a b\\'\"$()`tick'"
    );
    assert_eq!(
        Shell::Pwsh.export_line(hostile),
        "$env:VIBEVM_SHELL_HOME = '/tmp/a b''\"$()`tick'; $env:VIBEVM_HOME = '/tmp/a b''\"$()`tick'"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#activation", r = 2)]
fn shims_read_the_current_pointer() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    write_shims(&store).unwrap();
    let bin_dir = store.shim_dir();
    for (command, binary) in [("vibe", BINARY_NAME), ("vibe-index", INDEX_BINARY_NAME)] {
        let posix = fs::read_to_string(bin_dir.join(command)).unwrap();
        assert!(posix.contains("vibevm/current"), "reads the live pointer");
        assert!(posix.contains("$VIBEVM_SHELL_HOME"));
        assert!(
            posix.find("home=\"$VIBEVM_SHELL_HOME\"").unwrap()
                < posix.find("home=\"$(cat").unwrap()
        );
        assert!(
            posix.contains("$VIBEVM_HOME"),
            "falls back to the advisory env"
        );
        assert!(posix.contains(&format!("bin/{binary}")));
        assert!(posix.contains("vibe self use"));
        assert!(!posix.contains("vibe man use"));
    }
    if cfg!(windows) {
        for (command, binary) in [("vibe", BINARY_NAME), ("vibe-index", INDEX_BINARY_NAME)] {
            let cmd = fs::read_to_string(bin_dir.join(format!("{command}.cmd"))).unwrap();
            assert!(cmd.contains("current"));
            assert!(cmd.contains("%VIBEVM_SHELL_HOME%"));
            assert!(
                cmd.find("set \"VVM_HOME=%VIBEVM_SHELL_HOME%\"").unwrap()
                    < cmd.find("if \"%VVM_HOME%\"==\"\" if exist").unwrap()
            );
            assert!(cmd.contains(&format!("bin\\{binary}")));
            assert!(cmd.contains("vibe self use"));
            assert!(!cmd.contains("vibe man use"));
        }
    }
    assert!(shim_statuses(&store).iter().all(|(_, ok)| *ok));
    fs::write(bin_dir.join("vibe-index"), b"tampered").unwrap();
    let statuses = shim_statuses(&store);
    assert!(statuses.iter().any(|(name, ok)| *name == "vibe" && *ok));
    assert!(
        statuses
            .iter()
            .any(|(name, ok)| *name == "vibe-index" && !*ok)
    );
}

#[test]
fn failed_atomic_shim_write_preserves_the_previous_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("vibe");
    fs::write(&target, b"previous shim").unwrap();
    let error = write_shim_atomic_using(&target, true, |file| {
        file.write_all(b"partial")?;
        Err(io::Error::other("injected write failure"))
    })
    .unwrap_err();
    let error = format!("{error:#}");
    assert!(error.contains("injected write failure"), "{error}");
    assert_eq!(fs::read(&target).unwrap(), b"previous shim");

    write_shim_atomic(&target, b"exact shim", true).unwrap();
    write_shim_atomic(&target, b"exact shim", true).unwrap();
    assert_eq!(fs::read(target).unwrap(), b"exact shim");
}

#[test]
fn atomic_shim_writer_never_deletes_an_unowned_colliding_temp() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("vibe");
    let collision = temp.path().join("preexisting.tmp");
    fs::write(&collision, b"owner bytes").unwrap();
    assert!(write_shim_atomic_at(&destination, &collision, true, |_| Ok(())).is_err());
    assert_eq!(fs::read(collision).unwrap(), b"owner bytes");
    assert!(!destination.exists());
}

#[test]
fn atomic_rc_writer_never_deletes_an_unowned_colliding_temp() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join(".bashrc");
    let collision = temp.path().join("preexisting.tmp");
    fs::write(&collision, b"owner bytes").unwrap();
    assert!(write_rc_atomic_at(&destination, &collision, b"new", None).is_err());
    assert_eq!(fs::read(collision).unwrap(), b"owner bytes");
    assert!(!destination.exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 2)]
fn rc_persister_is_idempotent_and_repoints() {
    let tmp = tempfile::tempdir().unwrap();
    let rc = tmp.path().join(".bashrc");
    fs::write(&rc, "# user's own line\nexport EDITOR=vim\n").unwrap();
    let p = RcFilePersister::new(rc.clone(), Shell::Bash);

    let home_a = Path::new("/opt/vibevm/versions/tag/1.0.0");
    assert_eq!(p.set_vibevm_home(home_a).unwrap(), Persisted::Changed);
    assert_eq!(
        p.ensure_on_path(Path::new("/opt/bin")).unwrap(),
        Persisted::Changed
    );
    assert_eq!(p.set_vibevm_home(home_a).unwrap(), Persisted::Unchanged);
    assert_eq!(
        p.ensure_on_path(Path::new("/opt/bin")).unwrap(),
        Persisted::Unchanged
    );

    let home_b = Path::new("/opt/vibevm/versions/branch/main");
    assert_eq!(p.set_vibevm_home(home_b).unwrap(), Persisted::Changed);
    let text = fs::read_to_string(&rc).unwrap();
    assert_eq!(text.matches("export VIBEVM_HOME=").count(), 1);
    assert_eq!(text.matches(BLOCK_BEGIN).count(), 1);
    assert!(text.contains("branch/main"));
    assert!(!text.contains("tag/1.0.0"));
    assert!(text.contains("export EDITOR=vim"));
    assert!(!text.contains("VIBEVM_SHELL_HOME"));
}

#[test]
fn rc_persister_preserves_bom_crlf_prefix_and_existing_permissions() {
    let tmp = tempfile::tempdir().unwrap();
    let rc = tmp.path().join(".bashrc");
    let original = b"\xef\xbb\xbf# user\r\nexport EDITOR=vim\r\n";
    fs::write(&rc, original).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&rc).unwrap().permissions();
        permissions.set_mode(0o640);
        fs::set_permissions(&rc, permissions).unwrap();
    }
    RcFilePersister::new(rc.clone(), Shell::Bash)
        .ensure_on_path(Path::new("/opt/bin"))
        .unwrap();
    let updated = fs::read(&rc).unwrap();
    assert!(updated.starts_with(original));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&rc).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }
}

#[test]
fn rc_persister_rejects_invalid_utf8_without_clobbering_owner_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let rc = tmp.path().join(".bashrc");
    let owner_bytes = b"owner=\xffbytes\n";
    fs::write(&rc, owner_bytes).unwrap();
    let error = RcFilePersister::new(rc.clone(), Shell::Bash)
        .ensure_on_path(Path::new("/opt/bin"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("reading shell rc"));
    assert_eq!(fs::read(rc).unwrap(), owner_bytes);
}

#[test]
fn rc_persister_quotes_hostile_paths_in_managed_lines() {
    let tmp = tempfile::tempdir().unwrap();
    let hostile = Path::new("/tmp/a b'\"$()`tick");
    let bash_rc = tmp.path().join(".bashrc");
    RcFilePersister::new(bash_rc.clone(), Shell::Bash)
        .set_vibevm_home(hostile)
        .unwrap();
    assert!(
        fs::read_to_string(bash_rc)
            .unwrap()
            .contains("export VIBEVM_HOME='/tmp/a b'\"'\"'\"$()`tick'")
    );

    let fish_rc = tmp.path().join("config.fish");
    RcFilePersister::new(fish_rc.clone(), Shell::Fish)
        .ensure_on_path(hostile)
        .unwrap();
    assert!(
        fs::read_to_string(fish_rc)
            .unwrap()
            .contains("fish_add_path '/tmp/a b\\'\"$()`tick'")
    );
}

#[cfg(unix)]
#[test]
fn rc_persister_rejects_symlink_without_touching_target() {
    use std::os::unix::fs::symlink;
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("real-rc");
    let link = tmp.path().join(".bashrc");
    fs::write(&target, "owner bytes\n").unwrap();
    symlink(&target, &link).unwrap();
    let error = RcFilePersister::new(link, Shell::Bash)
        .ensure_on_path(Path::new("/opt/bin"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("symlink shell rc"));
    assert_eq!(fs::read_to_string(target).unwrap(), "owner bytes\n");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 2)]
fn path_with_prefix_moves_shim_dir_to_front() {
    assert_eq!(
        path_with_prefix(r"C:\u\.cargo\bin;C:\u\opt\bin", r"C:\u\opt\bin").as_deref(),
        Some(r"C:\u\opt\bin;C:\u\.cargo\bin")
    );
    assert_eq!(
        path_with_prefix(r"C:\u\.cargo\bin", r"C:\u\opt\bin").as_deref(),
        Some(r"C:\u\opt\bin;C:\u\.cargo\bin")
    );
    assert!(path_with_prefix(r"C:\u\opt\bin;C:\u\.cargo\bin", r"C:\u\opt\bin").is_none());
    assert_eq!(
        path_with_prefix("", r"C:\u\opt\bin").as_deref(),
        Some(r"C:\u\opt\bin")
    );
}

#[test]
#[cfg(windows)]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 2)]
fn path_core_expands_deduplicates_and_preserves_unrelated_raw_entries() {
    let cwd = Path::new(r"C:\work");
    let lookup = |name: &str| {
        name.eq_ignore_ascii_case("USERPROFILE")
            .then(|| r"C:\Users\Alice".to_string())
    };
    let current = concat!(
        r"%USERPROFILE%\go\bin;",
        r"%USERPROFILE%\.vibe\opt\bin\;",
        r"c:\users\alice\.VIBE\opt\bin;",
        r"D:\Tools"
    );
    let target = r"C:\Users\Alice\.vibe\opt\bin";
    assert_eq!(
        path_with_prefix_core(current, target, cwd, lookup).as_deref(),
        Some(concat!(
            r"C:\Users\Alice\.vibe\opt\bin;",
            r"%USERPROFILE%\go\bin;",
            r"D:\Tools"
        ))
    );
}

#[test]
#[cfg(windows)]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 2)]
fn path_core_compares_full_lexical_paths_and_keeps_expandable_text_raw() {
    let cwd = Path::new(r"C:\work");
    let current = r".\shim\;%USERPROFILE%\go\bin";
    let target = r"C:\work\shim";
    let lookup = |name: &str| {
        name.eq_ignore_ascii_case("USERPROFILE")
            .then(|| r"C:\Users\Alice".to_string())
    };
    assert_eq!(
        path_with_prefix_core(current, target, cwd, lookup).as_deref(),
        Some(r"C:\work\shim;%USERPROFILE%\go\bin")
    );
}

#[test]
fn registry_value_kind_accepts_only_string_forms() {
    assert_eq!(
        RegistryValueKind::parse("String").unwrap(),
        RegistryValueKind::String
    );
    assert_eq!(
        RegistryValueKind::parse("ExpandString").unwrap(),
        RegistryValueKind::ExpandString
    );
    assert!(RegistryValueKind::parse("Binary").is_err());
}
