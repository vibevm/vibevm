use super::*;

fn diff_args(dir: &Path) -> DocDiffArgs {
    DocDiffArgs {
        from: "1.0.0".into(),
        to: "2.0.0".into(),
        allow_now: false,
        format: "md".into(),
        path: dir.to_path_buf(),
        binary: None,
        timeout: 300,
    }
}

/// A comparison against the working tree is a hint to a reconciliation,
/// never a version. It stays behind a flag so the decision to take it is
/// something somebody typed.
#[test]
fn now_is_refused_until_the_operator_asks_for_it() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let args = DocDiffArgs {
        to: NOW.into(),
        ..diff_args(tmp.path())
    };
    let error = load(tmp.path(), NOW, &args, &DocEnv::default()).expect_err("a refusal");
    let error = error.to_string();
    assert!(error.contains("`now` is not a version"), "{error}");
    assert!(error.contains("--allow-now"), "{error}");
    assert!(error.contains("OBS-VERSION-CONTRACT"), "{error}");
}

#[test]
fn a_version_with_no_snapshot_is_refused_by_name_and_says_what_is_there() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let args = diff_args(tmp.path());
    let error = load(tmp.path(), "1.0.0", &args, &DocEnv::default())
        .expect_err("a refusal")
        .to_string();
    assert!(error.contains("no surface recorded for `1.0.0`"), "{error}");
    assert!(error.contains("no snapshots at all"), "{error}");
    assert!(error.contains("vibe doc surface --record 1.0.0"), "{error}");
}

#[test]
fn a_recorded_version_is_found_where_the_package_keeps_it() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let surface = DocSurface {
        schema_version: vibe_doc::surface::SCHEMA_VERSION,
        version: "1.0.0".into(),
        commands: Vec::new(),
        manifest_fields: Vec::new(),
        lock_fields: Vec::new(),
        schemas: Vec::new(),
        facts: Vec::new(),
        formats: Vec::new(),
    };
    vibe_doc::surface::write(&surface, &vibe_doc::surface::path_for(tmp.path(), "1.0.0"))
        .expect("a written snapshot");
    let args = diff_args(tmp.path());
    let read = load(tmp.path(), "1.0.0", &args, &DocEnv::default()).expect("the snapshot");
    assert_eq!(read.version, "1.0.0");
}

/// A run outside a checkout has no observed corpus, so it records no
/// obligations. The other five halves of a surface are still a surface.
#[test]
fn a_run_with_no_working_directory_records_no_obligations() {
    assert!(
        obligations(&None, &vibe_core::progress::Progress::default())
            .expect("no corpus")
            .is_empty()
    );
}
