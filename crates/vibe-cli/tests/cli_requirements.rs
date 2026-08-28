//! End-to-end coverage of `vibe requirements` — the R7.5 read-only
//! requirements surface (PROP-054 `##FACT-QUERY-CONTRACT` /
//! `##REF-REQUIREMENTS-SURFACES`; R7 architecture §6.1).
//!
//! The library's own acceptance matrix lives in `vibe-requirements`.
//! What only a binary-level test can prove is here: that a refused
//! question never reaches the filesystem, that `--json` is the generated
//! root and nothing else, that the human and quiet projections stay
//! bounded and prose-free, that `--relations` is what decides whether a
//! map is read at all, and that the optional lifecycle join key is
//! carried for one node only.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REF-REQUIREMENTS-SURFACES");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use vibe_test_support::UserScratch;
use vibe_wire::behaviour::requirements_report::validate;
use vibe_wire::generated::requirements_report::RequirementsReport;

const RUN_ID: &str = "0123456789abcdef0123456789abcdef";

/// Two addressed facts under the host coordinate `org.example/demo`,
/// sorted `ALPHA` before `BETA`, with a canary word in `BETA`'s prose no
/// bounded answer may echo.
const TWO_FACTS: &str = "# Rules\n\n@fact:ALPHA Alpha rule. @status:impl/done\n\n\
                         @fact:BETA Beta mentions PROSECANARY here. @status:spec/plan\n";

/// A standalone project: host `org.example/demo`, one spec document, no
/// `.vibe`, no lock, no adoption registry.
fn project(body: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("project root");
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let specs = root.path().join(vibe_core::layout::current_specs_root());
    fs::create_dir_all(&specs).unwrap();
    fs::write(specs.join("RULE.md"), body).unwrap();
    root
}

/// Every file under `root` with its exact bytes, keyed by the
/// workspace-relative forward-slashed path — the before/after pair that
/// decides whether an invocation mutated anything at all. Bytes, not
/// just names: a refusal that rewrote a file in place would otherwise
/// pass a name-only comparison.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    out
}

/// `vibe requirements --path <root> …`, run under an isolated user home.
fn requirements(user: &UserScratch, root: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut command = user.vibe();
    command.arg("requirements").arg("--path").arg(root);
    for arg in args {
        command.arg(arg);
    }
    command.assert()
}

/// Parse stdout as EXACTLY one generated requirements report: one JSON
/// value with exactly the root's members, deserialisable into the
/// generated type, and accepted by the wire owner's own validator.
fn parse_report(stdout: &[u8]) -> (RequirementsReport, Value) {
    let text = std::str::from_utf8(stdout).expect("utf-8 stdout");
    let value: Value = serde_json::from_str(text).expect("stdout is exactly one JSON document");
    let object = value.as_object().expect("the root is an object");
    let members: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    assert_eq!(
        members,
        BTreeSet::from([
            "observation",
            "query",
            "relation_sources",
            "requirements",
            "rows",
            "sources",
            "truncated",
        ]),
        "`--json` is the generated root exactly — no envelope, no stamp, no second document",
    );
    let report: RequirementsReport =
        serde_json::from_str(text).expect("the document IS the generated requirements root");
    validate(&report).expect("the emitted report satisfies its own wire laws");
    (report, value)
}

/// The headline case: a real project with addressed facts answers with
/// the generated root, carrying the effective query it was asked.
#[test]
fn json_emits_exactly_one_generated_requirements_root() {
    let user = UserScratch::new();
    let root = project(TWO_FACTS);

    let assert = requirements(&user, root.path(), &["--json"]).success();
    let (report, _) = parse_report(&assert.get_output().stdout);

    assert_eq!(report.requirements, 1);
    assert_eq!(report.observation.selected, ".");
    assert_eq!(report.query.limit, 100, "the default row bound is restated");
    assert!(!report.query.relations);
    assert!(report.query.address_prefix.is_none());
    assert!(!report.truncated);
    let addresses: Vec<&str> = report.rows.iter().map(|row| row.address.as_str()).collect();
    assert_eq!(
        addresses,
        [
            "spec://org.example/demo/RULE#ALPHA",
            "spec://org.example/demo/RULE#BETA"
        ],
    );
    assert!(
        report.observation.lifecycle_run_id.is_none(),
        "a project that never ran a phase has no run join key",
    );
    assert!(
        !root.path().join(".vibe").exists(),
        "a read-only query begins no state",
    );
}

/// The human projection is the shared bounded text: metadata columns
/// only, and never a fact's prose.
#[test]
fn human_output_is_the_bounded_projection_and_carries_no_fact_prose() {
    let user = UserScratch::new();
    let root = project(TWO_FACTS);

    let assert = requirements(&user, root.path(), &[]).success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        stdout.starts_with("requirements 1: selected=. sources=1 rows=2 truncated=false\n"),
        "the summary line opens the answer: {stdout}",
    );
    assert!(stdout.contains("source host org.example/demo: available"));
    assert!(stdout.contains(
        "spec://org.example/demo/RULE#ALPHA authoring=marked=impl/done \
         adoption=not-applicable relations=0"
    ));
    assert!(stdout.contains("relations org.example/demo: not-requested"));
    assert!(
        !stdout.contains("PROSECANARY"),
        "a bounded answer never echoes a fact's prose: {stdout}",
    );
}

/// Quiet's contract is one line, and that line is the projection's own
/// summary — still prose-free.
#[test]
fn quiet_output_stays_one_bounded_line() {
    let user = UserScratch::new();
    let root = project(TWO_FACTS);

    let assert = requirements(&user, root.path(), &["--quiet"]).success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert_eq!(
        stdout.lines().count(),
        1,
        "quiet emits exactly one line: {stdout}",
    );
    assert_eq!(
        stdout.trim_end(),
        "requirements 1: selected=. sources=1 rows=2 truncated=false",
    );
    assert!(!stdout.contains("PROSECANARY"));
}

/// The prefix scopes the answer and is restated in it; a prefix that
/// matches nothing answers with zero rows rather than with everything.
#[test]
fn the_address_prefix_scopes_the_rows() {
    let user = UserScratch::new();
    let root = project(TWO_FACTS);

    let assert = requirements(
        &user,
        root.path(),
        &[
            "--json",
            "--address-prefix",
            "spec://org.example/demo/RULE#B",
        ],
    )
    .success();
    let (report, _) = parse_report(&assert.get_output().stdout);
    assert_eq!(
        report
            .rows
            .iter()
            .map(|row| row.address.as_str())
            .collect::<Vec<_>>(),
        ["spec://org.example/demo/RULE#BETA"],
    );
    assert_eq!(
        report.query.address_prefix.as_deref(),
        Some("spec://org.example/demo/RULE#B"),
        "the answer restates the question it was scoped by",
    );

    let assert = requirements(
        &user,
        root.path(),
        &[
            "--json",
            "--address-prefix",
            "spec://org.example/demo/RULE#ZZZ",
        ],
    )
    .success();
    let (report, _) = parse_report(&assert.get_output().stdout);
    assert!(report.rows.is_empty());
    assert!(!report.truncated);
}

/// The bound cuts, and the answer says it cut.
#[test]
fn the_limit_bounds_the_rows_and_names_the_truncation() {
    let user = UserScratch::new();
    let root = project(TWO_FACTS);

    let assert = requirements(&user, root.path(), &["--json", "--limit", "1"]).success();
    let (report, _) = parse_report(&assert.get_output().stdout);
    assert_eq!(report.query.limit, 1);
    assert!(report.truncated, "two facts cut at one is truncated");
    assert_eq!(
        report
            .rows
            .iter()
            .map(|row| row.address.as_str())
            .collect::<Vec<_>>(),
        ["spec://org.example/demo/RULE#ALPHA"],
        "the cut happens after the global sort",
    );

    let assert = requirements(&user, root.path(), &["--limit", "1"]).success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("truncated: row set cut at the query's limit (1)"),
        "the human projection names the cut: {stdout}",
    );
}

/// An unacceptable question is refused before ANY filesystem access:
/// the tree is byte-identical afterwards and `.vibe` never appears.
#[test]
fn an_invalid_query_refuses_without_touching_the_filesystem() {
    let user = UserScratch::new();
    let root = project(TWO_FACTS);
    let before = snapshot(root.path());

    for argv in [
        vec!["--limit", "0"],
        vec!["--limit", "257"],
        vec!["--address-prefix", "req-one"],
        vec!["--address-prefix", "spec:/org.example/demo"],
    ] {
        requirements(&user, root.path(), &argv).failure();
        assert_eq!(
            snapshot(root.path()),
            before,
            "`{argv:?}` mutated the project",
        );
        assert!(
            !root.path().join(".vibe").exists(),
            "`{argv:?}` created state for a question that was never answered",
        );
    }
}

/// The ORDERING red, and the decisive one: point the command at a path
/// that could never resolve and ask an unacceptable question at the same
/// time. If the grammar decided first, the refusal is the query's; if a
/// filesystem lookup happened first, it would be the path's. A tree that
/// simply stayed unchanged cannot tell those two apart — this can.
#[test]
fn the_grammar_refuses_before_the_path_is_ever_resolved() {
    let user = UserScratch::new();
    let nowhere = tempfile::tempdir().unwrap();
    let missing = nowhere.path().join("no-such-directory");

    for argv in [vec!["--limit", "0"], vec!["--address-prefix", "req-one"]] {
        let mut command = user.vibe();
        command.arg("requirements").arg("--path").arg(&missing);
        for arg in &argv {
            command.arg(arg);
        }
        let assert = command.assert().failure();
        let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
        assert!(
            stderr.contains("the effective query is not one the surfaces would accept"),
            "`{argv:?}` must be refused by the grammar: {stderr}",
        );
        assert!(
            !stderr.contains("resolving the selected node at"),
            "`{argv:?}` reached path resolution before the grammar decided: {stderr}",
        );
    }
    assert!(!missing.exists(), "no refused invocation creates its path");
}

/// The `--json` document is the generated root even under an invocation
/// that WOULD be stamped by the shared envelope: `--invoked-by` adds no
/// member here, because this surface prints the report directly.
#[test]
fn json_carries_no_envelope_stamp() {
    let user = UserScratch::new();
    let root = project(TWO_FACTS);

    let assert = requirements(
        &user,
        root.path(),
        &["--json", "--invoked-by", "claude-code", "--unattended"],
    )
    .success();
    // `parse_report` pins the member set exactly, so an `invoked_by` or
    // `unattended` stamp turns this red.
    let (report, value) = parse_report(&assert.get_output().stdout);
    assert_eq!(report.rows.len(), 2);
    assert!(value.get("invoked_by").is_none());
    assert!(value.get("unattended").is_none());
}

/// The grammar itself: an option this verb does not have, and a limit
/// that is not a number, never reach the command.
#[test]
fn unknown_options_and_wrong_types_refuse() {
    let user = UserScratch::new();
    let root = project(TWO_FACTS);
    let before = snapshot(root.path());

    for argv in [
        vec!["--evidence"],
        vec!["--prefix", "spec://org.example/demo"],
        vec!["--limit", "many"],
    ] {
        requirements(&user, root.path(), &argv).failure();
        assert_eq!(snapshot(root.path()), before);
    }
}

/// The decisive relations red. A MALFORMED `specmap.toml` is the probe:
/// without `--relations` the answer is `not-requested`, which is only
/// possible if the config was never read; with it, the same file is read
/// and typed `invalid`. The base fact rows survive either way.
#[test]
fn relations_decide_whether_a_map_is_read_at_all() {
    let user = UserScratch::new();
    let root = project(TWO_FACTS);
    fs::write(root.path().join("specmap.toml"), "not = = toml\n").unwrap();

    let assert = requirements(&user, root.path(), &["--json"]).success();
    let (report, _) = parse_report(&assert.get_output().stdout);
    assert_eq!(report.relation_sources.len(), 1);
    assert_eq!(
        serde_json::to_value(&report.relation_sources[0].state).unwrap(),
        Value::from("not-requested"),
        "a malformed config the query never read cannot be `invalid`",
    );
    assert!(report.relation_sources[0].reason_code.is_none());
    assert_eq!(report.rows.len(), 2, "base rows are unaffected");

    let assert = requirements(&user, root.path(), &["--json", "--relations"]).success();
    let (report, _) = parse_report(&assert.get_output().stdout);
    assert_eq!(
        serde_json::to_value(&report.relation_sources[0].state).unwrap(),
        Value::from("invalid"),
        "with `--relations` the same file IS read, and its malformation is typed",
    );
    assert_eq!(
        report.relation_sources[0].reason_code.as_deref(),
        Some("project-map-config-invalid"),
    );
    assert_eq!(report.rows.len(), 2, "enrichment loss never costs a row");
    assert!(report.query.relations);
}

/// `--relations` with no specmap config at all: typed unavailable
/// enrichment, named in the human projection, base rows intact.
#[test]
fn relations_without_a_config_are_typed_unavailable() {
    let user = UserScratch::new();
    let root = project(TWO_FACTS);

    let assert = requirements(&user, root.path(), &["--json", "--relations"]).success();
    let (report, _) = parse_report(&assert.get_output().stdout);
    assert_eq!(
        serde_json::to_value(&report.relation_sources[0].state).unwrap(),
        Value::from("unavailable"),
    );
    assert_eq!(
        report.relation_sources[0].reason_code.as_deref(),
        Some("project-map-config-absent"),
    );
    assert_eq!(report.rows.len(), 2);
    assert!(report.rows.iter().all(|row| row.relations.is_empty()));

    let assert = requirements(&user, root.path(), &["--relations"]).success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("relations org.example/demo: unavailable (project-map-config-absent)"),
        "the loss is visible in the bounded text: {stdout}",
    );
}

/// A minimal durable lifecycle state naming `selected` as its author.
fn write_state(root: &Path, selected: &str) -> PathBuf {
    let dir = root.join(".vibe");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("lifecycle.toml");
    fs::write(
        &path,
        format!(
            "schema = 1\n\n[execution]\n\n[run]\nchain = [\"validate\"]\n\
             requested = \"validate\"\nstarted = \"2026-01-01T00:00:00Z\"\n\
             run_id = \"{RUN_ID}\"\nselected = \"{selected}\"\n"
        ),
    )
    .unwrap();
    path
}

/// The optional join key is carried for the node that authored the run,
/// and for no other — and reading it changes nothing on disk.
#[test]
fn the_lifecycle_run_id_joins_only_the_same_selected_node() {
    let user = UserScratch::new();
    let root = project(TWO_FACTS);

    let state = write_state(root.path(), ".");
    let before = fs::read(&state).unwrap();
    let assert = requirements(&user, root.path(), &["--json"]).success();
    let (report, _) = parse_report(&assert.get_output().stdout);
    assert_eq!(
        report.observation.lifecycle_run_id.as_deref(),
        Some(RUN_ID),
        "this node's own run is the join key",
    );
    assert_eq!(
        fs::read(&state).unwrap(),
        before,
        "peeking at durable state never rewrites it",
    );

    write_state(root.path(), "members/tool");
    let assert = requirements(&user, root.path(), &["--json"]).success();
    let (report, _) = parse_report(&assert.get_output().stdout);
    assert!(
        report.observation.lifecycle_run_id.is_none(),
        "a sibling member's run is a different node's evidence",
    );
}

/// A PRESENT durable state that could not be safely decoded is NOT an
/// absence. Degrading it to "no join key" would emit a generated report
/// whose missing `lifecycle_run_id` is indistinguishable from the honest
/// absence two tests above — while the MCP surface, asking the identical
/// question, refuses. So this refuses: no requirements root reaches
/// stdout, the refusal names the read it could not make, and the tree
/// stays byte-identical, malformed file included.
#[test]
fn a_malformed_lifecycle_state_refuses_instead_of_degrading() {
    let user = UserScratch::new();
    let root = project(TWO_FACTS);
    fs::create_dir_all(root.path().join(".vibe")).unwrap();
    fs::write(root.path().join(".vibe/lifecycle.toml"), "schema = = 1\n").unwrap();
    let before = snapshot(root.path());

    for argv in [vec!["--json"], vec![], vec!["--quiet"]] {
        let assert = requirements(&user, root.path(), &argv).failure();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
        assert!(
            stdout.trim().is_empty(),
            "`{argv:?}` emitted output over a state it could not read: {stdout}",
        );
        assert!(
            serde_json::from_str::<RequirementsReport>(&stdout).is_err(),
            "`{argv:?}` must emit no requirements root",
        );
        let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
        assert!(
            stderr.contains("requirements run-id join"),
            "`{argv:?}` must name the read-only join it could not make: {stderr}",
        );
        assert!(
            stderr.contains("repair or remove"),
            "`{argv:?}` must be actionable: {stderr}",
        );
        assert_eq!(
            snapshot(root.path()),
            before,
            "`{argv:?}` changed the tree while refusing",
        );
    }
}

/// The whole surface refuses a path that is not a workspace node, and
/// says which path it could not resolve.
#[test]
fn a_path_that_is_not_a_project_refuses() {
    let user = UserScratch::new();
    let empty = tempfile::tempdir().unwrap();

    requirements(&user, empty.path(), &["--json"])
        .failure()
        .stderr(predicates::str::contains("resolving the selected node at"));
}
