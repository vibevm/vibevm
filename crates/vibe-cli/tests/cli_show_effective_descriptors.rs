//! Descriptor-aware compatibility output for `vibe show effective`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;
use serde_json::Value;
use vibe_core::manifest::SpecFormat;
use vibe_core::{ContentHash, Group};

fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn run(user: &UserScratch, root: &Path) -> std::process::Output {
    user.vibe()
        .args(["--json", "show", "effective", "--path"])
        .arg(root)
        .output()
        .unwrap()
}

fn selected_legacy_footprint() -> (tempfile::TempDir, PathBuf, String) {
    let root = tempfile::tempdir().unwrap();
    write(
        &root.path().join("vibe.toml"),
        "[project]\ngroup='org.host'\nname='reader'\nversion='1.1.0'\n",
    );
    let source = tempfile::tempdir().unwrap();
    let specs = vibe_core::layout::current_specs_root();
    let vibe = specs.join("common/guide.xml");
    let raw_md = specs.join("common/guide.md");
    let raw_xml = specs.join("common/data.xml");
    write(&source.path().join(&vibe), b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><RULE fact=\"true\" status=\"spec/done\">selected XML</RULE></p></spec>\n");
    write(
        &source.path().join(&raw_md),
        b"# Literal\n\n@fact:FAKE stays ordinary.\n",
    );
    write(&source.path().join(&raw_xml), b"<data>raw XML</data>\n");
    let slash = |path: &Path| path.to_string_lossy().replace('\\', "/");
    let manifest = format!(
        "[package]\ngroup='org.acme'\nname='lib'\nkind='flow'\nversion='1.1.0'\nformat='normal'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n",
        slash(&vibe),
        slash(&raw_md),
        slash(&raw_xml),
    );
    write(&source.path().join("vibe.toml"), &manifest);
    let group = Group::parse("org.acme").unwrap();
    let version: semver::Version = "1.1.0".parse().unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap();
    vibe_workspace::vibedeps::materialise_with_spec_format(
        root.path(),
        &group,
        "lib",
        &version,
        source.path(),
        vibe_workspace::vibedeps::CopyMode::Copy,
        SpecFormat::Mixed,
        &hash,
    )
    .unwrap();
    let slot = vibe_workspace::vibedeps::slot_abs_path(root.path(), &group, "lib", &version);
    let paths = [&vibe, &raw_md, &raw_xml]
        .iter()
        .map(|relative| slash(&slot.join(relative).strip_prefix(root.path()).unwrap()))
        .collect::<Vec<_>>();
    let lock = format!(
        "[meta]\ngenerated_by='test'\ngenerated_at='2026-10-01T00:00:00Z'\nschema_version={}\n\
         [[package]]\nkind='flow'\nname='lib'\ngroup='org.acme'\nversion='1.1.0'\n\
         source_url='file:///fixture'\ncontent_hash='{}'\nfiles_written=['{}','{}','{}']\n",
        vibe_core::manifest::CURRENT_SCHEMA_VERSION,
        hash.as_str(),
        paths[0],
        paths[1],
        paths[2],
    );
    write(&root.path().join("vibe.lock"), &lock);
    (root, slot, lock)
}

#[test]
fn synthetic_nonempty_lock_projects_only_selected_vibe_output() {
    let (root, slot, _) = selected_legacy_footprint();
    let user = UserScratch::new();
    let output = run(&user, root.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    let sections = payload["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 1, "{sections:?}");
    assert!(
        sections[0]["body"]
            .as_str()
            .unwrap()
            .contains("selected XML")
    );
    assert!(!sections[0]["body"].as_str().unwrap().contains("FAKE"));
    assert_eq!(
        fs::read(
            slot.join(vibe_core::layout::current_specs_root())
                .join("common/guide.md")
        )
        .unwrap(),
        b"# Literal\n\n@fact:FAKE stays ordinary.\n"
    );
}

#[test]
fn synthetic_nonempty_lock_refuses_foreign_hash_drift_and_removed_rows() {
    let (root, slot, lock) = selected_legacy_footprint();
    let user = UserScratch::new();
    write(
        &root.path().join("vibe.lock"),
        lock.replace(&"1".repeat(64), &"2".repeat(64)),
    );
    let foreign = run(&user, root.path());
    assert!(!foreign.status.success());
    assert!(String::from_utf8_lossy(&foreign.stderr).contains("source hash"));
    write(&root.path().join("vibe.lock"), &lock);
    write(
        &slot
            .join(vibe_core::layout::current_specs_root())
            .join("common/guide.xml"),
        b"changed XML\n",
    );
    let drift = run(&user, root.path());
    assert!(!drift.status.success());
    assert!(String::from_utf8_lossy(&drift.stderr).contains("changed bytes"));
    let manifest = fs::read_to_string(slot.join("vibe.toml")).unwrap();
    write(
        &slot.join("vibe.toml"),
        manifest.split("[[package.sources]]").next().unwrap(),
    );
    let removed = run(&user, root.path());
    assert!(!removed.status.success());
    assert!(String::from_utf8_lossy(&removed.stderr).contains("manifest bytes differ"));
}

#[test]
fn historical_package_boot_copy_cannot_bypass_descriptor_slot_proof() {
    let (root, _slot, lock) = selected_legacy_footprint();
    let boot_rel = vibe_core::layout::current_boot_dir().join("10-lib.md");
    write(
        &root.path().join(&boot_rel),
        b"# Unproved copy\n\n@fact:FAKE must not surface.\n",
    );
    let boot_wire = boot_rel.to_string_lossy().replace('\\', "/");
    let lock = lock.replacen(
        "files_written=",
        &format!("boot_snippet='{boot_wire}'\nfiles_written="),
        1,
    );
    write(&root.path().join("vibe.lock"), lock);
    let output = run(&UserScratch::new(), root.path());
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot prove a legacy host boot copy"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn opted_in_host_boot_omits_raw_same_stem_and_xml_data() {
    let root = tempfile::tempdir().unwrap();
    let boot = vibe_core::layout::current_boot_dir();
    let vibe = boot.join("rule.md");
    let raw_md = boot.join("ordinary.md");
    let raw_xml = boot.join("rule.xml");
    let slash = |path: &Path| path.to_string_lossy().replace('\\', "/");
    write(
        &root.path().join("vibe.toml"),
        format!(
            "[project]\ngroup='org.host'\nname='reader'\nversion='1.1.0'\n\
         [[project.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
         [[project.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
         [[project.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n",
            slash(&vibe),
            slash(&raw_md),
            slash(&raw_xml),
        ),
    );
    write(&root.path().join(&vibe), b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><RULE fact=\"true\" status=\"spec/done\">host Vibe XML</RULE></p></spec>\n");
    write(
        &root.path().join(&raw_md),
        b"# Host raw\n\n@fact:FALSE literal.\n",
    );
    write(&root.path().join(&raw_xml), b"<data>host raw</data>\n");
    let user = UserScratch::new();
    let output = run(&user, root.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    let sections = payload["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 1, "{sections:?}");
    assert!(
        sections[0]["body"]
            .as_str()
            .unwrap()
            .contains("host Vibe XML")
    );
    assert!(sections[0]["body"].as_str().unwrap().contains("@fact:RULE"));
    assert!(!sections[0]["body"].as_str().unwrap().contains("<spec"));
    assert_eq!(
        fs::read(root.path().join(raw_xml)).unwrap(),
        b"<data>host raw</data>\n"
    );
}

#[test]
fn opted_in_host_vibe_doc_uses_document_vocabulary() {
    let root = tempfile::tempdir().unwrap();
    let doc = vibe_core::layout::current_boot_dir().join("doc.xml");
    let wire = doc.to_string_lossy().replace('\\', "/");
    write(
        &root.path().join("vibe.toml"),
        format!(
            "[project]\ngroup='org.host'\nname='reader'\nversion='1.1.0'\n\
             [[project.sources]]\nfile='{wire}'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='scan'\n"
        ),
    );
    write(
        &root.path().join(&doc),
        b"<spec xmlns='https://vibevm.org/spec/1'><note kind='tip'>Doc-only host content.</note></spec>",
    );
    let output = run(&UserScratch::new(), root.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    let sections = payload["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 1);
    assert!(
        sections[0]["body"]
            .as_str()
            .unwrap()
            .contains("Doc-only host content.")
    );
}

#[test]
fn old_no_descriptor_footprint_and_current_empty_footprint_keep_legacy_output() {
    let root = tempfile::tempdir().unwrap();
    write(
        &root.path().join("vibe.toml"),
        "[project]\ngroup='org.host'\nname='reader'\nversion='1.1.0'\n",
    );
    let boot = vibe_core::layout::current_boot_dir().join("00-core.md");
    write(&root.path().join(&boot), b"# Host core\n");
    let group = Group::parse("org.acme").unwrap();
    let version: semver::Version = "1.0.1".parse().unwrap();
    let slot = vibe_workspace::vibedeps::slot_abs_path(root.path(), &group, "legacy", &version);
    write(
        &slot.join("vibe.toml"),
        "[package]\ngroup='org.acme'\nname='legacy'\nkind='flow'\nversion='1.0.1'\n",
    );
    let legacy = vibe_core::layout::current_specs_root().join("common/legacy.md");
    write(&slot.join(&legacy), b"# Legacy rule\n");
    let footprint = slot
        .join(&legacy)
        .strip_prefix(root.path())
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let prefix = format!(
        "[meta]\ngenerated_by='test'\ngenerated_at='2026-10-01T00:00:00Z'\nschema_version={}\n\
         [[package]]\nkind='flow'\nname='legacy'\ngroup='org.acme'\nversion='1.0.1'\n\
         source_url='file:///fixture'\ncontent_hash='sha256:{}'\n",
        vibe_core::manifest::CURRENT_SCHEMA_VERSION,
        "3".repeat(64),
    );
    let user = UserScratch::new();
    write(
        &root.path().join("vibe.lock"),
        format!("{prefix}files_written=['{footprint}']\n"),
    );
    let old = run(&user, root.path());
    assert!(
        old.status.success(),
        "{}",
        String::from_utf8_lossy(&old.stderr)
    );
    let old: Value = serde_json::from_slice(&old.stdout).unwrap();
    assert_eq!(old["sections"].as_array().unwrap().len(), 2);
    write(&root.path().join("vibe.lock"), &prefix);
    let current = run(&user, root.path());
    assert!(
        current.status.success(),
        "{}",
        String::from_utf8_lossy(&current.stderr)
    );
    let current: Value = serde_json::from_slice(&current.stdout).unwrap();
    assert_eq!(current["sections"].as_array().unwrap().len(), 1);
    assert!(
        current["sections"][0]["body"]
            .as_str()
            .unwrap()
            .contains("Host core")
    );
}
