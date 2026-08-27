//! What a cancellation report may claim about a declaration that is GONE.
//!
//! Both cases remove a HOST declaration, which is the shape that exposes
//! invented provenance: a host row is not a dependency, so a synthetic
//! cancellation row that hardcodes `tier = "dependency"` is simply false about
//! it. The state records a status, a phase and a typed scope, and nothing
//! else about where the row came from — so the report says exactly that much
//! and names an explicit sentinel for the rest.
//!
//! The phase case can be exact about its point, because the row's own phase is
//! persisted. The slot case cannot: `slot:pre-install` and `slot:post-install`
//! are different facts and the state records neither, so it never guesses.

mod common;

use std::fs;
use std::path::Path;

use common::UserScratch;
use common::agent_provider::{MockProvider, configure_provider};
use vibe_wire::generated::install_report::InstallReport;
use vibe_wire::generated::lifecycle_report::LifecycleReport;
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;

const RESULT: &str = r#"{"outputs":[{"path":"docs/guide.md","content":"paid\n"}]}"#;

/// A host agent row at `point`, plus a builtin sentinel after it. The sentinel
/// is the mutation detector: the run must continue past the cancelled row.
fn declarations(point: &str) -> String {
    format!(
        r#"
[[extension]]
id = "produce-docs"
point = "{point}"
handler = {{ kind = "agent", prompt = "spec://org.demo/demo/common/agent-prompt#root" }}
config.outputs = [
  {{ path = "docs/guide.md", kind = "file", accept = "non-empty file" }},
]

[[extension]]
id = "after-agent"
point = "{point}"
handler = {{ kind = "builtin", name = "log" }}
config = {{ message = "SENTINEL-AFTER-AGENT" }}
"#
    )
}

/// A project whose HOST manifest carries `extensions`, with the coordinate a
/// prompt address needs.
fn project(extensions: &str) -> (UserScratch, tempfile::TempDir) {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.vibe()
        .args(["init", "--no-registry", "--author", "Agent"])
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    let manifest_path = project.path().join("vibe.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let mut patched = String::new();
    for line in manifest.lines() {
        let line = if line.starts_with("name = ") && !line.contains("demo") {
            "name = \"demo\"".to_string()
        } else {
            line.to_string()
        };
        patched.push_str(&line);
        patched.push('\n');
        if line.starts_with("name = ") && !patched.contains("group = ") {
            patched.push_str("group = \"org.demo\"\n");
        }
    }
    patched.push_str(extensions);
    fs::write(&manifest_path, patched).unwrap();
    let specs = project.path().join("vibevm/vibespecs/common");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("agent-prompt.md"),
        "# Documentation prompt {#root}\n\nWrite the declared documentation files.\n",
    )
    .unwrap();
    (user, project)
}

/// Delete the agent declaration, keep the sentinel. This is the operator
/// action the whole reconciliation exists for.
fn remove_the_agent_row(project: &Path) {
    let manifest_path = project.join("vibe.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let cut = manifest
        .find("[[extension]]\nid = \"produce-docs\"")
        .unwrap();
    let keep = manifest
        .find("[[extension]]\nid = \"after-agent\"")
        .unwrap();
    fs::write(
        &manifest_path,
        format!("{}{}", &manifest[..cut], &manifest[keep..]),
    )
    .unwrap();
}

fn hosted(user: &UserScratch, project: &Path, verb: &str) -> std::process::Output {
    user.vibe()
        .args([verb, "--assume-yes", "--json"])
        .args(["--agent-mode", "agent"])
        .arg("--path")
        .arg(project)
        .output()
        .unwrap()
}

fn assert_ok(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn documents(bytes: &[u8]) -> Vec<serde_json::Value> {
    serde_json::Deserializer::from_slice(bytes)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// Every provenance claim a cancelled row may NOT make, in one place.
fn assert_no_invented_provenance(provider: &str, tier: &str, reference: Option<&str>, point: &str) {
    assert_ne!(
        tier, "dependency",
        "a removed HOST declaration was never a dependency",
    );
    assert_eq!(tier, "<unknown>", "the tier is an explicit sentinel");
    assert_eq!(provider, "<removed-declaration>");
    assert_eq!(reference, Some("<removed-declaration>"));
    assert!(!point.is_empty(), "the point is never empty");
    assert_ne!(point, "phase:", "and never a bare phase prefix");
    assert_ne!(point, "slot:", "and never a bare slot prefix");
}

/// PHASE scope. The row's own phase is persisted, so the point is EXACT —
/// `phase:create`, taken from the record, not from whichever execution happens
/// to survive in the new plan.
#[test]
fn a_cancelled_phase_row_reports_its_persisted_phase_and_no_guessed_provenance() {
    let provider = MockProvider::serving(RESULT);
    let (user, project) = project(&declarations("phase:create"));
    configure_provider(&user, &provider.endpoint());

    let parked = hosted(&user, project.path(), "create");
    assert_ok(&parked);
    let report: LifecycleReport =
        serde_json::from_value(documents(&parked.stdout).pop().unwrap()).unwrap();
    assert!(report.delegation.is_some(), "the host row parked");

    remove_the_agent_row(project.path());
    let output = hosted(&user, project.path(), "create");
    assert_ok(&output);
    let report: LifecycleReport =
        serde_json::from_value(documents(&output.stdout).pop().unwrap()).unwrap();
    let row = report
        .contributions
        .iter()
        .find(|row| row.status == "cancelled")
        .unwrap_or_else(|| panic!("the cancellation is reported: {:?}", report.contributions));

    assert!(row.key.ends_with("produce-docs"), "{row:?}");
    assert_eq!(
        row.point, "phase:create",
        "the point comes from the row's OWN persisted phase: {row:?}",
    );
    assert_eq!(row.phase, "create");
    assert_eq!(row.handler, "agent", "only agent rows delegate");
    assert_no_invented_provenance(
        &row.provider,
        &row.tier,
        row.reference.as_deref(),
        &row.point,
    );
    assert!(
        report
            .contributions
            .iter()
            .any(|row| row.key.ends_with("after-agent") && row.status == "ok"),
        "and the run continues past it: {:?}",
        report.contributions,
    );
    assert_eq!(provider.hits(), 0);
}

/// SLOT scope. The state records a slot SCOPE and never which slot point, so
/// the point is an explicit sentinel — pretending to know pre versus post
/// would be the same invention as claiming a tier.
#[test]
fn a_cancelled_slot_row_reports_a_sentinel_point_and_no_guessed_provenance() {
    if !common::git_available() {
        eprintln!("skipping host slot cancellation e2e: git not on PATH");
        return;
    }
    let provider = MockProvider::serving(RESULT);
    // A host slot row needs a real slot to fire against, so the project takes
    // one ordinary dependency that declares nothing of its own. The ONLY
    // lifecycle declaration in play is the host's.
    let outer = tempfile::tempdir().unwrap();
    common::hosted_slot::publish_plain(outer.path(), "0.1.0");
    let user = UserScratch::new();
    let project = common::hosted_slot::project_at(&user, outer.path());
    configure_provider(&user, &provider.endpoint());
    let manifest_path = project.path().join("vibe.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    fs::write(
        &manifest_path,
        format!(
            "{}{}",
            manifest.replace(
                "name = \"demo\"",
                "name = \"demo\"
group = \"org.demo\""
            ),
            declarations("slot:post-install"),
        ),
    )
    .unwrap();
    let specs = project.path().join("vibevm/vibespecs/common");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("agent-prompt.md"),
        "# Documentation prompt {#root}

Write the declared documentation files.
",
    )
    .unwrap();
    // An EXPLICIT pkgref always runs the full pipeline, so the slot callbacks
    // this host row hangs off actually fire; a bare install on a fresh lock
    // would take the fast path and reach no slot at all.
    let install = |user: &UserScratch| {
        user.vibe()
            .args(["install", "org.demo/plain", "--assume-yes", "--json"])
            .args(["--agent-mode", "agent"])
            .arg("--path")
            .arg(project.path())
            .output()
            .unwrap()
    };

    let parked = install(&user);
    assert_ok(&parked);
    let report: InstallReport =
        serde_json::from_value(documents(&parked.stdout).pop().unwrap()).unwrap();
    assert!(
        report.delegation.is_some(),
        "the host slot row parked: {report:?}",
    );

    remove_the_agent_row(project.path());
    let output = install(&user);
    assert_ok(&output);
    let report: InstallReport =
        serde_json::from_value(documents(&output.stdout).pop().unwrap()).unwrap();
    let row = report
        .contributions
        .iter()
        .find(|row| row.status == "cancelled")
        .unwrap_or_else(|| panic!("the cancellation is reported: {:?}", report.contributions));

    assert!(row.key.contains("produce-docs"), "{row:?}");
    assert_eq!(
        row.point, "<removed-slot-declaration>",
        "pre versus post is not recorded, so it is not claimed: {row:?}",
    );
    assert_eq!(row.handler, "agent");
    assert_no_invented_provenance(
        &row.provider,
        &row.tier,
        row.reference.as_deref(),
        &row.point,
    );
    assert!(
        row.slot_target.is_none(),
        "and no slot target is invented either: {row:?}",
    );

    let state: vibe_wire::generated::lifecycle_state::LifecycleState =
        toml::from_str(&fs::read_to_string(project.path().join(".vibe/lifecycle.toml")).unwrap())
            .unwrap();
    assert!(
        state
            .execution
            .values()
            .all(|row| row.status != ExecutionRecordStatus::Delegated),
        "no live delegated row survives: {state:?}",
    );
    assert!(state.run.slot_continuation.is_none());
    assert_eq!(provider.hits(), 0);
}
