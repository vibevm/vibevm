use super::*;
use vibe_wire::generated::boot_debugger::e1::{request as q, result as r};
use vibe_workspace::boot_debugger::{
    scenario::*,
    viewer::{Action, Viewer},
};

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    for (name, text) in [
        ("shared", "shared bytes"),
        ("old", "old branch"),
        ("next", "next branch"),
    ] {
        let package = root
            .path()
            .join(format!("vibevm/vibepacks/org.example/{name}/v1.0.0"));
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("vibe.toml"), format!("[package]\ngroup='org.example'\nname='{name}'\nkind='flow'\nversion='1.0.0'\n[boot_snippet]\nsource='boot.md'\n")).unwrap();
        std::fs::write(package.join("boot.md"), text).unwrap();
    }
    std::fs::write(root.path().join("vibe.toml"), "[project]\nname='fixture'\nversion='1.0.0'\n[requires.packages]\n'org.example/shared'={version='=1.0.0',link='static'}\n'org.example/old'={version='=1.0.0',link='static'}\n").unwrap();
    root
}
fn inventory(root: &std::path::Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &std::path::Path, at: &std::path::Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for e in std::fs::read_dir(at).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                walk(root, &e.path(), out);
            } else {
                out.push((
                    e.path().strip_prefix(root).unwrap().into(),
                    std::fs::read(e.path()).unwrap(),
                ));
            }
        }
    }
    let mut out = vec![];
    walk(root, root, &mut out);
    out.sort();
    out
}
fn args(root: &std::path::Path, send: Option<String>) -> BootArgs {
    BootArgs {
        root: root.into(),
        registry: None,
        derivative_records: vec![],
        artifact: "STATIC.md".into(),
        kind: ArtifactKind::Static,
        view: send.is_some(),
        send,
    }
}
fn draft(edits: Vec<Edit>) -> Draft {
    Draft {
        baseline_selections: Selections::default(),
        edits,
    }
}
#[test]
fn ordinary_constructor_headless_controls_and_retained_replay_leave_every_file_unchanged() {
    let root = fixture();
    let before = inventory(root.path());
    let drafts = [
        draft(vec![Edit::Add {
            package: "org.example/next@=1.0.0".into(),
            loading: Some(vibe_core::manifest::LinkType::Static),
        }]),
        draft(vec![Edit::Remove {
            coordinate: "org.example/old".into(),
        }]),
        draft(vec![Edit::Toggle {
            package: "org.example/old@=1.0.0".into(),
            enabled: false,
            loading: None,
        }]),
        draft(vec![Edit::Reconfigure {
            selections: Selections::default(),
        }]),
    ];
    let script = drafts
        .iter()
        .map(|d| format!("scenario os:linux {}", serde_json::to_string(d).unwrap()))
        .collect::<Vec<_>>()
        .join(";");
    let mut output = vec![];
    run_with_io(
        args(root.path(), Some(script)),
        std::io::Cursor::new([]),
        &mut output,
    )
    .unwrap();
    let frames: Vec<serde_json::Value> = output
        .split(|b| *b == b'\n')
        .filter(|b| !b.is_empty())
        .map(|b| serde_json::from_slice(b).unwrap())
        .collect();
    let previews: Vec<r::BodyPreview> = frames
        .iter()
        .filter_map(|f| f.get("observation").filter(|v| !v.is_null()))
        .map(|v| {
            let result: r::Result = serde_json::from_value(v.clone()).unwrap();
            let r::Body::Preview(p) = result.body else {
                panic!()
            };
            *p
        })
        .collect();
    assert_eq!(previews.len(), 4, "{frames:?}");
    for p in &previews {
        assert_eq!(p.status, r::BodyPreviewStatus::Complete, "{p:?}");
        assert!(p.comparison_json.is_some());
    }
    let route = open_route(&args(root.path(), None)).unwrap();
    let service = viewer::RouteService(&route);
    let mut viewer = Viewer::new(service).unwrap();
    let first = viewer
        .apply(Action::Scenario {
            target: "os:linux".into(),
            draft_json: serde_json::to_string(&drafts[0]).unwrap(),
        })
        .unwrap()
        .unwrap();
    let repeated = viewer
        .apply(Action::Scenario {
            target: "os:linux".into(),
            draft_json: serde_json::to_string(&drafts[0]).unwrap(),
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&first.body).unwrap(),
        serde_json::to_value(&repeated.body).unwrap()
    );
    let r::Body::Preview(p) = &first.body else {
        panic!()
    };
    let replay = q::CommandPreview {
        scenario_id: p.scenario_id.clone(),
        expected_baseline: p.baseline_world.clone(),
        expected_candidate: p.candidate_world.clone(),
        expected_catalog: p.candidate_catalog.clone(),
        loading_overrides: Default::default(),
    };
    // Drive the ordinary structured stdin dispatcher over this same retained route.
    let request = boot_debugger::input(&q::Request {
        schema: 1,
        context_id: route.context_id().into(),
        command: q::Command::Preview(Box::new(replay.clone())),
    });
    let mut bytes = serde_json::to_vec(&request).unwrap();
    bytes.push(b'\n');
    let mut output = vec![];
    serve(&route, std::io::Cursor::new(bytes), &mut output).unwrap();
    let envelopes: Vec<vibe_wire::generated::machine::e1::operation_envelope::OperationEnvelope> =
        output
            .split(|b| *b == b'\n')
            .filter(|b| !b.is_empty())
            .map(|b| serde_json::from_slice(b).unwrap())
            .collect();
    let vibe_wire::generated::machine::e1::operation_envelope::Body::Result(answer) =
        &envelopes[1].body
    else {
        panic!()
    };
    let again: r::Result = serde_json::from_str(answer.payload_json.as_deref().unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(&first.body).unwrap(),
        serde_json::to_value(&again.body).unwrap()
    );
    let mut stale = replay.clone();
    stale.expected_catalog.push_str("changed");
    let result = viewer.apply(Action::Preview(stale)).unwrap().unwrap();
    let r::Body::Preview(p) = result.body else {
        panic!()
    };
    assert_eq!(p.status, r::BodyPreviewStatus::Stale);
    assert_eq!(inventory(root.path()), before);
    std::fs::remove_dir_all(root.path().join("vibevm")).unwrap();
    let retained = viewer.apply(Action::Preview(replay)).unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(first.body).unwrap(),
        serde_json::to_value(retained.body).unwrap()
    );
    // Restoring deleted test inputs is not part of the command: compare its earlier census.
    assert!(!root.path().join("STATIC.md").exists());
    assert!(!root.path().join("vibe.lock").exists());
    assert_eq!(
        before
            .iter()
            .find(|(p, _)| p == std::path::Path::new("vibe.toml"))
            .unwrap()
            .1,
        std::fs::read(root.path().join("vibe.toml")).unwrap()
    );
}
#[test]
fn ordinary_missing_preparation_is_incomplete_without_install() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname='fixture'\nversion='1.0.0'\n",
    )
    .unwrap();
    let before = inventory(root.path());
    let script = format!(
        "scenario os:linux {}",
        serde_json::to_string(&draft(vec![])).unwrap()
    );
    let mut output = vec![];
    run_with_io(
        args(root.path(), Some(script)),
        std::io::Cursor::new([]),
        &mut output,
    )
    .unwrap();
    assert!(String::from_utf8(output).unwrap().contains("incomplete"));
    assert_eq!(inventory(root.path()), before);
}

fn configured(branch: Option<&str>) -> Selections {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/consumer-configuration-request/e1/valid/omitted.json"
    )))
    .unwrap();
    let edge = &mut value["edges"][0];
    edge["dependency"] = serde_json::json!("org.example/switcher");
    edge["package"] = serde_json::json!({"kind":"vibe","reference":"org.example/switcher"});
    edge["selections"] = match branch {
        Some(branch) => {
            serde_json::json!({"continuity":{"intent":"exact","provenance":"fixture;explicit-choice","members":[branch]}})
        }
        None => serde_json::json!({}),
    };
    Selections {
        configuration: Some(serde_json::from_value(value).unwrap()),
        ..Default::default()
    }
}
#[test]
fn actual_continuity_reconfiguration_preserves_shared_contribution_and_missing_choice_is_typed() {
    let root = fixture();
    let dir = root
        .path()
        .join("vibevm/vibepacks/org.example/switcher/v1.0.0");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("vibe.toml"), "[package]\ngroup='org.example'\nname='switcher'\nkind='flow'\nversion='1.0.0'\n[boot_snippet]\nsource='boot.md'\n[requires.packages]\n'org.example/shared'='=1.0.0'\n'org.example/old'='=1.0.0'\n'org.example/next'='=1.0.0'\n[features]\nprofile='constrained-r1'\noptional=['org.example/old','org.example/next']\n[features.effects]\nold=[{dependency='org.example/old'}]\nnext=[{dependency='org.example/next'}]\n[features.groups.continuity]\ncardinality='exactly-one'\nmembers=['old','next']\n").unwrap();
    std::fs::write(dir.join("boot.md"), "switcher bytes").unwrap();
    std::fs::write(root.path().join("vibe.toml"), "[project]\nname='fixture'\nversion='1.0.0'\n[boot]\ndefault_link='static'\n[requires.packages]\n'org.example/switcher'='=1.0.0'\n'org.example/shared'='=1.0.0'\n").unwrap();
    let before = inventory(root.path());
    let drafts = [Some("next"), None].map(|branch| Draft {
        baseline_selections: configured(Some("old")),
        edits: vec![Edit::Reconfigure {
            selections: configured(branch),
        }],
    });
    let script = drafts
        .iter()
        .map(|d| format!("scenario os:linux {}", serde_json::to_string(d).unwrap()))
        .collect::<Vec<_>>()
        .join(";");
    let mut output = vec![];
    run_with_io(
        args(root.path(), Some(script)),
        std::io::Cursor::new([]),
        &mut output,
    )
    .unwrap();
    let previews: Vec<r::BodyPreview> = output
        .split(|b| *b == b'\n')
        .filter(|b| !b.is_empty())
        .filter_map(|b| {
            let value: serde_json::Value = serde_json::from_slice(b).unwrap();
            let observation = value.get("observation")?.clone();
            if observation.is_null() {
                return None;
            }
            let result: r::Result = serde_json::from_value(observation).unwrap();
            let r::Body::Preview(p) = result.body else {
                panic!()
            };
            Some(*p)
        })
        .collect();
    assert_eq!(previews.len(), 2, "{}", String::from_utf8_lossy(&output));
    assert_eq!(
        previews[0].status,
        r::BodyPreviewStatus::Complete,
        "{:?}",
        previews[0]
    );
    let c: serde_json::Value =
        serde_json::from_str(previews[0].comparison_json.as_deref().unwrap()).unwrap();
    let names = |world: &str| {
        c[world]["packages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["coordinate"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    assert!(names("baseline").contains(&"org.example/old".into()));
    assert!(!names("baseline").contains(&"org.example/next".into()));
    assert!(names("candidate").contains(&"org.example/next".into()));
    assert!(!names("candidate").contains(&"org.example/old".into()));
    let shared = |world: &str| {
        c[world]["packages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["coordinate"] == "org.example/shared")
            .unwrap()
            .clone()
    };
    assert_eq!(shared("baseline"), shared("candidate"));
    assert!(
        c["baseline"]["resolution_evidence"]
            .as_str()
            .unwrap()
            .contains("old")
    );
    assert!(
        c["candidate"]["resolution_evidence"]
            .as_str()
            .unwrap()
            .contains("next")
    );
    assert_eq!(
        previews[1].status,
        r::BodyPreviewStatus::Incompatible,
        "{:?}",
        previews[1]
    );
    assert!(previews[1].missing.join(" ").contains("selection"));
    assert_eq!(inventory(root.path()), before);
}
