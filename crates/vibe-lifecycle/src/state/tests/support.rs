//! Shared fixtures for the state-transaction tests: one canonical store
//! shape, the third-writer TOML, and the `.vibe` inventory a refusal must
//! leave untouched. Split from `tests/recovery.rs` when that file crossed the
//! 600-line budget; the same helpers serve the publication-failure and
//! recovery-window cells.

use std::fs;
use std::path::Path;

use vibe_core::manifest::{ExtensionConfig, ExtensionDecl, ExtensionHandler, ExtensionsControl};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_wire::generated::lifecycle::e1::context::{
    Context, Execution, Io, Project, Run, RunAgentMode, World,
};
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecordStatus, SlotContinuation, SlotTargetRecord,
};

use super::{RUN_ID, lease, record_for};
use crate::LifecycleStateStore;
use crate::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionWorld,
    HostExtensionSource, HostIdentity, HostProvider, SelectorSubject, collect_extensions,
};

pub(super) const KEY: &str = "org.demo/tools#produce";
pub(super) const OTHER: &str = "org.demo/tools#consume";

pub(super) fn open(root: &Path) -> LifecycleStateStore {
    LifecycleStateStore::begin(
        lease(root),
        "create".into(),
        vec!["validate".into(), "install".into(), "create".into()],
        "2026-08-28T00:00:00Z".into(),
        RUN_ID.into(),
        ".".into(),
        false,
    )
    .unwrap()
}

/// A store already carrying one durable success row, plus the exact bytes of
/// that disk state — the "prior" every transaction case measures against.
pub(super) fn prior_store(root: &Path) -> (LifecycleStateStore, Vec<u8>) {
    let mut store = open(root);
    store
        .checkpoint(
            KEY.into(),
            record_for(KEY, RUN_ID, ExecutionRecordStatus::Ok, "sha256:prior"),
        )
        .unwrap();
    let bytes = fs::read(store.path()).unwrap();
    (store, bytes)
}

/// A valid state no party in the test wrote: the third writer's bytes.
pub(super) fn third_state_toml() -> &'static str {
    "schema = 1\n\
     [run]\nrequested = 'other'\nchain = []\nstarted = '2020-01-01T00:00:00Z'\n\
     [execution]\n"
}

pub(super) fn targets() -> SlotContinuation {
    SlotContinuation {
        targets: vec![SlotTargetRecord {
            group: "org.demo".into(),
            name: "tools".into(),
            version: "0.1.0".into(),
        }],
    }
}

/// Everything the `.vibe` directory holds BESIDES the mutation lease's own
/// lock file: a refused or poisoned store must leave exactly the bytes that
/// were already there, and no staging residue. `.vibe/lifecycle.lock` is
/// infrastructure of the acquiring command (created by `try_lock` itself),
/// not lifecycle state, so it is not part of the state inventory a refusal
/// must account for.
pub(super) fn vibe_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root.join(".vibe"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name != crate::lease::LOCK_NAME)
        .collect();
    names.sort();
    names
}

/// The fingerprint-fixture builders, split from `tests.rs` when the cell
/// crossed its line budget: one host row, one dependency row and the
/// envelope context that delivers a row's effective config.
pub(super) fn config(message: &str) -> ExtensionConfig {
    ExtensionConfig::from_table(toml::from_str(&format!("message={message:?}")).unwrap())
}

pub(super) fn row(
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

pub(super) fn dependency_row(
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
            controls: ExtensionsControl::default(),
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

pub(super) fn context(root: &std::path::Path, config: &ExtensionConfig) -> Context {
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
