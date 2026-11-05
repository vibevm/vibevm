//! A refused root releases its slot while successful secret descendants stay reserved.
#![cfg(all(test, unix))]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE");
use super::*;

fn retry_graph(home: &Home, source: bool) {
    graph(home, false);
    let root_b = home.source("root-b", "1.0.0").join("vibe.toml");
    let text = fs::read_to_string(&root_b).unwrap();
    let text = text.replace("org.example/child@^1", "org.example/root-a@1");
    fs::write(root_b, format!("{text}\n[[installer_options.configuration.parameters]]\nid='token'\nname='Root token'\ntype='string'\nsecret=true\n")).unwrap();
    let root_a = home.source("root-a", "1.0.0");
    let recipe = fs::read_to_string(root_a.join("build.sh")).unwrap();
    fs::write(root_a.join("build.sh"), format!("printf 'attempt\\n' >> \"$VIBE_PACKAGE/attempts\"\nif ! test -f \"$VIBE_PACKAGE/retry\"; then printf ready > \"$VIBE_PACKAGE/retry\"; exit 23; fi\n{recipe}")).unwrap();
    if source {
        fs::write(root_a.join("retry"), "ready").unwrap();
    }
}
fn retry_source() -> (Home, apps::AppsManifest) {
    let home = Home::new();
    retry_graph(&home, true);
    let progress = Progress::default();
    let resolver = |name: &str| home.resolve(name);
    let chooser = |_: &str, _: &[crate::Choice]| Ok(Some(0));
    let prompt = |p: &vibe_core::manifest::ParameterDecl, _: Option<&str>| {
        Ok(p.secret.then(|| SOURCE_ANSWER.to_owned()))
    };
    let ctx = OperationContext {
        chooser: Some(&chooser),
        parameter_prompt: Some(&prompt),
        ..context(&home, &progress, &resolver)
    };
    for key in [ROOT_A, ROOT_B] {
        crate::install(&ctx, &home.exact(key), &InstallRequest::default()).unwrap();
    }
    let PortableDocument::Full(document) = export_portable(
        &home.settings,
        &PortableExport {
            all: false,
            arrangement: false,
        },
        false,
    )
    .unwrap() else {
        panic!("full source")
    };
    (home, document)
}
fn root_answers() -> ImportSecrets {
    crate::parse_import_secrets(&serde_json::to_vec(&serde_json::json!({"protocol":"vibe-app-import-secrets/1","applications":{(ROOT_B):{"token":TARGET_ANSWER}}})).unwrap()).unwrap()
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
fn refused_root_retry_reuses_reserved_successful_secret_descendants() {
    let (_a, document) = retry_source();
    let b = Home::new();
    retry_graph(&b, false);
    let child_answers = answers();
    let (mut prepared, _, _) = prepared_files(&b, &document, &child_answers);
    let error = begin(&b, &mut prepared, &child_answers).unwrap_err();
    assert!(
        matches!(error, AppError::Configuration(crate::configuration::ConfigurationError::ImportInputRequired { ref key,.. }) if key == ROOT_B),
        "{error}"
    );
    let saved = cohort(&b);
    assert_eq!(saved.cohort.cursor, 1);
    assert_eq!(
        saved.cohort.results[0].action,
        w::ApplicationPendingImportResultAction::Refused
    );
    assert!(
        saved.cohort.results[0].target.is_some(),
        "historical Refused attempt is not a success receipt"
    );
    assert_eq!(saved.cohort.completed_bindings.len(), 2);
    for key in [MIDDLE, CHILD1] {
        let receipt = &saved.cohort.completed_bindings[key];
        assert_eq!(
            receipt.state,
            w::ApplicationImportCompletedBindingState::Detached
        );
        assert_eq!(receipt.origin_row, 0);
        assert_eq!(receipt.origin_root.key, ROOT_A);
        assert_eq!(
            state(&b, key).operation.as_ref().unwrap().parent,
            Some(crate::operations::pending::import_reference(
                &saved.target,
                0
            ))
        );
    }
    let child_generation = saved.cohort.completed_bindings[CHILD1]
        .target
        .operation
        .clone();
    let index = Index::load(&b.settings).unwrap();
    assert!(!index.installations.contains_key(ROOT_A));
    assert!(index.installations.contains_key(CHILD1));
    let progress = Progress::default();
    let resolver = |name: &str| b.resolve(name);
    let ctx = context(&b, &progress, &resolver);
    let index_bytes = fs::read(b.settings.index_path()).unwrap();
    let payload_before = b.receipt(CHILD1);
    let replace = crate::install(
        &ctx,
        &b.exact(CHILD1),
        &InstallRequest {
            parameters: vec![("token".into(), "A_DIFFERENT_OPAQUE_ANSWER".into())],
            ..Default::default()
        },
    );
    let error = replace.unwrap_err();
    assert_eq!(error.code(), "resume_mismatch", "{error}");
    assert!(
        error.to_string().contains(&saved.target.operation),
        "{error}"
    );
    assert_eq!(fs::read(b.settings.index_path()).unwrap(), index_bytes);
    assert_eq!(b.receipt(CHILD1), payload_before);
    assert_eq!(
        state(&b, CHILD1)
            .operation
            .as_ref()
            .unwrap()
            .target
            .operation,
        child_generation
    );
    let error = crate::uninstall(&ctx, CHILD1, &crate::RemoveRequest::default()).unwrap_err();
    assert_eq!(error.code(), "resume_mismatch", "{error}");
    assert!(
        error.to_string().contains(&saved.target.operation),
        "{error}"
    );
    assert_eq!(fs::read(b.settings.index_path()).unwrap(), index_bytes);
    assert_eq!(b.receipt(CHILD1), payload_before);
    assert_eq!(b.count("child", "1.0.0"), 1);
    let result = resume_cohort(&b, &saved.target.operation, &root_answers());
    assert_eq!(
        result.results()[0].action,
        w::ApplicationPendingImportResultAction::Refused
    );
    assert_eq!(
        result.results()[1].action,
        w::ApplicationPendingImportResultAction::Installed
    );
    assert_eq!(b.count("child", "1.0.0"), 1);
    assert_eq!(b.count("middle", "1.0.0"), 1);
    assert_eq!(
        fs::read_to_string(b.source("root-a", "1.0.0").join("attempts"))
            .unwrap()
            .lines()
            .count(),
        2
    );
    let final_index = Index::load(&b.settings).unwrap();
    assert_eq!(
        final_index.installations[ROOT_A].lifecycle.mark,
        Mark::Manual
    );
    assert!(final_index.installations[ROOT_A].pending.is_none());
    let work = crate::work(&b.settings, None).unwrap();
    assert!(work.imports.is_empty() && work.entries.is_empty());
    clean_answers(b.settings.path());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
fn refused_root_does_not_manufacture_imported_receipts_for_target_only_children() {
    let (_a, document) = retry_source();
    let b = Home::new();
    retry_graph(&b, false);
    b.package("extra", "1.0.0", false);
    let manifest = b.source("root-a", "1.0.0").join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(&manifest, text.replace("requires=[{package='org.example/middle@1',scope='build'}]", "requires=[{package='org.example/middle@1',scope='build'},{package='org.example/extra@1',scope='build'}]")).unwrap();
    let child_answers = answers();
    let (mut prepared, _, _) = prepared_files(&b, &document, &child_answers);
    let error = begin(&b, &mut prepared, &child_answers).unwrap_err();
    assert!(
        matches!(error, AppError::Configuration(crate::configuration::ConfigurationError::ImportInputRequired {ref key,..}) if key==ROOT_B),
        "{error}"
    );
    let saved = cohort(&b);
    assert!(
        !saved
            .cohort
            .completed_bindings
            .contains_key("org.example/extra@1.0.0")
    );
    assert_eq!(b.count("extra", "1.0.0"), 1);
    resume_cohort(&b, &saved.target.operation, &root_answers());
    assert_eq!(b.count("extra", "1.0.0"), 1);
    assert!(crate::work(&b.settings, None).unwrap().imports.is_empty());
}

#[cfg(test)]
#[path = "full_receipt_recovery_tests.rs"]
mod full_receipt_recovery_tests;

#[cfg(test)]
#[path = "full_lookup_tests.rs"]
mod full_lookup_tests;
