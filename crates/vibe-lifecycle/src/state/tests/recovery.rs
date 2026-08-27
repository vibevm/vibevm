//! The post-publication recovery window: what a store does when a
//! publication crossed the rename boundary and then failed.
//!
//! The rename is the only boundary a staged atomic replace has, so a
//! `PossiblyPublished` failure means the disk may already hold the candidate
//! bytes — and the store may not guess. These cases drive the real safefs
//! injection where one exists (a genuine publication that fails after its
//! rename), and the crate's own deterministic seams for the outcomes no
//! single process can produce honestly: a rename whose effect was raced
//! away, a third writer's bytes, an unsafe shape. The provably-invisible
//! `BeforePublication` class lives in `tests/publication.rs`.

use std::fs;
use std::path::Path;

use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, LifecycleState};

use super::support::{KEY, OTHER, prior_store, targets, third_state_toml, vibe_names};
use super::{RUN_ID, record_for};
use crate::state::inject;
use crate::{LifecycleStateError, LifecycleStateStore, PostPublicationRecovery};

/// A `PossiblyPublished` failure whose re-read proves the exact CANDIDATE
/// bytes durable: memory adopts the candidate, the diagnostic still fails
/// the verb, and the store remains usable because memory and disk agree.
#[test]
fn a_post_publication_fault_with_candidate_visible_adopts_the_candidate() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _prior_bytes) = prior_store(dir.path());

    vibe_safefs::fail_after_publish(Some(LifecycleStateStore::FILE));
    let error = store
        .checkpoint(
            OTHER.into(),
            record_for(OTHER, RUN_ID, ExecutionRecordStatus::Ok, "sha256:next"),
        )
        .expect_err("an injected post-publication failure is still a failure");
    vibe_safefs::fail_after_publish(None);

    let LifecycleStateError::PostPublication {
        stage: vibe_safefs::PublishStage::PossiblyPublished,
        recovery,
        ..
    } = &error
    else {
        panic!("the diagnostic is the typed post-publication one: {error}");
    };
    assert_eq!(*recovery, PostPublicationRecovery::CandidateAdopted);
    let rendered = error.to_string();
    assert!(
        rendered.contains("injected post-publication failure"),
        "the original failure is preserved verbatim: {rendered}",
    );
    assert!(
        rendered.contains("adopted in memory"),
        "the recovery outcome is part of the diagnostic: {rendered}",
    );
    assert!(
        rendered.contains("after the rename was attempted"),
        "the original stage is part of the diagnostic: {rendered}",
    );

    // Memory holds the candidate, and the disk holds exactly the candidate's
    // bytes: the two agree, so the store is healthy, not poisoned.
    assert!(
        store.prior(OTHER).is_some(),
        "the candidate became the in-memory state",
    );
    let durable = fs::read(store.path()).unwrap();
    assert_eq!(
        durable,
        toml::to_string_pretty(store.state()).unwrap().into_bytes(),
        "the durable bytes are the candidate's exact bytes",
    );
    assert!(store.poison_reason().is_none());
    store
        .checkpoint(
            "org.demo/tools#third".into(),
            record_for(
                "org.demo/tools#third",
                RUN_ID,
                ExecutionRecordStatus::Ok,
                "sha256:x",
            ),
        )
        .expect("an adopted-candidate store still mutates");
}

/// A `PossiblyPublished` failure whose re-read proves the exact PRIOR bytes
/// durable — the rename's effect was raced away. The prior state stays
/// current in memory, the prior bytes stay on disk, and the verb still fails.
#[test]
fn a_post_publication_fault_with_prior_visible_retains_the_prior() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, prior_bytes) = prior_store(dir.path());
    let before = store.state().clone();

    inject::fail_state_publication_possibly(Some("injected raced-away rename"));
    let error = store
        .checkpoint(
            OTHER.into(),
            record_for(OTHER, RUN_ID, ExecutionRecordStatus::Ok, "sha256:next"),
        )
        .expect_err("an unconfirmed publication is still a failure");
    inject::fail_state_publication_possibly(None);

    let LifecycleStateError::PostPublication {
        stage: vibe_safefs::PublishStage::PossiblyPublished,
        recovery,
        ..
    } = &error
    else {
        panic!("the diagnostic is the typed post-publication one: {error}");
    };
    assert_eq!(*recovery, PostPublicationRecovery::PriorRetained);
    assert!(error.to_string().contains("injected raced-away rename"));

    assert_eq!(*store.state(), before, "the prior state stayed current");
    assert!(
        store.prior(OTHER).is_none(),
        "the candidate was not adopted"
    );
    assert_eq!(
        fs::read(store.path()).unwrap(),
        prior_bytes,
        "the exact prior bytes — formatting included — stayed durable",
    );
    assert!(store.poison_reason().is_none());
    store
        .checkpoint(
            OTHER.into(),
            record_for(OTHER, RUN_ID, ExecutionRecordStatus::Ok, "sha256:next"),
        )
        .expect("a prior-retaining store still mutates");
}

/// The same window against a FRESH begin whose prior is absence: the re-read
/// sees no file, which is the exact prior condition, so the prior absence is
/// retained — begin fails, no store escapes, and nothing was written.
#[test]
fn a_possibly_published_initial_begin_over_absence_retains_the_absence() {
    let dir = tempfile::tempdir().unwrap();
    inject::fail_state_publication_possibly(Some("injected raced-away initial rename"));
    let error = LifecycleStateStore::begin(
        dir.path(),
        "create".into(),
        vec!["create".into()],
        "2026-08-28T00:00:00Z".into(),
        RUN_ID.into(),
        false,
    )
    .expect_err("an unconfirmed initial publication fails the begin");
    inject::fail_state_publication_possibly(None);

    let LifecycleStateError::PostPublication {
        stage: vibe_safefs::PublishStage::PossiblyPublished,
        recovery,
        ..
    } = &error
    else {
        panic!("the diagnostic is the typed post-publication one: {error}");
    };
    assert_eq!(*recovery, PostPublicationRecovery::PriorRetained);
    assert!(
        !dir.path().join(LifecycleStateStore::FILE).exists(),
        "the retained prior is absence, and absence is what stayed durable",
    );
}

/// A `PossiblyPublished` failure whose re-read finds a THIRD state — valid
/// TOML some other writer published — poisons the store: memory keeps the
/// last proven prior, disk keeps the third bytes, and no write is attempted
/// against them, ever again, from this store.
#[test]
fn a_third_durable_state_poisons_the_store_and_freezes_every_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, prior_bytes) = prior_store(dir.path());
    // The third writer's bytes — a valid state this store neither wrote nor
    // promised. Formatted unlike either side, so byte equality cannot save it.
    let third = third_state_toml().as_bytes().to_vec();
    assert_ne!(third, prior_bytes);
    fs::write(store.path(), &third).unwrap();

    inject::fail_state_publication_possibly(Some("injected third-state rename"));
    let error = store
        .checkpoint(
            OTHER.into(),
            record_for(OTHER, RUN_ID, ExecutionRecordStatus::Ok, "sha256:next"),
        )
        .expect_err("an unconfirmed publication is still a failure");
    inject::fail_state_publication_possibly(None);

    let LifecycleStateError::PostPublication {
        stage: vibe_safefs::PublishStage::PossiblyPublished,
        recovery,
        ..
    } = &error
    else {
        panic!("the diagnostic is the typed post-publication one: {error}");
    };
    let PostPublicationRecovery::Poisoned { reason } = recovery else {
        panic!("a third state poisons: {recovery}");
    };
    assert!(reason.contains("third state"), "{reason}");
    assert!(error.to_string().contains("injected third-state rename"));
    assert!(
        store.poison_reason().is_some(),
        "the store itself is poisoned, not just this one verb",
    );

    // The last proven state stays inspectable — exactly as the last PROVEN
    // state, never as current disk truth.
    assert!(
        store.prior(KEY).is_some(),
        "inspection still exposes the last proven state",
    );
    assert!(store.prior(OTHER).is_none());

    // Every mutating verb refuses before another write, and the third bytes
    // survive every refusal untouched.
    let record = record_for(OTHER, RUN_ID, ExecutionRecordStatus::Ok, "sha256:x");
    for label in [
        "checkpoint",
        "forget",
        "retain",
        "clear-continuation",
        "continuation",
    ] {
        let refused = match label {
            "checkpoint" => store.checkpoint(OTHER.into(), record.clone()),
            "forget" => store.forget(KEY),
            "retain" => store.retain_prefixed("org.demo/", &Default::default()),
            "clear-continuation" => store.clear_slot_continuation(),
            _ => store.record_slot_continuation(targets()),
        }
        .expect_err(label);
        assert!(
            matches!(refused, LifecycleStateError::Poisoned { .. }),
            "{label}: {refused}",
        );
    }
    assert_eq!(
        fs::read(store.path()).unwrap(),
        third,
        "a poisoned store never writes, so the third bytes survive",
    );
    assert_eq!(
        vibe_names(dir.path()),
        vec!["lifecycle.toml".to_string()],
        "no staging residue: refusing means refusing to stage, too",
    );
}

/// The unsafe outcomes of the same window: a state file the re-read cannot
/// prove safe (a second hard link) or a file that vanished outright. Each
/// poisons exactly like a third state — none is "healed" by a retrying write.
#[test]
fn unsafe_and_vanished_recovery_reads_poison_without_healing() {
    {
        // A second hard link: the bytes are the prior's own, but the file is
        // no longer exclusively owned, which is a third state by shape.
        let dir = tempfile::tempdir().unwrap();
        let (mut store, _prior) = prior_store(dir.path());
        fs::hard_link(store.path(), dir.path().join("second.toml")).unwrap();
        inject::fail_state_publication_possibly(Some("injected hardlinked window"));
        let error = store
            .checkpoint(
                OTHER.into(),
                record_for(OTHER, RUN_ID, ExecutionRecordStatus::Ok, "sha256:x"),
            )
            .unwrap_err();
        inject::fail_state_publication_possibly(None);
        let LifecycleStateError::PostPublication {
            stage: vibe_safefs::PublishStage::PossiblyPublished,
            recovery,
            ..
        } = &error
        else {
            panic!("{error}");
        };
        assert!(
            matches!(recovery, PostPublicationRecovery::Poisoned { .. }),
            "an unsafe shape poisons even when its bytes are the prior's: {error}",
        );
        assert!(store.poison_reason().is_some());
    }
    {
        // Vanished: the store proved prior bytes durable once, and the re-read
        // finds no file at all. That is not the prior absence — it is a disk
        // the store can no longer describe.
        let dir = tempfile::tempdir().unwrap();
        let (mut store, _prior) = prior_store(dir.path());
        fs::remove_file(store.path()).unwrap();
        inject::fail_state_publication_possibly(Some("injected vanished window"));
        let error = store
            .checkpoint(
                OTHER.into(),
                record_for(OTHER, RUN_ID, ExecutionRecordStatus::Ok, "sha256:x"),
            )
            .unwrap_err();
        inject::fail_state_publication_possibly(None);
        let LifecycleStateError::PostPublication {
            stage: vibe_safefs::PublishStage::PossiblyPublished,
            recovery,
            ..
        } = &error
        else {
            panic!("{error}");
        };
        let PostPublicationRecovery::Poisoned { reason } = recovery else {
            panic!("{recovery}");
        };
        assert!(reason.contains("vanished"), "{reason}");
        assert!(
            !store.path().is_file(),
            "a poisoned store does not heal a vanished state by rewriting it",
        );
    }
}

/// A symlink at the state name in the recovery window: the no-follow
/// re-read refuses it regardless of what it points at, and that refusal
/// poisons exactly like a third state. Privilege-gated on Windows (Win32
/// 1314 without Developer Mode), so the test carries an explicit
/// `#[ignore]` there and ASSERTS the symlink creation wherever it runs — a
/// skip reads as a skip, never as a pass.
#[test]
#[cfg_attr(
    windows,
    ignore = "requires Windows symlink/reparse privilege (worker host returned Win32 1314)"
)]
fn a_symlinked_state_name_in_the_recovery_window_poisons() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _prior) = prior_store(dir.path());
    let target = dir.path().join("elsewhere.toml");
    fs::write(&target, third_state_toml()).unwrap();
    let linked = dir.path().join("linked.toml");
    assert!(
        symlink(&target, &linked),
        "where this test runs, the symlink oracle must actually create the link",
    );
    fs::remove_file(store.path()).unwrap();
    fs::rename(&linked, store.path()).unwrap();
    inject::fail_state_publication_possibly(Some("injected symlinked window"));
    let error = store
        .checkpoint(
            OTHER.into(),
            record_for(OTHER, RUN_ID, ExecutionRecordStatus::Ok, "sha256:x"),
        )
        .unwrap_err();
    inject::fail_state_publication_possibly(None);
    let LifecycleStateError::PostPublication {
        stage: vibe_safefs::PublishStage::PossiblyPublished,
        recovery,
        ..
    } = &error
    else {
        panic!("{error}");
    };
    assert!(
        matches!(recovery, PostPublicationRecovery::Poisoned { .. }),
        "a link at the state name is never read through: {error}",
    );
}

#[cfg(unix)]
fn symlink(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn symlink(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}

/// A third state over a PRIOR ABSENCE renders the prior as absence, never as
/// a zero-length prior file: "0 prior bytes" asserts a file that never
/// existed, in the one diagnostic an operator uses to decide what to keep.
/// The plant seam stands in for the concurrent writer that puts the third
/// state on disk inside the publication window of a fresh `begin` — the one
/// sequence in which `durable` is `None` and the re-read still finds bytes.
#[test]
fn a_third_state_over_a_prior_absence_says_absence_not_zero_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let state_path = dir.path().join(LifecycleStateStore::FILE);
    assert!(
        !state_path.exists(),
        "the fixture starts from a genuine prior absence",
    );
    let plant_dir = dir.path().to_path_buf();
    inject::fail_state_publication_possibly_planting(
        "injected third state over absence",
        Box::new(move || {
            fs::create_dir_all(plant_dir.join(".vibe")).unwrap();
            fs::write(plant_dir.join(".vibe/lifecycle.toml"), third_state_toml()).unwrap();
        }),
    );
    let error = LifecycleStateStore::begin(
        dir.path(),
        "create".into(),
        vec!["create".into()],
        "2026-08-28T00:00:00Z".into(),
        RUN_ID.into(),
        false,
    )
    .expect_err("an unconfirmed initial publication fails the begin");

    let LifecycleStateError::PostPublication {
        stage: vibe_safefs::PublishStage::PossiblyPublished,
        recovery: PostPublicationRecovery::Poisoned { reason },
        ..
    } = &error
    else {
        panic!("a third state over absence poisons: {error}");
    };
    assert!(
        !reason.contains("0 prior bytes"),
        "absence must render as absence, never as a zero-length prior file: {reason}",
    );
    assert!(
        reason.contains("absent"),
        "the diagnostic says the prior was absent: {reason}",
    );
    assert!(
        reason.contains("third state"),
        "and still names what the durable bytes are: {reason}",
    );
}

/// The store's durable-bytes ledger is built from the file's ORIGINAL bytes,
/// not a reserialization of the parsed prior. The proof has to catch the
/// ledger BEFORE anything rewrites the file: an oddly formatted but valid
/// prior is read by `begin`, the initial publication then fails
/// possibly-published, and the one re-read must recognise the AUTHOR'd bytes
/// as the prior — `PriorRetained`, not a third state — leaving them on disk
/// byte-exact. Had the ledger held a reserialization, the re-read would have
/// seen bytes that match neither candidate nor disk and poisoned instead.
#[test]
fn an_oddly_formatted_prior_survives_a_possibly_published_initial_begin_byte_exact() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(LifecycleStateStore::FILE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    // A valid state whose formatting `toml::to_string_pretty` would never
    // produce: fields reordered, odd spacing, no pretty-table blank lines.
    let authored = "schema = 1\n[run]\nchain = []\nstarted = '2026-08-01T00:00:00Z'\n     \
                    requested = 'create'\n[execution]\n";
    let authored = authored.as_bytes();
    fs::write(&path, authored).unwrap();
    // The pretty encoding of the same parsed state is NOT these bytes — that
    // difference is what makes a reserialized ledger detectable here.
    let parsed: LifecycleState = toml::from_str(&String::from_utf8_lossy(authored)).unwrap();
    assert_ne!(
        toml::to_string_pretty(&parsed).unwrap().into_bytes(),
        authored,
        "the fixture's formatting must differ from the pretty encoding",
    );

    inject::fail_state_publication_possibly(Some("injected raced-away initial rename"));
    let error = LifecycleStateStore::begin(
        dir.path(),
        "create".into(),
        vec!["create".into()],
        "2026-08-28T00:00:00Z".into(),
        RUN_ID.into(),
        false,
    )
    .expect_err("an unconfirmed initial publication fails the begin");
    inject::fail_state_publication_possibly(None);

    let LifecycleStateError::PostPublication {
        stage: vibe_safefs::PublishStage::PossiblyPublished,
        recovery: PostPublicationRecovery::PriorRetained,
        ..
    } = &error
    else {
        panic!("the authored bytes must be recognised as the prior: {error}");
    };
    assert_eq!(
        fs::read(&path).unwrap(),
        authored,
        "the exact authored bytes — original formatting included — stayed durable",
    );
    // And the recognition is provably byte-based: a store that DID adopt a
    // rewrite would have left the pretty encoding here instead.
}
