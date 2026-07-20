use super::*;
use crate::commands::vvm::model::{Kind, Origin, Profile, VersionId};
use specmark::verifies;

/// Build an external record sourced from `path`.
fn ext(kind: Kind, id: &str, instance: u64, path: &str) -> InstallRecord {
    InstallRecord {
        kind,
        id: id.into(),
        instance,
        commit: "c".into(),
        toolchain: "t".into(),
        profile: Profile::Debug,
        installed_at: "now".into(),
        origin: Origin::External,
        source_path: Some(path.into()),
    }
}

/// A managed record (no source_path).
fn managed(kind: Kind, id: &str, instance: u64) -> InstallRecord {
    InstallRecord {
        kind,
        id: id.into(),
        instance,
        commit: "c".into(),
        toolchain: "t".into(),
        profile: Profile::Debug,
        installed_at: "now".into(),
        origin: Origin::Managed,
        source_path: None,
    }
}

fn state_of(installs: Vec<InstallRecord>) -> model::State {
    model::State {
        next_instance: installs.len() as u64 + 1,
        installs,
    }
}

#[test]
#[verifies("spec://vibevm/common/PROP-019#relocate", r = 1)]
fn infer_old_source_picks_the_most_common_external_path() {
    let state = state_of(vec![
        ext(Kind::Branch, "main", 1, "C:/old/vibevm"),
        ext(Kind::Branch, "main", 2, "C:/old/vibevm"),
        ext(Kind::Branch, "main", 3, "C:/other/vibevm"),
        managed(Kind::Tag, "1.0.0", 1),
    ]);
    assert_eq!(
        infer_old_source(&state),
        Some(PathBuf::from("C:/old/vibevm"))
    );

    // No external records → nothing to infer.
    let managed_only = state_of(vec![managed(Kind::Branch, "main", 1)]);
    assert_eq!(infer_old_source(&managed_only), None);
}

/// The oracle property (scaffold D): the plan partitions the inventory so
/// that every external record whose source matches `old` is either
/// repointed (active) or deleted (not), with nothing missed and nothing
/// double-counted.
#[test]
#[verifies("spec://vibevm/common/PROP-019#relocate", r = 1)]
fn plan_partitions_old_matching_records() {
    let old = "C:/old/vibevm";
    let installs = vec![
        ext(Kind::Branch, "main", 1, old), // non-active old → delete
        ext(Kind::Branch, "main", 2, old), // non-active old → delete
        ext(Kind::Branch, "main", 3, old), // ACTIVE old → repoint
        ext(Kind::Tag, "1.0.0", 1, old),   // non-active old → delete
        ext(Kind::Branch, "main", 4, "C:/new/vibevm"), // different path → untouched
        managed(Kind::Branch, "main", 5),  // managed → untouched
    ];
    let state = state_of(installs.clone());
    let active = &installs[2];
    let plan = plan_relocate(&state, Path::new(old), "C:/new/vibevm", Some(active));

    // The active old instance is repointed, never deleted.
    assert_eq!(
        plan.repoint,
        vec![(VersionId::new(Kind::Branch, "main"), 3)]
    );
    // The other old instances are deleted.
    assert_eq!(
        plan.delete,
        vec![
            (VersionId::new(Kind::Branch, "main"), 1),
            (VersionId::new(Kind::Branch, "main"), 2),
            (VersionId::new(Kind::Tag, "1.0.0"), 1),
        ]
    );
    assert_eq!(plan.untouched, 2);

    // Oracle: the union of repoint + delete is exactly the old-matching
    // external records, with no overlap.
    let mut covered: Vec<InstanceRef> = plan.repoint.clone();
    covered.extend(plan.delete.iter().cloned());
    let expected: Vec<InstanceRef> = installs
        .iter()
        .filter(|r| r.origin == Origin::External && r.source_path.as_deref() == Some(old))
        .map(|r| (r.version_id(), r.instance))
        .collect();
    let mut covered_sorted = covered.clone();
    covered_sorted.sort_by(|a, b| a.1.cmp(&b.1));
    let mut expected_sorted = expected.clone();
    expected_sorted.sort_by(|a, b| a.1.cmp(&b.1));
    assert_eq!(
        covered_sorted, expected_sorted,
        "every old record is covered"
    );
    // disjoint: no instance appears in both repoint and delete.
    for r in &plan.repoint {
        assert!(!plan.delete.contains(r), "repoint/delete overlap on {r:?}");
    }
}

/// The active instance is protected: even when it is the only old-sourced
/// instance, it is repointed, not deleted (PROP-019 §2.17).
#[test]
#[verifies("spec://vibevm/common/PROP-019#relocate", r = 1)]
fn plan_keeps_the_active_instance() {
    let old = "C:/old/vibevm";
    let installs = vec![ext(Kind::Branch, "main", 9, old)];
    let state = state_of(installs.clone());
    let plan = plan_relocate(&state, Path::new(old), "C:/new/vibevm", Some(&installs[0]));
    assert_eq!(plan.repoint.len(), 1);
    assert!(plan.delete.is_empty());
}

/// apply_relocate removes the stale instance dirs, forgets their records,
/// and repoints the active record's source_path — in one state write
/// (scaffold H: a temp-dir store, no real-machine mutation).
#[test]
#[verifies("spec://vibevm/common/PROP-019#relocate", r = 1)]
fn apply_removes_dirs_repoints_active_and_forgets_deleted() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    let old_id = VersionId::new(Kind::Branch, "main");
    // Three instances of branch:main, all from the old tree; #3 is active.
    for n in [1u64, 2, 3] {
        store
            .record_install(ext(Kind::Branch, "main", n, "C:/old/vibevm"))
            .unwrap();
        fs::create_dir_all(store.instance_dir(&old_id, n)).unwrap();
    }
    store
        .write_current(&store.instance_dir(&old_id, 3))
        .unwrap();

    let active = store.active().unwrap().unwrap();
    let state = store.load_state().unwrap();
    let plan = plan_relocate(
        &state,
        Path::new("C:/old/vibevm"),
        "C:/new/vibevm",
        Some(&active),
    );
    apply_relocate(&quiet(), &store, &plan).unwrap();

    // Non-active old instance dirs are gone; the active one is kept.
    assert!(!store.instance_dir(&old_id, 1).exists());
    assert!(!store.instance_dir(&old_id, 2).exists());
    assert!(store.instance_dir(&old_id, 3).exists());

    // The inventory keeps only the active instance, now repointed.
    let remaining: Vec<InstallRecord> = store.load_state().unwrap().installs;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].instance, 3);
    assert_eq!(remaining[0].source_path.as_deref(), Some("C:/new/vibevm"));
}

/// An empty plan touches nothing (no record sourced from the old location).
#[test]
#[verifies("spec://vibevm/common/PROP-019#relocate", r = 1)]
fn apply_is_a_noop_on_an_empty_plan() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    let id = VersionId::new(Kind::Branch, "main");
    store
        .record_install(ext(Kind::Branch, "main", 1, "C:/elsewhere/vibevm"))
        .unwrap();
    fs::create_dir_all(store.instance_dir(&id, 1)).unwrap();

    let plan = RelocatePlan {
        old: PathBuf::from("C:/old/vibevm"),
        new: "C:/new/vibevm".into(),
        repoint: vec![],
        delete: vec![],
        untouched: 1,
    };
    apply_relocate(&quiet(), &store, &plan).unwrap();

    assert!(store.instance_dir(&id, 1).exists());
    assert_eq!(store.load_state().unwrap().installs.len(), 1);
}

/// rewrite_state forgets a removed instance but KEEPS a locked one's record
/// (so its dir is not orphaned — a later run, `self gc`, or `self remove`
/// finishes the job), repoints the active instance, and leaves the rest
/// alone (PROP-019 §2.17). Pure: no filesystem involved.
#[test]
#[verifies("spec://vibevm/common/PROP-019#relocate", r = 1)]
fn rewrite_state_keeps_locked_forgets_removed_and_repoints_active() {
    let old = "C:/old/vibevm";
    let active = ext(Kind::Branch, "main", 3, old);
    let mut state = state_of(vec![
        ext(Kind::Branch, "main", 1, old), // deleted, removed
        ext(Kind::Branch, "main", 2, old), // deleted, LOCKED (kept)
        active.clone(),                    // ACTIVE → repoint
        managed(Kind::Branch, "main", 4),  // untouched
    ]);
    let plan = RelocatePlan {
        old: PathBuf::from(old),
        new: "C:/new/vibevm".into(),
        repoint: vec![(VersionId::new(Kind::Branch, "main"), 3)],
        delete: vec![
            (VersionId::new(Kind::Branch, "main"), 1),
            (VersionId::new(Kind::Branch, "main"), 2),
        ],
        untouched: 1,
    };
    // instance #2 was locked → kept.
    let kept = vec![(VersionId::new(Kind::Branch, "main"), 2)];
    let counts = rewrite_state(&mut state, &plan, &kept);

    let by_inst: HashMap<u64, &InstallRecord> =
        state.installs.iter().map(|r| (r.instance, r)).collect();
    assert!(!by_inst.contains_key(&1), "removed instance forgotten");
    assert!(by_inst.contains_key(&2), "locked instance record kept");
    assert_eq!(
        by_inst[&3].source_path.as_deref(),
        Some("C:/new/vibevm"),
        "active repointed"
    );
    assert_eq!(
        by_inst[&4].origin,
        Origin::Managed,
        "untouched record preserved"
    );
    // 2 deletions planned, 1 kept → 1 removed, 1 skipped.
    assert_eq!(
        counts,
        ApplyCounts {
            removed: 1,
            skipped: 1
        }
    );
}

/// A target that is not a vibevm checkout is refused before anything mutates.
#[test]
#[verifies("spec://vibevm/common/PROP-019#relocate", r = 1)]
fn run_rejects_a_non_source_tree_target() {
    let tmp = tempfile::tempdir().unwrap();
    let env = VvmEnv {
        root: Some(tmp.path().to_path_buf()),
        cwd: None,
        home: None,
        shell: None,
        path_var: None,
    };
    let args = VvmRelocateArgs {
        target: tmp.path().display().to_string(),
        from: None,
        dry_run: false,
        yes: true,
    };
    let err = run_relocate_cmd(&quiet(), &env, args).unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("not a vibevm source tree"), "got: {msg}");
    assert!(msg.contains("spec://vibevm/common/PROP-019#relocate"));
}

fn quiet() -> output::Context {
    output::Context::from_flags(true, false, None, true)
}
