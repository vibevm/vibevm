use super::*;
use vibe_workspace::derivatives::MaterializedDerivative;
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/service_fixture.rs"
));

#[test]
fn real_reconcile_updates_then_removes_owned_tree_preserving_neighbor_and_inert_script() {
    let f = Fixture::new("nested-tree");
    let materialized =
        MaterializedDerivative::derive(&f.text, &f.base, &f.local, &f.output, "result").unwrap();
    let consumer = tempfile::tempdir().unwrap();
    let declaration = vibe_core::manifest::SkillDecl {
        name: "local-skill".into(),
        path: "payload".into(),
        source: None,
        description: None,
        agents: vec!["claude".into()],
        include: vec![],
        resources: vec![],
    };
    let binding =
        prepare_materialized_binding(&materialized, consumer.path(), declaration.clone()).unwrap();
    super::super::reconcile_project_skill_binding(consumer.path(), &binding).unwrap();
    let target = binding.targets[0].path.clone();
    fs::write(target.join("FOREIGN.txt"), b"neighbor").unwrap();
    let mut retained = f.binding.local().clone();
    let skill = retained
        .files
        .iter_mut()
        .find(|f| f.path == "payload/SKILL.md")
        .unwrap();
    skill.bytes.extend_from_slice(b"\nUpdated local body.\n");
    f.local.write_atomic(&skill.path, &skill.bytes).unwrap();
    let updated = DerivationBinding::new(
        f.binding.base().clone(),
        retained,
        f.binding.base_capture().to_owned(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    let updated = MaterializedDerivative::derive(
        &toml::to_string(&updated).unwrap(),
        &f.base,
        &f.local,
        &f.output,
        "updated",
    )
    .unwrap();
    let binding = prepare_materialized_binding(&updated, consumer.path(), declaration).unwrap();
    super::super::reconcile_project_skill_binding(consumer.path(), &binding).unwrap();
    assert!(
        fs::read_to_string(target.join("SKILL.md"))
            .unwrap()
            .contains("Updated local body.")
    );
    assert_eq!(
        fs::read(target.join("scripts/helper.sh")).unwrap(),
        b"#!/bin/sh\nprintf DO_NOT_EXECUTE\n"
    );
    super::super::reconcile_vanished_project_skill_bindings(consumer.path(), &Default::default())
        .unwrap();
    assert_eq!(fs::read(target.join("FOREIGN.txt")).unwrap(), b"neighbor");
    assert!(!target.join("SKILL.md").exists());
    assert!(!target.join("scripts/helper.sh").exists());
}
