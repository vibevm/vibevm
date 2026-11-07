//! Reopen exact durable transfer cuts using real failed and successful executions.
#![cfg(all(test, unix))]
use super::*;
use std::os::unix::fs::PermissionsExt;

fn paused() -> (Home, import_work::ImportWork) {
    let (_a, document) = retry_source();
    let b = Home::new();
    retry_graph(&b, false);
    let secrets = answers();
    let (mut prepared, _, _) = prepared_files(&b, &document, &secrets);
    assert!(matches!(
        begin(&b, &mut prepared, &secrets),
        Err(AppError::Configuration(
            crate::configuration::ConfigurationError::ImportInputRequired { .. }
        ))
    ));
    let saved = cohort(&b);
    assert_eq!(saved.cohort.completed_bindings.len(), 2);
    let installed = Index::load(&b.settings).unwrap();
    for receipt in saved.cohort.completed_bindings.values() {
        let child = state(&b, &receipt.target.key);
        let evidence = child.source_evidence.get(&receipt.target.key).unwrap();
        assert_eq!(
            installed.installations[&receipt.target.key]
                .source_evidence
                .as_ref(),
            Some(evidence)
        );
        assert_eq!(evidence.acquisition, Some(w::SourceAcquisition::Directory));
        assert!(
            evidence
                .source_url
                .as_ref()
                .unwrap()
                .contains(b.settings.path().parent().unwrap().to_str().unwrap())
        );
        let parent = state(&b, &receipt.previous_parent.key);
        assert!(!parent.source_evidence.contains_key(&receipt.target.key));
    }
    (b, saved)
}
fn path(home: &Home, target: &w::ApplicationPendingTarget) -> PathBuf {
    crate::operations::pending::work(
        &home.settings,
        &crate::operations::pending::coordinate(&target.key).unwrap(),
    )
}
fn save(home: &Home, saved: &import_work::ImportWork) {
    let file = home
        .settings
        .app_work_dir()
        .join("imports")
        .join(&saved.target.operation)
        .join("import.json");
    fs::write(file, serde_json::to_vec_pretty(saved).unwrap()).unwrap();
}
fn historic_edge(
    home: &Home,
    parent: &w::ApplicationPendingTarget,
    child: &w::ApplicationPendingTarget,
) -> w::ApplicationPendingChild {
    fn scan(
        directory: &Path,
        parent: &w::ApplicationPendingTarget,
        child: &w::ApplicationPendingTarget,
    ) -> Option<w::ApplicationPendingChild> {
        if directory.join("steps.json").is_file()
            && let Ok(Some(state)) = crate::steps::StepsFile::load(directory)
            && let Some(op) = state.operation
            && op.target == *parent
            && let Some(edge) = op
                .children
                .into_iter()
                .find(|edge| edge.target.as_ref() == Some(child))
        {
            return Some(edge);
        }
        for entry in fs::read_dir(directory).ok()?.flatten() {
            if entry.file_type().ok()?.is_dir()
                && let Some(edge) = scan(&entry.path(), parent, child)
            {
                return Some(edge);
            }
        }
        None
    }
    scan(&home.settings.app_runs_dir(), parent, child)
        .expect("real run snapshot retains the original acknowledged edge")
}
// Restore the exact old edge and owner from actual retained run evidence;
// neither an installation nor its successful publication is fabricated.
fn restore_cut(
    home: &Home,
    saved: &mut import_work::ImportWork,
    stage: w::ApplicationImportCompletedBindingState,
    old_edges: bool,
) {
    let receipts = saved
        .cohort
        .completed_bindings
        .values()
        .cloned()
        .collect::<Vec<_>>();
    for receipt in &receipts {
        let parent_path = path(home, &receipt.previous_parent);
        let mut parent = crate::steps::StepsFile::load(&parent_path)
            .unwrap()
            .unwrap();
        let op = parent.operation.as_mut().unwrap();
        assert_eq!(op.target, receipt.previous_parent);
        op.parent = Some(receipt.previous_parent_owner.clone());
        op.children
            .retain(|edge| edge.target.as_ref() != Some(&receipt.target));
        if old_edges {
            op.children.push(historic_edge(
                home,
                &receipt.previous_parent,
                &receipt.target,
            ));
        }
        parent.save(&parent_path).unwrap();
    }
    for receipt in &receipts {
        let child_path = path(home, &receipt.target);
        let mut child = crate::steps::StepsFile::load(&child_path).unwrap().unwrap();
        child.operation.as_mut().unwrap().parent = Some(
            if stage == w::ApplicationImportCompletedBindingState::Captured {
                crate::operations::pending::application_reference(&receipt.previous_parent)
            } else {
                crate::operations::pending::import_reference(&saved.target, receipt.origin_row)
            },
        );
        child.save(&child_path).unwrap();
        saved
            .cohort
            .completed_bindings
            .get_mut(&receipt.target.key)
            .unwrap()
            .state = stage.clone();
    }
    save(home, saved);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
fn public_resume_recovers_captured_moved_and_unlinked_transfer_cuts() {
    for (stage, old_edges) in [
        (w::ApplicationImportCompletedBindingState::Captured, true),
        (w::ApplicationImportCompletedBindingState::Reparented, true),
        (w::ApplicationImportCompletedBindingState::Reparented, false),
    ] {
        let (home, mut saved) = paused();
        restore_cut(&home, &mut saved, stage, old_edges);
        let result = resume_cohort(&home, &saved.target.operation, &root_answers());
        assert_eq!(
            result.results()[0].action,
            w::ApplicationPendingImportResultAction::Refused
        );
        assert_eq!(
            result.results()[1].action,
            w::ApplicationPendingImportResultAction::Installed
        );
        assert_eq!(home.count("child", "1.0.0"), 1);
        assert_eq!(home.count("middle", "1.0.0"), 1);
        let installed = Index::load(&home.settings).unwrap();
        for key in [MIDDLE, CHILD1] {
            assert_eq!(
                installed.installations[key].source_evidence,
                saved.source_evidence.get(key).cloned()
            );
        }
        let work = crate::work(&home.settings, None).unwrap();
        assert!(work.imports.is_empty() && work.entries.is_empty());
    }
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-RETAINED-SUBTREE"
)]
fn missing_captured_edge_or_changed_publication_refuses_before_future_root_effects() {
    for publication_change in [false, true] {
        let (home, mut saved) = paused();
        if publication_change {
            let mut index = Index::load(&home.settings).unwrap();
            index.installations.get_mut(CHILD1).unwrap().lifecycle.held = true;
            index
                .save(
                    &home.settings,
                    &crate::HomeEvent::new(crate::EventKind::Updated),
                )
                .unwrap();
        } else {
            restore_cut(
                &home,
                &mut saved,
                w::ApplicationImportCompletedBindingState::Captured,
                false,
            );
        }
        let file = home
            .settings
            .app_work_dir()
            .join("imports")
            .join(&saved.target.operation)
            .join("import.json");
        let before = fs::read(&file).unwrap();
        let progress = Progress::default();
        let resolver = |name: &str| home.resolve(name);
        let error = crate::resume_full_import(
            &context(&home, &progress, &resolver),
            &saved.target.operation,
            &root_answers(),
        )
        .unwrap_err();
        assert_eq!(error.code(), "resume_mismatch", "{error}");
        assert!(
            fs::read(&file).unwrap() == before,
            "rejected transfer must not checkpoint new input"
        );
        assert_eq!(home.count("root-b", "1.0.0"), 0);
        assert_eq!(home.count("child", "1.0.0"), 1);
    }
}
struct WritableOnDrop(PathBuf);
impl Drop for WritableOnDrop {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-FINALIZATION")]
fn completed_cohort_cleanup_reopens_without_sources_or_consuming_repeated_answers() {
    let (home, saved) = paused();
    let child = path(&home, &saved.cohort.completed_bindings[CHILD1].target);
    let reset = WritableOnDrop(child.clone());
    fs::set_permissions(&child, fs::Permissions::from_mode(0o500)).unwrap();
    let progress = Progress::default();
    let resolver = |name: &str| home.resolve(name);
    let error = crate::resume_full_import(
        &context(&home, &progress, &resolver),
        &saved.target.operation,
        &root_answers(),
    )
    .unwrap_err();
    let saved = cohort(&home);
    assert_eq!(saved.phase, w::ApplicationImportPhase::Complete, "{error}");
    assert!(saved.cohort.arrangement_done);
    assert_eq!(
        saved.cohort.results[1].action,
        w::ApplicationPendingImportResultAction::Installed
    );
    drop(reset);
    let index_before = fs::read(home.settings.index_path()).unwrap();
    fs::remove_dir_all(&home.registry).unwrap();
    let irrelevant=crate::parse_import_secrets(br#"{"protocol":"vibe-app-import-secrets/1","applications":{"org.example/unused@1.0.0":{"token":"NOT_CONSUMED"}}}"#).unwrap();
    let result = crate::resume_full_import(
        &context(&home, &progress, &resolver),
        &saved.target.operation,
        &irrelevant,
    )
    .unwrap();
    assert_eq!(result.results().len(), 2);
    assert_eq!(fs::read(home.settings.index_path()).unwrap(), index_before);
    let work = crate::work(&home.settings, None).unwrap();
    assert!(work.imports.is_empty() && work.entries.is_empty());
}
