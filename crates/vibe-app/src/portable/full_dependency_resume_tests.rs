//! Deep imported bindings and real public continuation preserve scoped secret ownership.
#![cfg(all(test, unix))]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION");
use super::*;
use std::cell::Cell;
use vibe_wire::generated::{application::e3::import_work, shared as w};

const MIDDLE: &str = "org.example/middle@1.0.0";
const SOURCE_ANSWER: &str = "SYN_DEEP_SOURCE_SECRET_5110";
const TARGET_ANSWER: &str = "SYN_DEEP_TARGET_SECRET_5110";

fn graph(home: &Home, finish_gate: bool) {
    home.package("child", "1.0.0", false);
    home.package("middle", "1.0.0", true);
    home.package("root-a", "1.0.0", true);
    home.package("root-b", "1.0.0", true);
    let middle = home.source("middle", "1.0.0").join("vibe.toml");
    let text = fs::read_to_string(&middle).unwrap();
    fs::write(
        middle,
        text.replace(
            "installer_options.configuration.parameters",
            "installer_options.parameters",
        ),
    )
    .unwrap();
    let root = home.source("root-a", "1.0.0").join("vibe.toml");
    let text = fs::read_to_string(&root).unwrap();
    assert!(text.contains("org.example/child@>=1"));
    fs::write(
        root,
        text.replace("org.example/child@>=1", "org.example/middle@1"),
    )
    .unwrap();
    let child = home.source("child", "1.0.0");
    let mut text = fs::read_to_string(child.join("vibe.toml")).unwrap();
    text.push_str("\n[[installer_options.configuration.parameters]]\nid='token'\nname='Optional token'\ntype='string'\nsecret=true\n");
    if finish_gate {
        text.push_str(&format!("\n[[installer_options.steps]]\nid='release'\ntitle='Await external release'\nkind='shell'\nphase='finish'\ncommand=\"test -f '{}'\"\n", home.settings.path().join("release").display()));
    }
    fs::write(child.join("vibe.toml"), text).unwrap();
    let script = fs::read_to_string(child.join("build.sh")).unwrap();
    fs::write(
        child.join("build.sh"),
        format!("set -e\n[ -n \"${{VIBE_PARAM_TOKEN-}}\" ]\n{script}"),
    )
    .unwrap();
}

fn exported_graph(finish_gate: bool) -> (Home, apps::AppsManifest) {
    let a = Home::new();
    graph(&a, finish_gate);
    if finish_gate {
        fs::create_dir_all(a.settings.path()).unwrap();
        fs::write(a.settings.path().join("release"), "ready").unwrap();
    }
    let progress = Progress::default();
    let resolver = |spelling: &str| a.resolve(spelling);
    let questions = Cell::new(0);
    let prompt = |parameter: &vibe_core::manifest::ParameterDecl, _: Option<&str>| {
        if parameter.secret {
            assert_eq!(parameter.id, "token");
            questions.set(questions.get() + 1);
            Ok(Some(SOURCE_ANSWER.to_owned()))
        } else {
            Ok(None)
        }
    };
    let chooser = |_: &str, _: &[crate::Choice]| Ok(Some(0));
    let ctx = OperationContext {
        chooser: Some(&chooser),
        parameter_prompt: Some(&prompt),
        ..context(&a, &progress, &resolver)
    };
    for key in [ROOT_A, ROOT_B] {
        crate::install(&ctx, &a.exact(key), &InstallRequest::default()).unwrap();
    }
    assert_eq!(
        questions.get(),
        1,
        "only the actual child asks for a secret"
    );
    let index = Index::load(&a.settings).unwrap();
    assert_eq!(index.installations[CHILD1].lifecycle.mark, Mark::Auto);
    assert!(index.installations[MIDDLE].configuration.is_none());
    assert!(index.installations[CHILD1].configuration.as_ref().unwrap().evaluation.parameters.iter().any(|p| matches!(p,w::ConfigurationParameterResult::Secret(v) if v.id=="token" && v.provided && matches!(v.origin,w::ConfigurationParameterOrigin::Explicit(_)))));
    let PortableDocument::Full(document) = export_portable(
        &a.settings,
        &PortableExport {
            all: false,
            arrangement: false,
        },
        false,
    )
    .unwrap() else {
        panic!("full source");
    };
    assert_eq!(document.roots, [ROOT_A, ROOT_B]);
    assert_eq!(document.application.len(), 4);
    assert_eq!(a.count("child", "1.0.0"), 1);
    clean_answers(a.settings.path());
    (a, document)
}

fn answers() -> ImportSecrets {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "protocol":"vibe-app-import-secrets/1",
        "applications":{(CHILD1):{"token":TARGET_ANSWER}}
    }))
    .unwrap();
    crate::parse_import_secrets(&bytes).unwrap()
}

fn prepared_files(
    home: &Home,
    document: &apps::AppsManifest,
    secrets: &ImportSecrets,
) -> (PreparedFullImport, PathBuf, PathBuf) {
    let portable = home._scratch.path().join("portable.json");
    let target_file = home._scratch.path().join("target.json");
    fs::write(
        &portable,
        PortableDocument::Full(document.clone()).json().unwrap(),
    )
    .unwrap();
    fs::write(
        &target_file,
        r#"{"protocol":"vibe-app-import-input/1","applications":{}}"#,
    )
    .unwrap();
    let PortableDocument::Full(parsed) =
        crate::parse_portable_document("portable.json", &fs::read_to_string(&portable).unwrap())
            .unwrap()
    else {
        panic!("full source");
    };
    let target = crate::parse_import_target(&fs::read(&target_file).unwrap()).unwrap();
    let resolved: Vec<_> = parsed
        .application
        .iter()
        .map(|r| home.exact(&r.key))
        .collect();
    let candidates: Vec<_> = parsed
        .application
        .iter()
        .zip(&resolved)
        .map(|(row, resolved)| FullImportCandidate {
            application: row,
            resolved,
            option: &resolved.options[0],
        })
        .collect();
    let progress = Progress::default();
    let resolver = |spelling: &str| home.resolve(spelling);
    let ctx = context(home, &progress, &resolver);
    let prepared = prepare_full_import(FullImportPreparation {
        document: &parsed,
        candidates: &candidates,
        target: &target,
        secrets,
        context: &ctx,
        skip_installed: false,
        accept_license: false,
    })
    .unwrap();
    (prepared, portable, target_file)
}

fn begin(
    home: &Home,
    prepared: &mut PreparedFullImport,
    secrets: &ImportSecrets,
) -> Result<(), AppError> {
    let progress = Progress::default();
    let resolver = |spelling: &str| home.resolve(spelling);
    crate::import_full(&context(home, &progress, &resolver), prepared, secrets)
}
fn resume_cohort(home: &Home, id: &str, secrets: &ImportSecrets) -> PreparedFullImport {
    let progress = Progress::default();
    let resolver = |spelling: &str| home.resolve(spelling);
    crate::resume_full_import(&context(home, &progress, &resolver), id, secrets).unwrap()
}
fn cohort(home: &Home) -> import_work::ImportWork {
    let listing = crate::work(&home.settings, None).unwrap();
    assert_eq!(listing.imports.len(), 1);
    let entry = &listing.imports[0];
    let bytes = fs::read(Path::new(&entry.path).join("import.json")).unwrap();
    let saved: import_work::ImportWork = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(saved.target, entry.target);
    assert_eq!(saved.protocol, "vibe-app-import-work/3");
    saved
}
fn state(home: &Home, key: &str) -> crate::steps::StepsFile {
    crate::steps::StepsFile::load(
        &home
            .settings
            .app_work_dir()
            .join(home.exact(key).package.directory_name()),
    )
    .unwrap()
    .unwrap()
}
fn clean_answers(path: &Path) {
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            clean_answers(&entry.unwrap().path());
        }
    } else {
        let bytes = fs::read(path).unwrap();
        for answer in [SOURCE_ANSWER, TARGET_ANSWER] {
            assert!(
                !bytes.windows(answer.len()).any(|p| p == answer.as_bytes()),
                "{}",
                path.display()
            );
        }
    }
}
fn assert_finished(home: &Home) {
    let index = Index::load(&home.settings).unwrap();
    assert_eq!(
        index.installations[ROOT_A].lifecycle.requires[0].key,
        MIDDLE
    );
    assert_eq!(
        index.installations[MIDDLE].lifecycle.requires[0].key,
        CHILD1
    );
    assert_required(&index, ROOT_B);
    assert!(index.installations[MIDDLE].configuration.is_none());
    assert!(index.installations[CHILD1].configuration.is_some());
    assert_eq!(index.installations[MIDDLE].lifecycle.mark, Mark::Auto);
    assert_eq!(index.installations[CHILD1].lifecycle.mark, Mark::Auto);
    for key in [ROOT_A, ROOT_B, MIDDLE, CHILD1] {
        assert!(index.installations[key].pending.is_none());
    }
    for name in ["root-a", "root-b", "middle"] {
        assert_eq!(home.count(name, "1.0.0"), 1);
    }
    let work = crate::work(&home.settings, None).unwrap();
    assert!(work.entries.is_empty() && work.imports.is_empty());
    let exported = export_portable(
        &home.settings,
        &PortableExport {
            all: false,
            arrangement: false,
        },
        false,
    )
    .unwrap();
    for text in [exported.json().unwrap(), exported.toml().unwrap()] {
        assert!(!text.contains(SOURCE_ANSWER) && !text.contains(TARGET_ANSWER));
    }
    clean_answers(home.settings.path());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
fn deep_legacy_and_direct_roots_share_one_scoped_secret_child() {
    let (_a, document) = exported_graph(false);
    let b = Home::new();
    graph(&b, false);
    let secrets = answers();
    let (mut prepared, _, _) = prepared_files(&b, &document, &secrets);
    begin(&b, &mut prepared, &secrets).unwrap();
    assert_eq!(prepared.results().len(), 2);
    assert_eq!(b.count("child", "1.0.0"), 1);
    assert_finished(&b);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION")]
fn missing_deep_secret_retains_real_owner_chain_then_resumes_without_public_files() {
    let (_a, document) = exported_graph(false);
    let b = Home::new();
    graph(&b, false);
    let empty = ImportSecrets::default();
    let (mut prepared, portable, target) = prepared_files(&b, &document, &empty);
    let error = begin(&b, &mut prepared, &empty).unwrap_err();
    assert!(!error.to_string().contains(SOURCE_ANSWER));
    let saved = cohort(&b);
    assert_eq!(saved.cohort.cursor, 0);
    assert!(saved.cohort.results.is_empty());
    let current = saved
        .cohort
        .current
        .as_ref()
        .expect("the root began before dependency descent");
    assert_eq!(current.key, ROOT_A);
    let waiting = saved
        .cohort
        .waiting
        .as_ref()
        .expect("scoped child input stays pending");
    assert_eq!(waiting.key, CHILD1);
    assert_eq!(waiting.parameters, ["token"]);
    let root = state(&b, ROOT_A);
    let middle = state(&b, MIDDLE);
    let root_op = root.operation.as_ref().unwrap();
    let middle_op = middle.operation.as_ref().unwrap();
    assert_eq!(&root_op.target, current);
    assert_eq!(
        root_op
            .children
            .iter()
            .find(|c| c.key == MIDDLE)
            .unwrap()
            .target
            .as_ref(),
        Some(&middle_op.target)
    );
    for name in ["root-a", "root-b", "middle", "child"] {
        assert_eq!(b.count(name, "1.0.0"), 0);
    }
    clean_answers(b.settings.path());
    fs::remove_file(&portable).unwrap();
    fs::remove_file(&target).unwrap();
    let result = resume_cohort(&b, &saved.target.operation, &answers());
    assert_eq!(result.results().len(), 2);
    assert!(!portable.exists() && !target.exists());
    assert_eq!(b.count("child", "1.0.0"), 1);
    assert_finished(&b);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-FINALIZATION")]
fn direct_child_finish_resume_allows_later_cohort_reuse_without_child_source_or_secret() {
    let (_a, document) = exported_graph(true);
    let b = Home::new();
    graph(&b, true);
    let secrets = answers();
    let (mut prepared, portable, target_file) = prepared_files(&b, &document, &secrets);
    begin(&b, &mut prepared, &secrets).unwrap_err();
    let saved = cohort(&b);
    assert_eq!(saved.cohort.cursor, 0);
    assert!(saved.cohort.results.is_empty());
    let index = Index::load(&b.settings).unwrap();
    let child = index.installations[CHILD1]
        .pending
        .clone()
        .expect("real required finish failed after publication");
    assert_eq!(b.count("child", "1.0.0"), 1);
    for name in ["middle", "root-a", "root-b"] {
        assert_eq!(b.count(name, "1.0.0"), 0);
    }
    let source = b.source("child", "1.0.0");
    let manifest = fs::read(source.join("vibe.toml")).unwrap();
    let script = fs::read(source.join("build.sh")).unwrap();
    fs::write(b.settings.path().join("release"), "ready").unwrap();
    let progress = Progress::default();
    let resolver = |spelling: &str| b.resolve(spelling);
    crate::resume(
        &context(&b, &progress, &resolver),
        &crate::ResumeRequest {
            key: child.key.clone(),
            operation: child.operation.clone(),
            parameters: vec![("token".into(), TARGET_ANSWER.into())],
        },
    )
    .unwrap();
    assert_eq!(
        b.count("child", "1.0.0"),
        1,
        "completed prepare must not rerun"
    );
    assert_eq!(fs::read(source.join("vibe.toml")).unwrap(), manifest);
    assert_eq!(fs::read(source.join("build.sh")).unwrap(), script);
    assert!(
        Index::load(&b.settings).unwrap().installations[CHILD1]
            .pending
            .is_none()
    );
    let receipt = b.receipt(CHILD1);
    fs::remove_dir_all(&source).unwrap();
    fs::remove_file(&portable).unwrap();
    fs::remove_file(&target_file).unwrap();
    let result = resume_cohort(&b, &saved.target.operation, &ImportSecrets::default());
    assert_eq!(result.results().len(), 2);
    assert!(!source.exists());
    assert_eq!(b.receipt(CHILD1), receipt);
    assert_finished(&b);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-FINALIZATION")]
fn full_export_sees_target_only_dependency_before_its_work_starts() {
    let (_a, document) = exported_graph(false);
    let b = Home::new();
    graph(&b, false);
    b.package("extra", "1.0.0", false);
    b.package("extra", "2.0.0", false);
    let progress = Progress::default();
    let resolver = |spelling: &str| b.resolve(spelling);
    crate::install(
        &context(&b, &progress, &resolver),
        &b.exact("org.example/extra@1.0.0"),
        &InstallRequest::default(),
    )
    .unwrap();
    let manifest = b.source("root-a", "1.0.0").join("vibe.toml");
    let before = fs::read_to_string(&manifest).unwrap();
    let old = "requires=[{package='org.example/middle@1',scope='build'}]";
    assert!(before.contains(old));
    fs::write(&manifest, before.replace(old, "requires=[{package='org.example/middle@1',scope='build'},{package='org.example/extra@2',scope='build'}]")).unwrap();
    let empty = ImportSecrets::default();
    let (mut prepared, _, _) = prepared_files(&b, &document, &empty);
    begin(&b, &mut prepared, &empty).unwrap_err();
    let root = state(&b, ROOT_A);
    assert!(
        root.operation
            .as_ref()
            .unwrap()
            .children
            .iter()
            .any(|c| c.key == "org.example/extra@2.0.0" && c.target.is_none())
    );
    let index_before = Index::load(&b.settings).unwrap();
    assert!(
        !index_before
            .installations
            .contains_key("org.example/extra@2.0.0")
    );
    let error = export_portable(
        &b.settings,
        &PortableExport {
            all: false,
            arrangement: false,
        },
        true,
    )
    .unwrap_err();
    assert!(error.to_string().contains("unfinished import"), "{error}");
    assert_eq!(b.count("extra", "2.0.0"), 0);
}

#[cfg(test)]
#[path = "full_refused_tests.rs"]
mod full_refused_tests;

#[cfg(test)]
#[path = "full_context_tests.rs"]
mod full_context_tests;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION")]
fn direct_resume_of_target_only_child_uses_only_its_exact_ephemeral_answers() {
    for supply in [false, true] {
        let (_a, document) = exported_graph(false);
        let b = Home::new();
        graph(&b, false);
        b.package("extra", "1.0.0", false);
        let extra = b.source("extra", "1.0.0");
        let mut text = fs::read_to_string(extra.join("vibe.toml")).unwrap();
        text.push_str("\n[[installer_options.configuration.parameters]]\nid='token'\nname='Target-only token'\ntype='string'\nsecret=true\ndefault='PUBLIC_EXTRA_DEFAULT'\n");
        text.push_str(&format!("\n[[installer_options.steps]]\nid='release'\ntitle='Release extra'\nkind='shell'\nphase='finish'\ncommand=\"test -f '{}'\"\n",b.settings.path().join("release-extra").display()));
        fs::write(extra.join("vibe.toml"), text).unwrap();
        let manifest = b.source("root-a", "1.0.0").join("vibe.toml");
        let text = fs::read_to_string(&manifest).unwrap();
        fs::write(&manifest,text.replace("requires=[{package='org.example/middle@1',scope='build'}]","requires=[{package='org.example/middle@1',scope='build'},{package='org.example/extra@1',scope='build'}]")).unwrap();
        let secrets = answers();
        let (mut prepared, _, _) = prepared_files(&b, &document, &secrets);
        begin(&b, &mut prepared, &secrets).unwrap_err();
        let saved = cohort(&b);
        assert!(
            !saved
                .cohort
                .bindings
                .contains_key("org.example/extra@1.0.0")
        );
        let index = Index::load(&b.settings).unwrap();
        let pending = index.installations["org.example/extra@1.0.0"]
            .pending
            .clone()
            .unwrap();
        fs::write(b.settings.path().join("release-extra"), "ready").unwrap();
        let progress = Progress::default();
        let resolver = |name: &str| b.resolve(name);
        crate::resume(
            &context(&b, &progress, &resolver),
            &crate::ResumeRequest {
                key: pending.key,
                operation: pending.operation,
                parameters: if supply {
                    vec![("token".into(), "EXACT_EXTRA_RESUPPLY".into())]
                } else {
                    vec![]
                },
            },
        )
        .unwrap();
        let index = Index::load(&b.settings).unwrap();
        let config = index.installations["org.example/extra@1.0.0"]
            .configuration
            .as_ref()
            .unwrap();
        assert!(config.evaluation.parameters.iter().any(|p| matches!(p,w::ConfigurationParameterResult::Secret(v) if v.id=="token" && v.provided==supply)));
        fs::remove_dir_all(&extra).unwrap();
        resume_cohort(&b, &saved.target.operation, &ImportSecrets::default());
        assert_finished(&b);
        assert!(
            !fs::read_to_string(b.settings.index_path())
                .unwrap()
                .contains("EXACT_EXTRA_RESUPPLY")
        );
    }
}

#[cfg(test)]
#[path = "full_target_only_tests.rs"]
mod full_target_only_tests;
