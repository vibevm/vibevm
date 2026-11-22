//! Ordered write and check scenarios for generated agent type ownership.

use super::{
    AGENT_TYPES_IGNORE_BODY, AGENT_TYPES_IGNORE_FIRST_LINE, AgentTypeBody, AgentTypeOutcome,
    AgentTypeStatus, AgentTypesError, SyncMode, builtin_agent_types, render_claude_agent_type,
    sync_agent_types,
};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-TYPES");

const IGNORE: &str = ".claude/agents/.gitignore";
const IGNORE_REASON: &str =
    "`.claude/agents/.gitignore` is missing or out of date, so this file is not kept out of Git";

fn resolve(body: &AgentTypeBody) -> Result<Vec<u8>, String> {
    Ok(match body {
        AgentTypeBody::ProjectLane => b"LANE\n".to_vec(),
        AgentTypeBody::PackageFile { .. } => b"RULES\n".to_vec(),
    })
}

fn no_lane(body: &AgentTypeBody) -> Result<Vec<u8>, String> {
    match body {
        AgentTypeBody::ProjectLane => Err("no lane".to_owned()),
        AgentTypeBody::PackageFile { .. } => resolve(body),
    }
}

fn assert_statuses(outcomes: &[AgentTypeOutcome], expected: [AgentTypeStatus; 2]) {
    let specs = builtin_agent_types();
    assert_eq!(outcomes.len(), specs.len());
    for ((outcome, spec), status) in outcomes.iter().zip(specs).zip(expected) {
        assert_eq!(outcome.name, spec.type_name());
        assert_eq!(outcome.file, spec.file());
        assert_eq!(outcome.status, status);
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-TYPES")]
fn write_creates_types_after_the_ignore_file_and_keeps_foreign_files() {
    use AgentTypeStatus::*;

    let node = tempfile::tempdir().unwrap();
    let project = vibe_safefs::Project::open(node.path()).unwrap();
    let specs = builtin_agent_types();
    let worker = specs[0].file();
    let executor = specs[1].file();
    let sync = |resolver: &dyn Fn(&AgentTypeBody) -> Result<Vec<u8>, String>| {
        sync_agent_types(node.path(), &specs, resolver, SyncMode::Write).unwrap()
    };

    let created = sync(&resolve);
    assert_statuses(&created, [Created, Created]);
    assert!(created.iter().all(|outcome| outcome.reason.is_none()));
    assert_eq!(
        project.read_file(IGNORE).unwrap().unwrap(),
        AGENT_TYPES_IGNORE_BODY.as_bytes()
    );
    for spec in &specs {
        assert_eq!(
            project.read_file(&spec.file()).unwrap().unwrap(),
            render_claude_agent_type(spec, &resolve(&spec.body).unwrap())
        );
    }

    assert_statuses(&sync(&resolve), [Unchanged, Unchanged]);

    project.write_atomic(&executor, b"mine\n").unwrap();
    project
        .write_atomic(".claude/agents/opus5.md", b"authored\n")
        .unwrap();
    assert_statuses(&sync(&resolve), [Unchanged, Foreign]);
    assert_eq!(project.read_file(&executor).unwrap().unwrap(), b"mine\n");

    let removed = sync(&no_lane);
    assert_statuses(&removed, [Removed, Foreign]);
    assert_eq!(removed[0].reason.as_deref(), Some("no lane"));
    assert!(project.read_file(&worker).unwrap().is_none());
    assert_eq!(project.read_file(&executor).unwrap().unwrap(), b"mine\n");
    assert_eq!(
        project
            .read_file(".claude/agents/opus5.md")
            .unwrap()
            .unwrap(),
        b"authored\n"
    );
    let unavailable = sync(&no_lane);
    assert_statuses(&unavailable, [Unavailable, Foreign]);
    assert_eq!(unavailable[0].reason.as_deref(), Some("no lane"));

    let fresh = tempfile::tempdir().unwrap();
    let fresh_project = vibe_safefs::Project::open(fresh.path()).unwrap();
    fresh_project.write_atomic(IGNORE, b"*.tmp\n").unwrap();
    for mode in [SyncMode::Write, SyncMode::Check] {
        let error = sync_agent_types(fresh.path(), &specs, &resolve, mode).unwrap_err();
        assert!(matches!(
            error,
            AgentTypesError::ForeignIgnoreFile { path } if path == fresh.path().join(IGNORE)
        ));
        for spec in &specs {
            assert!(fresh_project.read_file(&spec.file()).unwrap().is_none());
        }
        assert_eq!(
            fresh_project.read_file(IGNORE).unwrap().unwrap(),
            b"*.tmp\n"
        );
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-TYPES")]
fn check_reports_missing_current_stale_and_foreign() {
    use AgentTypeStatus::*;

    let node = tempfile::tempdir().unwrap();
    let project = vibe_safefs::Project::open(node.path()).unwrap();
    let specs = builtin_agent_types();
    let worker = specs[0].file();
    let executor = specs[1].file();
    let check = |resolver: &dyn Fn(&AgentTypeBody) -> Result<Vec<u8>, String>| {
        sync_agent_types(node.path(), &specs, resolver, SyncMode::Check).unwrap()
    };
    let write = || sync_agent_types(node.path(), &specs, &resolve, SyncMode::Write).unwrap();

    assert_statuses(&check(&resolve), [Missing, Missing]);
    assert!(!node.path().join(".claude").exists());

    write();
    assert_statuses(&check(&resolve), [Current, Current]);

    project.remove_file(IGNORE).unwrap();
    let unignored = check(&resolve);
    assert_statuses(&unignored, [Stale, Stale]);
    for outcome in unignored {
        assert_eq!(outcome.reason.as_deref(), Some(IGNORE_REASON));
    }
    assert!(project.read_file(IGNORE).unwrap().is_none());

    write();
    let mut edited = project.read_file(&worker).unwrap().unwrap();
    edited.extend_from_slice(b"x\n");
    project.write_atomic(&worker, &edited).unwrap();
    let stale = check(&resolve);
    assert_statuses(&stale, [Stale, Current]);
    assert!(stale[0].reason.is_none());
    assert_eq!(project.read_file(&worker).unwrap().unwrap(), edited);

    let unavailable = check(&no_lane);
    assert_statuses(&unavailable, [Stale, Current]);
    assert_eq!(unavailable[0].reason.as_deref(), Some("no lane"));
    assert_eq!(project.read_file(&worker).unwrap().unwrap(), edited);

    assert_statuses(&write(), [Updated, Unchanged]);
    project.write_atomic(&executor, b"mine\n").unwrap();
    assert_statuses(&check(&resolve), [Current, Foreign]);
    assert_eq!(project.read_file(&executor).unwrap().unwrap(), b"mine\n");

    project.remove_file(&worker).unwrap();
    let absent = check(&no_lane);
    assert_statuses(&absent, [Unavailable, Foreign]);
    assert_eq!(absent[0].reason.as_deref(), Some("no lane"));
    assert!(project.read_file(&worker).unwrap().is_none());

    // An owned but outdated ignore file is reported without being repaired.
    write();
    let outdated_ignore = format!("{AGENT_TYPES_IGNORE_FIRST_LINE}\r\n/vibe-worker.md\n");
    project
        .write_atomic(IGNORE, outdated_ignore.as_bytes())
        .unwrap();
    let outdated = check(&resolve);
    assert_statuses(&outdated, [Stale, Foreign]);
    assert_eq!(outdated[0].reason.as_deref(), Some(IGNORE_REASON));
    assert_eq!(
        project.read_file(IGNORE).unwrap().unwrap(),
        outdated_ignore.as_bytes()
    );
    write();
    assert_eq!(
        project.read_file(IGNORE).unwrap().unwrap(),
        AGENT_TYPES_IGNORE_BODY.as_bytes()
    );
}
