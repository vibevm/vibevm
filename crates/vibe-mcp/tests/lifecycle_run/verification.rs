//! `lifecycle_run({phase:"verify"})` returns the SAME member the CLI does
//! (R7.5 P2/A5b, PROP-054 `##EVIDENCE-WIRE-AND-SURFACES`).
//!
//! The hosted surface differs from the CLI in exactly two named places — it
//! ignores `emit_report`, and its text channel is MCP-native guidance — and
//! the evidence member is in neither. So the assertions here are the same
//! assertions the CLI e2e makes, which is the point: two projections of one
//! generated document, neither rebuilding it.

use std::fs;
use std::path::Path;

use vibe_wire::behaviour::verification_evidence::validate;
use vibe_wire::generated::shared::EvidenceStatus;

use super::support::{append, context, project, report, run};

const DECLARED_BUILD: &str = r#"
[[extension]]
id = "declared-build"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "DECLARED-BUILD" }
inputs = ["data/**"]
"#;

fn with_declared_input(extra: &str) -> tempfile::TempDir {
    let dir = project(&format!("{DECLARED_BUILD}{extra}"));
    fs::create_dir_all(dir.path().join("data")).unwrap();
    fs::write(dir.path().join("data/a.txt"), "one").unwrap();
    dir
}

/// A create contribution that rewrites a MEASURED build input inside the same
/// invocation — the uninterrupted stale path, hosted.
fn mutating_create(root: &Path) {
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::write(
        root.join("scripts/touch.sh"),
        "printf 'two' > data/a.txt\n\
         printf '%s' '{\"artifacts\":[],\"envelope\":1,\"status\":\"ok\",\"tasks\":[]}' > \"$VIBE_REPLY\"\n",
    )
    .unwrap();
    fs::write(
        root.join("scripts/touch.ps1"),
        "'two' | Set-Content -NoNewline data/a.txt\n\
         '{\"artifacts\":[],\"envelope\":1,\"status\":\"ok\",\"tasks\":[]}' | Set-Content -NoNewline $env:VIBE_REPLY\n",
    )
    .unwrap();
    append(
        root,
        "\n[[extension]]\nid = \"mutating-create\"\npoint = \"phase:create\"\n\
         handler = { kind = \"script\", base = \"scripts/touch\" }\n",
    );
}

#[test]
fn hosted_verify_returns_a_valid_matched_member() {
    let dir = with_declared_input("");
    let ctx = context(dir.path());

    let output = run(&ctx, "verify").expect("the hosted verify executes");
    let root = report(&output);
    assert!(root.ok);
    let member = root
        .verification
        .clone()
        .unwrap_or_else(|| panic!("the hosted surface owes the member: {root:?}"));
    validate(&member).expect("what the hosted surface published is a valid member");
    assert_eq!(member.status, EvidenceStatus::Matched);
    assert_eq!(member.run.requested, "verify");
}

/// The hosted twin of the CLI stale stop: the structured output is still the
/// generated root, still carrying the exact comparison, while the text channel
/// carries the failure.
#[test]
fn a_hosted_stale_stop_still_returns_its_member() {
    let dir = with_declared_input("");
    mutating_create(dir.path());
    let ctx = context(dir.path());

    let output = run(&ctx, "verify").expect("an executed failure is still a tool result");
    let root = report(&output);
    assert!(!root.ok, "the command's own axis is false");
    let member = root
        .verification
        .clone()
        .unwrap_or_else(|| panic!("a stop must carry its comparison: {root:?}"));
    validate(&member).expect("a stopping member is still a valid member");
    assert_eq!(member.status, EvidenceStatus::Stale);
}

/// A hosted run that never asked for verify omits the KEY entirely.
#[test]
fn a_hosted_build_run_omits_the_member() {
    let dir = with_declared_input("");
    let ctx = context(dir.path());

    let output = run(&ctx, "build").expect("the hosted build executes");
    assert!(report(&output).verification.is_none());
    let structured = serde_json::to_string(output.structured()).unwrap();
    assert!(
        !structured.contains("verification"),
        "an absent member is an absent key: {structured}",
    );
}
