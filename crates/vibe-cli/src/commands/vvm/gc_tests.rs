use std::sync::{Arc, Mutex};

use super::*;
use crate::cli::AgentModeArg;
use crate::commands::vvm::model::{Kind, Origin, Profile};
use crate::commands::vvm::store::{BINARY_NAME, INDEX_BINARY_NAME};
use vibe_core::progress::{ProgressEvent, ProgressEventKind, ProgressObserver, TaskId};

#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<ProgressEvent>>,
}

impl ProgressObserver for Recorder {
    fn observe(&self, event: ProgressEvent) {
        self.events.lock().unwrap().push(event);
    }
}

fn record(kind: Kind, id: &str, instance: u64) -> InstallRecord {
    InstallRecord {
        kind,
        id: id.into(),
        instance,
        commit: "c".into(),
        toolchain: "t".into(),
        profile: Profile::Debug,
        installed_at: "now".into(),
        origin: Origin::Managed,
        source_path: None,
        payload_sha256: None,
        distribution_manifest_sha256: None,
    }
}

fn installed_store(instances: &[u64]) -> (tempfile::TempDir, VersionStore, VersionId) {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path());
    let id = VersionId::new(Kind::Tag, "1.0.0");
    for &instance in instances {
        store
            .record_install(record(Kind::Tag, "1.0.0", instance))
            .unwrap();
        let directory = store.instance_dir(&id, instance);
        let bin = directory.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let vibe = bin.join(BINARY_NAME);
        let index = bin.join(INDEX_BINARY_NAME);
        fs::write(&vibe, format!("payload-{instance}")).unwrap();
        fs::write(&index, format!("index-{instance}")).unwrap();
        #[cfg(unix)]
        for path in [&vibe, &index] {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).unwrap();
        }
        let manifest = crate::commands::vvm::placer::manifest_for(&[
            (vibe, format!("bin/{BINARY_NAME}")),
            (index, format!("bin/{INDEX_BINARY_NAME}")),
        ])
        .unwrap();
        fs::write(
            directory.join(".vvm-manifest.toml"),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
        store.write_current(&directory).unwrap();
    }
    (temp, store, id)
}

fn quiet() -> output::Context {
    output::Context::from_flags(true, false, None, true, AgentModeArg::Auto)
}

fn observe_gc(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmGcArgs,
) -> (GcOutcome, Vec<ProgressEvent>) {
    let recorder = Arc::new(Recorder::default());
    let progress = Progress::new(recorder.clone());
    let overall = progress.task("Garbage collecting vibevm");
    let outcome = run_gc_with_progress(ctx, env, args, &overall.progress()).unwrap();
    match outcome {
        GcOutcome::Finished => overall.finish(),
        GcOutcome::Skipped(reason) => overall.skip(reason),
    }
    let events = recorder.events.lock().unwrap().clone();
    (outcome, events)
}

fn task_id(events: &[ProgressEvent], label: &str) -> TaskId {
    events
        .iter()
        .find_map(|event| match &event.kind {
            ProgressEventKind::Started { label: found } if found == label => Some(event.task_id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing progress task `{label}`"))
}

fn start_position(events: &[ProgressEvent], label: &str) -> usize {
    events
        .iter()
        .position(|event| {
            matches!(&event.kind, ProgressEventKind::Started { label: found } if found == label)
        })
        .unwrap_or_else(|| panic!("missing progress task `{label}`"))
}

fn terminal_kind<'a>(events: &'a [ProgressEvent], label: &str) -> &'a ProgressEventKind {
    let id = task_id(events, label);
    &events
        .iter()
        .find(|event| {
            event.task_id == id
                && matches!(
                    event.kind,
                    ProgressEventKind::Finished
                        | ProgressEventKind::Skipped { .. }
                        | ProgressEventKind::Failed { .. }
                        | ProgressEventKind::Stopped
                )
        })
        .unwrap_or_else(|| panic!("missing terminal progress event for `{label}`"))
        .kind
}

#[test]
fn gc_progress_preflights_before_effects_and_counts_known_instances() {
    let (temp, store, id) = installed_store(&[1, 2, 3]);
    fs::create_dir_all(store.build_dir()).unwrap();
    fs::write(store.build_dir().join("cache"), b"cache").unwrap();
    let env = VvmEnv {
        root: Some(temp.path().to_path_buf()),
        ..VvmEnv::default()
    };

    let (outcome, events) = observe_gc(
        &quiet(),
        &env,
        VvmGcArgs {
            build: false,
            prune_others: true,
            yes: true,
        },
    );

    assert_eq!(outcome, GcOutcome::Finished);
    assert!(
        start_position(&events, "Preflighting garbage collection targets")
            < start_position(&events, "Recording activation pointers")
    );
    assert!(
        start_position(&events, "Recording activation pointers")
            < start_position(&events, "Removing unused instances")
    );
    assert!(
        start_position(&events, "Removing unused instances")
            < start_position(&events, "Saving version inventory")
    );
    assert!(
        start_position(&events, "Saving version inventory")
            < start_position(&events, "Removing Rust build cache")
    );
    let removals = task_id(&events, "Removing unused instances");
    assert!(events.iter().any(|event| {
        event.task_id == removals
            && matches!(
                &event.kind,
                ProgressEventKind::Progress {
                    completed: 0,
                    total: Some(1),
                    unit,
                } if unit == "instances"
            )
    }));
    assert!(events.iter().any(|event| {
        event.task_id == removals
            && matches!(
                &event.kind,
                ProgressEventKind::Progress {
                    completed: 1,
                    total: Some(1),
                    unit,
                } if unit == "instances"
            )
    }));
    assert!(matches!(
        terminal_kind(&events, "Removing tag:1.0.0#1"),
        ProgressEventKind::Finished
    ));
    assert!(!store.instance_dir(&id, 1).exists());
    assert!(!store.build_dir().exists());
}

#[test]
fn gc_build_cache_reports_the_guarded_component() {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path());
    fs::create_dir_all(store.build_dir()).unwrap();
    fs::write(store.build_dir().join("cache"), b"cache").unwrap();
    let env = VvmEnv {
        root: Some(temp.path().to_path_buf()),
        ..VvmEnv::default()
    };

    let (outcome, events) = observe_gc(
        &quiet(),
        &env,
        VvmGcArgs {
            build: true,
            prune_others: false,
            yes: true,
        },
    );

    assert_eq!(outcome, GcOutcome::Finished);
    assert!(matches!(
        terminal_kind(&events, "Preflighting Rust build cache"),
        ProgressEventKind::Finished
    ));
    assert!(matches!(
        terminal_kind(&events, "Removing Rust build cache"),
        ProgressEventKind::Finished
    ));
    assert!(!store.build_dir().exists());
}

#[test]
fn gc_empty_build_cache_is_skipped_at_component_and_overall_levels() {
    let temp = tempfile::tempdir().unwrap();
    let env = VvmEnv {
        root: Some(temp.path().to_path_buf()),
        ..VvmEnv::default()
    };

    let (outcome, events) = observe_gc(
        &quiet(),
        &env,
        VvmGcArgs {
            build: true,
            prune_others: false,
            yes: true,
        },
    );

    assert_eq!(outcome, GcOutcome::Skipped("build cache already empty"));
    assert!(matches!(
        terminal_kind(&events, "Removing Rust build cache"),
        ProgressEventKind::Skipped { reason } if reason == "already empty"
    ));
    assert!(matches!(
        terminal_kind(&events, "Garbage collecting vibevm"),
        ProgressEventKind::Skipped { reason } if reason == "build cache already empty"
    ));
}

#[test]
fn gc_nothing_to_prune_records_pointers_then_skips_overall() {
    let (temp, _store, _id) = installed_store(&[1, 2]);
    let env = VvmEnv {
        root: Some(temp.path().to_path_buf()),
        ..VvmEnv::default()
    };

    let (outcome, events) = observe_gc(
        &quiet(),
        &env,
        VvmGcArgs {
            build: false,
            prune_others: true,
            yes: true,
        },
    );

    assert_eq!(outcome, GcOutcome::Skipped("nothing to prune"));
    assert!(matches!(
        terminal_kind(&events, "Recording activation pointers"),
        ProgressEventKind::Finished
    ));
    assert!(!events.iter().any(|event| {
        matches!(&event.kind, ProgressEventKind::Started { label } if label == "Removing unused instances")
    }));
    assert!(matches!(
        terminal_kind(&events, "Garbage collecting vibevm"),
        ProgressEventKind::Skipped { reason } if reason == "nothing to prune"
    ));
}

#[test]
fn gc_progress_is_inert_in_quiet_and_json_contexts() {
    for context in [
        output::Context::from_flags(true, false, None, true, AgentModeArg::Auto)
            .with_progress(true, output::ProgressMode::Plain),
        output::Context::from_flags(false, true, None, true, AgentModeArg::Auto)
            .with_progress(true, output::ProgressMode::Plain),
    ] {
        assert_eq!(context.progress().task("gc").id().get(), 0);
    }
}

#[test]
fn gc_partial_best_effort_cleanup_skips_the_aggregate_outcome() {
    assert_eq!(
        prune_outcome(1, false),
        GcOutcome::Skipped("completed with retained items")
    );
    assert_eq!(
        prune_outcome(0, true),
        GcOutcome::Skipped("completed with retained items")
    );
    assert_eq!(prune_outcome(0, false), GcOutcome::Finished);
}
