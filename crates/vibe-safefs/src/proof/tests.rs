//! The swap window, closed. Every red here plants a different object at a
//! name between the inspection and the removal and proves the removal refuses.

use std::fs;

use crate::{Project, ProofRefusal};

fn project() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    (dir, project)
}

#[test]
fn a_proved_file_is_inspected_and_then_removed() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    fs::write(dir.path().join("archive/spent.json"), "payload").unwrap();

    let (proof, len) = project
        .inspect_file_in(&archive, "spent.json")
        .unwrap()
        .expect("an ordinary owned file has a proof");
    assert_eq!(len, 7);
    project
        .remove_file_proved_in(&archive, "spent.json", &proof)
        .expect("the proved object is still there");
    assert!(!dir.path().join("archive/spent.json").exists());

    // The same proof does not license a second removal.
    let again = project
        .remove_file_proved_in(&archive, "spent.json", &proof)
        .expect_err("a spent proof licenses nothing");
    assert!(again.changed(), "{again}");
}

#[test]
fn absence_and_unowned_entries_yield_no_proof() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    assert!(
        project
            .inspect_file_in(&archive, "absent.json")
            .unwrap()
            .is_none()
    );
    fs::create_dir(dir.path().join("archive/a-directory")).unwrap();
    assert!(project.inspect_file_in(&archive, "a-directory").is_err());

    fs::write(dir.path().join("archive/one"), "shared").unwrap();
    if fs::hard_link(
        dir.path().join("archive/one"),
        dir.path().join("archive/two"),
    )
    .is_ok()
    {
        assert!(
            project.inspect_file_in(&archive, "one").is_err(),
            "a file with two names is not exclusively owned",
        );
    }
}

/// **The file swap.** The candidate is inspected and judged deletable; the
/// hook then rebinds that exact name to a different ordinary file. The
/// removal must refuse, and the replacement must survive byte-for-byte.
#[test]
fn a_file_swapped_after_inspection_is_never_removed() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    let path = dir.path().join("archive/0000.json");
    fs::write(&path, "the inspected payload").unwrap();

    let (proof, _) = project
        .inspect_file_in(&archive, "0000.json")
        .unwrap()
        .unwrap();

    let planted = dir.path().to_path_buf();
    crate::arm_before_proved_removal(Some(Box::new(move |_, name| {
        // Somebody else's file, at exactly the name we judged.
        let at = planted.join("archive").join(name);
        fs::remove_file(&at).unwrap();
        fs::write(&at, "SOMEBODY ELSE'S FILE").unwrap();
    })));
    let refusal = project.remove_file_proved_in(&archive, "0000.json", &proof);
    crate::arm_before_proved_removal(None);

    let refusal = refusal.expect_err("a swapped name is not the proved object");
    assert!(refusal.changed(), "{refusal}");
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "SOMEBODY ELSE'S FILE",
        "the replacement survives byte-for-byte",
    );
}

/// The same window, but the name is rebound to a DIRECTORY: the removal must
/// not fall back to any by-name unlink, and the directory survives.
#[test]
fn a_file_replaced_by_a_directory_is_never_removed() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    let path = dir.path().join("archive/0000.json");
    fs::write(&path, "payload").unwrap();
    let (proof, _) = project
        .inspect_file_in(&archive, "0000.json")
        .unwrap()
        .unwrap();

    let planted = dir.path().to_path_buf();
    crate::arm_before_proved_removal(Some(Box::new(move |_, name| {
        let at = planted.join("archive").join(name);
        fs::remove_file(&at).unwrap();
        fs::create_dir(&at).unwrap();
        fs::write(at.join("someone-elses.txt"), "keep").unwrap();
    })));
    let refusal = project.remove_file_proved_in(&archive, "0000.json", &proof);
    crate::arm_before_proved_removal(None);

    assert!(
        refusal
            .expect_err("a directory is not the proved file")
            .changed()
    );
    assert!(path.is_dir());
    assert_eq!(
        fs::read_to_string(path.join("someone-elses.txt")).unwrap(),
        "keep"
    );
}

#[test]
fn a_proved_directory_is_removed_only_while_empty_and_unchanged() {
    let (dir, project) = project();
    let trace = project.dir(&["trace"], true).unwrap();
    fs::create_dir(dir.path().join("trace/run")).unwrap();
    let run = trace.open_child("run").unwrap();
    let proof = run.proof().unwrap();
    // The caller's own capability must be released; the proof is what travels.
    drop(run);

    // Non-empty refuses, and says so distinctly from a swap.
    fs::write(dir.path().join("trace/run/leftover"), "x").unwrap();
    let refusal = project
        .remove_dir_proved_in(&trace, "run", &proof)
        .expect_err("a non-empty directory stays");
    assert!(
        matches!(refusal, ProofRefusal::NotEmpty { .. }),
        "{refusal}"
    );
    assert!(!refusal.changed());
    assert!(dir.path().join("trace/run").is_dir());

    fs::remove_file(dir.path().join("trace/run/leftover")).unwrap();
    project
        .remove_dir_proved_in(&trace, "run", &proof)
        .expect("the proved, empty directory goes");
    assert!(!dir.path().join("trace/run").exists());
}

/// **The directory swap.** The run directory is proved deletable, then the
/// hook replaces that name with a different, non-empty directory. The removal
/// must refuse on identity — not merely on emptiness — and the replacement
/// must survive in place.
#[test]
fn a_directory_swapped_after_inspection_is_never_removed() {
    let (dir, project) = project();
    let trace = project.dir(&["trace"], true).unwrap();
    fs::create_dir(dir.path().join("trace/run")).unwrap();
    let run = trace.open_child("run").unwrap();
    let proof = run.proof().unwrap();
    drop(run);

    let planted = dir.path().to_path_buf();
    crate::arm_before_proved_removal(Some(Box::new(move |_, name| {
        let at = planted.join("trace").join(name);
        fs::remove_dir(&at).unwrap();
        fs::create_dir(&at).unwrap();
        fs::write(at.join("someone-elses.txt"), "keep").unwrap();
    })));
    let refusal = project.remove_dir_proved_in(&trace, "run", &proof);
    crate::arm_before_proved_removal(None);

    let refusal = refusal.expect_err("a swapped directory is not the proved one");
    assert!(
        refusal.changed(),
        "identity, not emptiness, is what refused: {refusal}"
    );
    assert!(dir.path().join("trace/run").is_dir());
    assert_eq!(
        fs::read_to_string(dir.path().join("trace/run/someone-elses.txt")).unwrap(),
        "keep",
        "the replacement is untouched",
    );
}

/// A directory whose name is rebound to an EMPTY replacement is the sharpest
/// case: emptiness alone would have licensed the removal, so only the
/// identity proof can stop it.
#[test]
fn an_empty_replacement_directory_is_still_refused_on_identity() {
    let (dir, project) = project();
    let trace = project.dir(&["trace"], true).unwrap();
    fs::create_dir(dir.path().join("trace/run")).unwrap();
    let run = trace.open_child("run").unwrap();
    let proof = run.proof().unwrap();
    drop(run);

    let planted = dir.path().to_path_buf();
    crate::arm_before_proved_removal(Some(Box::new(move |_, name| {
        let at = planted.join("trace").join(name);
        fs::remove_dir(&at).unwrap();
        fs::create_dir(&at).unwrap();
    })));
    let refusal = project.remove_dir_proved_in(&trace, "run", &proof);
    crate::arm_before_proved_removal(None);

    assert!(
        refusal
            .expect_err("a new directory is a new object")
            .changed()
    );
    assert!(dir.path().join("trace/run").is_dir());
}

/// Two distinct files never share a proof, and a proof is not transferable
/// between names.
#[test]
fn a_proof_names_one_object_and_does_not_transfer() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    fs::write(dir.path().join("archive/a"), "same bytes").unwrap();
    fs::write(dir.path().join("archive/b"), "same bytes").unwrap();

    let (a, _) = project.inspect_file_in(&archive, "a").unwrap().unwrap();
    let (b, _) = project.inspect_file_in(&archive, "b").unwrap().unwrap();
    assert_ne!(a, b, "equal bytes are not one object");

    let refusal = project
        .remove_file_proved_in(&archive, "b", &a)
        .expect_err("a's proof does not license removing b");
    assert!(refusal.changed(), "{refusal}");
    assert!(dir.path().join("archive/b").exists());
}

/// The hook is single-shot and always disarmed, so it cannot leak into the
/// next removal on this thread.
#[test]
fn the_removal_hook_fires_once_and_leaves_nothing_armed() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    fs::write(dir.path().join("archive/a"), "a").unwrap();
    fs::write(dir.path().join("archive/b"), "b").unwrap();
    let (a, _) = project.inspect_file_in(&archive, "a").unwrap().unwrap();
    let (b, _) = project.inspect_file_in(&archive, "b").unwrap().unwrap();

    let planted = dir.path().to_path_buf();
    crate::arm_before_proved_removal(Some(Box::new(move |_, name| {
        let at = planted.join("archive").join(name);
        fs::remove_file(&at).unwrap();
        fs::write(&at, "swapped").unwrap();
    })));
    assert!(project.remove_file_proved_in(&archive, "a", &a).is_err());
    // Not re-armed: `b` is removed normally.
    project
        .remove_file_proved_in(&archive, "b", &b)
        .expect("the hook fired once");
    assert!(!dir.path().join("archive/b").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("archive/a")).unwrap(),
        "swapped"
    );
}
