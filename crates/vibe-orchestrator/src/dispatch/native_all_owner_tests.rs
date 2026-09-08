#[path = "native_all_owner_tests/support.rs"]
mod support;

#[path = "native_all_owner_tests/minify_parity.rs"]
mod minify_parity;

use support::Fixture;

#[test]
fn all_owner_build_replay_target_and_row_run_in_order_with_safe_dedup() {
    let fixture = Fixture::new(false);
    let context = fixture.native_context();
    let platform =
        vibe_lifecycle::native::NativePlatform::from_key(context.platform_key()).unwrap();
    assert_eq!(
        super::mechanism::preflight_all_owner_native_sources(
            &context,
            fixture.root(),
            platform,
            true,
            "2026-09-08T00:00:00Z",
        )
        .unwrap()
        .len(),
        2,
        "dependency consumers dedup once while the package self-host stays distinct"
    );
    for (owner, files) in fixture.native_outputs() {
        let output = output_text(&files);
        assert!(
            output.contains("vibe:transforms-pending"),
            "{owner} starts Pending"
        );
    }
    drop(context);
    let outcome = fixture.run();
    let values = match outcome {
        crate::PhaseOutcome::Completed(values) => values,
        other => panic!("complete all-owner fence succeeds: {other:?}"),
    };
    let records = fixture.native_records();
    assert_eq!(records.len(), 2, "each exact source/home group built once");
    let roots = records
        .iter()
        .map(|record| record["path_relative"]["root"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(roots, ["project", "slot"].into_iter().collect());
    let witnesses = records
        .iter()
        .map(|record| record["freshness"]["inputs"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(witnesses.len(), 2, "source/home witnesses stay distinct");
    assert!(
        fixture.observer_exists(),
        "authored target ran after replay"
    );
    assert!(
        values
            .contributions
            .iter()
            .any(|row| row.key.ends_with("#after") && row.status == "ok"),
        "phase row ran after the fence"
    );
    for (owner, files) in fixture.native_outputs() {
        let output = output_text(&files);
        assert!(
            !output.contains("vibe:transforms-pending"),
            "{owner}: {output}"
        );
        assert!(output.contains("# Compiler"), "{owner}: {output}");
    }
}

fn output_text(files: &[(String, Vec<u8>)]) -> String {
    files
        .iter()
        .map(|(_, bytes)| String::from_utf8_lossy(bytes))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn provider_pin_conflict_refuses_before_any_build_or_target_side_effect() {
    let fixture = Fixture::new(false);
    let context = fixture.native_context();
    let before = fixture.native_outputs();
    let platform =
        vibe_lifecycle::native::NativePlatform::from_key(context.platform_key()).unwrap();
    let error =
        super::mechanism::preflight_with_test_pins(&context, fixture.root(), platform, |owner| {
            match owner {
                vibe_workspace::extension_world::OwnerRuntimeId::Node { rel }
                    if rel == "member" =>
                {
                    "org.demo/member#foreign".into()
                }
                _ => "org.vibevm/vibe#cargo".into(),
            }
        })
        .unwrap_err();
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("conflicting effective rows or build provider"),
        "{rendered}"
    );
    assert_eq!(fixture.native_record_count(), 0);
    assert!(!fixture.observer_exists());
    assert_eq!(fixture.native_outputs(), before, "preflight writes nothing");
}

#[derive(Clone, Copy, Debug)]
enum FenceFailure {
    NativeBuild,
    ReplayPrepare,
    AuthoredTarget,
}

#[test]
fn complete_fence_failures_preserve_their_exact_boundary_and_suppress_later_rows() {
    for failure in [
        FenceFailure::NativeBuild,
        FenceFailure::ReplayPrepare,
        FenceFailure::AuthoredTarget,
    ] {
        let fixture = Fixture::new(false);
        match failure {
            FenceFailure::NativeBuild => fixture.break_native_source(),
            FenceFailure::ReplayPrepare => fixture.make_replay_fail(),
            FenceFailure::AuthoredTarget => fixture.break_authored_target(),
        }
        let context = fixture.native_context();
        let pending = fixture.native_outputs();
        drop(context);
        let repeated = fixture.native_context();
        assert_eq!(
            fixture.native_outputs(),
            pending,
            "{failure:?} repeated Collect is byte-stable before the fence"
        );
        drop(repeated);

        let outcome = fixture.run();
        let crate::PhaseOutcome::Failed {
            measurement,
            original,
            ..
        } = outcome
        else {
            panic!("{failure:?} must stop the complete fence")
        };
        let rows = match measurement {
            crate::failure::Measurement::Lifecycle { rows, .. } => rows,
            other => panic!("{failure:?} remains lifecycle-shaped: {other:?}"),
        };
        assert!(
            rows.iter().all(|row| !row.key.ends_with("#after")),
            "{failure:?} suppresses the later phase row: {rows:?}"
        );
        let rendered = format!("{original:#}");
        match failure {
            FenceFailure::NativeBuild => {
                assert!(
                    rendered.contains("building native source group"),
                    "{rendered}"
                );
                assert_eq!(fixture.native_record_count(), 0);
                assert!(!fixture.observer_exists());
                assert_eq!(fixture.native_outputs(), pending);
            }
            FenceFailure::ReplayPrepare => {
                assert!(
                    rendered.contains("converging pending compiler-native boot artifacts"),
                    "{rendered}"
                );
                assert_eq!(fixture.native_record_count(), 2);
                assert!(!fixture.observer_exists());
                assert_eq!(fixture.native_outputs(), pending);
            }
            FenceFailure::AuthoredTarget => {
                assert!(
                    rendered.contains("executing the declared [[artifacts.build]] targets"),
                    "{rendered}"
                );
                assert_eq!(fixture.native_record_count(), 2);
                assert!(
                    fixture.observer_exists(),
                    "the failing authored target observed converged boot before rustc refused"
                );
                for (owner, files) in fixture.native_outputs() {
                    let output = output_text(&files);
                    assert!(
                        !output.contains("vibe:transforms-pending"),
                        "{owner}: {output}"
                    );
                }
            }
        }
    }
}
