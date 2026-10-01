use super::*;

#[test]
fn missing_one_retained_link_landed_refuses_aggregate_cold() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; aggregate fixture skipped");
        return;
    };
    let world = world(&binary);
    world.stage_links(2);
    let approved = world.held();
    assert!(matches!(
        retain_complete_pending(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            &world.source,
            &approved,
            &binary,
            limits()
        ),
        AggregateInspection::AggregateCompleteButNotReady { .. }
    ));
    let home = world.root.path().join(".vibe/package-git-operations");
    let link_landed = fs::read_dir(home)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .contains(".package-git-gitlink-")
                && !path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains("aggregate")
                && !path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains("-pending")
        })
        .expect("per-link Landed absent from complete fixture");
    fs::remove_file(link_landed).unwrap();
    assert!(matches!(
        inspect_complete_cold(
            &world.lease,
            &world.namespace,
            &world.scope,
            "aggregate",
            None,
            None,
            &binary,
            limits()
        ),
        AggregateInspection::Uncertain { .. }
    ));
}
