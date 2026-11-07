use super::*;
use std::fs;
fn fixture(rows: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        format!("[project]\nname='ungrouped'\nversion='1.0.0'\n{rows}"),
    )
    .unwrap();
    fs::create_dir_all(root.path().join(vibe_core::layout::current_boot_dir())).unwrap();
    root
}
fn row(name: &str, media: &str, interpretation: &str, loading: &str) -> String {
    let role = if interpretation == "vibe-spec" {
        "spec"
    } else {
        "resource"
    };
    format!(
        "[[project.sources]]\nfile='vibevm/vibespecs/boot/{name}'\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='{loading}'\n"
    )
}
#[test]
fn original_scan_capture_is_narrow_media_driven_and_not_addressed() {
    let rows = row("a.payload", "application/xml", "vibe-spec", "scan")
        + &row("a.xml", "application/xml", "xml-data", "scan")
        + &row("missing.md", "text/markdown", "vibe-spec", "explicit");
    let root = fixture(&rows);
    let boot = root.path().join(vibe_core::layout::current_boot_dir());
    fs::write(boot.join("a.payload"), b"original XML bytes").unwrap();
    fs::write(boot.join("a.xml"), b"\xff#use invalid").unwrap();
    let policy = observe_package_policy(root.path()).unwrap();
    let captured = policy.capture_authored_boot().unwrap();
    assert_eq!(captured.resources().len(), 1);
    assert_eq!(captured.resources()[0].media(), "application/xml");
    assert_eq!(captured.resources()[0].bytes(), b"original XML bytes");
    // Capture owns observation, not parsing; the caller validates native syntax.
    policy.recheck_authored_boot(&captured).unwrap();
}
#[test]
fn immediate_scope_excludes_subdirectories_generated_and_raw_but_tracks_new_native() {
    let root = fixture(
        "[[project.sources]]\ndirectory='vibevm/vibespecs/boot'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n",
    );
    let boot = root.path().join(vibe_core::layout::current_boot_dir());
    fs::create_dir(boot.join("nested")).unwrap();
    fs::write(boot.join("nested/bad.xml"), b"bad").unwrap();
    let policy = observe_package_policy(root.path()).unwrap();
    let captured = policy.capture_authored_boot().unwrap();
    for name in [
        "STATIC.xml",
        "STATIC.md",
        "INDEX.md",
        ".vibe-boot-artifacts.lock",
    ] {
        fs::write(boot.join(name), b"generated").unwrap();
    }
    policy.recheck_authored_boot(&captured).unwrap();
    fs::write(boot.join("new.md"), b"# New").unwrap();
    assert!(policy.recheck_authored_boot(&captured).is_err());
}
#[test]
fn promised_native_absence_and_actual_native_claim_collision_refuse() {
    let rows = row("a.md", "text/markdown", "vibe-spec", "scan")
        + &row("a.xml", "application/xml", "vibe-spec", "scan");
    let root = fixture(&rows);
    let policy = observe_package_policy(root.path()).unwrap();
    assert!(
        policy
            .capture_authored_boot()
            .unwrap_err()
            .to_string()
            .contains("absent")
    );
    let boot = root.path().join(vibe_core::layout::current_boot_dir());
    fs::write(boot.join("a.md"), b"# A").unwrap();
    fs::write(boot.join("a.xml"), b"<spec/>").unwrap();
    assert!(
        policy
            .capture_authored_boot()
            .unwrap_err()
            .to_string()
            .contains("claim")
    );
}
#[cfg(unix)]
#[test]
fn selected_shared_inode_allowed_and_selected_symlink_refused() {
    let root = fixture(&row("a.md", "text/markdown", "vibe-spec", "scan"));
    let path = root
        .path()
        .join(vibe_core::layout::current_boot_dir())
        .join("a.md");
    fs::write(root.path().join("shared"), b"# shared").unwrap();
    fs::hard_link(root.path().join("shared"), &path).unwrap();
    let policy = observe_package_policy(root.path()).unwrap();
    policy.capture_authored_boot().unwrap();
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(root.path().join("shared"), &path).unwrap();
    assert!(policy.capture_authored_boot().is_err());
}
#[test]
fn ambiguity_outside_automatic_instruction_loading_does_not_poison_boot() {
    for loading in ["scan", "explicit"] {
        let interpretation = if loading == "scan" {
            "commonmark"
        } else {
            "vibe-spec"
        };
        let rows = row("ignored.md", "text/markdown", interpretation, loading)
            + &row("ignored.md", "text/markdown", "commonmark", "explicit");
        let root = fixture(&rows);
        let path = root
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join("ignored.md");
        fs::write(path, b"\xffnot parsed").unwrap();
        assert!(
            observe_package_policy(root.path())
                .unwrap()
                .capture_authored_boot()
                .unwrap()
                .resources()
                .is_empty()
        );
    }
}
