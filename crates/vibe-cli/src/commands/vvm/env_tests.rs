use super::*;
use specmark::verifies;

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#activation", r = 1)]
fn shell_detect_and_export_line() {
    assert_eq!(Shell::detect(Some("/usr/bin/zsh")), Shell::Zsh);
    assert_eq!(Shell::detect(Some("/bin/bash")), Shell::Bash);
    assert_eq!(Shell::detect(Some("/usr/local/bin/fish")), Shell::Fish);
    let home = Path::new("/opt/vibevm/versions/branch/main");
    assert!(
        Shell::Bash
            .export_line(home)
            .starts_with("export VIBEVM_HOME=")
    );
    assert!(
        Shell::Fish
            .export_line(home)
            .starts_with("set -gx VIBEVM_HOME")
    );
    assert!(
        Shell::Pwsh
            .export_line(home)
            .starts_with("$env:VIBEVM_HOME")
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#activation", r = 1)]
fn shims_read_the_current_pointer() {
    let tmp = tempfile::tempdir().unwrap();
    write_shims(tmp.path()).unwrap();
    let posix = fs::read_to_string(tmp.path().join("vibe")).unwrap();
    assert!(posix.contains("vibevm/current"), "reads the live pointer");
    assert!(
        posix.contains("$VIBEVM_HOME"),
        "falls back to the advisory env"
    );
    assert!(posix.contains(BINARY_NAME));
    assert!(posix.contains("vibe self use"));
    assert!(!posix.contains("vibe man use"));
    if cfg!(windows) {
        let cmd = fs::read_to_string(tmp.path().join("vibe.cmd")).unwrap();
        assert!(cmd.contains("current"));
        assert!(cmd.contains(BINARY_NAME));
        assert!(cmd.contains("vibe self use"));
        assert!(!cmd.contains("vibe man use"));
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 1)]
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
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 1)]
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
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 1)]
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
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 1)]
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
