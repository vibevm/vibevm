use std::fs;

use vibe_core::manifest::{
    ExtensionConfig, ExtensionDecl, ExtensionHandler, ExtensionKey, ExtensionsControl,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_wire::generated::lifecycle::e1::context::{
    Context, Execution, Io, Project, Run, RunAgentMode, World,
};
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecord, ExecutionRecordStatus, LifecycleState, StateArtifact,
};

use super::*;
use crate::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExecutionSession,
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, RunMetadata, SelectorSubject,
    collect_extensions,
};

#[cfg(test)]
#[path = "tests/adoption.rs"]
mod adoption;

#[cfg(test)]
#[path = "tests/lockfile.rs"]
mod lockfile;

#[cfg(test)]
#[path = "tests/publication.rs"]
mod publication;

#[cfg(test)]
#[path = "tests/recovery.rs"]
mod recovery;

#[cfg(test)]
#[path = "tests/safety.rs"]
mod safety;

#[cfg(test)]
#[path = "tests/selected.rs"]
mod selected;

#[cfg(test)]
#[path = "tests/ownership.rs"]
mod ownership;

#[cfg(test)]
#[path = "tests/support.rs"]
mod support;

#[cfg(test)]
#[path = "tests/transaction.rs"]
mod transaction;

#[cfg(test)]
#[path = "tests/trace_sticky.rs"]
mod trace_sticky;

const RUN_ID: &str = "00112233445566778899aabbccddeeff";
const OTHER_RUN: &str = "ffeeddccbbaa99887766554433221100";

/// One real temp lease over `root` — the single-writer proof every store
/// construction in these tests must now carry. Sequential acquisitions are
/// fine (a dropped lease releases); two LIVE leases on one root refuse.
fn lease(root: &std::path::Path) -> std::sync::Arc<crate::LifecycleLease> {
    std::sync::Arc::new(crate::LifecycleLease::acquire(root).expect("a temp root is leasable"))
}

fn record(status: ExecutionRecordStatus, fingerprint: &str) -> ExecutionRecord {
    record_for("key", RUN_ID, status, fingerprint)
}

/// A record whose `tasks` obey the semantic invariant: a delegated row names
/// exactly the task `(run id, execution key)` deterministically owns, and
/// every other status names none.
fn record_for(
    key: &str,
    run_id: &str,
    status: ExecutionRecordStatus,
    fingerprint: &str,
) -> ExecutionRecord {
    ExecutionRecord {
        artifacts: vec![StateArtifact {
            id: "a".into(),
            kind: "text".into(),
            path: "C:/out".into(),
        }],
        duration_ms: 7,
        fingerprint: fingerprint.into(),
        phase: "build".into(),
        status: status.clone(),
        tasks: matches!(status, ExecutionRecordStatus::Delegated)
            .then(|| vec![crate::outbox_task_path(run_id, key).unwrap()])
            .unwrap_or_default(),
        scope: matches!(status, ExecutionRecordStatus::Delegated)
            .then_some(vibe_wire::generated::lifecycle_state::ExecutionRecordScope::Phase),
    }
}

#[test]
fn missing_state_writes_initial_run_and_preserves_unselected_rows() {
    let dir = tempfile::tempdir().unwrap();
    let chain = vec!["validate", "install", "generate", "build"]
        .into_iter()
        .map(str::to_string)
        .collect();
    let mut store = LifecycleStateStore::begin(
        lease(dir.path()),
        "build".into(),
        chain,
        "2026-08-25T12:00:00Z".into(),
        "00112233445566778899aabbccddeeff".into(),
        ".".into(),
        false,
    )
    .unwrap();
    assert!(store.path().is_file());
    assert!(store.state().execution.is_empty());
    store
        .checkpoint(
            "__host__/demo#row".into(),
            record(ExecutionRecordStatus::Ok, "sha256:a"),
        )
        .unwrap();
    // One live writer per workspace: the first store's lease is released
    // before the second epoch may acquire it.
    drop(store);

    let store = LifecycleStateStore::begin(
        lease(dir.path()),
        "test".into(),
        vec![
            "validate".into(),
            "install".into(),
            "generate".into(),
            "build".into(),
            "test".into(),
        ],
        "2026-08-25T13:00:00Z".into(),
        "00112233445566778899aabbccddeeff".into(),
        ".".into(),
        false,
    )
    .unwrap();
    assert!(store.prior("__host__/demo#row").is_some());
    let raw = fs::read_to_string(store.path()).unwrap();
    assert!(raw.contains("[execution.\"__host__/demo#row\"]"), "{raw}");
    let parsed: LifecycleState = toml::from_str(&raw).unwrap();
    assert_eq!(parsed.run.started, "2026-08-25T13:00:00Z");
}

#[test]
fn malformed_unknown_and_unsupported_state_name_path_and_remediation() {
    for (body, needle) in [
        ("not = [valid", "malformed lifecycle state"),
        (
            "schema = 2\n[run]\nrequested='x'\nchain=[]\nstarted='t'\n[execution]\n",
            "unsupported lifecycle state schema 2",
        ),
        (
            "schema=1\nunknown=true\n[run]\nrequested='x'\nchain=[]\nstarted='t'\n[execution]\n",
            "unknown field",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LifecycleStateStore::FILE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, body).unwrap();
        let error = LifecycleStateStore::begin(
            lease(dir.path()),
            "x".into(),
            vec![],
            "t".into(),
            String::new(),
            ".".into(),
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains(needle), "{error}");
        assert!(error.contains(&path.display().to_string()), "{error}");
        assert!(error.contains("remove this erasable cache"), "{error}");
    }
}

#[test]
fn only_ok_skip_and_fresh_are_reusable_and_fresh_artifacts_survive() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = LifecycleStateStore::begin(
        lease(dir.path()),
        "build".into(),
        vec![],
        "t".into(),
        "00112233445566778899aabbccddeeff".into(),
        ".".into(),
        false,
    )
    .unwrap();
    for (status, reusable) in [
        (ExecutionRecordStatus::Ok, true),
        (ExecutionRecordStatus::Skip, true),
        (ExecutionRecordStatus::Fresh, true),
        (ExecutionRecordStatus::Fail, false),
        (ExecutionRecordStatus::Delegated, false),
    ] {
        store
            .checkpoint("key".into(), record(status, "sha256:x"))
            .unwrap();
        assert_eq!(store.reusable("key", "sha256:x"), reusable);
        assert!(!store.reusable("key", "sha256:other"));
    }
    assert_eq!(store.prior("key").unwrap().artifacts[0].id, "a");
}

fn config(message: &str) -> ExtensionConfig {
    ExtensionConfig::from_table(toml::from_str(&format!("message={message:?}")).unwrap())
}

fn row(
    root: &std::path::Path,
    message: &str,
    version: &str,
    inputs: Option<Vec<String>>,
) -> crate::ExtensionRegistryRow {
    let declaration = ExtensionDecl {
        id: "announce".into(),
        point: "phase:build".parse().unwrap(),
        handler: ExtensionHandler::Builtin { name: "log".into() },
        config: Some(config(message)),
        auto: None,
        inputs,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    };
    let registry = collect_extensions(ExtensionWorld {
        installed: vec![],
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: root.into(),
                version: version.into(),
                kind: None,
                content_hash: None,
            },
            declarations: vec![declaration],
            controls: ExtensionsControl::default(),
        },
        effective_stack: None,
    })
    .unwrap();
    registry.plan("phase:build".parse().unwrap(), SelectorSubject::unscoped())[0].clone()
}

fn dependency_row(
    provider_root: &std::path::Path,
    id: &str,
    inputs: Option<Vec<String>>,
    content_hash: &str,
) -> crate::ExtensionRegistryRow {
    let declaration = ExtensionDecl {
        id: id.into(),
        point: "phase:build".parse().unwrap(),
        handler: ExtensionHandler::Builtin { name: "log".into() },
        config: Some(config(id)),
        auto: None,
        inputs,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    };
    let registry = collect_extensions(ExtensionWorld {
        installed: vec![DependencyExtensionSource {
            provider: DependencyProvider {
                id: DependencyProviderId::new(
                    Group::parse("org.demo").unwrap(),
                    PackageName::parse("rust-stack").unwrap(),
                ),
                root: provider_root.into(),
                version: "0.1.0".into(),
                kind: PackageKind::Stack,
                content_hash: ContentHash::parse(content_hash).unwrap(),
            },
            declarations: vec![declaration],
        }],
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: std::path::PathBuf::from("unused-host-root"),
                version: "0.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: vec![],
            controls: ExtensionsControl::default(),
        },
        effective_stack: None,
    })
    .unwrap();
    registry.plan("phase:build".parse().unwrap(), SelectorSubject::unscoped())[0].clone()
}

fn context(root: &std::path::Path, config: &ExtensionConfig) -> Context {
    let root_text = root.to_string_lossy().replace('\\', "/");
    Context {
        slot_target: None,
        artifacts: vec![],
        envelope: 1,
        execution: Execution {
            config: config
                .as_table()
                .iter()
                .map(|(key, value)| (key.clone(), Some(serde_json::to_value(value).unwrap())))
                .collect(),
            id: "announce".into(),
            package: "__host__/demo".into(),
        },
        io: Io {
            scratch: format!("{root_text}/.vibe/lifecycle/run/key/"),
        },
        point: "phase:build".into(),
        project: Project {
            kind: "project".into(),
            manifest: format!("{root_text}/vibe.toml"),
            name: "demo".into(),
            root: root_text.clone(),
            spec_roots: vec![],
            version: "0.1.0".into(),
        },
        run: Run {
            agent_mode: RunAgentMode::Cli,
            assume_yes: false,
            chain: vec![
                "validate".into(),
                "install".into(),
                "generate".into(),
                "build".into(),
            ],
            force: false,
            offline: false,
            phase: "build".into(),
            requested: "build".into(),
        },
        world: World {
            deps_root: format!("{root_text}/vibevm/vibedeps"),
            lockfile: format!("{root_text}/vibe.lock"),
            packages: vec![],
        },
    }
}

#[test]
fn fingerprint_tracks_config_provider_and_declared_inputs_but_not_dynamic_or_state() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname='demo'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::write(dir.path().join("vibe.lock"), "lock").unwrap();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let one = row(dir.path(), "one", "0.1.0", Some(vec!["*.txt".into()]));
    let mut ctx = context(dir.path(), one.effective_config().unwrap());
    let base = fingerprint_execution(&one, &ctx).unwrap();

    ctx.run.force = true;
    ctx.run.assume_yes = true;
    ctx.io.scratch.push_str("different");
    assert_eq!(fingerprint_execution(&one, &ctx).unwrap(), base);
    fs::create_dir_all(dir.path().join(".vibe")).unwrap();
    fs::write(dir.path().join(".vibe/lifecycle.toml"), "changed").unwrap();
    assert_eq!(fingerprint_execution(&one, &ctx).unwrap(), base);

    let config_changed = row(dir.path(), "two", "0.1.0", Some(vec!["*.txt".into()]));
    let config_ctx = context(dir.path(), config_changed.effective_config().unwrap());
    assert_ne!(
        fingerprint_execution(&config_changed, &config_ctx).unwrap(),
        base
    );
    let provider_changed = row(dir.path(), "one", "0.2.0", Some(vec!["*.txt".into()]));
    assert_ne!(
        fingerprint_execution(&provider_changed, &ctx).unwrap(),
        base
    );
    fs::write(dir.path().join("a.txt"), "two").unwrap();
    assert_ne!(fingerprint_execution(&one, &ctx).unwrap(), base);

    let all = row(dir.path(), "one", "0.1.0", Some(vec!["**".into()]));
    let all_ctx = context(dir.path(), all.effective_config().unwrap());
    let before_pruned = fingerprint_execution(&all, &all_ctx).unwrap();
    for excluded in [".git", ".vibe", "target", "node_modules"] {
        fs::create_dir_all(dir.path().join(excluded)).unwrap();
        fs::write(dir.path().join(excluded).join("ignored.txt"), "ignored").unwrap();
    }
    assert_eq!(
        fingerprint_execution(&all, &all_ctx).unwrap(),
        before_pruned
    );
}

#[test]
fn dependency_declared_inputs_select_project_sources_not_provider_shadows() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    let provider = dir.path().join("provider");
    for path in [
        project.join("src"),
        project.join("tests"),
        provider.join("src"),
    ] {
        fs::create_dir_all(path).unwrap();
    }
    fs::write(
        project.join("vibe.toml"),
        "[project]\nname='demo'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::write(project.join("vibe.lock"), "lock").unwrap();
    fs::write(project.join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(project.join("tests/preset.rs"), "#[test] fn works() {}\n").unwrap();
    fs::write(provider.join("src/main.rs"), "provider shadow\n").unwrap();

    let source = dependency_row(
        &provider,
        "source",
        Some(vec!["src/**".into()]),
        "sha256:aa",
    );
    let tests = dependency_row(
        &provider,
        "tests",
        Some(vec!["tests/**".into()]),
        "sha256:aa",
    );
    let no_inputs = dependency_row(&provider, "none", None, "sha256:aa");
    let source_context = context(&project, source.effective_config().unwrap());
    let tests_context = context(&project, tests.effective_config().unwrap());
    let none_context = context(&project, no_inputs.effective_config().unwrap());
    let source_base = fingerprint_execution(&source, &source_context).unwrap();
    let tests_base = fingerprint_execution(&tests, &tests_context).unwrap();
    let none_base = fingerprint_execution(&no_inputs, &none_context).unwrap();

    let provider_changed = dependency_row(
        &provider,
        "source",
        Some(vec!["src/**".into()]),
        "sha256:bb",
    );
    let provider_changed_context = context(&project, provider_changed.effective_config().unwrap());
    assert_ne!(
        fingerprint_execution(&provider_changed, &provider_changed_context).unwrap(),
        source_base,
        "typed provider content identity must move independently of project inputs"
    );

    fs::write(provider.join("src/main.rs"), "changed provider shadow\n").unwrap();
    assert_eq!(
        fingerprint_execution(&source, &source_context).unwrap(),
        source_base
    );
    fs::write(project.join("README.md"), "unrelated\n").unwrap();
    assert_eq!(
        fingerprint_execution(&source, &source_context).unwrap(),
        source_base
    );
    assert_eq!(
        fingerprint_execution(&tests, &tests_context).unwrap(),
        tests_base
    );

    fs::write(
        project.join("src/main.rs"),
        "fn main() { println!(\"changed\"); }\n",
    )
    .unwrap();
    let source_changed = fingerprint_execution(&source, &source_context).unwrap();
    assert_ne!(source_changed, source_base);
    assert_eq!(
        fingerprint_execution(&tests, &tests_context).unwrap(),
        tests_base
    );
    assert_eq!(
        fingerprint_execution(&no_inputs, &none_context).unwrap(),
        none_base
    );

    fs::write(project.join("tests/preset.rs"), "#[test] fn changed() {}\n").unwrap();
    assert_eq!(
        fingerprint_execution(&source, &source_context).unwrap(),
        source_changed
    );
    assert_ne!(
        fingerprint_execution(&tests, &tests_context).unwrap(),
        tests_base
    );
}

#[test]
fn invalid_input_paths_fail_actionably() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("vibe.toml"), "x").unwrap();
    for pattern in [
        "../secret",
        "/absolute",
        "//server/share",
        "C:/secret",
        "bad\\glob",
    ] {
        let row = row(dir.path(), "x", "0.1.0", Some(vec![pattern.into()]));
        let ctx = context(dir.path(), row.effective_config().unwrap());
        let error = fingerprint_execution(&row, &ctx).unwrap_err().to_string();
        assert!(error.contains(pattern), "{error}");
        assert!(error.contains("project-root-relative"), "{error}");
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_input_names_stay_outside_the_utf8_glob_namespace() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("vibe.toml"), "x").unwrap();
    let row = row(dir.path(), "x", "0.1.0", Some(vec!["**".into()]));
    let ctx = context(dir.path(), row.effective_config().unwrap());
    let before = fingerprint_execution(&row, &ctx).unwrap();
    fs::write(dir.path().join(OsString::from_vec(vec![b'x', 0x80])), "x").unwrap();
    fs::write(dir.path().join(OsString::from_vec(vec![b'x', 0x81])), "y").unwrap();

    assert_eq!(fingerprint_execution(&row, &ctx).unwrap(), before);
}

#[test]
fn preparation_error_fingerprint_is_stable_uniform_and_error_text_free() {
    let key = ExtensionKey::authored("__host__/demo#bad");
    let first = preparation_error_fingerprint(&key, "build");
    assert_eq!(first, preparation_error_fingerprint(&key, "build"));
    assert_ne!(first, preparation_error_fingerprint(&key, "test"));
    assert!(first.starts_with("sha256:"));
    assert_eq!(first.len(), 71);
    assert!(!first.contains("secret"));
}

#[test]
fn fresh_artifact_hydration_enters_downstream_envelope_and_fingerprint() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("vibe.toml"), "x").unwrap();
    let row = row(dir.path(), "x", "0.1.0", None);
    let base = context(dir.path(), row.effective_config().unwrap());
    let mut session = ExecutionSession::new(
        base.project.clone(),
        base.world.clone(),
        RunMetadata {
            requested: "build".into(),
            chain: vec![
                "validate".into(),
                "install".into(),
                "generate".into(),
                "build".into(),
            ],
            offline: false,
            assume_yes: false,
            agent_mode: RunAgentMode::Cli,
            force: false,
            trace_compile: false,
            run_id: "fixed".into(),
            started: "2026-08-25T12:00:00Z".into(),
            selected: ".".into(),
        },
    );
    let before = session.envelope_for("build", &row).unwrap();
    let before_fp = fingerprint_execution(&row, &before).unwrap();
    session.hydrate_artifacts(
        "generate",
        &[StateArtifact {
            id: "generated".into(),
            path: "C:/out/generated".into(),
            kind: "text".into(),
        }],
    );
    let after = session.envelope_for("build", &row).unwrap();
    assert_eq!(after.artifacts[0].phase, "generate");
    assert_ne!(fingerprint_execution(&row, &after).unwrap(), before_fp);
}
