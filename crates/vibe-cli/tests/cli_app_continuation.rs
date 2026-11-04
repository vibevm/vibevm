//! Public continuation through the actual CLI and its retained operation owner.
#![cfg(all(test, target_os = "linux"))]

mod common;

use common::app_registry::{AppRegistry, json, ok};
use common::fake_agent::refused;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Output,
};

const VERSION: &str = "1.0.0";
const BUILD: &str = r#"
[[installer_options.steps]]
id = 'build'
title = 'Build the payload'
kind = 'recipe'
interpreter = 'posix_sh'
script = 'build.sh'
"#;
const SKIP: &str = r#"
[[installer_options.features]]
id = 'unused'
name = 'Unused preparation'
[[installer_options.steps]]
id = 'skip'
title = 'Skipped preparation'
kind = 'shell'
when = { feature = 'unused' }
command = 'exit 91'
"#;

fn key(name: &str) -> String {
    format!("org.example/{name}@{VERSION}")
}

fn agent(phase: &str) -> String {
    format!(
        "\n[[installer_options.steps]]\nid='approve'\ntitle='Hosted approval'\n\
         kind='agent'\nphase='{phase}'\nrequired=true\n\
         prompt='Inspect this bounded fixture and return the published result protocol.'\n"
    )
}

// Every effect counter is outside package/source/work snapshots. No source edit
// is needed to answer a hosted task or permit the optional finish retry.
fn package(r: &AppRegistry, name: &str, body: &str) -> PathBuf {
    let p = r.package(name, VERSION, name, &format!("result='owned'\n{body}"));
    fs::write(
        p.join("build.sh"),
        format!(
            "set -e\nprintf 'build\\n' >> '{}'\nmkdir -p \"$VIBE_OUT\"\n\
             printf 'payload\\n' > \"$VIBE_OUT/receipt\"\n",
            r.root.path().join("build-count").display()
        ),
    )
    .unwrap();
    p
}

fn count(r: &AppRegistry, name: &str) -> usize {
    fs::read_to_string(r.root.path().join(name))
        .unwrap_or_default()
        .lines()
        .count()
}

// Same child CLI composition as AppRegistry; hosted posture must survive its
// usual ambient-agent cleanup. sh is registered but must never be spawned in
// agent mode: it has no valid application task executor arguments.
fn hosted(r: &AppRegistry, args: &[&str]) -> Output {
    let mut command = r.user.vibe();
    command
        .current_dir(r.root.path())
        .env("HOME", r.home())
        .env("VIBE_AGENT_COMMAND", "sh")
        .env("VIBE_AGENT_ARGS", "[]")
        .env_remove("VIBE_INVOKED_BY")
        .env_remove("VIBE_APP_USE")
        .args(args)
        .args([
            "--agent-mode",
            "agent",
            "--unattended",
            "--offline",
            "--json",
        ]);
    command.output().unwrap()
}

fn install(r: &AppRegistry, name: &str, extra: &[&str]) -> Output {
    let registry = r.registry();
    let exact = format!("org.example/{name}@={VERSION}");
    let mut args = vec![
        "install",
        "-g",
        exact.as_str(),
        "--registry",
        registry.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    hosted(r, &args)
}

fn error(output: &Output, exit: i32, expected: &str) -> Value {
    let (code, value) = refused(output);
    assert_eq!(code, Some(exit), "unexpected exit status");
    assert_eq!(value["code"], expected, "wrong public refusal category");
    value
}

fn work(r: &AppRegistry, name: &str) -> Value {
    let listing = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let expected = key(name);
    let rows: Vec<_> = listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["key"] == expected)
        .collect();
    assert_eq!(rows.len(), 1, "one retained work entry for the exact key");
    let row = (*rows[0]).clone();
    assert_eq!(row["operation"]["target"]["key"], expected);
    assert!(row["operation"]["target"]["home"].as_str().is_some());
    assert!(row["operation"]["target"]["operation"].as_str().is_some());
    row
}

fn operation(row: &Value) -> &str {
    row["operation"]["target"]["operation"].as_str().unwrap()
}

fn work_path(row: &Value) -> PathBuf {
    PathBuf::from(row["path"].as_str().unwrap())
}

fn resume(r: &AppRegistry, target: &Value, extra: &[&str]) -> Output {
    let mut args = vec![
        "app",
        "resume",
        target["key"].as_str().unwrap(),
        "--operation",
        target["operation"].as_str().unwrap(),
        "-g",
    ];
    args.extend_from_slice(extra);
    hosted(r, &args)
}

fn answer(parked: &Value) {
    // `run` is the existing public delegated-error path, also printed in the
    // hosted task. This never parses or constructs private steps2 state.
    let result = Path::new(parked["run"].as_str().unwrap()).join("result.json");
    assert!(result.parent().unwrap().is_dir());
    fs::write(
        result,
        r#"{"protocol":"vibe-agent-hook-result/1","status":"ok","message":"fixture approved"}"#,
    )
    .unwrap();
}

fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        if !path.exists() {
            return;
        }
        let metadata = fs::symlink_metadata(path).unwrap();
        if metadata.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                walk(root, &entry.unwrap().path(), result);
            }
        } else if metadata.is_file() {
            assert!(
                metadata.len() <= 4 * 1024 * 1024,
                "fixture file exceeds audit bound"
            );
            assert!(result.len() < 2048, "fixture tree exceeds audit bound");
            result.insert(
                path.strip_prefix(root).unwrap().into(),
                fs::read(path).unwrap(),
            );
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

fn unchanged(path: &Path, before: &BTreeMap<PathBuf, Vec<u8>>) {
    let after = tree(path);
    assert!(
        before == &after,
        "refusal changed file membership or bytes under {}",
        path.display()
    );
}

fn absent_secret(output: &Output, secret: &str) {
    for bytes in [&output.stdout, &output.stderr] {
        assert!(
            !bytes.windows(secret.len()).any(|w| w == secret.as_bytes()),
            "supplied secret reached CLI output"
        );
    }
}

fn framework_has_no_secret(r: &AppRegistry, secret: &str) {
    // Includes index, kept work, app-runs, source snapshots and hosted outbox.
    // Expected secret bytes are outside settings, in a test-owned oracle file.
    for (path, bytes) in tree(r.settings()) {
        assert!(
            !bytes.windows(secret.len()).any(|w| w == secret.as_bytes()),
            "supplied answer persisted in {}",
            path.display()
        );
    }
}

fn step(record: &Value, id: &str) -> Value {
    record["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap()
        .clone()
}

fn installed(r: &AppRegistry, name: &str) -> Value {
    r.index()["installations"][key(name)].clone()
}

fn payload(r: &AppRegistry, name: &str) -> PathBuf {
    r.settings()
        .join(format!("opt/apps/{name}/org.example.{name}@{VERSION}"))
}

#[test]
fn hosted_prepare_requires_optional_secret_resupply_without_argv_replay_or_prefix_repeat() {
    let r = AppRegistry::new();
    let name = "continuation-secret";
    let secret = "SYN_CONTINUATION_SUPPLIED_8d7f3a";
    let oracle = r.root.path().join("expected-secret");
    fs::write(&oracle, secret).unwrap();
    let body = format!(
        "{BUILD}{}\n[[installer_options.steps]]\nid='consume'\ntitle='Consume supplied secret'\n\
         kind='recipe'\ninterpreter='posix_sh'\nscript='consume.sh'\n\
         [[installer_options.configuration.parameters]]\nid='token'\nname='Token'\n\
         type='string'\nsecret=true\nrequired=false\ndefault='AUTHORED_PUBLIC_DEFAULT'\n",
        agent("prepare")
    );
    let p = package(&r, name, &body);
    fs::write(
        p.join("consume.sh"),
        format!(
            "set -e\nprintf '%s' \"$VIBE_PARAM_TOKEN\" | cmp -s - '{}'\n\
         printf 'authorized\\n' >> '{}'\n",
            oracle.display(),
            r.root.path().join("authorized-count").display()
        ),
    )
    .unwrap();
    let setter = format!("token={secret}");
    let first = install(&r, name, &["--set", &setter, "--keep-work"]);
    absent_secret(&first, secret);
    let parked = error(&first, 75, "delegated");
    let row = work(&r, name);
    let target = row["operation"]["target"].clone();
    let command = parked["resume"].as_str().unwrap();
    assert!(command.contains("app resume") && command.contains(&key(name)));
    assert!(command.contains("--operation") && command.contains(operation(&row)));
    assert!(command.split_whitespace().any(|word| word == "-g"));
    for forbidden in ["--set", "--registry", "--configuration-input", "install -g"] {
        assert!(
            !command.contains(forbidden),
            "resume command replays old invocation inputs"
        );
    }
    assert_eq!(count(&r, "build-count"), 1);
    assert_eq!(count(&r, "authorized-count"), 0);
    assert!(!r.settings().join("applications/index.json").exists());
    framework_has_no_secret(&r, secret);
    answer(&parked);
    let checkpoint = tree(&work_path(&row));
    let missing = resume(&r, &target, &[]);
    absent_secret(&missing, secret);
    // Common optional-secret resupply uses the frozen unresolved category.
    error(&missing, 2, "configuration_unresolved");
    unchanged(&work_path(&row), &checkpoint);
    assert_eq!(count(&r, "build-count"), 1);
    assert_eq!(count(&r, "authorized-count"), 0);
    let completed = resume(&r, &target, &["--set", &setter]);
    absent_secret(&completed, secret);
    ok(&completed);
    assert_eq!(count(&r, "build-count"), 1);
    assert_eq!(count(&r, "authorized-count"), 1);
    assert_eq!(
        fs::read(payload(&r, name).join("receipt")).unwrap(),
        b"payload\n"
    );
    assert_eq!(step(&installed(&r, name), "consume")["outcome"], "done");
    framework_has_no_secret(&r, secret);
}

#[test]
fn skipped_prepare_does_not_rewind_a_parked_finish_for_exact_or_manual_resume() {
    for exact in [true, false] {
        let r = AppRegistry::new();
        let name = "continuation-finish";
        let body = format!(
            "{SKIP}{BUILD}{}\n[[installer_options.steps]]\nid='verify'\ntitle='Verify placed payload'\nkind='shell'\nphase='finish'\ncommand='test -f \"$VIBE_APP_ROOT/receipt\" && echo finish >> \"{}\"'\n",
            agent("finish"),
            r.root.path().join("finish-count").display()
        );
        package(&r, name, &body);
        let parked = error(&install(&r, name, &["--keep-work"]), 75, "delegated");
        let row = work(&r, name);
        let record = installed(&r, name);
        assert_eq!(step(&record, "skip")["outcome"], "skipped");
        assert_eq!(step(&record, "build")["outcome"], "done");
        assert_eq!(step(&record, "approve")["outcome"], "delegated");
        let placed = tree(&payload(&r, name));
        assert_eq!(
            fs::read(payload(&r, name).join("receipt")).unwrap(),
            b"payload\n"
        );
        assert_eq!(count(&r, "build-count"), 1);
        answer(&parked);
        let completed = if exact {
            resume(&r, &row["operation"]["target"], &[])
        } else {
            install(&r, name, &["--resume", "--keep-work"])
        };
        ok(&completed);
        assert_eq!(count(&r, "build-count"), 1, "prepare must stay settled");
        assert_eq!(count(&r, "finish-count"), 1);
        unchanged(&payload(&r, name), &placed);
        assert_eq!(step(&installed(&r, name), "approve")["outcome"], "done");
    }
}

#[test]
fn stale_and_wrong_home_exact_targets_cannot_replace_the_current_checkpoint() {
    let r = AppRegistry::new();
    let name = "continuation-generation";
    package(&r, name, &format!("{BUILD}{}", agent("prepare")));
    error(&install(&r, name, &["--keep-work"]), 75, "delegated");
    let old = work(&r, name);
    // Deliberate fresh replacement at the same key, not an exact continuation.
    error(&install(&r, name, &["--keep-work"]), 75, "delegated");
    let new = work(&r, name);
    assert_ne!(operation(&old), operation(&new));
    assert_eq!(count(&r, "build-count"), 2);
    let snapshot = tree(&work_path(&new));
    error(
        &resume(&r, &old["operation"]["target"], &[]),
        2,
        "resume_mismatch",
    );
    unchanged(&work_path(&new), &snapshot);
    assert_eq!(count(&r, "build-count"), 2);

    let other = AppRegistry::new();
    package(&other, name, &format!("{BUILD}{}", agent("prepare")));
    error(&install(&other, name, &["--keep-work"]), 75, "delegated");
    let other_row = work(&other, name);
    assert_ne!(
        new["operation"]["target"]["home"],
        other_row["operation"]["target"]["home"]
    );
    let other_snapshot = tree(&work_path(&other_row));
    error(
        &resume(&other, &new["operation"]["target"], &[]),
        2,
        "resume_mismatch",
    );
    unchanged(&work_path(&other_row), &other_snapshot);
    unchanged(&work_path(&new), &snapshot);
    assert_eq!(count(&other, "build-count"), 1);
}

#[test]
fn optional_finish_failure_is_completed_but_deliberate_manual_retry_gets_a_new_generation() {
    let r = AppRegistry::new();
    let name = "continuation-optional";
    let gate = r.root.path().join("allow-finish");
    let body = format!(
        "{BUILD}\n[[installer_options.steps]]\nid='optional'\ntitle='Retryable optional finish'\nkind='shell'\nphase='finish'\noptional=true\ncommand='echo attempt >> \"{}\"; test -f \"{}\" && test -f \"$VIBE_APP_ROOT/receipt\"'\n",
        r.root.path().join("finish-count").display(),
        gate.display()
    );
    package(&r, name, &body);
    ok(&install(&r, name, &["--keep-work"]));
    let first = work(&r, name);
    assert_eq!(step(&installed(&r, name), "optional")["outcome"], "failed");
    assert_eq!(count(&r, "build-count"), 1);
    assert_eq!(count(&r, "finish-count"), 1);
    let checkpoint = tree(&work_path(&first));
    error(
        &resume(&r, &first["operation"]["target"], &[]),
        2,
        "nothing_to_resume",
    );
    unchanged(&work_path(&first), &checkpoint);
    assert_eq!(count(&r, "finish-count"), 1);
    fs::write(gate, b"allow").unwrap();
    ok(&install(&r, name, &["--resume", "--keep-work"]));
    let retried = work(&r, name);
    assert_ne!(operation(&first), operation(&retried));
    assert_eq!(
        count(&r, "build-count"),
        1,
        "placed prepare payload is reused"
    );
    assert_eq!(count(&r, "finish-count"), 2);
    assert_eq!(step(&installed(&r, name), "optional")["outcome"], "done");
    assert_eq!(
        fs::read(payload(&r, name).join("receipt")).unwrap(),
        b"payload\n"
    );
}

#[test]
fn settled_manual_resume_refuses_while_explicit_from_step_rebuilds_owned_out() {
    let r = AppRegistry::new();
    let name = "continuation-settled";
    let body = format!(
        "{SKIP}{BUILD}\n[[installer_options.steps]]\nid='finish'\ntitle='Count finish'\nkind='shell'\nphase='finish'\ncommand='test -f \"$VIBE_APP_ROOT/receipt\" && echo finish >> \"{}\"'\n",
        r.root.path().join("finish-count").display()
    );
    package(&r, name, &body);
    ok(&install(&r, name, &["--keep-work"]));
    let first = work(&r, name);
    let record = installed(&r, name);
    assert_eq!(step(&record, "skip")["outcome"], "skipped");
    assert_eq!(step(&record, "build")["outcome"], "done");
    assert_eq!(step(&record, "finish")["outcome"], "done");
    let checkpoint = tree(&work_path(&first));
    error(
        &install(&r, name, &["--resume", "--keep-work"]),
        2,
        "nothing_to_resume",
    );
    unchanged(&work_path(&first), &checkpoint);
    assert_eq!(count(&r, "build-count"), 1);
    assert_eq!(count(&r, "finish-count"), 1);
    // A finish-only replay may lack owned out. This replay deliberately starts
    // at the real output producer, preserving the established suffix contract.
    ok(&install(&r, name, &["--from-step", "build", "--keep-work"]));
    let replayed = work(&r, name);
    assert_ne!(operation(&first), operation(&replayed));
    assert_eq!(count(&r, "build-count"), 2);
    assert_eq!(count(&r, "finish-count"), 2);
    assert_eq!(
        fs::read(payload(&r, name).join("receipt")).unwrap(),
        b"payload\n"
    );
}
