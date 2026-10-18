use super::{hypothetical::HostCapture, scenario::*};
use std::collections::BTreeMap;
use vibe_core::manifest::{LinkType, Manifest};

fn host() -> HostCapture {
    let text = "[project]\nname='fixture'\nversion='1.0.0'\n[requires.packages]\n'org.example/shared'='=1.0.0'\n'org.example/old'={version='=1.0.0',link='static'}\n";
    HostCapture {
        id: "retained-baseline".into(),
        manifest: Manifest::parse_str(text).unwrap(),
        files: BTreeMap::from([("vibe.toml".into(), text.as_bytes().to_vec())]),
    }
}
#[test]
fn immutable_add_remove_toggle_and_reconfigure_preserve_unrelated_intent() {
    let baseline = host();
    let before = serde_json::to_vec(&baseline.manifest).unwrap();
    let files = baseline.files.clone();
    let draft = Draft {
        baseline_selections: Selections::default(),
        edits: vec![
            Edit::Add {
                package: "org.example/added@=1.0.0".into(),
                loading: Some(LinkType::Dynamic),
            },
            Edit::Remove {
                coordinate: "org.example/old".into(),
            },
            Edit::Toggle {
                package: "org.example/toggled@=1.0.0".into(),
                enabled: true,
                loading: None,
            },
            Edit::Toggle {
                package: "org.example/toggled@=1.0.0".into(),
                enabled: false,
                loading: None,
            },
            Edit::Reconfigure {
                selections: Selections {
                    features: vec!["branch-next".into()],
                    no_defaults: true,
                    ..Default::default()
                },
            },
        ],
    };
    let candidate = draft.candidate(&baseline, "os:linux").unwrap();
    assert_eq!(
        candidate
            .host
            .manifest
            .requires
            .packages
            .iter()
            .map(|p| p.qualified_name())
            .collect::<Vec<_>>(),
        ["org.example/shared", "org.example/added"]
    );
    assert_eq!(candidate.selections.features, ["branch-next"]);
    assert!(candidate.selections.no_defaults);
    assert_eq!(
        candidate
            .host
            .manifest
            .requires
            .links
            .get("org.example/added"),
        Some(&LinkType::Dynamic)
    );
    assert!(
        !candidate
            .host
            .manifest
            .requires
            .links
            .contains_key("org.example/old")
    );
    assert_eq!(serde_json::to_vec(&baseline.manifest).unwrap(), before);
    assert_eq!(baseline.files, files);
    assert_eq!(candidate.host.files, files);
    assert_eq!(candidate.host.id, baseline.id);
    assert_eq!(
        candidate.draft_identity,
        draft
            .candidate(&baseline, "os:linux")
            .unwrap()
            .draft_identity
    );
    assert_ne!(
        candidate.draft_identity,
        draft
            .candidate(&baseline, "os:windows")
            .unwrap()
            .draft_identity
    );
    let mut changed_capture = baseline.clone();
    changed_capture.id = "different-capture".into();
    assert_ne!(
        candidate.draft_identity,
        draft
            .candidate(&changed_capture, "os:linux")
            .unwrap()
            .draft_identity
    );
}
#[test]
fn ambiguous_edits_and_non_os_target_refuse_without_touching_baseline() {
    let baseline = host();
    for edit in [
        Edit::Add {
            package: "shared".into(),
            loading: None,
        },
        Edit::Add {
            package: "org.example/shared@=1.0.0".into(),
            loading: None,
        },
        Edit::Remove {
            coordinate: "org.example/absent".into(),
        },
        Edit::Remove {
            coordinate: "org.example/old@=1.0.0".into(),
        },
        Edit::Toggle {
            package: "org.example/shared@=2.0.0".into(),
            enabled: true,
            loading: None,
        },
    ] {
        let draft = Draft {
            baseline_selections: Selections::default(),
            edits: vec![edit],
        };
        assert!(draft.candidate(&baseline, "os:linux").is_err());
    }
    let draft = Draft {
        baseline_selections: Selections::default(),
        edits: vec![],
    };
    assert!(draft.candidate(&baseline, "future:unknown").is_err());
    assert_eq!(baseline.manifest.requires.packages.len(), 2);
}
#[test]
fn strict_draft_roundtrip_keeps_explicit_disabled_toggle() {
    let input = r#"{"baseline_selections":{"features":[],"no_defaults":false,"all_features":false,"configuration":null},"edits":[{"action":"toggle","package":"org.example/old@=1.0.0","enabled":false,"loading":null}]}"#;
    let draft: Draft = serde_json::from_str(input).unwrap();
    let replay: Draft = serde_json::from_slice(&serde_json::to_vec(&draft).unwrap()).unwrap();
    let baseline = host();
    assert_eq!(
        draft
            .candidate(&baseline, "os:linux")
            .unwrap()
            .draft_identity,
        replay
            .candidate(&baseline, "os:linux")
            .unwrap()
            .draft_identity
    );
    assert!(
        serde_json::from_str::<Draft>(
            &input.replace("\"enabled\":false", "\"enabled\":false,\"install\":true")
        )
        .is_err()
    );
}
