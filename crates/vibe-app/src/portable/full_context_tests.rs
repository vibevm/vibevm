//! Imported context binds public job meaning before accepting new ephemeral answers.
#![cfg(all(test, unix))]
use super::*;

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTEXT-IDENTITY"
)]
fn changed_future_binding_refuses_before_resupply_checkpoint_or_effects() {
    let (_a, document) = exported_graph(false);
    let b = Home::new();
    graph(&b, false);
    let empty = ImportSecrets::default();
    let (mut prepared, _, _) = prepared_files(&b, &document, &empty);
    begin(&b, &mut prepared, &empty).unwrap_err();
    let mut saved = cohort(&b);
    saved
        .cohort
        .bindings
        .get_mut(ROOT_B)
        .unwrap()
        .application
        .held = true;
    let file = b
        .settings
        .app_work_dir()
        .join("imports")
        .join(&saved.target.operation)
        .join("import.json");
    fs::write(&file, serde_json::to_vec_pretty(&saved).unwrap()).unwrap();
    let before = fs::read(&file).unwrap();
    let progress = Progress::default();
    let resolver = |name: &str| b.resolve(name);
    for secrets in [ImportSecrets::default(), answers()] {
        let error = crate::resume_full_import(
            &context(&b, &progress, &resolver),
            &saved.target.operation,
            &secrets,
        )
        .unwrap_err();
        assert_eq!(error.code(), "resume_mismatch", "{error}");
        assert!(
            fs::read(&file).unwrap() == before,
            "binding drift must be rejected before current secret obligations are checkpointed"
        );
    }
    for name in ["root-a", "root-b", "middle", "child"] {
        assert_eq!(b.count(name, "1.0.0"), 0);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTEXT-IDENTITY"
)]
fn immutable_scope_changes_for_roots_and_history_but_not_current_secret_answers() {
    let (_a, document) = exported_graph(false);
    let b = Home::new();
    graph(&b, false);
    let empty = ImportSecrets::default();
    let (mut prepared, _, _) = prepared_files(&b, &document, &empty);
    begin(&b, &mut prepared, &empty).unwrap_err();
    let saved = cohort(&b);
    let scope = |roots, bindings, requirements, secrets| {
        crate::ImportScope::new(saved.target.clone(), roots, bindings, requirements, secrets)
            .unwrap()
            .fingerprint()
            .to_owned()
    };
    let original = scope(
        saved.cohort.roots.clone(),
        saved.cohort.bindings.clone(),
        Default::default(),
        &empty,
    );
    let offered = answers();
    assert_eq!(
        original,
        scope(
            saved.cohort.roots.clone(),
            saved.cohort.bindings.clone(),
            std::collections::BTreeMap::from([(CHILD1.into(), vec!["token".into()])]),
            &offered
        )
    );
    let mut reordered = saved.cohort.roots.clone();
    reordered.reverse();
    assert_ne!(
        original,
        scope(
            reordered,
            saved.cohort.bindings.clone(),
            Default::default(),
            &empty
        )
    );
    let mut history = saved.cohort.bindings.clone();
    let w::PortableConfigurationState::Configured(c) =
        &mut history.get_mut(CHILD1).unwrap().application.configuration
    else {
        panic!("configured child")
    };
    for p in &mut c.history.parameters {
        if let w::PortableParameterHistory::Secret(p) = p {
            p.provided = false;
            p.origin = w::ConfigurationParameterOrigin::Absent(Box::new(
                w::ConfigurationParameterOriginAbsent {},
            ));
        }
    }
    assert_ne!(
        original,
        scope(
            saved.cohort.roots.clone(),
            history,
            Default::default(),
            &empty
        )
    );
}
