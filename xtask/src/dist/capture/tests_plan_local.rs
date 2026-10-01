use super::{World, absent_remote, output_bytes};
use crate::dist::run_plan_local_into;
use crate::dist::smoke::prepare::prepare_local_with;
use vibe_safefs::Project;

#[test]
fn plan_local_reads_existing_evidence_without_new_effects_or_claiming_readiness() {
    let world = World::new();
    let output = Project::open(&world.out).unwrap();
    let prepared = prepare_local_with(world.repo.path(), &output, || Ok(())).unwrap();
    let remote = world.repo.path().join("remote-snapshot.json");
    std::fs::write(
        &remote,
        serde_json::to_vec(&serde_json::json!({
            "generation_ref": "fixture-remote-absent",
            "tag": absent_remote().tag,
            "release": absent_remote().release,
        }))
        .unwrap(),
    )
    .unwrap();
    let before = output_bytes(&world.out);
    let mut stdout = Vec::new();
    run_plan_local_into(
        world.repo.path(),
        "1.1.0",
        &world.out,
        &prepared.report.relative_path,
        &remote,
        false,
        &mut stdout,
    )
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(parsed["remote_observation"], "operator-supplied-unverified");
    assert_eq!(parsed["apply_ready"], false);
    assert_eq!(parsed["smoke_gate_passed"], false);
    assert_eq!(parsed["smoke_report_sha256"], prepared.report.digest);
    assert!(
        parsed["effect_plan"]["partitions"]
            .as_array()
            .unwrap()
            .len()
            > 1
    );
    assert_eq!(output_bytes(&world.out), before);

    let report_path = world.out.join(&prepared.report.relative_path);
    std::fs::write(&report_path, b"{\"tampered\":true}").unwrap();
    let mut rejected_stdout = Vec::new();
    assert!(
        run_plan_local_into(
            world.repo.path(),
            "1.1.0",
            &world.out,
            &prepared.report.relative_path,
            &remote,
            false,
            &mut rejected_stdout,
        )
        .is_err()
    );
    assert!(rejected_stdout.is_empty());
}

#[test]
fn plan_local_rejects_bad_remote_or_report_without_stdout() {
    let world = World::new();
    let remote = world.repo.path().join("remote-snapshot.json");
    std::fs::write(&remote, b"{} ").unwrap();
    let mut stdout = Vec::new();
    assert!(
        run_plan_local_into(
            world.repo.path(),
            "1.1.0",
            &world.out,
            "smoke/e1/missing.json",
            &remote,
            false,
            &mut stdout,
        )
        .is_err()
    );
    assert!(stdout.is_empty());
    std::fs::write(
        &remote,
        serde_json::to_vec(&serde_json::json!({
            "generation_ref": "fixture-remote-absent",
            "tag": absent_remote().tag,
            "release": absent_remote().release,
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(
        run_plan_local_into(
            world.repo.path(),
            "1.1.0",
            &world.out,
            "smoke/e1/missing.json",
            &remote,
            false,
            &mut stdout,
        )
        .is_err()
    );
    assert!(stdout.is_empty());
}
