use super::*;

fn prepared(content: Vec<u8>, digest: String) -> health::PreparedHealth {
    health::PreparedHealth {
        plan_id: "plan".to_owned(),
        baseline: health::BaselinePolicy::Strict,
        max_stdout_bytes: 1,
        max_stderr_bytes: 1,
        max_result_bytes: 1,
        termination_grace_seconds: 1,
        checks: vec![health::PreparedHealthcheck {
            id: "custom".to_owned(),
            kind: health::HealthcheckKind::Custom,
            root: ".".to_owned(),
            applicability: health::Applicability::Applicable,
            tests: None,
            network: health::NetworkMode::Inherit,
            assets: Vec::new(),
            commands: Vec::new(),
            effects: health::EffectPlan {
                reads: vec!["**".to_owned()],
                writes: Vec::new(),
                spawn: true,
            },
            sandbox: health::SandboxRequirement::for_check(
                health::NetworkMode::Inherit,
                true,
                true,
            ),
            protocol: health::ResultProtocol::ExitCode,
            custom_bundle: Some(health::CustomBundle {
                sha256: digest.clone(),
                source: "verify.rs".to_owned(),
                entries: vec![health::BundleEntry {
                    path: "verify.rs".to_owned(),
                    kind: health::BundleEntryKind::File,
                    sha256: Some(digest),
                    bytes: Some(content.len() as u64),
                    mode: Some(0o644),
                    content: Some(content),
                }],
            }),
            assurance_reductions: Vec::new(),
            timeout_seconds: 1,
        }],
        blockers: Vec::new(),
    }
}

#[test]
fn recovery_reattaches_only_the_exact_journaled_bundle_file() {
    let content = b"run".to_vec();
    let digest = format!("sha256:{:x}", Sha256::digest(&content));
    let health_bytes = health::snapshot_bytes(&prepared(content.clone(), digest.clone())).unwrap();
    assert!(!String::from_utf8_lossy(&health_bytes).contains(r#""content":"#));
    let snapshots = vec![tx::SnapshotRecord {
        kind: tx::SnapshotKind::Verifier,
        name: "verifier/custom/verify.rs".to_owned(),
        sha256: tx::Digest(digest.clone()),
        bytes: content.len() as u64,
        mode: Some(0o644),
    }];
    let verifier =
        PreparedHealthVerifier::from_snapshot_records(&health_bytes, &snapshots, |name| {
            assert_eq!(name, "verifier/custom/verify.rs");
            Ok(content.clone())
        })
        .unwrap();
    assert_eq!(
        verifier.prepared.checks[0]
            .custom_bundle
            .as_ref()
            .unwrap()
            .entries[0]
            .content
            .as_deref(),
        Some(b"run".as_slice())
    );

    let mut wrong = snapshots;
    wrong[0].mode = None;
    let error = PreparedHealthVerifier::from_snapshot_records(&health_bytes, &wrong, |_| {
        Ok(b"run".to_vec())
    })
    .err()
    .expect("mode drift refuses")
    .to_string();
    assert!(error.contains("differs from sealed bundle"));

    wrong[0].mode = Some(0o644);
    wrong[0].kind = tx::SnapshotKind::Contract;
    let error = PreparedHealthVerifier::from_snapshot_records(&health_bytes, &wrong, |_| {
        panic!("a wrong-kind record must refuse before reading its payload")
    })
    .err()
    .expect("wrong snapshot kind refuses")
    .to_string();
    assert!(error.contains("differs from sealed bundle"));
}
