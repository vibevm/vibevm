//! Finite membership, held input and origin boundaries of original inventory.
use super::*;
use std::fs;
const COORD: &str = "org.example/demo";
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"),"[project]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\n[[project.sources]]\ndirectory='src'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n").unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(root.path().join("src/a"), b"one").unwrap();
    root
}
#[test]
fn finite_inventory_rechecks_membership_policy_bytes_parent_and_materialized_role() {
    for change in [
        "membership",
        "bytes",
        "replace",
        "parent",
        "manifest",
        "record",
    ] {
        let root = fixture();
        let policy = observe_package_policy(root.path()).unwrap();
        let captured = policy.capture_host_inventory(COORD).unwrap();
        match change {
            "membership" => fs::write(root.path().join("src/b"), b"two").unwrap(),
            "bytes" => fs::write(root.path().join("src/a"), b"changed").unwrap(),
            "replace" => {
                fs::rename(root.path().join("src/a"), root.path().join("old-a")).unwrap();
                fs::write(root.path().join("src/a"), b"one").unwrap();
            }
            "parent" => {
                fs::rename(root.path().join("src"), root.path().join("old-src")).unwrap();
                fs::create_dir(root.path().join("src")).unwrap();
                fs::hard_link(root.path().join("old-src/a"), root.path().join("src/a")).unwrap();
            }
            "manifest" => {
                let path = root.path().join("vibe.toml");
                fs::write(&path, fs::read_to_string(&path).unwrap() + "\n# drift\n").unwrap();
            }
            _ => fs::write(
                root.path().join(vibe_core::layout::SLOT_RECORD_FILENAME),
                b"not decoded",
            )
            .unwrap(),
        }
        assert!(
            policy.recheck_host_inventory(&captured).is_err(),
            "{change}"
        );
    }
}
#[test]
fn absent_default_is_empty_and_exact_absence_is_unavailable() {
    let root = fixture();
    fs::remove_file(root.path().join("src/a")).unwrap();
    fs::remove_dir(root.path().join("src")).unwrap();
    let policy = observe_package_policy(root.path()).unwrap();
    let captured = policy.capture_host_inventory(COORD).unwrap();
    assert!(captured.resources().is_empty());
    fs::create_dir(root.path().join("src")).unwrap();
    assert!(policy.recheck_host_inventory(&captured).is_err());
    let path = root.path().join("vibe.toml");
    fs::write(&path,fs::read_to_string(&path).unwrap()+"\n[[project.sources]]\nfile='absent'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n").unwrap();
    assert!(
        observe_package_policy(root.path())
            .unwrap()
            .capture_host_inventory(COORD)
            .is_err()
    );
}

#[test]
fn native_scope_is_retained_without_broadening_or_changing_all_resource_inventory() {
    let root = fixture();
    let path = root.path().join("vibe.toml");
    fs::write(&path, format!("{}\n[[project.sources]]\nfile='src/page.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[[project.sources]]\nfile='absent'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n", fs::read_to_string(&path).unwrap())).unwrap();
    fs::write(
        root.path().join("src/page.payload"),
        "selected native bytes",
    )
    .unwrap();
    let policy = observe_package_policy(root.path()).unwrap();
    assert!(policy.capture_host_inventory(COORD).is_err());
    assert!(
        policy
            .capture_native_inventory(COORD, SourceInterpretation::Opaque)
            .is_err()
    );
    assert!(
        policy
            .capture_native_inventory(COORD, SourceInterpretation::VibeDoc)
            .is_err()
    );
    let captured = policy
        .capture_native_inventory(COORD, SourceInterpretation::VibeSpec)
        .unwrap();
    assert_eq!(captured.resources().len(), 1);
    assert_eq!(captured.resources()[0].relative(), "src/page.payload");
    fs::write(root.path().join("src/a"), "ordinary changed").unwrap();
    policy.recheck_host_inventory(&captured).unwrap();
    fs::write(root.path().join("src/page.payload"), "native changed").unwrap();
    assert!(policy.recheck_host_inventory(&captured).is_err());
}
#[test]
fn generated_ownership_and_git_control_are_excluded_but_not_other_dot_files() {
    let root = fixture();
    let path = root.path().join("vibe.toml");
    fs::write(&path,fs::read_to_string(&path).unwrap()+&format!("\n[[project.sources]]\ndirectory='{}'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='scan'\n",vibe_core::layout::current_specs_root().display())).unwrap();
    fs::create_dir_all(root.path().join(vibe_core::layout::current_boot_dir())).unwrap();
    fs::write(
        root.path()
            .join(vibe_core::layout::current_boot_static_xml()),
        b"excluded",
    )
    .unwrap();
    fs::write(
        root.path().join(vibe_core::layout::current_boot_index()),
        b"excluded",
    )
    .unwrap();
    fs::create_dir(root.path().join("src/.git")).unwrap();
    fs::write(root.path().join("src/.git/config"), b"excluded").unwrap();
    fs::write(root.path().join("src/.gitignore"), b"included").unwrap();
    fs::create_dir(root.path().join("src/.github")).unwrap();
    fs::write(root.path().join("src/.github/a"), b"included").unwrap();
    let captured = observe_package_policy(root.path())
        .unwrap()
        .capture_host_inventory(COORD)
        .unwrap();
    assert_eq!(
        captured
            .resources()
            .iter()
            .map(|resource| resource.relative())
            .collect::<Vec<_>>(),
        vec!["src/.github/a", "src/.gitignore", "src/a"]
    );
}
#[cfg(unix)]
#[test]
fn shared_regular_inputs_are_allowed_but_symlinks_specials_and_rewritten_names_refuse() {
    use std::os::unix::fs::symlink;
    let root = fixture();
    fs::hard_link(root.path().join("src/a"), root.path().join("shared")).unwrap();
    let policy = observe_package_policy(root.path()).unwrap();
    assert!(policy.capture_host_inventory(COORD).is_ok());
    symlink(root.path().join("shared"), root.path().join("src/link")).unwrap();
    assert!(policy.capture_host_inventory(COORD).is_err());
    fs::remove_file(root.path().join("src/link")).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(root.path().join("src/fifo"))
            .status()
            .unwrap()
            .success()
    );
    assert!(policy.capture_host_inventory(COORD).is_err());
    fs::remove_file(root.path().join("src/fifo")).unwrap();
    fs::write(root.path().join("src/literal\\name"), b"not rewritten").unwrap();
    assert!(policy.capture_host_inventory(COORD).is_err());
}
