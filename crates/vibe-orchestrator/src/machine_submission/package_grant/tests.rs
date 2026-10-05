use super::*;
use std::{fs, path::Path};
use vibe_lifecycle::StaticFileJournalInspection;

fn service(root: &Path) -> PackageGrantService {
    PackageGrantService::from_trusted_host(
        Arc::new(Project::open(root).unwrap()),
        "vibe-cli-project-local/e1".into(),
        TargetOs::Linux,
    )
    .unwrap()
}
fn pointer(service: &PackageGrantService) -> Pointer {
    serde_json::from_slice(&fs::read(service.project.root_path().join(&service.path)).unwrap())
        .unwrap()
}
fn namespace_count(root: &Path) -> usize {
    let bytes = fs::read(root.join(".vibe/machine-submissions.json")).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["namespaces"].as_object().unwrap().len()
}

#[test]
fn cold_reattach_replays_one_package_run_without_a_new_namespace() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source.bin"), b"copy once").unwrap();
    let first = service(root.path());
    let host = first.issue(60_000).unwrap();
    assert_eq!(first.scope.target, "launcher");
    assert!(first.scope.context.starts_with("cli-package-host:e1:"));
    let first_result = host.submit("same-key").unwrap();
    assert!(matches!(
        first_result,
        StaticFileJournalInspection::Completed { .. }
    ));
    let output = root
        .path()
        .join("target/vibe-package/launcher/launcher.bin");
    let modified = fs::metadata(&output).unwrap().modified().unwrap();
    let c1 = fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap();
    let retained = pointer(&first);
    drop(host);
    drop(first);
    let second = service(root.path());
    let cold = second.reattach().unwrap();
    assert_eq!(pointer(&second), retained);
    assert_eq!(
        cold.inspect("same-key").unwrap(),
        Some(first_result.clone())
    );
    assert_eq!(cold.submit("same-key").unwrap(), first_result);
    assert_eq!(fs::metadata(output).unwrap().modified().unwrap(), modified);
    assert_eq!(
        fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap(),
        c1
    );
    assert!(matches!(
        second.issue(60_000),
        Err(PackageGrantError::AlreadyIssued)
    ));
}

#[test]
fn issuing_and_missing_pointer_never_mint_replacement() {
    let root = tempfile::tempdir().unwrap();
    let first_service = service(root.path());
    let lease = first_service.lease().unwrap();
    first_service
        .publish(&lease, &first_service.pointer(GrantState::Issuing))
        .unwrap();
    drop(lease);
    assert!(matches!(
        first_service.issue(60_000),
        Err(PackageGrantError::IssuanceIncomplete)
    ));
    assert!(matches!(
        first_service.reattach(),
        Err(PackageGrantError::IssuanceIncomplete)
    ));
    assert!(!root.path().join(".vibe/machine-submissions.json").exists());
    let other = tempfile::tempdir().unwrap();
    let service = service(other.path());
    service.issue(60_000).unwrap();
    fs::remove_file(other.path().join(&service.path)).unwrap();
    assert!(matches!(
        service.reattach(),
        Err(PackageGrantError::MissingPointer)
    ));
    assert!(matches!(
        service.issue(60_000),
        Err(PackageGrantError::MissingPointer)
    ));
    assert_eq!(namespace_count(other.path()), 1);
}

#[test]
fn foreign_pointer_or_c1_scope_refuses_without_reissue() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    service.issue(60_000).unwrap();
    let mut damaged = pointer(&service);
    damaged.scope.target = "other".into();
    fs::write(
        root.path().join(&service.path),
        serde_json::to_vec(&damaged).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        service.reattach(),
        Err(PackageGrantError::ForeignBinding)
    ));
    assert!(matches!(
        service.issue(60_000),
        Err(PackageGrantError::ForeignBinding)
    ));
    assert_eq!(namespace_count(root.path()), 1);
}

#[test]
fn live_policy_change_refuses_new_submit_but_old_key_can_be_inspected() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source.bin"), b"first").unwrap();
    let service = service(root.path());
    let host = service.issue(60_000).unwrap();
    let first = host.submit("old").unwrap();
    let lease = service.lease().unwrap();
    let owner = service.current_owner(&lease).unwrap();
    let old = owner.current_under_lease(&lease).unwrap();
    let mut changed = package_policy_facts();
    changed.allow_detach = true;
    owner
        .update_under_lease(&lease, old.generation(), service.source.clone(), changed)
        .unwrap();
    drop(lease);
    assert!(matches!(
        host.submit("new"),
        Err(FixedPackageHostError::Policy(
            PolicyError::StartupDisagreement
        ))
    ));
    assert_eq!(host.inspect("old").unwrap(), Some(first));
    assert!(matches!(
        service.reattach(),
        Err(PackageGrantError::Policy(PolicyError::StartupDisagreement))
    ));
    assert_eq!(namespace_count(root.path()), 1);
}

#[test]
fn expired_namespace_preserves_old_key_and_refuses_fresh_copy() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source.bin"), b"first").unwrap();
    let service = service(root.path());
    let host = service.issue(1_000).unwrap();
    let first = host.submit("old").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1_100));
    let cold = service.reattach().unwrap();
    assert_eq!(cold.inspect("old").unwrap(), Some(first));
    assert!(matches!(
        cold.submit("new").unwrap(),
        StaticFileJournalInspection::Retired { .. }
    ));
    assert_eq!(namespace_count(root.path()), 1);
}

#[test]
fn uncertain_initial_pointer_blocks_c1_and_reissue() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let path = root.path().join(&service.path);
    vibe_safefs::arm_before_publish_verify(Some(Box::new(move |_, name| {
        if path.file_name() == Some(std::ffi::OsStr::new(name)) {
            fs::write(&path, b"uncertain").unwrap();
        }
    })));
    let result = service.issue(60_000);
    vibe_safefs::arm_before_publish_verify(None);
    assert!(matches!(
        result,
        Err(PackageGrantError::UncertainPublication(_))
    ));
    assert!(service.reattach().is_err());
    assert!(service.issue(60_000).is_err());
    assert!(!root.path().join(".vibe/machine-submissions.json").exists());
}

#[test]
fn final_pointer_uncertainty_retains_one_c1_namespace() {
    let root = tempfile::tempdir().unwrap();
    let service = service(root.path());
    let path = root.path().join(&service.path);
    corrupt_final_publication(path.clone());
    let result = service.issue(60_000);
    vibe_safefs::arm_before_publish_verify(None);
    assert!(matches!(
        result,
        Err(PackageGrantError::UncertainPublication(_))
    ));
    assert_eq!(namespace_count(root.path()), 1);
    assert!(service.reattach().is_err());
    assert!(service.issue(60_000).is_err());
    fs::remove_file(path).unwrap();
    assert!(matches!(
        service.issue(60_000),
        Err(PackageGrantError::MissingPointer)
    ));
    assert_eq!(namespace_count(root.path()), 1);
}

fn corrupt_final_publication(path: std::path::PathBuf) {
    vibe_safefs::arm_before_publish_verify(Some(Box::new(move |_, name| {
        let issued = path.file_name() == Some(std::ffi::OsStr::new(name))
            && matches!(
                serde_json::from_slice::<Pointer>(&fs::read(&path).unwrap())
                    .unwrap()
                    .grant,
                GrantState::Issued { .. }
            );
        if issued {
            fs::write(&path, b"damaged-final-pointer").unwrap();
        } else {
            corrupt_final_publication(path.clone());
        }
    })));
}

#[test]
fn package_and_validate_have_distinct_scopes_and_pointers() {
    use crate::machine_submission::{HostPolicyFacts, cli_grant::CliGrantService};
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname = 'grant-fixture'\nversion = '1.1.0'\n",
    )
    .unwrap();
    let project = Arc::new(Project::open(root.path()).unwrap());
    let package = service(root.path());
    let validate = CliGrantService::from_trusted_host(
        project,
        "launcher".into(),
        "vibe-cli-project-local/e1".into(),
        TargetOs::Linux,
        TrustedPolicySource::from_host_identifier("vibe-cli/fixed-validate/e1".into()).unwrap(),
        HostPolicyFacts {
            approve_captured_script: false,
            approve_exact_local_install: false,
            allow_detach: false,
            allow_cancel_pre_dispatch: false,
            allow_cancel_deferred_boundary: false,
        },
    )
    .unwrap();
    assert!(package.reattach().is_err());
    let _validate_host = validate.issue(60_000).unwrap();
    assert!(package.reattach().is_err());
    let _package_host = package.issue(60_000).unwrap();
    assert_eq!(namespace_count(root.path()), 2);
    assert!(validate.reattach().is_ok());
    assert!(package.reattach().is_ok());
    assert!(package.scope.context.starts_with("cli-package-host:e1:"));
}

#[test]
fn foreign_root_and_host_coordinates_refuse() {
    let parent = tempfile::tempdir().unwrap();
    let selected = parent.path().join("selected");
    let moved = parent.path().join("moved");
    fs::create_dir(&selected).unwrap();
    let service = service(&selected);
    service.issue(60_000).unwrap();
    let changed_principal = PackageGrantService::from_trusted_host(
        Arc::new(Project::open(&selected).unwrap()),
        "different-principal".into(),
        TargetOs::Linux,
    )
    .unwrap();
    assert!(matches!(
        changed_principal.reattach(),
        Err(PackageGrantError::MissingPointer)
    ));
    fs::rename(&selected, &moved).unwrap();
    fs::create_dir(&selected).unwrap();
    assert!(matches!(
        service.reattach(),
        Err(PackageGrantError::ForeignBinding)
    ));
}

#[test]
fn concurrent_first_issue_retains_one_namespace() {
    let root = tempfile::tempdir().unwrap();
    let a = service(root.path());
    let b = service(root.path());
    std::thread::scope(|threads| {
        let left = threads.spawn(|| a.issue(60_000));
        let right = threads.spawn(|| b.issue(60_000));
        let successes = [left.join().unwrap().is_ok(), right.join().unwrap().is_ok()]
            .into_iter()
            .filter(|ok| *ok)
            .count();
        assert_eq!(successes, 1);
    });
    assert_eq!(namespace_count(root.path()), 1);
    assert!(service(root.path()).reattach().is_ok());
}

#[test]
fn process_reattach_worker() {
    let Some(root) = std::env::var_os("C4_PACKAGE_GRANT_FIXTURE_ROOT") else {
        return;
    };
    let service = service(Path::new(&root));
    let host = service.reattach().unwrap();
    assert!(matches!(
        host.inspect("process-key").unwrap(),
        Some(StaticFileJournalInspection::Completed { .. })
    ));
    assert!(matches!(
        host.submit("process-key").unwrap(),
        StaticFileJournalInspection::Completed { .. }
    ));
}

#[test]
fn separate_os_process_reattaches_same_package_result() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source.bin"), b"once").unwrap();
    let host = service(root.path()).issue(60_000).unwrap();
    let first = host.submit("process-key").unwrap();
    let output = root
        .path()
        .join("target/vibe-package/launcher/launcher.bin");
    let modified = fs::metadata(&output).unwrap().modified().unwrap();
    let c1 = fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap();
    drop(host);
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("machine_submission::package_grant::tests::process_reattach_worker")
        .env("C4_PACKAGE_GRANT_FIXTURE_ROOT", root.path())
        .status()
        .unwrap();
    assert!(result.success());
    assert_eq!(
        service(root.path())
            .reattach()
            .unwrap()
            .inspect("process-key")
            .unwrap(),
        Some(first)
    );
    assert_eq!(fs::metadata(output).unwrap().modified().unwrap(), modified);
    assert_eq!(
        fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap(),
        c1
    );
}
