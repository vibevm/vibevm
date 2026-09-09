use std::collections::BTreeMap;

use super::*;

fn prepared() -> d::PreparedHealth {
    d::PreparedHealth {
        plan_id: "sha256:plan".to_owned(),
        baseline: d::BaselinePolicy::NoRegression,
        max_stdout_bytes: 11,
        max_stderr_bytes: 12,
        max_result_bytes: 13,
        termination_grace_seconds: 14,
        checks: vec![d::PreparedHealthcheck {
            id: "custom".to_owned(),
            kind: d::HealthcheckKind::Custom,
            root: ".".to_owned(),
            applicability: d::Applicability::SkippedWhenMissing {
                path: "optional.txt".to_owned(),
            },
            tests: None,
            network: d::NetworkMode::Inherit,
            assets: vec![d::AssetIdentity {
                id: "runner".to_owned(),
                role: d::AssetRole::CustomInterpreter,
                display_path: "C:/runner.exe".to_owned(),
                sha256: "sha256:runner".to_owned(),
                bytes: 21,
                mode: None,
                platform_identity: "platform".to_owned(),
                version: "sha256:version".to_owned(),
                version_kind: d::VersionKind::Content,
                source: d::AssetSource::Bundle {
                    path: "tools/runner.exe".to_owned(),
                },
                live_identity: None,
            }],
            commands: vec![d::PreparedCommand {
                step: d::CommandStep::Verify,
                executable_asset_id: "runner".to_owned(),
                argv: vec![
                    d::PreparedArg::Literal("literal".to_owned()),
                    d::PreparedArg::Root,
                    d::PreparedArg::Scratch,
                    d::PreparedArg::Result,
                    d::PreparedArg::Phase,
                    d::PreparedArg::AssetPath("runner".to_owned()),
                    d::PreparedArg::BundlePath("verify.rs".to_owned()),
                ],
                environment: BTreeMap::from([
                    (
                        "ASSET".to_owned(),
                        d::EnvironmentValue::AssetPath("runner".to_owned()),
                    ),
                    (
                        "LITERAL".to_owned(),
                        d::EnvironmentValue::Literal("value".to_owned()),
                    ),
                    (
                        "SCRATCH".to_owned(),
                        d::EnvironmentValue::ScratchPath("tmp".to_owned()),
                    ),
                ]),
                accepted_exit_codes: vec![0, 2],
            }],
            effects: d::EffectPlan {
                reads: vec!["**".to_owned()],
                writes: Vec::new(),
                spawn: true,
            },
            sandbox: d::SandboxRequirement::for_check(d::NetworkMode::Inherit, true, true),
            protocol: d::ResultProtocol::VibeHealthJsonV1,
            custom_bundle: Some(d::CustomBundle {
                sha256: "sha256:bundle".to_owned(),
                source: "verify.rs".to_owned(),
                entries: vec![
                    d::BundleEntry {
                        path: "support".to_owned(),
                        kind: d::BundleEntryKind::Directory,
                        sha256: None,
                        bytes: None,
                        mode: None,
                        content: None,
                    },
                    d::BundleEntry {
                        path: "verify.rs".to_owned(),
                        kind: d::BundleEntryKind::File,
                        sha256: Some("sha256:file".to_owned()),
                        bytes: Some(3),
                        mode: Some(0o644),
                        content: Some(b"run".to_vec()),
                    },
                ],
            }),
            assurance_reductions: vec!["different-path".to_owned()],
            timeout_seconds: 15,
        }],
        blockers: vec![d::HealthBlocker {
            code: "blocked".to_owned(),
            check_id: None,
            message: "detail".to_owned(),
        }],
    }
}

#[test]
fn schema2_snapshot_round_trips_every_restart_value_and_omits_only_live_payloads() {
    let input = prepared();
    let bytes = snapshot_bytes(&input).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.starts_with(r#"{"schema":2,"plan_id":"sha256:plan""#));
    assert!(!text.contains("live_identity"));
    assert!(!text.contains(r#""content":"#));
    for required_null in [
        r#""tests":null"#,
        r#""mode":null"#,
        r#""sha256":null"#,
        r#""bytes":null"#,
        r#""check_id":null"#,
    ] {
        assert!(text.contains(required_null), "missing {required_null}");
    }

    let mut expected = input;
    expected.checks[0].custom_bundle.as_mut().unwrap().entries[1].content = None;
    assert_eq!(snapshot_from_bytes(&bytes).unwrap(), expected);
}

#[test]
fn handwritten_prepublic_snapshot_refuses_with_the_restart_recipe() {
    let bytes = serde_json::to_vec(&prepared()).unwrap();
    let error = snapshot_from_bytes(&bytes).unwrap_err().to_string();
    assert!(error.contains("pre-public schema-1 snapshots cannot be recovered"));
    assert!(error.contains("restart the scrape transaction"));
}

#[test]
fn malformed_schema_type_is_not_misreported_as_the_prepublic_shape() {
    let error = snapshot_from_bytes(br#"{"schema":"2"}"#)
        .unwrap_err()
        .to_string();
    assert!(error.contains("decoding schema-2 health snapshot"));
    assert!(!error.contains("pre-public schema-1"));
}
