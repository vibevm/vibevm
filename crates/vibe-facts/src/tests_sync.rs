//! Focused host sync tests for descriptor selection and safe reads.

use std::fs;
use tempfile::tempdir;
use vibe_core::layout;

use crate::sync;
use crate::{FactEntry, FactStatus, Registry};

fn status(value: &str) -> FactStatus {
    FactStatus::parse(value).expect("test status")
}

fn facts_rel(rel: &str) -> String {
    layout::current_vibefacts_root()
        .join(rel)
        .to_string_lossy()
        .replace('\\', "/")
}

fn declared_host_fixture(root: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let specs = layout::current_specs_root().join("common");
    let rule = specs.join("RULE.xml");
    let guide_md = specs.join("guide.md");
    let guide_xml = specs.join("guide.xml");
    let forward = |path: &std::path::Path| path.to_string_lossy().replace('\\', "/");
    let manifest = format!(
        "[project]\ngroup='org.example'\nname='demo'\nversion='1.1.0'\n\
         [[project.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
         [[project.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
         [[project.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='scan'\n",
        forward(&rule),
        forward(&guide_md),
        forward(&guide_xml)
    );
    fs::write(root.join("vibe.toml"), manifest).unwrap();
    fs::create_dir_all(root.join(&specs)).unwrap();
    fs::write(root.join(&rule), b"<spec xmlns='https://vibevm.org/spec/1'><p><RULE fact='true' status='impl/done'>real selected fact</RULE></p></spec>\n").unwrap();
    fs::write(
        root.join(&guide_md),
        b"# Ordinary\n\n@fact:FAKE #use stays literal.\n",
    )
    .unwrap();
    fs::write(root.join(&guide_xml), b"<spec xmlns='https://vibevm.org/spec/1'><p><FAKE fact='true' status='impl/done'>raw data</FAKE></p></spec>\n").unwrap();
    (root.join(rule), root.join(guide_md))
}

fn host_entry(address: &str, status_text: &str) -> FactEntry {
    FactEntry::for_host(address, "org.example/demo", Some(status(status_text)), None).unwrap()
}

#[test]
fn declared_host_sync_uses_only_selected_vibe_beside_raw_same_stem_pair() {
    let temp = tempdir().unwrap();
    let (rule, guide_md) = declared_host_fixture(temp.path());
    let guide_before = fs::read(&guide_md).unwrap();
    let address = "spec://org.example/demo/common/RULE#RULE";
    let absent = "spec://org.example/demo/common/RULE#ABSENT";
    let mut registry = Registry::default();
    registry
        .upsert(temp.path(), host_entry(address, "spec/work"))
        .unwrap();
    registry
        .upsert(temp.path(), host_entry(absent, "spec/work"))
        .unwrap();
    let mismatches = sync::check(temp.path(), &registry).unwrap();
    assert_eq!(mismatches.len(), 2);
    assert!(
        mismatches
            .iter()
            .any(|row| row.address == address && row.spec_status == Some(status("impl/done")))
    );
    assert!(
        mismatches
            .iter()
            .any(|row| row.address == absent && row.spec_status.is_none())
    );
    sync::reconcile(temp.path(), &mut registry).unwrap();
    assert_eq!(
        registry.get(address).unwrap().status,
        Some(status("impl/done"))
    );
    assert!(
        registry.get(absent).is_none(),
        "only a genuinely missing anchor is removed"
    );
    assert_eq!(fs::read(&guide_md).unwrap(), guide_before);
    assert!(
        fs::read_to_string(rule)
            .unwrap()
            .contains("real selected fact")
    );
}

#[test]
fn declared_host_sync_refuses_raw_or_missing_citation_before_registry_mutation() {
    let temp = tempdir().unwrap();
    let (rule, _) = declared_host_fixture(temp.path());
    let raw_address = "spec://org.example/demo/common/guide#FAKE";
    let mut registry = Registry::default();
    registry
        .upsert(temp.path(), host_entry(raw_address, "spec/work"))
        .unwrap();
    let registry_path = temp.path().join(facts_rel("spec.toml"));
    let before = fs::read(&registry_path).unwrap();
    let error = sync::reconcile(temp.path(), &mut registry)
        .unwrap_err()
        .to_string();
    assert!(error.contains("no selected Vibe source"), "{error}");
    assert_eq!(fs::read(&registry_path).unwrap(), before);
    assert_eq!(
        registry.get(raw_address).unwrap().status,
        Some(status("spec/work"))
    );

    registry.remove(temp.path(), raw_address).unwrap();
    let vibe_address = "spec://org.example/demo/common/RULE#RULE";
    registry
        .upsert(temp.path(), host_entry(vibe_address, "spec/work"))
        .unwrap();
    let before = fs::read(&registry_path).unwrap();
    fs::remove_file(&rule).unwrap();
    let error = sync::reconcile(temp.path(), &mut registry)
        .unwrap_err()
        .to_string();
    assert!(error.contains("unavailable"), "{error}");
    assert_eq!(fs::read(&registry_path).unwrap(), before);
    assert_eq!(
        registry.get(vibe_address).unwrap().status,
        Some(status("spec/work"))
    );

    fs::write(&rule, b"<spec xmlns='https://vibevm.org/spec/1'><p><RULE fact='true' status='impl/done'>restored</RULE></p></spec>").unwrap();
    let md = rule.with_extension("md");
    fs::write(
        &md,
        b"# Rule\n\n@fact:RULE second Vibe form. @status:impl/done\n",
    )
    .unwrap();
    let manifest_path = temp.path().join("vibe.toml");
    let mut manifest = fs::read_to_string(&manifest_path).unwrap();
    manifest.push_str(&format!("[[project.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n",
        md.strip_prefix(temp.path()).unwrap().to_string_lossy().replace('\\', "/")));
    fs::write(&manifest_path, &manifest).unwrap();
    let error = sync::reconcile(temp.path(), &mut registry)
        .unwrap_err()
        .to_string();
    assert!(error.contains("two selected Vibe sources"), "{error}");
    assert_eq!(fs::read(&registry_path).unwrap(), before);

    manifest.push_str("[[project.sources]]\ndirectory='vibevm/vibespecs'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='scan'\n");
    fs::write(&manifest_path, manifest).unwrap();
    let error = sync::reconcile(temp.path(), &mut registry)
        .unwrap_err()
        .to_string();
    assert!(error.contains("complete captured census"), "{error}");
    assert_eq!(fs::read(&registry_path).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn declared_host_sync_refuses_symlink_directory_escape_before_registry_mutation() {
    use std::os::unix::fs::symlink;
    let temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let (rule, _) = declared_host_fixture(temp.path());
    let common = rule.parent().unwrap();
    fs::remove_dir_all(common).unwrap();
    fs::write(outside.path().join("RULE.xml"), b"<spec/>").unwrap();
    symlink(outside.path(), common).unwrap();
    let address = "spec://org.example/demo/common/RULE#RULE";
    let mut registry = Registry::default();
    registry
        .upsert(temp.path(), host_entry(address, "spec/work"))
        .unwrap();
    let registry_path = temp.path().join(facts_rel("spec.toml"));
    let before = fs::read(&registry_path).unwrap();
    let error = sync::reconcile(temp.path(), &mut registry)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("capture") || error.contains("pin"),
        "{error}"
    );
    assert_eq!(fs::read(&registry_path).unwrap(), before);
    assert_eq!(
        registry.get(address).unwrap().status,
        Some(status("spec/work"))
    );
}

#[cfg(unix)]
#[test]
fn host_sync_refuses_oversize_or_symlink_manifest_before_registry_mutation() {
    use std::os::unix::fs::symlink;

    let temp = tempdir().unwrap();
    declared_host_fixture(temp.path());
    let address = "spec://org.example/demo/common/RULE#RULE";
    let mut registry = Registry::default();
    registry
        .upsert(temp.path(), host_entry(address, "spec/work"))
        .unwrap();
    let registry_path = temp.path().join(facts_rel("spec.toml"));
    let before = fs::read(&registry_path).unwrap();
    let manifest_path = temp.path().join("vibe.toml");
    let original = fs::read(&manifest_path).unwrap();

    let mut oversize = original.clone();
    oversize.extend_from_slice(b"\n# ");
    oversize.extend(std::iter::repeat_n(b'x', 1024 * 1024));
    fs::write(&manifest_path, oversize).unwrap();
    let error = sync::reconcile(temp.path(), &mut registry)
        .unwrap_err()
        .to_string();
    assert!(error.contains("cannot capture host manifest"), "{error}");
    assert_eq!(fs::read(&registry_path).unwrap(), before);
    assert_eq!(
        registry.get(address).unwrap().status,
        Some(status("spec/work"))
    );

    let outside = tempdir().unwrap();
    fs::write(outside.path().join("vibe.toml"), original).unwrap();
    fs::remove_file(&manifest_path).unwrap();
    symlink(outside.path().join("vibe.toml"), &manifest_path).unwrap();
    let error = sync::reconcile(temp.path(), &mut registry)
        .unwrap_err()
        .to_string();
    assert!(error.contains("cannot capture host manifest"), "{error}");
    assert_eq!(fs::read(&registry_path).unwrap(), before);
    assert_eq!(
        registry.get(address).unwrap().status,
        Some(status("spec/work"))
    );
}

#[cfg(unix)]
#[test]
fn legacy_sync_refuses_cited_symlink_but_skips_unrelated_one() {
    use std::os::unix::fs::symlink;

    let temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    fs::write(
        temp.path().join("vibe.toml"),
        "[project]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\n",
    )
    .unwrap();
    let specs = temp.path().join(layout::current_specs_root());
    fs::create_dir_all(&specs).unwrap();
    fs::create_dir_all(specs.join("common")).unwrap();
    fs::write(
        specs.join("common/RULE.md"),
        "# Rule\n\n@fact:RULE real. @status:impl/done\n",
    )
    .unwrap();
    fs::write(outside.path().join("OTHER.md"), "# outside\n").unwrap();
    symlink(outside.path(), specs.join("unrelated")).unwrap();

    let address = "spec://org.example/demo/common/RULE#RULE";
    let mut registry = Registry::default();
    registry
        .upsert(temp.path(), host_entry(address, "spec/work"))
        .unwrap();
    assert_eq!(sync::check(temp.path(), &registry).unwrap().len(), 1);
    let registry_path = temp.path().join(facts_rel("spec.toml"));
    let before = fs::read(&registry_path).unwrap();

    fs::remove_file(specs.join("common/RULE.md")).unwrap();
    fs::remove_dir(specs.join("common")).unwrap();
    symlink(outside.path(), specs.join("common")).unwrap();
    let error = sync::reconcile(temp.path(), &mut registry)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("cited host spec path is a symlink"),
        "{error}"
    );
    assert_eq!(fs::read(&registry_path).unwrap(), before);
    assert_eq!(
        registry.get(address).unwrap().status,
        Some(status("spec/work"))
    );
}
