//! One borrowed run really does cover both kinds of boot compile, in an order
//! no hash table can permute.

use tempfile::TempDir;
use vibe_core::manifest::SpecFormat;
use vibe_wire::generated::compiler_trace_index::e1::index::{ScopeKind, ScopeStatus};

use super::super::test_helpers::*;
use super::super::*;
use super::support::*;
use crate::compile_trace::node_descriptor;

const PARENT: &str = "unit:org.vibevm/parent#static-md::attempt:1";
const ROOT: &str = "node:.#static-md::attempt:1";

/// RED 1 — one run over a fixture with a dirty package unit plus the root
/// node: both scope kinds in one index, one dense global sequence, and BOTH
/// scope ids present in the events — including in snapshot-bearing events, so
/// a nonempty global list can never stand in for per-scope coverage.
#[test]
fn one_run_covers_a_dirty_unit_and_the_root_node() {
    let (ws_dir, ws, resolution, _srcs) = unit_and_root_fixture();
    let run = traced_run(&ws.root);
    apply_traced(&ws, &resolution, Some(&run)).expect("the traced install succeeds");

    let index = run_index(&ws.root);
    let kinds: Vec<(&str, &ScopeKind)> = index
        .scopes
        .iter()
        .map(|scope| (scope.id.as_str(), &scope.kind))
        .collect();
    assert_eq!(
        kinds,
        vec![(PARENT, &ScopeKind::Unit), (ROOT, &ScopeKind::Node)],
        "units emit before nodes, both kinds in ONE index"
    );
    assert_eq!(index.scopes[0].label, "org.vibevm/parent@1.0.0");
    assert_eq!(index.scopes[1].label, ".");
    assert!(
        index
            .scopes
            .iter()
            .all(|scope| scope.status == ScopeStatus::Compiled),
        "a green install leaves every occurrence terminal"
    );
    for (position, event) in index.events.iter().enumerate() {
        assert_eq!(event.sequence, position as u32, "one dense global sequence");
    }

    // EACH scope, not the global list: both occurrences recorded events of
    // their own, and both published at least one snapshot of their own.
    for id in [PARENT, ROOT] {
        assert!(
            index.events.iter().any(|event| event.scope == id),
            "`{id}` recorded pass events of its own"
        );
        let snapshots: Vec<&str> = index
            .events
            .iter()
            .filter(|event| event.scope == id)
            .filter_map(|event| event.snapshot.as_deref())
            .collect();
        assert!(
            !snapshots.is_empty(),
            "`{id}` published a snapshot of its own"
        );
        for name in snapshots {
            assert!(
                ws.root
                    .join(".vibe")
                    .join("trace")
                    .join(RUN)
                    .join(name)
                    .is_file(),
                "`{name}` landed in the one run directory"
            );
        }
    }
    // The install itself produced the artifacts it always would have.
    let parent_static = fs::read_to_string(unit_static(ws_dir.path(), "parent")).unwrap();
    assert!(parent_static.contains("# parent boot"), "{parent_static}");
    assert!(parent_static.contains("# child boot"), "{parent_static}");
}

/// RED 2 — permuting resolution/map/set construction yields the IDENTICAL
/// scope, event and snapshot-name order once duration values are ignored.
#[test]
fn unit_order_is_stable_under_permuted_construction() {
    fn install(permuted: bool) -> Vec<String> {
        let ws_dir = TempDir::new().unwrap();
        write(
            ws_dir.path(),
            "vibe.toml",
            "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n\n\
             [requires.packages]\n\
             \"org.vibevm/parent-a\" = \"^1.0\"\n\"org.vibevm/parent-b\" = \"^1.0\"\n\
             \"org.vibevm/corelib\" = { version = \"^1.0\", link = \"static\" }\n",
        );
        write(ws_dir.path(), boot_rel("00-core.md"), "# core");
        let (a, a_src, ca, ca_src) = static_pair("parent-a", "child-a");
        let (b, b_src, cb, cb_src) = static_pair("parent-b", "child-b");
        let (corelib, corelib_src) = dep_with_boot(
            "corelib",
            "1.0.0",
            "[boot_snippet]\nsource = \"boot/corelib.md\"",
            "boot/corelib.md",
            "# corelib boot",
        );
        let ws = Workspace::load(ws_dir.path()).unwrap();
        let resolution = if permuted {
            vec![cb, corelib, b, ca, a]
        } else {
            vec![a, ca, corelib, b, cb]
        };
        let run = traced_run(&ws.root);
        apply_traced(&ws, &resolution, Some(&run)).expect("both orders install");
        drop((a_src, ca_src, b_src, cb_src, corelib_src));

        // The trace projection a reader compares: scope identities and event
        // identities with their snapshot names — durations deliberately gone.
        let index = run_index(&ws.root);
        let mut projection = Vec::new();
        for scope in &index.scopes {
            projection.push(format!(
                "{}|{:?}|{}|{}|{:?}|{:?}",
                scope.id, scope.kind, scope.label, scope.artifact, scope.target, scope.status
            ));
        }
        for event in &index.events {
            projection.push(format!(
                "{}|{}|{}|{}|{:?}|{}",
                event.sequence,
                event.scope,
                event.pass,
                event.invocation,
                event.status,
                event.snapshot.as_deref().unwrap_or("-")
            ));
        }
        projection
    }
    let forward = install(false);
    let permuted = install(true);
    assert_eq!(
        forward, permuted,
        "the trace is order-authoritative, not hash-order-shaped"
    );
    assert!(
        forward[0].starts_with("unit:org.vibevm/parent-a#static-md::attempt:1|Unit|"),
        "units sort by canonical (group, name): {}",
        forward[0]
    );
    assert!(
        forward
            .iter()
            .any(|line| line.starts_with("unit:org.vibevm/parent-b#static-md::attempt:1|Unit|")),
        "the second unit is present after the first"
    );
    assert!(
        forward
            .iter()
            .any(|line| line.starts_with("node:.#static-md::attempt:1|Node|")),
        "the root node follows the units"
    );
}

/// The target belongs to the BASE (correction §4): after the Markdown install
/// compiled `node:.#static-md`, the same node rel under XML is a different
/// base — it opens its own series at `attempt:1` instead of continuing the
/// Markdown one, and the compiled Markdown occurrence stays untouched.
#[test]
fn one_node_rel_under_two_targets_is_two_bases() {
    let (_ws_dir, ws, resolution, _srcs) = unit_and_root_fixture();
    let run = traced_run(&ws.root);
    apply_traced(&ws, &resolution, Some(&run)).expect("the markdown install succeeds");

    let xml = run
        .acquire_scope_lossy(&node_descriptor(".", SpecFormat::Xml))
        .expect("the xml base is free — it is not the markdown one");
    assert_eq!(xml.id(), "node:.#static-xml::attempt:1");

    let index = run_index(&ws.root);
    let node_ids: Vec<&str> = index
        .scopes
        .iter()
        .filter(|scope| scope.id.starts_with("node:."))
        .map(|scope| scope.id.as_str())
        .collect();
    assert_eq!(
        node_ids,
        vec![ROOT, "node:.#static-xml::attempt:1"],
        "one rel, two targets, two independent attempt series"
    );
    assert_eq!(index.scopes[1].status, ScopeStatus::Compiled);
}
