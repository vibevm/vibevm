#[test]
fn second_identical_execution_observes_prior_bytes_and_reports_fresh() {
    let root = TempDir::new().unwrap();
    let registry = registry(root.path());
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let execution = execution(root.path(), &targets, &registry, &routes);
    let first_calls = FakeCalls::new(root.path(), Mode::Happy);
    let first = execute_with(
        &execution,
        &targets[0],
        &entry(PIN),
        &binding(PIN),
        &first_calls,
    )
    .unwrap();
    let output = std::fs::read(root.path().join(&first[0].path_relative)).unwrap();
    let record = std::fs::read(root.path().join(&first[0].record)).unwrap();

    let second_calls = FakeCalls::new(root.path(), Mode::Happy);
    let second = execute_with(
        &execution,
        &targets[0],
        &entry(PIN),
        &binding(PIN),
        &second_calls,
    )
    .unwrap();
    assert!(second_calls.observed_prior.get());
    assert!(second.iter().all(|artifact| artifact.fresh));
    assert_eq!(
        std::fs::read(root.path().join(&second[0].path_relative)).unwrap(),
        output
    );
    assert_eq!(
        std::fs::read(root.path().join(&second[0].record)).unwrap(),
        record
    );
    assert!(!rollback_root(root.path()).exists());
}

#[test]
fn mutation_then_apply_or_verify_failure_restores_tree_and_occupied_records() {
    let root = TempDir::new().unwrap();
    let registry = registry(root.path());
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let execution = execution(root.path(), &targets, &registry, &routes);
    let first = execute_with(
        &execution,
        &targets[0],
        &entry(PIN),
        &binding(PIN),
        &FakeCalls::new(root.path(), Mode::Happy),
    )
    .unwrap();
    let stage = tree_bytes(&root.path().join("target/vibe-native/native-demo"));
    let records = first
        .iter()
        .map(|artifact| std::fs::read(root.path().join(&artifact.record)).unwrap())
        .collect::<Vec<_>>();
    for mode in [
        Mode::MutateApplyFail,
        Mode::MutateApplyFault,
        Mode::MutateVerifyFail,
        Mode::MutateVerifyMismatch,
    ] {
        assert!(
            execute_with(
                &execution,
                &targets[0],
                &entry(PIN),
                &binding(PIN),
                &FakeCalls::new(root.path(), mode),
            )
            .is_err()
        );
        assert_eq!(
            tree_bytes(&root.path().join("target/vibe-native/native-demo")),
            stage
        );
        for (artifact, prior) in first.iter().zip(&records) {
            assert_eq!(
                std::fs::read(root.path().join(&artifact.record)).unwrap(),
                *prior
            );
        }
        assert!(!rollback_root(root.path()).exists());
    }
}

#[test]
fn unsafe_staging_occupants_refuse_before_apply() {
    for planted in ["file", "link"] {
        let root = TempDir::new().unwrap();
        let stage = root.path().join("target/vibe-native/native-demo");
        std::fs::create_dir_all(stage.parent().unwrap()).unwrap();
        let outside = if planted == "file" {
            std::fs::write(&stage, b"occupant").unwrap();
            None
        } else {
            let outside = TempDir::new().unwrap();
            if !link_directory(outside.path(), &stage) {
                continue;
            }
            Some(outside)
        };
        let registry = registry(root.path());
        let routes = MechanismRoutes::default();
        let targets = [target()];
        let execution = execution(root.path(), &targets, &registry, &routes);
        let calls = FakeCalls::new(root.path(), Mode::Happy);
        assert!(execute_with(&execution, &targets[0], &entry(PIN), &binding(PIN), &calls).is_err());
        assert_eq!(calls.calls.borrow().as_slice(), &["plan", "fingerprint"]);
        drop(outside);
    }
}

fn tree_bytes(root: &Path) -> Vec<(String, Vec<u8>)> {
    must(walk_tree(root), "tree snapshot")
        .into_iter()
        .map(|(relative, absolute)| (relative, must(std::fs::read(absolute), "tree member")))
        .collect()
}

fn rollback_root(root: &Path) -> PathBuf {
    root.join("target/.vibe-native-rollback-native-demo")
}

#[cfg(unix)]
fn link_directory(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn link_directory(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_dir(target, link).is_ok()
}
