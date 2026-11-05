//! Saved lookup is enforced only when an ordinary target-only query is consumed.
#![cfg(all(test, unix))]
use super::*;
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-FINALIZATION")]
fn target_only_query_refuses_changed_route_before_callback_then_accepts_saved_route() {
    let (_a, document) = retry_source();
    let b = Home::new();
    retry_graph(&b, false);
    b.package("extra", "1.0.0", false);
    let file = b.source("root-b", "1.0.0").join("vibe.toml");
    let text = fs::read_to_string(&file).unwrap();
    let original = "requires=[{package='org.example/root-a@1',scope='build'}]";
    assert!(text.contains(original));
    fs::write(file,text.replace(original,"requires=[{package='org.example/root-a@1',scope='build'},{package='org.example/extra@1',scope='build'}]")).unwrap();
    let secrets = answers();
    let (mut prepared, _, _) = prepared_files(&b, &document, &secrets);
    begin(&b, &mut prepared, &secrets).unwrap_err();
    let saved = cohort(&b);
    assert_eq!(saved.cohort.cursor, 1);
    let other = tempfile::tempdir().unwrap();
    let calls = Cell::new(0);
    let resolver = |name: &str| {
        calls.set(calls.get() + 1);
        b.resolve(name)
    };
    let progress = Progress::default();
    let wrong = OperationContext {
        resolver_scope: crate::ResolverScope::Registry(other.path()),
        ..context(&b, &progress, &resolver)
    };
    let error =
        crate::resume_full_import(&wrong, &saved.target.operation, &root_answers()).unwrap_err();
    assert_eq!(error.code(), "resume_mismatch");
    assert!(
        error.to_string().contains("retained resolver route"),
        "{error}"
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(b.count("root-b", "1.0.0"), 0);
    assert_eq!(b.count("extra", "1.0.0"), 0);
    crate::resume_full_import(
        &context(&b, &progress, &resolver),
        &saved.target.operation,
        &root_answers(),
    )
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(b.count("extra", "1.0.0"), 1);
    assert_eq!(b.count("child", "1.0.0"), 1);
}
