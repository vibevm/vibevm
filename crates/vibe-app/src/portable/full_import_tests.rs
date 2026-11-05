//! Real single-root A installation/export reaches B's existing full-import driver.
//! Dependency composition and full resume are separate, still-required gates.
#![cfg(all(test, unix))]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION");

use super::*;
use crate::{AgentHooks, HostFacts, InstallRequest, OperationContext, ResolverScope, SettingsRoot};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use vibe_core::progress::Progress;
use vibe_wire::generated::{
    application::{e1::import_input::ImportTargetInput, e2::apps, e2::import_work},
    shared as w,
};

const KEY: &str = "org.example/portable@1.0.0";
const CANARY: &str = "SYN_FULL_IMPORT_OPTIONAL_SECRET_5109";

fn package(root: &Path) -> ResolvedPackage {
    let source = root.join("package");
    fs::create_dir_all(&source).unwrap();
    let manifest = format!(
        r#"
[package]
group='org.example'
name='portable'
kind='app'
version='1.0.0'
[application]
id='portable'
[[installer_options]]
id='main'
name='Main'
platform='{}'
platform_name='Host'
arch='{}'
result='owned'
[[installer_options.steps]]
id='build'
title='Build receipt'
kind='recipe'
script='build.sh'
[[installer_options.configuration.features]]
id='enabled'
name='Enabled'
default=true
[[installer_options.configuration.parameters]]
id='root'
name='Data directory'
type='path'
default='source-data'
must_exist=true
[[installer_options.configuration.parameters]]
id='token'
name='Optional token'
type='string'
secret=true
"#,
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
    fs::write(source.join("vibe.toml"), &manifest).unwrap();
    fs::write(
        source.join("build.sh"),
        "set -e\nmkdir -p \"$VIBE_OUT\"\nprintf '%s\\n' \"$VIBE_PARAM_ROOT\" > \"$VIBE_OUT/receipt\"\nprintf ran > \"$VIBE_PACKAGE/executed\"\n",
    )
    .unwrap();
    crate::from_manifest(
        vibe_core::manifest::Manifest::parse_str(&manifest).unwrap(),
        source,
        crate::PackageCoordinate {
            group: "org.example".into(),
            name: "portable".into(),
            version: "1.0.0".into(),
        },
    )
    .unwrap()
}

fn context<'a>(
    settings: &'a SettingsRoot,
    progress: &'a Progress,
    facts: HostFacts<'a>,
) -> OperationContext<'a> {
    OperationContext {
        import_scope: None,
        settings,
        json: false,
        offline: true,
        chooser: None,
        proxy_image: Path::new("unused-proxy"),
        facts,
        feature_chooser: None,
        parameter_prompt: None,
        license_prompt: None,
        on_plan: None,
        on_notice: None,
        progress,
        quiet_children: true,
        panel: None,
        on_step_log: None,
        interrupt: None,
        agent: AgentHooks::default(),
        resolver: None,
        resolver_scope: ResolverScope::Caller,
        source_reader: None,
        environment: None,
        search_path: None,
    }
}

fn exported(
    supplied_secret: bool,
) -> (
    tempfile::TempDir,
    apps::AppsManifest,
    crate::configuration::ConfigurationRecord,
) {
    let a = tempfile::tempdir().unwrap();
    let source_data = a.path().join("source-data");
    fs::create_dir(&source_data).unwrap();
    let resolved = package(a.path());
    let settings = SettingsRoot::new(&a.path().join("settings"));
    let progress = Progress::default();
    let resolve = |raw: &str| a.path().join(raw);
    let ctx = context(
        &settings,
        &progress,
        HostFacts {
            resolve_path: &resolve,
            ..HostFacts::default()
        },
    );
    let request = InstallRequest {
        parameters: if supplied_secret {
            vec![("token".into(), CANARY.into())]
        } else {
            vec![]
        },
        ..Default::default()
    };
    let installed = crate::install(&ctx, &resolved, &request).unwrap();
    assert_eq!(
        fs::read_to_string(installed.app_root.unwrap().join("receipt")).unwrap(),
        format!("{}\n", source_data.display())
    );
    assert!(resolved.root.join("executed").exists());
    let index = crate::Index::load(&settings).unwrap();
    let original = index.installations[KEY].configuration.clone().unwrap();
    let PortableDocument::Full(document) = export_portable(
        &settings,
        &PortableExport {
            all: false,
            arrangement: false,
        },
        false,
    )
    .unwrap() else {
        panic!("actual common installation selects full export");
    };
    assert_eq!(document.roots, [KEY]);
    assert_eq!(document.application.len(), 1);
    assert!(document.application[0].requires.is_empty());
    assert!(!serde_json::to_string(&document).unwrap().contains(CANARY));
    fs::remove_dir(&source_data).unwrap();
    assert!(!source_data.exists());
    (a, document, original)
}

fn target(root: &Path) -> (ResolvedPackage, SettingsRoot, ImportTargetInput) {
    let resolved = package(root);
    let data = root.join("target-data");
    fs::create_dir(&data).unwrap();
    let input = ImportTargetInput {
        protocol: "vibe-app-import-input/1".into(),
        applications: BTreeMap::from([(
            KEY.into(),
            w::PortableTargetInput {
                path_bindings: BTreeMap::from([("root".into(), data.to_str().unwrap().into())]),
                installer: Some("main".into()),
                review_definition: None,
                configuration: None,
            },
        )]),
    };
    (resolved, SettingsRoot::new(&root.join("settings")), input)
}

fn forbidden_resolution(_: &str) -> PathBuf {
    panic!("B must not resolve A's lexical source path against its cwd")
}

fn public_root(
    record: &crate::configuration::ConfigurationRecord,
) -> &w::ConfigurationParameterResultPublic {
    record
        .evaluation
        .parameters
        .iter()
        .find_map(|p| match p {
            w::ConfigurationParameterResult::Public(p) if p.id == "root" => Some(p.as_ref()),
            _ => None,
        })
        .unwrap()
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION")]
fn actual_a_install_export_then_b_import_preserves_input_and_executes_bound_payload() {
    let (_a, document, original) = exported(false);
    let b = tempfile::tempdir().unwrap();
    let (resolved, settings, target) = target(b.path());
    let progress = Progress::default();
    let ctx = context(
        &settings,
        &progress,
        HostFacts {
            resolve_path: &forbidden_resolution,
            ..HostFacts::default()
        },
    );
    let secrets = ImportSecrets::default();
    let candidates = [FullImportCandidate {
        application: &document.application[0],
        resolved: &resolved,
        option: &resolved.options[0],
    }];
    let mut prepared = prepare_full_import(FullImportPreparation {
        document: &document,
        candidates: &candidates,
        target: &target,
        secrets: &secrets,
        context: &ctx,
        skip_installed: false,
        accept_license: false,
    })
    .unwrap();
    assert!(
        !settings.path().exists(),
        "public preparation creates no home/work"
    );
    assert!(!resolved.root.join("executed").exists());
    crate::import_full(&ctx, &mut prepared, &secrets).unwrap();
    let index = crate::Index::load(&settings).unwrap();
    let installed = &index.installations[KEY];
    let configuration = installed.configuration.as_ref().unwrap();
    assert_eq!(configuration.evaluation.input, original.evaluation.input);
    assert_eq!(
        public_root(configuration).origin,
        public_root(&original).origin
    );
    assert_eq!(
        public_root(configuration).source,
        public_root(&original).source
    );
    assert_eq!(
        serde_json::to_value(&public_root(configuration).origin).unwrap()["kind"],
        "default"
    );
    assert_eq!(
        installed.parameters["root"],
        target.applications[KEY].path_bindings["root"]
    );
    let storage = crate::Storage::load(&settings).unwrap();
    let payload = crate::launch::app_root(installed, &storage).unwrap();
    assert_eq!(
        fs::read_to_string(payload.join("receipt")).unwrap(),
        format!("{}\n", target.applications[KEY].path_bindings["root"])
    );
    assert_eq!(
        fs::read_to_string(resolved.root.join("executed")).unwrap(),
        "ran"
    );
    assert!(installed.pending.is_none());
    assert_eq!(prepared.results().len(), 1);
    assert_eq!(
        prepared.results()[0].action,
        w::ApplicationPendingImportResultAction::Installed
    );
    assert!(crate::work(&settings, None).unwrap().imports.is_empty());
}

fn assert_no_answer(path: &Path) {
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            assert_no_answer(&entry.unwrap().path());
        }
    } else {
        let bytes = fs::read(path).unwrap();
        assert!(
            !bytes
                .windows(CANARY.len())
                .any(|part| part == CANARY.as_bytes()),
            "{}",
            path.display()
        );
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-SECRET-INPUT")]
fn missing_previously_supplied_secret_keeps_exact_cohort_waiting_before_effects() {
    let (_a, document, _) = exported(true);
    let b = tempfile::tempdir().unwrap();
    let (resolved, settings, target) = target(b.path());
    let progress = Progress::default();
    let ctx = context(
        &settings,
        &progress,
        HostFacts {
            resolve_path: &forbidden_resolution,
            ..HostFacts::default()
        },
    );
    let secrets = ImportSecrets::default();
    let candidates = [FullImportCandidate {
        application: &document.application[0],
        resolved: &resolved,
        option: &resolved.options[0],
    }];
    let mut prepared = prepare_full_import(FullImportPreparation {
        document: &document,
        candidates: &candidates,
        target: &target,
        secrets: &secrets,
        context: &ctx,
        skip_installed: false,
        accept_license: false,
    })
    .unwrap();
    assert!(!settings.path().exists());
    let error = crate::import_full(&ctx, &mut prepared, &secrets).unwrap_err();
    assert_eq!(error.code(), "import_input_required");
    assert!(!error.to_string().contains(CANARY));
    assert!(prepared.results().is_empty());
    assert!(
        prepared.current().is_none(),
        "no child attempt began before required input"
    );
    assert!(!settings.index_path().exists());
    assert!(!resolved.root.join("executed").exists());
    let listing = crate::work(&settings, None).unwrap();
    assert_eq!(listing.imports.len(), 1);
    let retained = &listing.imports[0];
    assert_eq!(retained.cursor, 0);
    assert_eq!(retained.total, 1);
    assert!(retained.current.is_none());
    assert!(!retained.target.operation.is_empty());
    assert_eq!(
        PathBuf::from(&retained.target.home),
        fs::canonicalize(settings.path()).unwrap()
    );
    let bytes = fs::read(Path::new(&retained.path).join("import.json")).unwrap();
    let saved: import_work::ImportWork = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(saved.protocol, "vibe-app-import-work/2");
    assert_eq!(saved.target, retained.target);
    assert_eq!(saved.cohort.roots, [KEY]);
    assert_eq!(saved.cohort.cursor, 0);
    assert!(saved.cohort.current.is_none());
    assert!(saved.cohort.results.is_empty());
    let waiting = saved.cohort.waiting.unwrap();
    assert_eq!(waiting.key, KEY);
    assert_eq!(waiting.parameters, ["token"]);
    let crate::AppError::Configuration(
        crate::configuration::ConfigurationError::ImportInputRequired {
            target,
            key,
            parameters,
        },
    ) = &error
    else {
        panic!("expected scoped full-import input request")
    };
    assert_eq!(target, &saved.target);
    assert_eq!(key, &waiting.key);
    assert_eq!(parameters, &waiting.parameters);
    assert_no_answer(settings.path());
    // No resume route is fabricated here: waiting is deliberately non-success.
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT")]
fn same_key_candidate_with_altered_history_refuses_before_cohort_or_effects() {
    let (_a, document, _) = exported(false);
    let b = tempfile::tempdir().unwrap();
    let (resolved, settings, target) = target(b.path());
    let progress = Progress::default();
    let ctx = context(&settings, &progress, HostFacts::default());
    let secrets = ImportSecrets::default();
    let mut forged = document.application[0].clone();
    let w::PortableConfigurationState::Configured(configuration) = &mut forged.configuration else {
        panic!("configured source");
    };
    configuration.history.input.intent.default_features = false;
    let candidates = [FullImportCandidate {
        application: &forged,
        resolved: &resolved,
        option: &resolved.options[0],
    }];
    let error = prepare_full_import(FullImportPreparation {
        document: &document,
        candidates: &candidates,
        target: &target,
        secrets: &secrets,
        context: &ctx,
        skip_installed: false,
        accept_license: false,
    })
    .unwrap_err();
    assert!(
        error.to_string().contains("candidate does not identify"),
        "{error}"
    );
    assert!(!settings.path().exists());
    assert!(!resolved.root.join("executed").exists());
}
