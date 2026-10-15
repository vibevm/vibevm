//! Actual public inspection boundary; no provider or client commissioning.
use super::*;
use std::fs;

fn manifest(source: &str) -> String {
    format!(
        r#"[package]
group = "org.example"
name = "fixture"
kind = "tool"
version = "1.0.0"
authors = ["Fixture"]
license = "MIT"
description = "Fixture"
keywords = ["fixture"]

[[embedded_source]]
name = "upstream"
kind = "git"
url = "https://example.invalid/upstream.git"
commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
content_hash = "sha256-tree/1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
auth = "none"
upstream_authors = ["Fixture"]
upstream_license = "MIT"
license_path = "LICENSE"
license_url = "https://example.invalid/upstream/blob/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/LICENSE"

[[skill]]
name = "demo"
path = "skills/demo"
agents = ["claude"]
include = ["**"]
{source}
"#
    )
}

fn inventory(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let key = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                out.insert(key, None);
                walk(root, &path, out);
            } else {
                out.insert(key, Some(fs::read(path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn public_inspect_embedded_skill_refuses_before_hydration() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        manifest("source = \"upstream\""),
    )
    .unwrap();
    let before = inventory(root.path());
    let error = match crate::inspect(root.path()) {
        Ok(_) => panic!("embedded source must require preparation"),
        Err(error) => error,
    };
    assert!(
        error
            .downcast_ref::<vibe_agent_projection::pkgskill::SourcePreparationRequired>()
            .is_some(),
        "{error:#}"
    );
    assert_eq!(inventory(root.path()), before);
}

#[test]
fn admission_keeps_embedded_skill_row_without_preparing_payload() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        manifest("source = \"upstream\""),
    )
    .unwrap();
    let workspace = Workspace::discover(root.path()).unwrap();
    let before = inventory(root.path());
    let loaded = capture_admission_metadata(root.path(), &workspace).unwrap();
    let rows = loaded.registry.exhaustive(SelectorSubject::unscoped());
    assert!(
        rows.iter().any(
            |view| view.row.key().to_string() == "@vibe/package/skill/org.example/fixture/demo"
        )
    );
    assert!(loaded.package_bindings.is_empty());
    assert!(plan_from_loaded(loaded, &[Phase::Package]).is_err());
    assert_eq!(inventory(root.path()), before);
}

#[test]
fn public_inspect_local_skill_preserves_selected_resources_and_disk() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), manifest("")).unwrap();
    fs::create_dir_all(root.path().join("skills/demo/references")).unwrap();
    fs::write(root.path().join("skills/demo/SKILL.md"), "body").unwrap();
    fs::write(
        root.path().join("skills/demo/references/a.txt"),
        "reference",
    )
    .unwrap();
    let before = inventory(root.path());
    let loaded = crate::inspect(root.path()).unwrap();
    assert_eq!(loaded.package_bindings.len(), 1);
    let binding = loaded.package_bindings.values().next().unwrap();
    assert_eq!(binding.selected_files.as_ref().unwrap().len(), 2);
    assert_eq!(binding.targets.len(), 1);
    assert_eq!(inventory(root.path()), before);
}

#[test]
fn callback_and_phase_planning_consume_retained_runtime_sidecar() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), manifest("")).unwrap();
    fs::create_dir_all(root.path().join("skills/demo")).unwrap();
    fs::write(root.path().join("skills/demo/SKILL.md"), "held body").unwrap();
    let workspace = Workspace::discover(root.path()).unwrap();
    let (world, lowering, sidecar) =
        prepare_owner_runtime_inputs(root.path(), &workspace, &[], true).unwrap();
    let epoch = vibe_workspace::extension_world::lower_owner_runtimes(&workspace, &world, lowering)
        .unwrap()
        .bind_run(vibe_workspace::extension_world::OwnerRuntimeRunFacts {
            run_id: "prepared-adapter".into(),
            state_root: root.path().join(".vibe"),
            platform: "test".into(),
            offline: true,
            created_at: "2026-10-15T00:00:00Z".into(),
        });
    // Payload tamper is separate from the planning interval. This is the
    // execution carrier, not an apply grant: apply retains its own source fence.
    fs::remove_dir_all(root.path().join("skills")).unwrap();
    let before = inventory(root.path());
    for phases in [
        &[Phase::Validate, Phase::Install][..],
        &[Phase::Package][..],
    ] {
        let ritual =
            plan_with_runtime(root.path(), &workspace, Some((&epoch, &sidecar)), phases).unwrap();
        let binding = ritual.package_bindings.values().next().unwrap();
        assert_eq!(
            binding.selected_files.as_ref().unwrap()["SKILL.md"],
            b"held body"
        );
    }
    assert_eq!(inventory(root.path()), before);
    let foreign = tempfile::tempdir().unwrap();
    assert!(
        plan_with_runtime(
            foreign.path(),
            &workspace,
            Some((&epoch, &sidecar)),
            &[Phase::Package]
        )
        .is_err()
    );
}
