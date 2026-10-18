use super::*;
use clap::Parser;
use std::io::{BufReader, Cursor};

const STATIC: &str = "<!-- vibe:static org.test/pkg — authored.md -->\n<!-- embed: spec://org.test/pkg/fragment -->\nhello\n<!-- /embed: spec://org.test/pkg/fragment -->\n";

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("vibe.toml"), "[project]\n").unwrap();
    std::fs::write(root.path().join("STATIC.md"), STATIC).unwrap();
    root
}
fn args(root: &std::path::Path, script: Option<&str>) -> BootArgs {
    BootArgs {
        registry: None,
        root: root.into(),
        artifact: "STATIC.md".into(),
        kind: ArtifactKind::Static,
        view: true,
        send: script.map(str::to_owned),
    }
}
fn frames(output: &[u8]) -> Vec<serde_json::Value> {
    output
        .split(|b| *b == b'\n')
        .filter(|b| !b.is_empty())
        .map(|b| serde_json::from_slice(b).unwrap())
        .collect()
}

#[test]
fn m_06_c_2_cases_standard_command_constructor_headless_and_interactive() {
    let root = fixture();
    let mut output = vec![];
    // Same argument parser and command function used by main's Boot arm.
    let cli = crate::cli::Cli::try_parse_from([
        "vibe",
        "boot",
        "--root",
        root.path().to_str().unwrap(),
        "--artifact",
        "STATIC.md",
        "--kind",
        "static",
        "--view",
        "--send",
        "child;detail;source;compare 0 0;select org.test/pkg;select 999;snapshot 3",
    ])
    .unwrap();
    assert_eq!(
        crate::boot_viewer_policy(&cli.command),
        Some(crate::command_progress::CommandPolicy::Interactive)
    );
    let crate::cli::Command::Boot(boot) = cli.command else {
        panic!()
    };
    run_with_io(boot, Cursor::new([]), &mut output).unwrap();
    let frames = frames(&output);
    assert_eq!(frames.len(), 8);
    assert_eq!(frames[1]["selected"], 1);
    assert_eq!(
        frames[2]["observation"]["body"]["chain"],
        serde_json::json!([0, 1])
    );
    assert_eq!(
        frames[3]["observation"]["body"]["hex_bytes"],
        STATIC
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    assert_eq!(
        frames[4]["observation"]["body"]["changed"],
        serde_json::json!([])
    );
    for frame in &frames[5..] {
        assert!(frame["diagnostic"].is_string());
        assert_eq!(frame["selected"], 1);
    }
    assert_eq!(
        frames[1]["sources"][1]["authored_resource_id"],
        serde_json::Value::Null
    );
    let mut human = vec![];
    run_with_io(
        args(root.path(), None),
        Cursor::new(b"child\ndetail\nsource\nparent\nquit\n"),
        &mut human,
    )
    .unwrap();
    let human = String::from_utf8(human).unwrap();
    assert!(
        human.contains("boot> ")
            && human.contains("Captured bytes:")
            && human.contains("spec://org.test/pkg/fragment")
    );
    assert!(human.contains("Gap:"));
    assert_eq!(
        std::fs::read_to_string(root.path().join("STATIC.md")).unwrap(),
        STATIC
    );
    assert!(!root.path().join(".vibe").exists());
}

#[test]
fn m_06_c_2_cases_headless_matches_api_nodes_sources_exact_differences_and_refusals() {
    let root = fixture();
    let route = open_route(&args(root.path(), None)).unwrap();
    let api = RouteService(&route);
    let mut output = vec![];
    serve(
        &route,
        Cursor::new([]),
        &mut output,
        Some("child;detail;source;compare 0 0"),
    )
    .unwrap();
    let headless = frames(&output);
    let id = headless[0]["snapshot_id"].as_str().unwrap();
    let result = api
        .query(q::Command::Inspect(Box::new(q::CommandInspect {
            snapshot_id: id.into(),
        })))
        .unwrap();
    let r::Body::Inspect(inspected) = result.body else {
        panic!()
    };
    assert_eq!(
        headless[0]["nodes"],
        serde_json::to_value(&inspected.nodes).unwrap()
    );
    assert_eq!(
        headless[0]["sources"],
        serde_json::to_value(&inspected.sources).unwrap()
    );
    assert_eq!(
        headless[0]["gaps"],
        serde_json::to_value(&inspected.gaps).unwrap()
    );
    let comparison = api
        .query(q::Command::Compare(Box::new(q::CommandCompare {
            snapshot_id: id.into(),
            other_snapshot_id: id.into(),
        })))
        .unwrap();
    assert_eq!(
        headless[4]["observation"],
        serde_json::to_value(comparison).unwrap()
    );
    let mut viewer = Viewer::new(RouteService(&route)).unwrap();
    let before = viewer.frame(None, None).snapshot_id;
    let inspected = api
        .query(q::Command::Inspect(Box::new(q::CommandInspect {
            snapshot_id: before.clone(),
        })))
        .unwrap();
    let r::Body::Inspect(inspected) = inspected.body else {
        panic!()
    };
    assert_eq!(viewer.frame(None, None).nodes, inspected.nodes);
    assert_eq!(viewer.frame(None, None).sources, inspected.sources);
    std::fs::write(
        root.path().join("STATIC.md"),
        STATIC.replace("hello", "changed"),
    )
    .unwrap();
    viewer.apply(Action::Refresh).unwrap();
    let after = viewer.frame(None, None).snapshot_id;
    assert_ne!(before, after);
    let diff = viewer.apply(Action::Compare(0, 1)).unwrap().unwrap();
    assert_eq!(
        diff,
        api.query(q::Command::Compare(Box::new(q::CommandCompare {
            snapshot_id: before.clone(),
            other_snapshot_id: after
        })))
        .unwrap()
    );
    let r::Body::Compare(diff) = diff.body else {
        panic!()
    };
    assert_eq!(diff.changed, ["STATIC.md"]);
    assert_eq!(viewer.frame(None, None).comparison_gaps.len(), 2);
    viewer.apply(Action::Snapshot(0)).unwrap();
    assert_eq!(viewer.frame(None, None).nodes, inspected.nodes);
    let raw = viewer.apply(Action::Source(0)).unwrap().unwrap();
    let r::Body::Chunk(raw) = raw.body else {
        panic!()
    };
    assert_eq!(
        raw.hex_bytes,
        STATIC
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    assert!(viewer.apply(Action::Source(u64::MAX)).is_err());
    assert!(
        api.query(q::Command::Inspect(Box::new(q::CommandInspect {
            snapshot_id: "STATIC.md".into()
        })))
        .is_err()
    );
    let wrong = boot_debugger::input(&q::Request {
        schema: 1,
        context_id: "foreign".into(),
        command: q::Command::Capture(Box::new(q::CommandCapture {})),
    });
    assert!(matches!(
        route.exchange(&serde_json::to_vec(&wrong).unwrap()).body,
        env::Body::Error(_)
    ));
}

struct ChangedInput {
    root: PathBuf,
    commands: Cursor<Vec<u8>>,
    changed: bool,
}
impl Read for ChangedInput {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if !self.changed {
            std::fs::write(
                self.root.join("STATIC.md"),
                STATIC.replace("hello", "later"),
            )?;
            self.changed = true;
        }
        self.commands.read(buf)
    }
}
#[test]
fn m_06_c_2_cases_interactive_refresh_compare_through_standard_entrypoint() {
    let root = fixture();
    let input = ChangedInput {
        root: root.path().into(),
        commands: Cursor::new(b"refresh\ncompare 0 1\nsnapshot 0\nsource\nquit\n".to_vec()),
        changed: false,
    };
    let mut output = vec![];
    run_with_io(args(root.path(), None), BufReader::new(input), &mut output).unwrap();
    let text = String::from_utf8(output).unwrap();
    assert!(
        text.contains("\"changed\": [\n      \"STATIC.md\""),
        "{text}"
    );
    assert!(text.contains("hello") && text.contains("Comparison gaps"));
    assert!(!text.contains("Diagnostic:"));
}

#[test]
fn m_06_c_2_cases_large_partial_graph_and_absent_artifact_remain_inspectable() {
    let root = fixture();
    let index = (0..1100)
        .map(|_| "[[entry]]\npath='same.md'\nkind='dynamic'\n")
        .collect::<String>();
    std::fs::write(root.path().join("INDEX.md"), index).unwrap();
    let mut boot = args(root.path(), Some("select 1099;detail;source;next;parent"));
    boot.artifact = "INDEX.md".into();
    boot.kind = ArtifactKind::Index;
    let mut output = vec![];
    run_with_io(boot, Cursor::new([]), &mut output).unwrap();
    let values = frames(&output);
    assert_eq!(values[0]["total"], 1100);
    assert_eq!(values[1]["selected"], 1099);
    assert!(values[1]["nodes"].as_array().unwrap().len() <= 20);
    assert!(!values[1]["gaps"].as_array().unwrap().is_empty());
    assert!(values[4]["diagnostic"].is_string());
    std::fs::remove_file(root.path().join("STATIC.md")).unwrap();
    let mut output = vec![];
    run_with_io(
        args(root.path(), Some("detail;source")),
        Cursor::new([]),
        &mut output,
    )
    .unwrap();
    let values = frames(&output);
    assert_eq!(values[0]["total"], 0);
    assert!(!values[0]["gaps"].as_array().unwrap().is_empty());
    assert!(values[1]["diagnostic"].is_string() && values[2]["diagnostic"].is_string());
}
