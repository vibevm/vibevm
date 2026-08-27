//! Behavior/corpus cell for the `lifecycle_tasks` report (R7.4): the
//! authored absent/idle/parked goldens decoded through the GENERATED types,
//! plus the status ↔ run/tasks/run-id relations JTD cannot express. This is
//! a cell over generated types, not a second handwritten DTO: every decode
//! goes through `vibe_wire::generated::lifecycle_tasks`, and the two tiny
//! helpers below that mirror `vibe-lifecycle`'s deterministic task-path law
//! exist because `vibe-wire` must not depend on `vibe-lifecycle` (the
//! dependency points the other way). They cover the short-key case the
//! corpus exercises; truncation/digest behaviour stays owned by the engine.

use serde::Serialize;
use serde::de::DeserializeOwned;
use vibe_wire::generated::lifecycle_tasks::{
    LifecycleTasks, LifecycleTasksStatus, PendingTaskScope,
};

fn corpus() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/lifecycle/e1")
}

/// Read one authored golden, decode it through the generated reader, and
/// prove the generated writer restates the authored value exactly.
fn read<T: DeserializeOwned + Serialize>(name: &str) -> T {
    let bytes = std::fs::read(corpus().join(name)).unwrap();
    let authored: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let value: T = serde_json::from_value(authored.clone()).unwrap();
    let round_trip = serde_json::to_value(&value).unwrap();
    assert_eq!(
        round_trip, authored,
        "{name} loses data on generated round-trip"
    );
    value
}

fn authored(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(corpus().join(name)).unwrap()).unwrap()
}

/// The run-id law the engine owns (`vibe_lifecycle::process::is_valid_run_id`)
/// : 32 lowercase hex characters, nothing else.
fn is_valid_run_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

/// Mirrors `vibe_lifecycle::delegation::task_filename`'s percent-encoding for
/// keys under the component cap (every corpus key is): uppercase `%XX` over a
/// conservative unreserved set, `task-` prefix, `.md` suffix.
fn encoded_stem(key: &str) -> String {
    let mut encoded = String::new();
    for &byte in key.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' => {
                encoded.push(byte as char);
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

/// Mirrors `vibe_lifecycle::delegation::outbox_task_path`: the
/// selected-node-relative outbox path a `(run id, execution key)` pair owns —
/// `.vibe/agentic/outbox/<run>/task-<pct-key>.md`, the SAME spelling for every
/// member (the engine publishes and records it against the selected project
/// root; `selected` names the root that interprets it).
fn outbox_task_path(run_id: &str, execution_key: &str) -> String {
    format!(
        ".vibe/agentic/outbox/{run_id}/task-{}.md",
        encoded_stem(execution_key)
    )
}

/// The selected identity's canonical SPELLING law, test-only: `.` for the
/// workspace root, otherwise ordinary forward-slash components. Refused:
/// empty, any backslash, an absolute spelling (leading `/` or a drive
/// prefix), and any `..`/`.`/empty component — so a stored identity is one
/// canonical rel, never an escape from the node it names. This validates
/// spelling only; it invents no identity key and defers ownership to the
/// state validator, which stays the product gate.
fn is_valid_selected(rel: &str) -> bool {
    if rel == "." {
        return true;
    }
    if rel.is_empty() || rel.contains('\\') || rel.starts_with('/') {
        return false;
    }
    let bytes = rel.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return false;
    }
    rel.split('/')
        .all(|part| !part.is_empty() && part != "." && part != "..")
}

/// The relation the schema states but JTD cannot express: only `absent`
/// omits `run`; `idle` carries `run` + empty tasks; `parked` carries `run`,
/// a valid run id, a canonically-spelled selected node identity and nonempty
/// tasks whose paths are the validated state-owned outbox paths, carried
/// unchanged against the selected node root.
fn check_relations(report: &LifecycleTasks) -> Result<(), String> {
    match report.status {
        LifecycleTasksStatus::Absent => {
            if report.run.is_some() {
                Err("absent has no state file, so it may not carry a run header".into())
            } else if !report.tasks.is_empty() {
                Err("absent has no state file, so it may not carry tasks".into())
            } else {
                Ok(())
            }
        }
        LifecycleTasksStatus::Idle => {
            report
                .run
                .as_ref()
                .ok_or_else(|| "idle is valid state: it carries its run header".to_string())?;
            if report.tasks.is_empty() {
                Ok(())
            } else {
                Err("idle has no delegated row: tasks must be empty".into())
            }
        }
        LifecycleTasksStatus::Parked => {
            let run = report
                .run
                .as_ref()
                .ok_or_else(|| "parked is valid state: it carries its run header".to_string())?;
            let run_id = run
                .run_id
                .as_deref()
                .filter(|id| is_valid_run_id(id))
                .ok_or_else(|| {
                    "parked owes a durable handoff: the run header needs a valid 32-hex run id"
                        .to_string()
                })?;
            run.selected
                .as_deref()
                .filter(|rel| is_valid_selected(rel))
                .ok_or_else(|| {
                    "parked tasks resolve against the selected node root: the run header needs a \
                     canonically-spelled selected identity (`.` or forward-slash components)"
                        .to_string()
                })?;
            if report.tasks.is_empty() {
                return Err("parked means at least one delegated row: tasks are nonempty".into());
            }
            for task in &report.tasks {
                if !run.chain.contains(&task.phase) {
                    return Err(format!(
                        "task `{}` names phase `{}`, which the run's chain does not contain",
                        task.execution, task.phase
                    ));
                }
                let expected = outbox_task_path(run_id, &task.execution);
                if task.path != expected {
                    return Err(format!(
                        "task `{}` carries `{}`, but run `{run_id}` owns `{expected}` against \
                         the selected node root — the unchanged outbox rel, never re-spelled",
                        task.execution, task.path
                    ));
                }
                if task.document.is_empty() {
                    return Err(format!(
                        "task `{}` carries an empty document; it must be the exact task bytes",
                        task.execution
                    ));
                }
            }
            Ok(())
        }
    }
}

#[test]
fn lifecycle_tasks_corpus_decodes_all_three_statuses_and_round_trips_stably() {
    let absent: LifecycleTasks = read("tasks_absent.json");
    let idle: LifecycleTasks = read("tasks_idle.json");
    let parked: LifecycleTasks = read("tasks_parked.json");
    for (report, status) in [
        (&absent, LifecycleTasksStatus::Absent),
        (&idle, LifecycleTasksStatus::Idle),
        (&parked, LifecycleTasksStatus::Parked),
    ] {
        assert_eq!(report.schema, 1);
        assert_eq!(report.status, status);
    }
    check_relations(&absent).unwrap();
    check_relations(&idle).unwrap();
    check_relations(&parked).unwrap();

    // Value-stability is the round trip above; BYTE-stability is serializing
    // the decoded value, decoding that, and serializing again: the two
    // serializations must be byte-identical for every corpus root.
    for report in [&absent, &idle, &parked] {
        let first = serde_json::to_string(&serde_json::to_value(report).unwrap()).unwrap();
        let reread: LifecycleTasks = serde_json::from_str(&first).unwrap();
        let second = serde_json::to_string(&serde_json::to_value(&reread).unwrap()).unwrap();
        assert_eq!(first, second, "serialization must be byte-stable");
    }
}

#[test]
fn lifecycle_tasks_absent_omits_run_idle_carries_it_with_empty_tasks() {
    let absent: LifecycleTasks = read("tasks_absent.json");
    assert!(absent.run.is_none(), "only absent omits run");
    assert!(absent.tasks.is_empty(), "tasks are emitted even when empty");

    let idle: LifecycleTasks = read("tasks_idle.json");
    let run = idle.run.as_ref().unwrap();
    assert_eq!(run.requested, "build");
    assert_eq!(run.chain, ["validate", "install", "generate", "build"]);
    assert!(is_valid_run_id(run.run_id.as_deref().unwrap()));
    assert_eq!(run.selected.as_deref(), Some("members/tool"));
    assert!(idle.tasks.is_empty());
}

#[test]
fn lifecycle_tasks_idle_still_reads_legacy_state_without_selected_or_run_id() {
    let legacy = serde_json::json!({
        "schema": 1,
        "status": "idle",
        "run": {
            "requested": "build",
            "chain": ["validate", "build"],
            "started": "2026-07-01T08:00:00Z"
        },
        "tasks": []
    });
    let report: LifecycleTasks = serde_json::from_value(legacy).unwrap();
    let run = report.run.as_ref().unwrap();
    assert_eq!(run.run_id, None);
    assert_eq!(run.selected, None);
    check_relations(&report).unwrap();
}

#[test]
fn lifecycle_tasks_parked_carries_identity_and_both_scopes_in_chain_order() {
    let parked: LifecycleTasks = read("tasks_parked.json");
    let run = parked.run.as_ref().unwrap();
    let run_id = run.run_id.as_deref().unwrap();
    assert!(is_valid_run_id(run_id));
    assert_eq!(run.selected.as_deref(), Some("members/tool"));
    assert_eq!(parked.tasks.len(), 2);

    let slot = &parked.tasks[0];
    assert_eq!(slot.scope, PendingTaskScope::Slot);
    assert_eq!(slot.execution, "org.demo/target#post-install");
    let phase = &parked.tasks[1];
    assert_eq!(phase.scope, PendingTaskScope::Phase);
    assert_eq!(phase.execution, "org.demo/provider#draft-guide");

    // Ordering is the durable chain's phase order: the slot row parked during
    // `install`, the phase row during `create`, and install precedes create.
    let position = |name: &str| {
        run.chain
            .iter()
            .position(|phase| phase == name)
            .unwrap_or_else(|| panic!("phase {name} missing from chain"))
    };
    assert!(position(&slot.phase) < position(&phase.phase));

    // Every path is the deterministic outbox file against the SELECTED NODE
    // root, in its UNCHANGED spelling: the same `.vibe/agentic/outbox/...`
    // rel the engine publishes and records — `selected` (`members/tool`)
    // names the root that interprets it, and no `..` re-spelling appears for
    // a non-root member. The state validator stays the product gate; this
    // compares against the short-key mirror of `outbox_task_path`.
    for task in &parked.tasks {
        assert_eq!(
            task.path,
            outbox_task_path(run_id, &task.execution),
            "path must be the unchanged state-owned outbox rel, node-root-relative"
        );
        assert!(
            task.document.starts_with("---\nrun: \"") && task.document.contains("## Request\n\n"),
            "document is the exact task bytes: frontmatter plus both prose sections"
        );
    }

    // The selected identity's SPELLING law (test-only; no identity key is
    // invented): `.` and ordinary forward-slash components parse; empty,
    // backslashed, absolute, drive-prefixed and dotdot/dot/empty-component
    // spellings refuse — and a mutated parked corpus carrying one is refused
    // by the relation layer above.
    for legal in [".", "members/tool", "tool"] {
        assert!(is_valid_selected(legal), "must accept `{legal}`");
    }
    for illegal in [
        "",
        "members\\tool",
        "/members/tool",
        "C:/members/tool",
        "..",
        "members/../tool",
        "members//tool",
        "members/",
        "./members",
    ] {
        assert!(
            !is_valid_selected(illegal),
            "must refuse the non-canonical spelling `{illegal}`"
        );
    }
    let mut escaped = authored("tasks_parked.json");
    escaped["run"]["selected"] = serde_json::json!("../sibling");
    let report: LifecycleTasks = serde_json::from_value(escaped).unwrap();
    let refusal = check_relations(&report).unwrap_err();
    assert!(
        refusal.contains("selected identity"),
        "refusal names the selected spelling: {refusal}"
    );
}

#[test]
fn lifecycle_tasks_scope_is_a_closed_vocabulary_and_the_reader_stays_permissive() {
    // A scope outside {phase, slot} refuses at the generated reader.
    let mut smuggled = authored("tasks_parked.json");
    smuggled["tasks"][0]["scope"] = serde_json::json!("repo");
    assert!(serde_json::from_value::<LifecycleTasks>(smuggled).is_err());

    // `foreign_parsers = "many"` computes a permissive reader: an additive
    // member from a newer writer decodes (and is dropped on rewrite) rather
    // than stranding the foreign parser that wrote it.
    let mut newer = authored("tasks_idle.json");
    newer["future_member"] = serde_json::json!(true);
    let report: LifecycleTasks = serde_json::from_value(newer).unwrap();
    check_relations(&report).unwrap();
    assert!(
        serde_json::to_value(&report)
            .unwrap()
            .get("future_member")
            .is_none()
    );
}

/// One mutation-backed negative relation per status: the mutated document
/// still DECODES (the shape is schema-legal) and the RELATION layer is what
/// refuses — the split the schema's own descriptions promise.
#[test]
fn lifecycle_tasks_relation_negatives_one_mutation_per_status() {
    // absent + an injected run header (mutation: idle's run) — illegal pair.
    let mut absent_with_run = authored("tasks_absent.json");
    absent_with_run["run"] = authored("tasks_idle.json")["run"].clone();
    let report: LifecycleTasks = serde_json::from_value(absent_with_run).unwrap();
    let refusal = check_relations(&report).unwrap_err();
    assert!(
        refusal.contains("absent"),
        "refusal names the status: {refusal}"
    );

    // idle + a pending task (mutation: parked's slot row) — idle is empty.
    let mut idle_with_task = authored("tasks_idle.json");
    idle_with_task["tasks"]
        .as_array_mut()
        .unwrap()
        .push(authored("tasks_parked.json")["tasks"][0].clone());
    let report: LifecycleTasks = serde_json::from_value(idle_with_task).unwrap();
    let refusal = check_relations(&report).unwrap_err();
    assert!(
        refusal.contains("idle"),
        "refusal names the status: {refusal}"
    );

    // parked without its run id (mutation: drop `run.run_id`) — a park has no
    // durable identity to hand the hosting agent.
    let mut parked_anonymous = authored("tasks_parked.json");
    parked_anonymous["run"]
        .as_object_mut()
        .unwrap()
        .remove("run_id");
    let report: LifecycleTasks = serde_json::from_value(parked_anonymous).unwrap();
    let refusal = check_relations(&report).unwrap_err();
    assert!(
        refusal.contains("run id"),
        "refusal names the missing identity: {refusal}"
    );
}
