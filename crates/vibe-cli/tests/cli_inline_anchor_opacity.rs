//! Candidate-built CLI checks use isolated settings and leave the project intact.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#PARSE-FACT-ANCHORS");
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use vibe_test_support::UserScratch;

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut result = BTreeMap::new();
    let mut pending = vec![root.to_owned()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let key = path.strip_prefix(root).unwrap().to_owned();
            if path.is_dir() {
                result.insert(key, None);
                pending.push(path);
            } else {
                result.insert(key, Some(std::fs::read(path).unwrap()));
            }
        }
    }
    result
}

fn check(body: &str, success: bool, duplicate: bool) {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("doc.md"), body).unwrap();
    std::fs::write(
        project.path().join("progress.toml"),
        "include = ['doc.md']\n",
    )
    .unwrap();
    let before = snapshot(project.path());
    let user = UserScratch::new();
    for family in ["progress", "facts"] {
        let mut command = user.vibe();
        command
            .args([
                family,
                "check",
                "--no-cache",
                "--json",
                "--offline",
                "--unattended",
                "--path",
            ])
            .arg(project.path());
        let output = command.output().unwrap();
        eprintln!(
            "{command:?}: exit={:?}\nstdout={}\nstderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(snapshot(project.path()), before, "check modified project");
        assert_eq!(output.status.success(), success, "{output:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).contains("DuplicateId"),
            duplicate
        );
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#PARSE-FACT-ANCHORS"
)]
fn quoted_definition_is_literal() {
    check(
        "# H {#h}\n\n@fact:SAME definition\n\n@fact:OTHER see `@fact:SAME` and ``@fact/code:SAME``\n",
        true,
        false,
    );
}

#[test]
fn actual_duplicate_is_still_an_error() {
    check(
        "# H {#h}\n\n@fact:SAME one\n\n@fact:SAME two\n",
        false,
        true,
    );
}

#[test]
fn legacy_reference_is_still_literal() {
    check(
        "# H {#h}\n\n@fact:SAME definition\n\n@fact:OTHER see `##SAME`\n",
        true,
        false,
    );
}

#[test]
fn fenced_example_is_still_literal() {
    check(
        "# H {#h}\n\n@fact:SAME definition\n\n```markdown\n@fact:SAME example\n```\n",
        true,
        false,
    );
}

/// Schema 3 could retain a false duplicate over these unchanged source bytes.
/// Check must reparse without rewriting either cache; scan may refresh only
/// derived state while retaining the tracked campaign verdicts.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#PARSE-FACT-ANCHORS"
)]
fn normal_checks_discard_old_parse_payload_and_scan_keeps_verdicts() {
    use progress_core::{
        cache::Cache,
        doc::{Issue, IssueCode, Severity},
        parse::parse_document,
        rollup::rollup_doc,
        sidecar,
    };

    for real_duplicate in [false, true] {
        let project = tempfile::tempdir().unwrap();
        let root = project.path().canonicalize().unwrap();
        let body = if real_duplicate {
            "# H {#h}\n\n@fact:SAME first\n\n@fact:SAME second\n"
        } else {
            "# H {#h}\n\n@fact:SAME definition\n\n@fact:OTHER see `@fact:SAME`\n"
        };
        std::fs::write(root.join("doc.md"), body).unwrap();
        std::fs::write(root.join("progress.toml"), "include = ['doc.md']\n").unwrap();
        let run = root.join("campaigns/warm/run");
        std::fs::create_dir_all(&run).unwrap();
        let user = UserScratch::new();
        let mut old_doc = parse_document("doc.md", body);
        if !real_duplicate {
            assert_eq!(old_doc.error_count(), 0);
            // This explicit compatibility fixture models schema 3's former
            // raw-token duplicate scan; it is not a new parser implementation.
            old_doc.issues.push(Issue {
                severity: Severity::Error,
                line: 5,
                code: IssueCode::DuplicateId,
                message: "fact id `@fact:SAME` is defined twice in this file: lines 3 and 5".into(),
            });
        }
        assert_eq!(old_doc.error_count(), 1);
        let mut cache = Cache::default();
        cache.upsert(&old_doc, &rollup_doc(&old_doc));
        let verdicts =
            serde_json::json!({"SAME":{"v":"accepted","evidence":"retained-owner-evidence"}});
        cache
            .files
            .get_mut("doc.md")
            .unwrap()
            .campaign
            .insert("verdicts".into(), verdicts.clone());
        let cache_path = run.join("cache.json");
        cache.store(&cache_path).unwrap();
        let bucket = sidecar::resolve_dir(&root, None, Some(&user.settings), None, "warm").unwrap();
        std::fs::create_dir_all(&bucket).unwrap();
        let payload_path = bucket.join(sidecar::PAYLOAD_FILE);
        std::fs::write(
            &payload_path,
            serde_json::to_vec(&serde_json::json!({"schema":3,"docs":{"doc.md":old_doc}})).unwrap(),
        )
        .unwrap();
        let before = snapshot(&root);
        let settings_before = snapshot(&user.settings);
        for family in ["progress", "facts"] {
            let output = user
                .vibe()
                .args([
                    family,
                    "check",
                    "--campaign",
                    "warm",
                    "--json",
                    "--offline",
                    "--unattended",
                    "--path",
                ])
                .arg(&root)
                .output()
                .unwrap();
            eprintln!("warm schema3 {family} duplicate={real_duplicate}: {output:?}");
            assert_eq!(snapshot(&root), before, "check rewrote tracked state");
            assert_eq!(
                snapshot(&user.settings),
                settings_before,
                "check rewrote sidecar"
            );
            assert_eq!(output.status.success(), !real_duplicate, "{output:?}");
            assert_eq!(
                String::from_utf8_lossy(&output.stdout).contains("DuplicateId"),
                real_duplicate
            );
        }
        let output = user
            .vibe()
            .args([
                "progress",
                "scan",
                "--campaign",
                "warm",
                "--json",
                "--offline",
                "--unattended",
                "--path",
            ])
            .arg(&root)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let refreshed = Cache::load(&cache_path).unwrap();
        assert_eq!(refreshed.schema, 2, "tracked cache format is unchanged");
        assert_eq!(refreshed.files["doc.md"].campaign["verdicts"], verdicts);
        assert_eq!(std::fs::read(root.join("doc.md")).unwrap(), body.as_bytes());
        let payload: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&payload_path).unwrap()).unwrap();
        assert_eq!(payload["schema"], 4);
        let current = sidecar::Payloads::load(Some(bucket));
        let doc = refreshed
            .cached_doc("doc.md", &refreshed.files["doc.md"].content_hash, &current)
            .expect("current payload remains reusable");
        assert_eq!(doc.error_count(), usize::from(real_duplicate));
    }
}
