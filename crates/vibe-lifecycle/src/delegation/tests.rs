//! The outbox law, judged at the unit seam: filename safety, an
//! uninjectable document, publication, and the narrow satisfied-resume
//! cleanup that proves ownership by recomputation.

use std::fs;
use std::path::Path;

use super::{
    DelegationError, OUTBOX_RELATIVE, TASK_CAP, cleanup_task, outbox_task_path, publish_task,
    task_filename,
};
use crate::agent::tests::support::{PROMPT, RecordingBackend, TWO_OUTPUTS, context, row};
use crate::agent::{PreparedAgent, prepare, system_prose, user_prose};
use vibe_wire::generated::lifecycle::e1::context::Context;

const RUN_ID: &str = "00112233445566778899aabbccddeeff";
const OTHER_RUN: &str = "ffeeddccbbaa99887766554433221100";

/// The unsafe-alphabet matrix the packet names: separators, `#`, colon,
/// wildcard, percent, trailing dot/space, device spellings — plus unicode and
/// backslashes for the Windows side of the law. Raw strings throughout, so a
/// reader counts the backslashes the key actually has.
#[test]
fn task_filenames_encode_the_unsafe_alphabet_and_carry_the_task_prefix() {
    let cases = [
        (
            "org.demo/tools#produce",
            "task-org.demo%2Ftools%23produce.md",
        ),
        ("a:b", "task-a%3Ab.md"),
        ("wild*card", "task-wild%2Acard.md"),
        ("100%", "task-100%25.md"),
        // A trailing `.`/space is re-encoded in the STEM, not hidden behind
        // the reserved `.md`: the stem itself may never end in one.
        ("trailing.", "task-trailing%2E.md"),
        ("trailing ", "task-trailing%20.md"),
        ("dot.dot.", "task-dot.dot%2E.md"),
        ("space here", "task-space%20here.md"),
        (r"back\slash", "task-back%5Cslash.md"),
        (r"back\\slash", "task-back%5C%5Cslash.md"),
        ("com²", "task-com%C2%B2.md"),
    ];
    for (key, expected) in cases {
        let name = task_filename(key).unwrap_or_else(|error| panic!("{key}: {error}"));
        assert_eq!(name, expected, "key `{key}`");
        vibe_safefs::ensure_safe_component(&name)
            .unwrap_or_else(|error| panic!("the shared law must accept `{name}`: {error}"));
    }
}

#[test]
fn device_spellings_can_never_become_the_basename() {
    for key in [
        "CON",
        "CON.md",
        "NUL.md",
        "COM1.json",
        "LPT9.log",
        "con².txt",
    ] {
        let name = task_filename(key).unwrap();
        assert!(
            !vibe_core::manifest::is_windows_device_name(&name),
            "`{key}` encoded to `{name}`, which the device table still accepts as a device"
        );
    }
}

/// The mandatory prefix AND suffix are reserved out of the component budget
/// before truncation, so even a 300-character key still names a `.md` file.
#[test]
fn an_overlong_key_stays_capped_still_ends_in_md_and_stays_distinct() {
    let long_a = "k".repeat(300);
    let long_b = "j".repeat(300);
    let a = task_filename(&long_a).unwrap();
    let b = task_filename(&long_b).unwrap();
    assert!(a.len() <= 128, "{} > 128: {a}", a.len());
    assert!(b.len() <= 128, "{} > 128: {b}", b.len());
    assert!(a.starts_with("task-"), "{a}");
    assert!(
        a.ends_with(".md"),
        "a truncated name is still a Markdown file: {a}"
    );
    assert!(b.ends_with(".md"), "{b}");
    assert_ne!(a, b, "the digest suffix must separate distinct long keys");
    let stem = &a["task-".len()..a.len() - ".md".len()];
    let digest = &stem[stem.len() - 16..];
    assert_eq!(&stem[stem.len() - 17..stem.len() - 16], "-", "{a}");
    assert!(digest.bytes().all(|b| b.is_ascii_hexdigit()), "{a}");
    assert_eq!(
        a,
        task_filename(&long_a).unwrap(),
        "the name is deterministic"
    );
    vibe_safefs::ensure_safe_component(&a).unwrap();
}

/// A truncated stem never keeps a half-copied `%XX`: the boundary backs off
/// rather than leaving a dangling `%` or `%A`.
#[test]
fn truncation_never_splits_a_percent_escape() {
    for width in 100..140 {
        let name = task_filename(&"#".repeat(width)).unwrap();
        let stem = &name["task-".len()..name.len() - ".md".len()];
        let body = &stem[..stem.len() - 17];
        assert_eq!(body.len() % 3, 0, "`%23` triples only: {name}");
        assert!(body.bytes().all(|b| b"%23".contains(&b)), "{name}");
    }
}

#[test]
fn an_invalid_run_id_refuses_before_anything_is_published() {
    let base = tempfile::tempdir().unwrap();
    let error = publish_task(
        Path::new("C:/never"),
        "nothex",
        "k",
        "create",
        &prepared(base.path(), TWO_OUTPUTS),
        &ctx(base.path(), TWO_OUTPUTS),
    )
    .unwrap_err();
    assert!(matches!(error, DelegationError::RunId { .. }), "{error}");
    assert!(matches!(
        outbox_task_path("nothex", "k").unwrap_err(),
        DelegationError::RunId { .. }
    ));
}

#[test]
fn publish_writes_the_frontmatter_contract_and_both_prose_sections() {
    let base = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let prepared = prepared(base.path(), TWO_OUTPUTS);
    let envelope = ctx(base.path(), TWO_OUTPUTS);
    let relative = publish_task(
        project.path(),
        RUN_ID,
        "org.demo/tools#produce",
        "create",
        &prepared,
        &envelope,
    )
    .unwrap();
    assert_eq!(
        relative,
        format!("{OUTBOX_RELATIVE}/{RUN_ID}/task-org.demo%2Ftools%23produce.md")
    );
    assert_eq!(
        relative,
        outbox_task_path(RUN_ID, "org.demo/tools#produce").unwrap(),
        "publication and the deterministic path law agree",
    );
    let document = fs::read_to_string(project.path().join(&relative)).unwrap();
    assert!(document.starts_with("---\n"), "{document}");
    assert!(document.contains(&format!("run: \"{RUN_ID}\"\n")));
    assert!(document.contains("execution: \"org.demo/tools#produce\"\n"));
    assert!(document.contains("phase: \"create\"\n"));
    assert!(document.contains("  - path: \"docs/guide.md\"\n"));
    assert!(document.contains("  - path: \"docs/reference.md\"\n"));
    let guide = document.find("docs/guide.md").unwrap();
    let reference = document.find("docs/reference.md").unwrap();
    assert!(guide < reference, "contract rows stay in declaration order");
    // The body is BOTH prose sections the paid call would have carried,
    // labelled and verbatim — not a paraphrase of either.
    assert!(
        document.contains(&system_prose()),
        "the exact system contract travels with the task: {document}"
    );
    assert!(
        document.contains(&user_prose(
            prepared.instructions(),
            &envelope,
            prepared.contract()
        )),
        "the exact request prose travels with the task: {document}"
    );
    assert!(document.contains("## System contract\n"), "{document}");
    assert!(document.contains("## Request\n"), "{document}");
}

/// Legal-but-hostile text is escaped by a real serializer: it can neither
/// break a row nor add a frontmatter field. The shared path law already
/// refuses quotes, backslashes and colons inside a declared path, so the
/// hostile alphabet a LEGAL path still carries is `#` (a YAML comment) and
/// flow-collection punctuation; the quote, backslash and colon arrive through
/// the execution key, which is not a path and is not filtered by that law.
#[test]
fn hostile_quotes_and_backslashes_cannot_break_or_add_a_frontmatter_row() {
    let hostile_outputs = r#"
outputs = [
  { path = "docs/#not-a-comment [x] {y}.md", kind = "file", accept = "non-empty file" },
  { path = "docs/- injected {a} [b].md", kind = "file", accept = "non-empty file" },
]
"#;
    let base = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let key = r#"org.demo/tools#say "hi"\injected: true"#;
    let relative = publish_task(
        project.path(),
        RUN_ID,
        key,
        "create",
        &prepared(base.path(), hostile_outputs),
        &ctx(base.path(), hostile_outputs),
    )
    .unwrap();
    let document = fs::read_to_string(project.path().join(&relative)).unwrap();
    let frontmatter = document
        .strip_prefix("---\n")
        .unwrap()
        .split_once("\n---\n")
        .unwrap()
        .0;
    assert_eq!(
        frontmatter.lines().collect::<Vec<_>>(),
        [
            &format!("run: \"{RUN_ID}\""),
            r#"execution: "org.demo/tools#say \"hi\"\\injected: true""#,
            r#"phase: "create""#,
            "outputs:",
            r#"  - path: "docs/#not-a-comment [x] {y}.md""#,
            r#"    kind: "file""#,
            r#"    accept: "non-empty file""#,
            r#"  - path: "docs/- injected {a} [b].md""#,
            r#"    kind: "file""#,
            r#"    accept: "non-empty file""#,
        ],
        "every string is escaped; no row is broken and no field is added",
    );
}

#[test]
fn republishing_replaces_the_task_atomically_at_the_same_path() {
    let base = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let first = publish(project.path(), RUN_ID, "k", base.path());
    let second = publish(project.path(), RUN_ID, "k", base.path());
    assert_eq!(first, second);
    let run_dir = project.path().join(&first);
    let run_dir = run_dir.parent().unwrap();
    let entries: Vec<_> = fs::read_dir(run_dir).unwrap().collect();
    assert_eq!(entries.len(), 1, "exactly the published task remains");
}

#[test]
fn an_occupied_outbox_ancestor_refuses_and_nothing_is_published() {
    let base = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir_all(project.path().join(".vibe/agentic")).unwrap();
    // `outbox` occupied by a regular FILE: the no-follow walk must refuse
    // rather than replace or follow the occupant.
    fs::write(project.path().join(".vibe/agentic/outbox"), "occupied").unwrap();
    let error = publish_task(
        project.path(),
        RUN_ID,
        "k",
        "create",
        &prepared(base.path(), TWO_OUTPUTS),
        &ctx(base.path(), TWO_OUTPUTS),
    )
    .unwrap_err();
    assert!(matches!(error, DelegationError::Publish { .. }), "{error}");
    assert_eq!(
        fs::read_to_string(project.path().join(".vibe/agentic/outbox")).unwrap(),
        "occupied",
        "the occupant is untouched",
    );
    assert!(
        !project.path().join(".vibe/agentic/outbox").is_dir(),
        "no run directory was created",
    );
}

#[test]
fn cleanup_removes_only_the_owned_task_and_never_a_non_empty_run_dir() {
    let base = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let task = publish(project.path(), RUN_ID, "k", base.path());
    // A bystander task in the SAME run directory must survive: cleanup prunes
    // only a PROVEN-EMPTY directory.
    let task_path = project.path().join(&task);
    let run_dir = task_path.parent().unwrap().to_path_buf();
    fs::write(run_dir.join("task-bystander.md"), "kept").unwrap();
    cleanup_task(project.path(), RUN_ID, "k", &task).unwrap();
    assert!(!task_path.exists(), "the owned task is gone");
    assert!(
        run_dir.join("task-bystander.md").exists(),
        "the bystander stays"
    );
    assert!(run_dir.exists(), "a non-empty run directory is not pruned");
}

#[test]
fn a_pruned_run_directory_disappears_once_proven_empty() {
    let base = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let task = publish(project.path(), RUN_ID, "solo", base.path());
    let task_path = project.path().join(&task);
    let run_dir = task_path.parent().unwrap().to_path_buf();
    cleanup_task(project.path(), RUN_ID, "solo", &task).unwrap();
    assert!(
        !run_dir.exists(),
        "the proven-empty run directory is pruned"
    );
    assert!(
        project.path().join(".vibe/agentic/outbox").exists(),
        "the outbox root itself stays"
    );
}

#[test]
fn an_absent_owned_task_is_named_honestly_not_treated_as_success() {
    let base = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let task = publish(project.path(), RUN_ID, "gone", base.path());
    fs::remove_file(project.path().join(&task)).unwrap();
    let notice = cleanup_task(project.path(), RUN_ID, "gone", &task).unwrap_err();
    assert!(
        notice.contains("already absent"),
        "the notice names the missing task: {notice}"
    );
}

/// Ownership is recomputation, not recognition: a path that merely LOOKS like
/// `.vibe/agentic/outbox/<hex>/task-*` is refused unless it is exactly the
/// task this `(run id, execution key)` pair owns.
#[test]
fn cleanup_refuses_every_path_this_run_and_key_do_not_own() {
    let base = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let task = publish(project.path(), RUN_ID, "k", base.path());
    let plausible = outbox_task_path(OTHER_RUN, "k").unwrap();
    let other_key = outbox_task_path(RUN_ID, "other").unwrap();
    for foreign in [
        "docs/guide.md",
        ".vibe/lifecycle.toml",
        ".vibe/agentic/outbox/not-hex/task-x.md",
        ".vibe/agentic/outbox/00112233445566778899aabbccddeeff/other-prefix.md",
        ".vibe/agentic/elsewhere/00112233445566778899aabbccddeeff/task-k.md",
        ".vibe/agentic/outbox/00112233445566778899aabbccddeeff/sub/task-k.md",
        plausible.as_str(),
        other_key.as_str(),
    ] {
        let error = cleanup_task(project.path(), RUN_ID, "k", foreign).unwrap_err();
        assert!(error.contains("is not the task run"), "{foreign}: {error}");
    }
    // A correct path under the WRONG run id is refused too: the recomputation
    // uses the run the caller claims, so a mismatch cannot delete.
    let error = cleanup_task(project.path(), OTHER_RUN, "k", &task).unwrap_err();
    assert!(error.contains("is not the task run"), "{error}");
    assert!(
        project.path().join(&task).is_file(),
        "no refusal removed the real task"
    );
}

#[test]
fn the_document_cap_refuses_before_any_write() {
    // The cap is on the COMPLETE document; a resolved prompt alone over it
    // must refuse before a single byte reaches the outbox.
    let huge = format!("# Prompt\n\n{}\n", "x".repeat(TASK_CAP));
    let backend = RecordingBackend::answering_prompt(&huge, "{}");
    let row = row(TWO_OUTPUTS, PROMPT);
    let scratch = tempfile::tempdir().unwrap();
    let base = context(scratch.path(), &row);
    let prepared = prepare(&backend, &row, &base).unwrap().unwrap();
    let project = tempfile::tempdir().unwrap();
    let error = publish_task(project.path(), RUN_ID, "k", "create", &prepared, &base).unwrap_err();
    assert!(matches!(error, DelegationError::TooLarge { .. }), "{error}");
    assert!(
        !project.path().join(".vibe/agentic").exists(),
        "the refusal wrote nothing at all"
    );
}

fn publish(project_root: &Path, run_id: &str, key: &str, base: &Path) -> String {
    publish_task(
        project_root,
        run_id,
        key,
        "create",
        &prepared(base, TWO_OUTPUTS),
        &ctx(base, TWO_OUTPUTS),
    )
    .unwrap()
}

fn prepared(root: &Path, outputs: &str) -> PreparedAgent {
    let backend = RecordingBackend::answering_prompt("Write the declared outputs.", "{}");
    let row = row(outputs, PROMPT);
    let base = context(root, &row);
    prepare(&backend, &row, &base).unwrap().unwrap()
}

fn ctx(root: &Path, outputs: &str) -> Context {
    let row = row(outputs, PROMPT);
    context(root, &row)
}
