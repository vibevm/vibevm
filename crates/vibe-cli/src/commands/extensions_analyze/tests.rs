//! The analyzer command's own laws: the fixture below is a real node with
//! a real materialised dependency, and every test drives the same path
//! the command drives — the workspace entry's observed compile, lowered,
//! re-read through the generated reader, validated through the cell.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use tempfile::TempDir;

use super::{Collector, lower, run, verify_through_the_reader};
use crate::cli::AgentModeArg;
use crate::cli::AnalyzeArgs;
use crate::output;

/// Write `body` at `root/rel`, creating parents.
fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("a rel path has a parent")).expect("mkdir");
    fs::write(&path, body).expect("write");
}

/// One analyzed node: a grouped project whose boot lane carries two
/// authored files and one statically-linked dependency snippet.
fn fixture() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    write(
        root,
        "vibe.toml",
        "[project]\nname = \"demo\"\ngroup = \"org.demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/lib\" = { version = \"1.0.0\", link = \"static\" }\n",
    );
    write(
        root,
        "vibevm/vibespecs/boot/00-core.md",
        "# Core\n\nFoundation text.\n",
    );
    write(
        root,
        "vibevm/vibespecs/boot/90-user.md",
        "# User\n\nOverride text.\n",
    );
    // The dependency's materialised slot: manifest + its own boot snippet.
    write(
        root,
        "vibevm/vibedeps/org.vibevm.lib/1.0.0/vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"lib\"\nkind = \"feat\"\nversion = \"1.0.0\"\n\n\
         [boot_snippet]\nsource = \"boot/10-lib.md\"\n",
    );
    write(
        root,
        "vibevm/vibedeps/org.vibevm.lib/1.0.0/boot/10-lib.md",
        "# Lib\n\nDependency text.\n",
    );
    dir
}

/// Compose, compile under the observer, and lower — the command's whole
/// semantic path without the printing. The report comes back BESIDE the
/// one emission event it was lowered from, so the tests can hold the
/// report to the evidence byte for byte (the sum law alone cannot see a
/// frame reallocated into a contribution — both stay self-consistent).
fn analyze(root: &Path) -> (super::ExtensionsAnalyze, vibe_spec::EmissionEvent) {
    let selected =
        vibe_workspace::Workspace::discover_selected(root).expect("the fixture discovers");
    let collector = Arc::new(Collector::default());
    let lane = vibe_workspace::install::analyze_node_lane(
        &selected.workspace,
        ".",
        Some(collector.clone()),
    )
    .expect("the lane composes and compiles")
    .expect("the fixture's node has static contributions");
    let report =
        lower(".", &lane, &collector, lane.artifact.bytes().len()).expect("the evidence lowers");
    let report = verify_through_the_reader(report).expect("the report re-validates");
    let event = collector
        .emissions
        .lock()
        .expect("the collector is at rest")
        .first()
        .cloned()
        .expect("the compile emitted one artifact");
    (report, event)
}

#[test]
fn the_analyzed_lane_reconciles_with_typed_providers() {
    let (report, event) = analyze(fixture().path());
    assert_eq!(report.artifacts.len(), 1);
    let artifact = &report.artifacts[0];
    assert_eq!(artifact.artifact_id, "static-md");
    assert!(
        artifact
            .total_emitted_bytes
            .bytes()
            .all(|byte| byte.is_ascii_digit()),
        "the total rides the wire as a decimal string"
    );

    // Reconciliation, recomputed the way a reader would: contributions
    // plus frame are the total (the cell enforces it; the test names it).
    let contributions: u128 = artifact
        .contributions
        .iter()
        .map(|row| row.bytes.parse::<u128>().expect("canonical decimal"))
        .sum();
    let frame: u128 = artifact
        .frame_overhead_bytes
        .parse()
        .expect("canonical decimal");
    let total: u128 = artifact
        .total_emitted_bytes
        .parse()
        .expect("canonical decimal");
    assert_eq!(contributions + frame, total);
    assert!(total > 0, "a three-entry lane emits real bytes");

    // Evidence fidelity, per row: every contribution's bytes are the
    // EVENT's bytes for that seat, and the frame is the event's frame —
    // a frame reallocated into a contribution keeps the sum law and only
    // this comparison can see it.
    assert_eq!(artifact.contributions.len(), event.contributions().len());
    for (row, evidence) in artifact.contributions.iter().zip(event.contributions()) {
        assert_eq!(
            row.bytes,
            vibe_wire::behaviour::extensions_analyze::spell_bytes(evidence.bytes() as u128)
        );
    }
    assert_eq!(
        artifact.frame_overhead_bytes,
        vibe_wire::behaviour::extensions_analyze::spell_bytes(event.frame_bytes() as u128)
    );

    // The lane's one contribution is the statically-linked dependency
    // snippet, attributed to its typed coordinate. (The node's own
    // authored files are INDEX entries — `dynamic`-linked by the
    // composition engine — so a node lane's contributions are its
    // dependencies'; the host arms of the provider one-of are the
    // vocabulary's other seats, pinned by the wire corpus.)
    assert_eq!(artifact.contributions.len(), 1);
    let row = &artifact.contributions[0];
    assert_eq!(row.origin, "org.vibevm/lib");
    assert_eq!(
        row.path,
        "vibevm/vibedeps/org.vibevm.lib/1.0.0/boot/10-lib.md"
    );
    let vibe_wire::generated::extensions_analyze::ProviderIdentity::Dependency(provider) =
        &row.provider
    else {
        panic!("the snippet attributes to its dependency coordinate");
    };
    assert_eq!(provider.group, "org.vibevm");
    assert_eq!(provider.name, "lib");
    // …and its bytes are the snippet's own material, not framing.
    let contribution_bytes: u128 = row.bytes.parse().expect("canonical decimal");
    assert!(contribution_bytes > 0);
    assert_eq!(row.occurrences, 1);
    assert_eq!(artifact.occurrence_count, 1);

    // The absent estimator form rides the wire as nulls.
    let wire = serde_json::to_value(artifact).expect("serializes");
    assert_eq!(wire["token_estimate"], serde_json::Value::Null);
    assert_eq!(wire["estimator_id"], serde_json::Value::Null);
    // No transform ran: the empty plan's statement.
    assert!(artifact.deltas.is_empty());
}

#[test]
fn partial_evidence_refuses_rather_than_reporting() {
    let dir = fixture();
    let selected = vibe_workspace::Workspace::discover_selected(dir.path()).expect("discovers");
    let lane = vibe_workspace::install::analyze_node_lane(&selected.workspace, ".", None)
        .expect("composes and compiles")
        .expect("has static contributions");
    // An observer that never saw the emission — the one artifact compiled,
    // zero events collected.
    let empty = Collector::default();
    let error = lower(".", &lane, &empty, lane.artifact.bytes().len())
        .expect_err("partial evidence must refuse");
    let text = format!("{error:#}");
    assert!(
        text.contains("partial evidence"),
        "the refusal names the law: {text}"
    );
}

#[test]
fn a_node_without_static_contributions_analyzes_to_the_empty_report() {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "vibe.toml",
        "[project]\nname = \"solo\"\nversion = \"0.1.0\"\n",
    );
    let selected = vibe_workspace::Workspace::discover_selected(dir.path()).expect("discovers");
    let lane = vibe_workspace::install::analyze_node_lane(&selected.workspace, ".", None)
        .expect("the empty node analyzes");
    assert!(lane.is_none(), "no static entries, no artifact, honestly");
}

#[test]
fn the_command_runs_end_to_end_and_writes_the_report_file() {
    let dir = fixture();
    let out = dir.path().join("analyze.json");
    let ctx = output::Context::from_flags(false, false, None, false, AgentModeArg::Auto);
    run(
        &ctx,
        AnalyzeArgs {
            path: dir.path().to_path_buf(),
            out: Some(out.clone()),
        },
    )
    .expect("the command runs");
    let text = fs::read_to_string(&out).expect("the report file is written");
    let document: serde_json::Value = serde_json::from_str(&text).expect("the file parses");
    assert_eq!(document["command"], "extensions-analyze");
    assert_eq!(
        document["artifacts"][0]["lane"]["node_rel"], ".",
        "the row names the analyzed node"
    );
}
