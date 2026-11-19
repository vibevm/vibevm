use super::*;

#[test]
fn a_stop_after_owned_directory_creation_reports_recovery_required() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let outer = scratch();
    let root = outer.path().join("user-project");
    let calls = Arc::new(AtomicUsize::new(0));
    let held = Arc::clone(&calls);
    let watched = root.clone();
    let cancellation = token().with_poll(Arc::new(move || {
        held.fetch_add(1, Ordering::SeqCst);
        watched.join("vibevm").exists()
    }));
    let failure = init(&root, &cancellation).unwrap_err();
    assert_eq!(failure.report().outcome, Outcome::Incomplete);
    assert_eq!(failure.report().publication, Publication::RecoveryRequired);
    assert_eq!(failure.report().effects, Effects::Changed);
    assert_eq!(cancellation.publication_state(), "recovery-required");
    assert!(root.join("vibevm").is_dir());
    assert!(!root.join(MANIFEST_NAME).exists());
    assert!(calls.load(Ordering::SeqCst) > 1);
}

#[test]
fn simultaneous_initializers_never_overwrite_or_remove_data() {
    use std::sync::{Arc, Barrier};
    let outer = scratch();
    let root = outer.path().join("user-project");
    let barrier = Arc::new(Barrier::new(4));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let root = root.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                init(&root, &token())
            })
        })
        .collect();
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert!(outcomes.iter().any(Result::is_ok));
    assert_eq!(
        fs::read_to_string(root.join(MANIFEST_NAME)).unwrap(),
        initial_manifest()
    );
    assert!(root.join("vibevm/vibespecs").is_dir());
    assert!(matches!(status(&root), Observation::Initialized { .. }));
}

#[test]
fn initialized_root_does_not_require_listing_all_authored_files() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join(MANIFEST_NAME), initial_manifest()).unwrap();
    for index in 0..100 {
        fs::write(root.join(format!("file-{index}")), b"keep").unwrap();
    }
    assert!(matches!(status(&root), Observation::Initialized { .. }));
    assert!(matches!(
        init(&root, &token()).unwrap(),
        Initialized::AlreadyInitialized(_)
    ));
    assert_eq!(names_of(&root), 101);
}

#[test]
fn failed_manifest_publication_keeps_partial_state_without_claiming_commit() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    vibe_safefs::fail_before_publish(Some(MANIFEST_NAME));
    let cancellation = token();
    let result = init(&root, &cancellation);
    vibe_safefs::fail_before_publish(None);
    let failure = result.unwrap_err();
    assert_eq!(failure.report().publication, Publication::RecoveryRequired);
    assert!(!failure.report().is_success());
    assert!(!root.join(MANIFEST_NAME).exists());
    assert!(root.join("vibevm/vibespecs").is_dir());
}

#[test]
fn failure_after_link_does_not_claim_a_clean_publication() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    vibe_safefs::fail_before_stage_cleanup(Some(MANIFEST_NAME));
    let cancellation = token();
    let result = init(&root, &cancellation);
    vibe_safefs::fail_before_stage_cleanup(None);
    let failure = result.unwrap_err();
    assert_eq!(failure.report().effects, Effects::Unknown);
    assert_eq!(failure.report().publication, Publication::RecoveryRequired);
    assert_eq!(cancellation.publication_state(), "recovery-required");
    assert!(root.join(MANIFEST_NAME).exists());
    assert!(matches!(status(&root), Observation::Unavailable(_)));
}

#[test]
fn root_creation_reopen_failure_is_reported_without_removing_the_entry() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    vibe_safefs::arm_after_create_dir(Some(Box::new(|_, _| {
        Some(std::io::Error::other("injected reopen failure"))
    })));
    let result = init(&root, &token());
    vibe_safefs::arm_after_create_dir(None);
    let failure = result.unwrap_err();
    assert_eq!(failure.report().publication, Publication::RecoveryRequired);
    assert_eq!(failure.report().effects, Effects::Unknown);
    assert!(root.is_dir());
    assert!(!root.join(MANIFEST_NAME).exists());
}

#[test]
fn invalid_later_component_refuses_before_creating_earlier_ancestors() {
    let outer = scratch();
    let root = outer
        .path()
        .join("missing/.vibe-stage-reserved/user-project");
    let failure = init(&root, &token()).unwrap_err();
    assert_eq!(failure.report().effects, Effects::None);
    assert_eq!(failure.report().publication, Publication::NotStarted);
    assert!(!outer.path().join("missing").exists());
}

#[test]
fn foreign_entry_created_at_checkpoint_blocks_adoption() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    let watched = root.clone();
    let count = Arc::new(AtomicUsize::new(0));
    let cancellation = token().with_poll(Arc::new(move || {
        if count.fetch_add(1, Ordering::SeqCst) == 1 {
            fs::write(watched.join("foreign.txt"), b"keep").unwrap();
        }
        false
    }));
    let failure = init(&root, &cancellation).unwrap_err();
    assert_eq!(failure.report().effects, Effects::None);
    assert_eq!(failure.report().publication, Publication::NotStarted);
    assert_eq!(fs::read(root.join("foreign.txt")).unwrap(), b"keep");
    assert!(!root.join(MANIFEST_NAME).exists());
}

#[test]
fn authored_layout_replacement_before_publication_refuses() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let outer = scratch();
    let root = outer.path().join("user-project");
    let watched = root.clone();
    let fired = Arc::new(AtomicBool::new(false));
    let cancellation = token().with_poll(Arc::new(move || {
        if watched.join("vibevm/vibespecs").exists() && !fired.swap(true, Ordering::SeqCst) {
            fs::rename(watched.join("vibevm"), watched.join("displaced")).unwrap();
            fs::create_dir_all(watched.join("vibevm/vibespecs")).unwrap();
        }
        false
    }));
    let failure = init(&root, &cancellation).unwrap_err();
    assert_eq!(failure.report().publication, Publication::RecoveryRequired);
    assert_eq!(failure.report().effects, Effects::Unknown);
    assert!(!root.join(MANIFEST_NAME).exists());
    assert!(root.join("displaced/vibespecs").is_dir());
}

#[test]
fn relocation_at_link_boundary_is_committed_but_not_successful() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    let moved = outer.path().join("moved-project");
    let from = root.clone();
    let to = moved.clone();
    vibe_safefs::arm_before_link(Some(Box::new(move |_, _| {
        fs::rename(&from, &to).unwrap();
        fs::create_dir(&from).unwrap();
        fs::write(from.join("foreign"), b"keep").unwrap();
    })));
    let cancellation = token();
    let failure = init(&root, &cancellation).unwrap_err();
    vibe_safefs::arm_before_link(None);
    assert_eq!(failure.report().publication, Publication::Committed);
    assert_eq!(failure.report().effects, Effects::Unknown);
    assert!(!failure.report().is_success());
    assert_eq!(cancellation.publication_state(), "committed");
    assert!(moved.join(MANIFEST_NAME).exists());
    assert!(!root.join(MANIFEST_NAME).exists());
    assert_eq!(fs::read(root.join("foreign")).unwrap(), b"keep");
}

#[test]
fn not_created_error_preserves_no_owned_effect_evidence() {
    let outer = scratch();
    let project = Project::open(outer.path()).unwrap();
    let parent = project.root_dir().unwrap();
    fs::write(outer.path().join("occupied"), b"foreign").unwrap();
    let cancellation = token();
    let mut owned = Vec::new();
    let mut uncertain = false;
    assert!(
        create_owned(
            &parent,
            "occupied",
            &["occupied".to_owned()],
            &cancellation,
            &mut owned,
            &mut uncertain
        )
        .is_err()
    );
    assert!(owned.is_empty());
    assert!(!uncertain);
    assert_eq!(fs::read(outer.path().join("occupied")).unwrap(), b"foreign");
}

#[cfg(unix)]
#[test]
fn first_mkdir_not_created_reports_no_owned_effects() {
    use std::os::unix::fs::PermissionsExt;
    let outer = scratch();
    let root = outer.path().join("user-project");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o500)).unwrap();
    let cancellation = token();
    let result = init(&root, &cancellation);
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let failure = result.unwrap_err();
    assert_eq!(failure.report().effects, Effects::None);
    assert_eq!(failure.report().publication, Publication::NotStarted);
    assert_eq!(failure.report().outcome, Outcome::Refused);
    assert_eq!(names_of(&root), 0);
    assert_eq!(cancellation.publication_state(), "restored");
}

#[test]
fn a_stop_requested_at_publication_verification_retains_commit() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    let cancellation = token();
    let observed = cancellation.clone();
    vibe_safefs::arm_before_publish_verify(Some(Box::new(move |_, _| {
        observed.cancel();
    })));
    let result = init(&root, &cancellation);
    vibe_safefs::arm_before_publish_verify(None);
    assert!(result.is_ok());
    assert_eq!(cancellation.publication_state(), "committed");
    assert!(cancellation.checkpoint().is_err());
    assert_eq!(cancellation.publication_state(), "committed");
    assert!(root.join(MANIFEST_NAME).is_file());
}

#[cfg(unix)]
#[test]
fn authored_symlink_replacement_is_not_followed() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let outer = scratch();
    let root = outer.path().join("user-project");
    let foreign = outer.path().join("foreign");
    fs::create_dir(&foreign).unwrap();
    fs::write(foreign.join("sentinel"), b"keep").unwrap();
    let watched = root.clone();
    let target = foreign.clone();
    let fired = Arc::new(AtomicBool::new(false));
    let cancellation = token().with_poll(Arc::new(move || {
        if watched.join("vibevm/vibespecs").is_dir() && !fired.swap(true, Ordering::SeqCst) {
            fs::rename(watched.join("vibevm"), watched.join("displaced")).unwrap();
            std::os::unix::fs::symlink(&target, watched.join("vibevm")).unwrap();
        }
        false
    }));
    assert!(init(&root, &cancellation).is_err());
    assert!(!root.join(MANIFEST_NAME).exists());
    assert_eq!(fs::read(foreign.join("sentinel")).unwrap(), b"keep");
    assert_eq!(names_of(&foreign), 1);
}

#[test]
fn moved_root_with_acknowledged_stop_still_reports_committed() {
    let outer = scratch();
    let root = outer.path().join("user-project");
    let moved = outer.path().join("moved");
    let from = root.clone();
    let to = moved.clone();
    let cancellation = token();
    let request = cancellation.clone();
    vibe_safefs::arm_before_link(Some(Box::new(move |_, _| {
        fs::rename(&from, &to).unwrap();
        request.cancel();
    })));
    let failure = init(&root, &cancellation).unwrap_err();
    vibe_safefs::arm_before_link(None);
    assert_eq!(failure.report().publication, Publication::Committed);
    assert!(cancellation.checkpoint().is_err());
    assert_eq!(cancellation.publication_state(), "committed");
    assert!(moved.join(MANIFEST_NAME).is_file());
}
