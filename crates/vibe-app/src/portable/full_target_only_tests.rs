//! Ordinary target-only requirements do not acquire invented imported input scope.
#![cfg(all(test, unix))]
use super::*;
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT")]
fn ordinary_child_question_is_not_mislabeled_as_root_secret_input() {
    let (_a, document) = exported_graph(false);
    let b = Home::new();
    graph(&b, false);
    b.package("extra", "1.0.0", false);
    let extra = b.source("extra", "1.0.0").join("vibe.toml");
    let text = fs::read_to_string(&extra).unwrap();
    fs::write(extra,format!("{text}\n[[installer_options.configuration.parameters]]\nid='token'\nname='Ordinary target-only token'\ntype='string'\nsecret=true\nrequired=true\n")).unwrap();
    let file = b.source("root-a", "1.0.0").join("vibe.toml");
    let text = fs::read_to_string(&file).unwrap();
    fs::write(file,text.replace("requires=[{package='org.example/middle@1',scope='build'}]","requires=[{package='org.example/middle@1',scope='build'},{package='org.example/extra@1',scope='build'}]")).unwrap();
    let secrets = answers();
    let (mut prepared, _, _) = prepared_files(&b, &document, &secrets);
    let error = begin(&b, &mut prepared, &secrets).unwrap_err();
    assert_eq!(error.code(), "requirement_unresolved", "{error}");
    assert!(error.to_string().contains("org.example/extra@1.0.0"));
    let saved = cohort(&b);
    assert!(saved.cohort.waiting.is_none());
    assert_eq!(b.count("extra", "1.0.0"), 0);
    let calls = Cell::new(0);
    let prompt = |p: &vibe_core::manifest::ParameterDecl, _: Option<&str>| {
        if p.secret {
            assert_eq!(p.id, "token");
            calls.set(calls.get() + 1);
            Ok(Some("EXTRA_ORDINARY_ANSWER".into()))
        } else {
            Ok(None)
        }
    };
    let chooser = |_: &str, _: &[crate::Choice]| Ok(Some(0));
    let progress = Progress::default();
    let resolver = |name: &str| b.resolve(name);
    let ctx = OperationContext {
        chooser: Some(&chooser),
        parameter_prompt: Some(&prompt),
        ..context(&b, &progress, &resolver)
    };
    crate::resume_full_import(&ctx, &saved.target.operation, &ImportSecrets::default()).unwrap();
    assert_eq!(
        calls.get(),
        1,
        "only the actual ordinary child asks; bound peers do not receive its value"
    );
    assert_eq!(b.count("extra", "1.0.0"), 1);
    assert_finished(&b);
    assert!(
        !fs::read_to_string(b.settings.index_path())
            .unwrap()
            .contains("EXTRA_ORDINARY_ANSWER")
    );
}
