//! `vibe create` through the real provider seam.
//!
//! The oracle is a loopback OpenAI-compatible endpoint that counts requests,
//! reached through the production `vibe-llm` transport rather than a test-only
//! hook: keyless HTTP is legal on literal loopback, so this exercises the
//! shipped configuration, endpoint policy and provider mapping end to end.
//! Every negative case asserts the counter as well as the tree — "refused
//! before spend" and "wrote nothing" are measurements, not claims.

mod common;

use std::fs;
use std::path::Path;

use common::UserScratch;
use common::agent_provider::{MockProvider, configure_provider};
use vibe_wire::generated::lifecycle_report::LifecycleReport;

const PROMPT_ADDRESS: &str = "spec://org.demo/demo/common/agent-prompt#root";

/// The declared contract used by every fixture below.
const CONTRACT: &str = r#"
[[extension]]
id = "produce-docs"
point = "phase:create"
handler = { kind = "agent", prompt = "spec://org.demo/demo/common/agent-prompt#root" }
config.outputs = [
  { path = "docs/guide.md", kind = "file", accept = "non-empty file" },
  { path = "docs/nested/reference.md", kind = "file", accept = "non-empty file" },
]

[[extension]]
id = "after-agent"
point = "phase:create"
handler = { kind = "builtin", name = "log" }
config = { message = "SENTINEL-AFTER-AGENT" }
"#;

/// The provider answer that satisfies [`CONTRACT`].
const GOOD_RESULT: &str = concat!(
    r#"{"outputs":["#,
    r#"{"path":"docs/guide.md","content":"guide body\n"},"#,
    r#"{"path":"docs/nested/reference.md","content":"reference body\n"}"#,
    r#"]}"#
);

fn project(extensions: &str) -> (UserScratch, tempfile::TempDir) {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.vibe()
        .args(["init", "--no-registry", "--author", "Agent"])
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();

    // The host must be a package coordinate for its own `spec://` prompt to
    // resolve at all (B-031: the host is addressed by `<group>/<name>`).
    let manifest_path = project.path().join("vibe.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let mut patched = String::new();
    for line in manifest.lines() {
        patched.push_str(line);
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

/// The `[project]` name `vibe init` derives is the temp directory name, so the
/// address in [`CONTRACT`] would not match it. Rename the project instead of
/// guessing the directory name.
fn rename_project_to_demo(project: &Path) {
    let manifest_path = project.join("vibe.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let patched = manifest
        .lines()
        .map(|line| {
            if line.starts_with("name = ") && !line.contains("demo") {
                "name = \"demo\"".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(manifest_path, patched).unwrap();
}

fn create(user: &UserScratch, project: &Path, force: bool) -> std::process::Output {
    let mut command = user.vibe();
    command
        .args(["create", "--json", "--assume-yes", "--path"])
        .arg(project);
    if force {
        command.arg("--force");
    }
    command.output().unwrap()
}

fn report(bytes: &[u8]) -> LifecycleReport {
    let documents: Vec<serde_json::Value> = serde_json::Deserializer::from_slice(bytes)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap();
    serde_json::from_value(documents.last().expect("one report document").clone()).unwrap()
}

fn agent_row(
    report: &LifecycleReport,
) -> &vibe_wire::generated::lifecycle_report::LifecycleContributionReport {
    report
        .contributions
        .iter()
        .find(|row| row.key.ends_with("produce-docs"))
        .expect("the agent contribution is reported")
}

#[test]
fn create_writes_the_declared_outputs_then_is_fresh_and_force_calls_again() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(CONTRACT);
    rename_project_to_demo(project.path());
    configure_provider(&user, &provider.endpoint());

    let output = create(&user, project.path(), false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let first = report(&output.stdout);
    assert_eq!(agent_row(&first).status, "ok");
    assert_eq!(provider.hits(), 1, "exactly one paid call");
    assert_eq!(
        fs::read_to_string(project.path().join("docs/guide.md")).unwrap(),
        "guide body\n"
    );
    assert_eq!(
        fs::read_to_string(project.path().join("docs/nested/reference.md")).unwrap(),
        "reference body\n",
        "a nested declared output creates its ancestors"
    );
    let message = agent_row(&first).message.as_deref().unwrap_or_default();
    assert!(
        message.contains("usage prompt=42 completion=9 total=51"),
        "provider-independent counters reach the report: {message}"
    );
    assert!(
        message.contains("the set was not one transaction"),
        "the report never claims cross-file atomicity: {message}"
    );
    assert!(
        first
            .contributions
            .iter()
            .any(|row| row.key.ends_with("after-agent") && row.status == "ok"),
        "a successful agent contribution does not stop the phase"
    );

    let output = create(&user, project.path(), false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(agent_row(&report(&output.stdout)).status, "fresh");
    assert_eq!(provider.hits(), 1, "a fresh execution sends no request");

    let output = create(&user, project.path(), true);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(agent_row(&report(&output.stdout)).status, "ok");
    assert_eq!(provider.hits(), 2, "`--force` pays once more");
}

/// PROP-054 `##PHASE-FINGERPRINT` names the prompt *documents* as create's
/// material. Editing one is an ordinary author action, and the run after it
/// must be a real run — a `fresh` here would silently serve stale outputs.
#[test]
fn a_host_prompt_edited_between_runs_reruns_instead_of_fresh_skipping() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(CONTRACT);
    rename_project_to_demo(project.path());
    configure_provider(&user, &provider.endpoint());
    let prompt = project
        .path()
        .join("vibevm/vibespecs/common/agent-prompt.md");

    let output = create(&user, project.path(), false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(provider.hits(), 1);
    assert_eq!(
        agent_row(&report(&create(&user, project.path(), false).stdout)).status,
        "fresh"
    );
    assert_eq!(provider.hits(), 1, "unchanged inputs stay fresh");

    fs::write(
        &prompt,
        "# Documentation prompt {#root}\n\nWrite the declared documentation files, tersely.\n",
    )
    .unwrap();
    let output = create(&user, project.path(), false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        agent_row(&report(&output.stdout)).status,
        "ok",
        "an edited prompt document must rerun"
    );
    assert_eq!(provider.hits(), 2, "and must actually call the provider");
}

/// Freshness for an agent row is not "the inputs are unchanged" alone: the
/// declared outputs are the work. A deleted or emptied one is missing work,
/// never phantom-fresh — and the probe that decides this reads no credential.
#[test]
fn a_deleted_or_emptied_output_reruns_instead_of_reporting_phantom_fresh() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(CONTRACT);
    rename_project_to_demo(project.path());
    configure_provider(&user, &provider.endpoint());

    assert!(create(&user, project.path(), false).status.success());
    assert_eq!(provider.hits(), 1);

    fs::remove_file(project.path().join("docs/guide.md")).unwrap();
    let output = create(&user, project.path(), false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        agent_row(&report(&output.stdout)).status,
        "ok",
        "a deleted declared output must rerun"
    );
    assert_eq!(provider.hits(), 2);
    assert_eq!(
        fs::read_to_string(project.path().join("docs/guide.md")).unwrap(),
        "guide body\n",
        "and the rerun really restores it"
    );

    fs::write(project.path().join("docs/nested/reference.md"), "").unwrap();
    let output = create(&user, project.path(), false);
    assert!(output.status.success());
    assert_eq!(
        agent_row(&report(&output.stdout)).status,
        "ok",
        "an emptied declared output fails its acceptance and must rerun"
    );
    assert_eq!(provider.hits(), 3);
}

#[test]
fn a_project_without_an_agent_contribution_never_reaches_the_provider() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(
        "\n[[extension]]\nid = \"plain\"\npoint = \"phase:create\"\n\
         handler = { kind = \"builtin\", name = \"log\" }\nconfig = { message = \"ALGORITHMIC\" }\n",
    );
    rename_project_to_demo(project.path());
    configure_provider(&user, &provider.endpoint());

    let output = create(&user, project.path(), false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        provider.hits(),
        0,
        "an ordinary lifecycle stays fully algorithmic"
    );
}

#[test]
fn a_missing_provider_fails_the_agent_contribution_with_remediation() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(CONTRACT);
    rename_project_to_demo(project.path());
    // Deliberately no `[llm]` anywhere.

    let output = create(&user, project.path(), false);
    assert!(
        !output.status.success(),
        "a selected agent contribution is never silently skipped"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no LLM provider is configured"),
        "the remediation must reach the operator: {stderr}"
    );
    assert!(
        stderr.contains("configure user `[llm]`") && stderr.contains("agent host"),
        "both remediations are named: {stderr}"
    );
    assert!(!project.path().join("docs").exists());
    assert_eq!(provider.hits(), 0);
}

#[test]
fn a_result_that_breaks_the_contract_writes_nothing_and_stops_the_phase() {
    let provider =
        MockProvider::serving(r#"{"outputs":[{"path":"docs/guide.md","content":"only one"}]}"#);
    let (user, project) = project(CONTRACT);
    rename_project_to_demo(project.path());
    configure_provider(&user, &provider.endpoint());

    let output = create(&user, project.path(), false);
    assert!(!output.status.success());
    assert_eq!(provider.hits(), 1, "the call happened and then was refused");
    assert!(
        !project.path().join("docs").exists(),
        "an incomplete result writes nothing at all"
    );
    let failed = report(&output.stdout);
    assert!(!failed.ok);
    assert_eq!(agent_row(&failed).status, "fail");
    assert!(
        !failed
            .contributions
            .iter()
            .any(|row| row.key.ends_with("after-agent")),
        "a failing agent contribution stops every later contribution"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("SENTINEL-AFTER-AGENT") && !stderr.contains("SENTINEL-AFTER-AGENT"),
        "the later contribution must not have run"
    );
}

#[test]
fn a_prompt_outside_the_declaring_provider_refuses_before_the_call() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(&CONTRACT.replace(
        PROMPT_ADDRESS,
        "spec://org.other/elsewhere/common/agent-prompt#root",
    ));
    rename_project_to_demo(project.path());
    configure_provider(&user, &provider.endpoint());

    let output = create(&user, project.path(), false);
    assert!(!output.status.success());
    assert_eq!(provider.hits(), 0, "a foreign prompt costs nothing");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("escapes provider"),
        "the refusal names the escape: {stderr}"
    );
    assert!(!project.path().join("docs").exists());
}

#[test]
fn an_unknown_acceptance_predicate_refuses_before_the_call() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(&CONTRACT.replace("non-empty file", "exists"));
    rename_project_to_demo(project.path());
    configure_provider(&user, &provider.endpoint());

    let output = create(&user, project.path(), false);
    assert!(!output.status.success());
    assert_eq!(provider.hits(), 0, "an unknown predicate costs nothing");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("declared output contract is invalid"),
        "the refusal names the contract: {stderr}"
    );
    assert!(!project.path().join("docs").exists());
}

#[test]
fn a_declared_output_escaping_the_project_refuses_before_the_call() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(&CONTRACT.replace("docs/guide.md", "../escape.md"));
    rename_project_to_demo(project.path());
    configure_provider(&user, &provider.endpoint());

    let outside = project.path().parent().unwrap().join("escape.md");
    let output = create(&user, project.path(), false);
    assert!(!output.status.success());
    assert_eq!(provider.hits(), 0);
    assert!(!outside.exists(), "nothing outside the project was created");
}

#[test]
fn an_unresolvable_prompt_refuses_before_the_call() {
    let provider = MockProvider::serving(GOOD_RESULT);
    let (user, project) = project(CONTRACT);
    rename_project_to_demo(project.path());
    configure_provider(&user, &provider.endpoint());
    fs::remove_file(
        project
            .path()
            .join("vibevm/vibespecs/common/agent-prompt.md"),
    )
    .unwrap();

    let output = create(&user, project.path(), false);
    assert!(!output.status.success());
    assert_eq!(provider.hits(), 0, "an unreachable prompt costs nothing");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not resolve inside its own provider instance"),
        "the refusal names the address: {stderr}"
    );
    assert!(!project.path().join("docs").exists());
}
