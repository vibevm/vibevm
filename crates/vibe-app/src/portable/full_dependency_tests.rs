//! Real imported exact bindings must precede installed-first dependency shortcuts.
#![cfg(all(test, unix))]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-AUTO-BINDINGS");

use super::*;
use crate::{AgentHooks, AppError, HostFacts, InstallRequest, OperationContext, ResolverScope};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use vibe_core::progress::Progress;
use vibe_wire::generated::{
    application::e1::import_input::ImportTargetInput, application::e2::apps,
};

const CHILD1: &str = "org.example/child@1.0.0";
const CHILD2: &str = "org.example/child@2.0.0";
const ROOT_A: &str = "org.example/root-a@1.0.0";
const ROOT_B: &str = "org.example/root-b@1.0.0";

struct Home {
    _scratch: tempfile::TempDir,
    settings: SettingsRoot,
    registry: PathBuf,
}
impl Home {
    fn new() -> Self {
        let scratch = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(&scratch.path().join("settings"));
        let registry = scratch.path().join("registry");
        Self {
            _scratch: scratch,
            settings,
            registry,
        }
    }
    fn source(&self, name: &str, version: &str) -> PathBuf {
        self.registry
            .join("org.example")
            .join(name)
            .join(format!("v{version}"))
    }
    fn package(&self, name: &str, version: &str, requires_child: bool) {
        let source = self.source(name, version);
        fs::create_dir_all(&source).unwrap();
        let requires = if requires_child && name == "root-b" {
            "requires=[{package='org.example/child@^1',scope='build'}]"
        } else if requires_child {
            "requires=[{package='org.example/child@>=1',scope='build'}]"
        } else {
            ""
        };
        let manifest = format!(
            r#"
[package]
group='org.example'
name='{name}'
kind='app'
version='{version}'
[application]
id='{name}'
[[installer_options]]
id='main'
name='Main'
platform='{}'
platform_name='Host'
arch='{}'
result='owned'
{requires}
[[installer_options.steps]]
id='build'
title='Build receipt'
kind='recipe'
script='build.sh'
[[installer_options.configuration.parameters]]
id='flavor'
name='Flavor'
type='string'
default='source-choice'
"#,
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        fs::write(source.join("vibe.toml"), manifest).unwrap();
        fs::write(source.join("build.sh"), "set -e\nmkdir -p \"$VIBE_OUT\"\nprintf '%s\\n' \"$VIBE_PARAM_FLAVOR\" > \"$VIBE_OUT/receipt\"\nprintf 'run\\n' >> \"$VIBE_PACKAGE/executions\"\n").unwrap();
    }
    fn resolve(&self, spelling: &str) -> Result<ResolvedPackage, AppError> {
        let registry = vibe_registry::LocalRegistry::new(self.registry.clone()).unwrap();
        crate::from_local_registry(&registry, &self.registry, spelling).map_err(Into::into)
    }
    fn exact(&self, key: &str) -> ResolvedPackage {
        let (name, version) = key.rsplit_once('@').unwrap();
        self.resolve(&format!("{name}@={version}")).unwrap()
    }
    fn install(&self, key: &str, request: &InstallRequest) {
        let progress = Progress::default();
        let resolver = |spelling: &str| self.resolve(spelling);
        crate::install(
            &context(self, &progress, &resolver),
            &self.exact(key),
            request,
        )
        .unwrap();
    }
    fn count(&self, name: &str, version: &str) -> usize {
        match fs::read_to_string(self.source(name, version).join("executions")) {
            Ok(text) => text.lines().count(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => panic!("cannot read fixture executions: {error}"),
        }
    }
    fn receipt(&self, key: &str) -> String {
        let index = Index::load(&self.settings).unwrap();
        let storage = crate::Storage::load(&self.settings).unwrap();
        let root = crate::launch::app_root(&index.installations[key], &storage).unwrap();
        fs::read_to_string(root.join("receipt")).unwrap()
    }
}

fn context<'a>(
    home: &'a Home,
    progress: &'a Progress,
    resolver: &'a dyn Fn(&str) -> Result<ResolvedPackage, AppError>,
) -> OperationContext<'a> {
    OperationContext {
        http_client: None,
        import_scope: None,
        settings: &home.settings,
        json: false,
        offline: true,
        chooser: None,
        proxy_image: Path::new("unused-proxy"),
        facts: HostFacts::default(),
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
        resolver: Some(resolver),
        resolver_scope: ResolverScope::Registry(&home.registry),
        source_reader: None,
        environment: None,
        search_path: Some(std::ffi::OsStr::new("/usr/bin:/bin")),
    }
}

fn source(two_roots: bool) -> (Home, apps::AppsManifest) {
    let a = Home::new();
    a.package("child", "1.0.0", false);
    a.package("root-a", "1.0.0", true);
    a.install(ROOT_A, &InstallRequest::default());
    if two_roots {
        a.package("root-b", "1.0.0", true);
        a.install(ROOT_B, &InstallRequest::default());
    }
    let index = Index::load(&a.settings).unwrap();
    assert_eq!(index.installations[CHILD1].lifecycle.mark, Mark::Auto);
    assert!(index.installations[CHILD1].lifecycle.hidden);
    assert!(index.installations[CHILD1].configuration.is_some());
    assert_eq!(
        index.installations[ROOT_A].lifecycle.requires[0].key,
        CHILD1
    );
    assert_eq!(a.count("child", "1.0.0"), 1);
    let PortableDocument::Full(document) = export_portable(
        &a.settings,
        &PortableExport {
            all: false,
            arrangement: false,
        },
        false,
    )
    .unwrap() else {
        panic!("actual common installations export full bindings");
    };
    assert_eq!(document.roots.len(), if two_roots { 2 } else { 1 });
    assert_eq!(document.application.len(), if two_roots { 3 } else { 2 });
    (a, document)
}
fn destination(two_roots: bool) -> Home {
    let b = Home::new();
    b.package("child", "1.0.0", false);
    b.package("root-a", "1.0.0", true);
    if two_roots {
        b.package("root-b", "1.0.0", true);
    }
    b
}
fn import(home: &Home, document: &apps::AppsManifest) -> Result<(), AppError> {
    let resolved: Vec<_> = document
        .application
        .iter()
        .map(|r| home.exact(&r.key))
        .collect();
    let candidates: Vec<_> = document
        .application
        .iter()
        .zip(&resolved)
        .map(|(row, resolved)| FullImportCandidate {
            application: row,
            resolved,
            option: &resolved.options[0],
        })
        .collect();
    let target = ImportTargetInput {
        protocol: "vibe-app-import-input/1".into(),
        applications: BTreeMap::new(),
    };
    let secrets = ImportSecrets::default();
    let progress = Progress::default();
    let resolver = |spelling: &str| home.resolve(spelling);
    let ctx = context(home, &progress, &resolver);
    let mut prepared = prepare_full_import(FullImportPreparation {
        document,
        candidates: &candidates,
        target: &target,
        secrets: &secrets,
        context: &ctx,
        skip_installed: false,
        accept_license: false,
    })?;
    crate::import_full(&ctx, &mut prepared, &secrets)
}
fn assert_required(index: &Index, root: &str) {
    assert_eq!(index.installations[root].lifecycle.requires.len(), 1);
    assert_eq!(index.installations[root].lifecycle.requires[0].key, CHILD1);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-AUTO-BINDINGS")]
fn imported_exact_configured_child_beats_a_newer_satisfying_installed_version() {
    let (a, document) = source(false);
    let b = destination(false);
    b.package("child", "2.0.0", false);
    b.install(CHILD2, &InstallRequest::default());
    assert!(
        semver::VersionReq::parse(">=1")
            .unwrap()
            .matches(&semver::Version::new(2, 0, 0))
    );
    let newer = Index::load(&b.settings).unwrap().installations[CHILD2].clone();
    import(&b, &document).unwrap();
    let index = Index::load(&b.settings).unwrap();
    assert_required(&index, ROOT_A);
    assert_eq!(index.installations[CHILD1].lifecycle.mark, Mark::Auto);
    assert_eq!(index.installations[CHILD2], newer);
    let original = Index::load(&a.settings).unwrap().installations[CHILD1]
        .configuration
        .clone()
        .unwrap();
    assert_eq!(
        index.installations[CHILD1]
            .configuration
            .as_ref()
            .unwrap()
            .evaluation
            .input,
        original.evaluation.input
    );
    assert_eq!(b.receipt(CHILD1), "source-choice\n");
    assert_eq!(b.count("child", "1.0.0"), 1);
    assert_eq!(b.count("child", "2.0.0"), 1);
    assert_eq!(b.count("root-a", "1.0.0"), 1);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-INSTALLED-REUSE"
)]
fn conflicting_same_key_child_refuses_before_parent_effect_and_preserves_the_child() {
    let (_a, document) = source(false);
    let b = destination(false);
    b.install(
        CHILD1,
        &InstallRequest {
            parameters: vec![("flavor".into(), "target-choice".into())],
            ..Default::default()
        },
    );
    let before = Index::load(&b.settings).unwrap().installations[CHILD1].clone();
    let receipt = b.receipt(CHILD1);
    let error = import(&b, &document).unwrap_err();
    assert!(
        error.to_string().contains(CHILD1),
        "the conflicting exact binding must be named: {error}"
    );
    let after = Index::load(&b.settings).unwrap();
    assert_eq!(after.installations[CHILD1], before);
    assert!(!after.installations.contains_key(ROOT_A));
    assert_eq!(b.receipt(CHILD1), receipt);
    assert_eq!(b.count("child", "1.0.0"), 1);
    assert_eq!(b.count("root-a", "1.0.0"), 0);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-AUTO-BINDINGS")]
fn compatible_roots_consume_the_imported_exact_child_once() {
    let (_a, document) = source(true);
    let b = destination(true);
    import(&b, &document).unwrap();
    let index = Index::load(&b.settings).unwrap();
    assert_required(&index, ROOT_A);
    assert_required(&index, ROOT_B);
    assert_eq!(index.installations[CHILD1].lifecycle.mark, Mark::Auto);
    assert_eq!(b.count("child", "1.0.0"), 1);
    assert_eq!(b.count("root-a", "1.0.0"), 1);
    assert_eq!(b.count("root-b", "1.0.0"), 1);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-INSTALLED-REUSE"
)]
fn compatible_existing_manual_child_is_reused_without_demotion_or_rerun() {
    let (_a, document) = source(true);
    let b = destination(true);
    b.install(CHILD1, &InstallRequest::default());
    crate::hold(
        &b.settings,
        CHILD1,
        true,
        crate::SlotExposure::image_only(Path::new("unused-proxy")),
    )
    .unwrap();
    let before_policy = Index::load(&b.settings).unwrap().installations[CHILD1]
        .lifecycle
        .clone();
    let before = Index::load(&b.settings).unwrap().installations[CHILD1]
        .configuration
        .clone()
        .unwrap();
    assert_eq!(b.count("child", "1.0.0"), 1);
    import(&b, &document).unwrap();
    let index = Index::load(&b.settings).unwrap();
    assert_required(&index, ROOT_A);
    assert_required(&index, ROOT_B);
    assert_eq!(index.installations[CHILD1].lifecycle.mark, Mark::Manual);
    assert_eq!(
        index.installations[CHILD1].lifecycle.hidden,
        before_policy.hidden
    );
    assert_eq!(
        index.installations[CHILD1].lifecycle.held,
        before_policy.held
    );
    assert_eq!(
        index.installations[CHILD1]
            .configuration
            .as_ref()
            .unwrap()
            .evaluation
            .input,
        before.evaluation.input
    );
    assert_eq!(
        b.count("child", "1.0.0"),
        1,
        "compatible Manual child is reused"
    );
    assert_eq!(b.count("root-a", "1.0.0"), 1);
    assert_eq!(b.count("root-b", "1.0.0"), 1);
}

#[cfg(test)]
#[path = "full_dependency_resume_tests.rs"]
mod full_dependency_resume_tests;
