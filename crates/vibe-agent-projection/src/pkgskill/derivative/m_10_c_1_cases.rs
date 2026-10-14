use super::*;
use vibe_workspace::derivatives::MaterializedDerivative;
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/service_fixture.rs"
));

fn declaration() -> vibe_core::manifest::SkillDecl {
    vibe_core::manifest::SkillDecl {
        name: "local-skill".into(),
        path: "payload".into(),
        source: None,
        description: None,
        agents: vec!["claude".into()],
        include: vec![],
        resources: vec![],
    }
}

#[test]
fn complete_real_tree_handoff_is_inert_and_receipts_preserve_unowned_neighbor() {
    let f = Fixture::new("nested-tree");
    let result =
        MaterializedDerivative::derive(&f.text, &f.base, &f.local, &f.output, "result").unwrap();
    assert_eq!(result.result().binding(), &f.binding);
    let consumer = tempfile::tempdir().unwrap();
    let binding = prepare_materialized_binding(&result, consumer.path(), declaration()).unwrap();
    assert_eq!(
        binding.selected_files.as_ref().unwrap().len(),
        f.expected.len()
    );
    assert!(!consumer.path().join(".claude").exists());
    for file in &f.expected {
        assert_eq!(
            binding.selected_files.as_ref().unwrap()[file.path.strip_prefix("payload/").unwrap()],
            file.bytes
        );
    }
    // Explicit consumer operation, distinct from passive derive/inspection.
    super::super::reconcile_project_skill_binding(consumer.path(), &binding).unwrap();
    let target = &binding.targets[0].path;
    fs::write(target.join("FOREIGN.txt"), b"unowned neighbor").unwrap();
    super::super::reconcile_project_skill_binding(consumer.path(), &binding).unwrap();
    super::super::reconcile_vanished_project_skill_bindings(consumer.path(), &Default::default())
        .unwrap();
    assert_eq!(
        fs::read(target.join("FOREIGN.txt")).unwrap(),
        b"unowned neighbor"
    );
    assert!(!target.join("SKILL.md").exists());
    assert_eq!(
        fs::read(result.tree().root_path().join("payload/scripts/helper.sh")).unwrap(),
        b"#!/bin/sh\nprintf DO_NOT_EXECUTE\n"
    );
}

#[test]
fn changed_result_and_hydration_requests_refuse_without_activation() {
    let f = Fixture::new("nested-tree");
    let result =
        MaterializedDerivative::derive(&f.text, &f.base, &f.local, &f.output, "result").unwrap();
    let consumer = tempfile::tempdir().unwrap();
    let mut decl = declaration();
    decl.source = Some("must-not-fetch".into());
    assert!(prepare_materialized_binding(&result, consumer.path(), decl).is_err());
    result
        .tree()
        .write_atomic("payload/assets/icons/palette.bin", b"changed")
        .unwrap();
    assert!(prepare_materialized_binding(&result, consumer.path(), declaration()).is_err());
    assert_eq!(fs::read_dir(consumer.path()).unwrap().count(), 0);
}
