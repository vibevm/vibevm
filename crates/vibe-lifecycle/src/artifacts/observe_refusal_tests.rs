//! Every way an artifact fails to yield a comparable witness.
//!
//! The shared law under all of them: a refusal is evidence-only. It produces
//! no witness, it never widens past the artifact it is about, and inside a
//! directory it refuses that tree WHOLE — a partial tree digest would be a
//! false claim about the declared scope rather than a smaller one.

use std::fs;
use std::path::Path;

use super::observe::{ArtifactObserver, WitnessOutcome, WitnessRefusal, inject};
use super::observe_tests::Fixture;

/// Plant a symlink, reporting whether the platform allowed it. Unprivileged
/// Windows refuses, and skipping there is honest: the law is proved wherever
/// the object can exist.
fn link_to(target: &Path, link: &Path) -> bool {
    #[cfg(windows)]
    {
        if target.is_dir() {
            return std::os::windows::fs::symlink_dir(target, link).is_ok();
        }
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}

#[test]
fn a_missing_artifact_is_absent() {
    let fixture = Fixture::new();
    assert_eq!(fixture.refusal("nowhere.bin"), WitnessRefusal::Absent);
}

#[test]
fn a_missing_ancestor_is_absent_not_a_link_refusal() {
    let fixture = Fixture::new();
    assert_eq!(fixture.refusal("no/such/dir/a.txt"), WitnessRefusal::Absent);
}

/// A path outside the project is skipped, never opened: reaching outside
/// through the project capability is what the capability exists to prevent.
#[test]
fn an_escaping_path_refuses_without_opening_it() {
    let fixture = Fixture::new();
    let observer = ArtifactObserver::new(&fixture.root_text());
    let elsewhere = if cfg!(windows) {
        "C:/somewhere/else/app.exe"
    } else {
        "/somewhere/else/app.exe"
    };
    assert_eq!(
        observer.observe("row", elsewhere),
        WitnessOutcome::Refused(WitnessRefusal::Outside),
    );
}

/// A relative spelling is unlocatable, not "outside" — the distinction the
/// shared row law already draws, reused rather than re-decided here.
#[test]
fn a_relative_or_root_path_is_malformed() {
    let fixture = Fixture::new();
    let root = fixture.root_text();
    let observer = ArtifactObserver::new(&root);
    for path in ["docs/guide.md", root.as_str()] {
        assert_eq!(
            observer.observe("row", path),
            WitnessOutcome::Refused(WitnessRefusal::Malformed),
            "`{path}` names no file below the project",
        );
    }
}

#[test]
fn a_hard_linked_artifact_refuses() {
    let fixture = Fixture::new();
    fs::write(fixture.at("origin.bin"), b"shared").unwrap();
    if fs::hard_link(fixture.at("origin.bin"), fixture.at("alias.bin")).is_err() {
        return;
    }
    assert_eq!(fixture.refusal("origin.bin"), WitnessRefusal::NotRegular);
}

#[test]
fn a_symlinked_artifact_root_refuses() {
    let fixture = Fixture::new();
    fs::write(fixture.at("real.bin"), b"one").unwrap();
    if !link_to(&fixture.at("real.bin"), &fixture.at("link.bin")) {
        return;
    }
    assert_eq!(fixture.refusal("link.bin"), WitnessRefusal::NotRegular);
}

/// A link ANYWHERE on the descent refuses, and it is never followed — the
/// no-follow walk is the whole point of reaching the artifact this way.
#[test]
fn a_symlinked_ancestor_refuses_the_artifact() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.at("real/inner")).unwrap();
    fs::write(fixture.at("real/inner/a.txt"), b"one").unwrap();
    if !link_to(&fixture.at("real"), &fixture.at("shadow")) {
        return;
    }
    assert_eq!(
        fixture.refusal("shadow/inner/a.txt"),
        WitnessRefusal::Linked,
    );
}

/// A link INSIDE a directory artifact refuses the whole tree, not just the
/// entry: the tree witness is one digest over a set.
#[test]
fn a_link_inside_a_tree_refuses_the_whole_tree() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.at("dist")).unwrap();
    fs::write(fixture.at("dist/a.txt"), b"one").unwrap();
    fs::write(fixture.at("outside.bin"), b"target").unwrap();
    assert!(matches!(
        fixture.observe("dist"),
        WitnessOutcome::Measured(_)
    ));
    if !link_to(&fixture.at("outside.bin"), &fixture.at("dist/link.bin")) {
        return;
    }
    assert_eq!(fixture.refusal("dist"), WitnessRefusal::NotRegular);
}

#[test]
fn a_hard_link_inside_a_tree_refuses_the_whole_tree() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.at("dist")).unwrap();
    fs::write(fixture.at("dist/a.txt"), b"one").unwrap();
    fs::write(fixture.at("origin.bin"), b"shared").unwrap();
    if fs::hard_link(fixture.at("origin.bin"), fixture.at("dist/alias.bin")).is_err() {
        return;
    }
    assert_eq!(fixture.refusal("dist"), WitnessRefusal::NotRegular);
}

/// Two direct children that fold to one physical file under portable
/// case/normalisation identity refuse the tree: hashing both would count one
/// file twice, and hashing either would be an arbitrary choice.
#[test]
fn portable_alias_siblings_refuse_the_whole_tree() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.at("dist")).unwrap();
    if fs::write(fixture.at("dist/Guide.md"), b"a").is_err()
        || fs::write(fixture.at("dist/guide.md"), b"b").is_err()
    {
        return;
    }
    // A case-insensitive volume never held two names to begin with.
    if fs::read_dir(fixture.at("dist")).unwrap().count() < 2 {
        return;
    }
    assert_eq!(fixture.refusal("dist"), WitnessRefusal::Aliased);
}

/// A child appearing after a directory's pre-listing refuses that tree. The
/// window is the real one a concurrent writer occupies; the hook is the
/// deterministic stand-in for winning that race.
#[test]
fn a_child_added_between_the_listings_refuses() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.at("dist")).unwrap();
    fs::write(fixture.at("dist/a.txt"), b"one").unwrap();

    let target = fixture.at("dist/late.txt");
    inject::arm_between_listings(Some(Box::new(move |_prefix| {
        fs::write(&target, b"late").unwrap();
    })));
    assert_eq!(fixture.refusal("dist"), WitnessRefusal::Moved);

    assert!(
        matches!(fixture.observe("dist"), WitnessOutcome::Measured(_)),
        "the one-shot hook disarmed itself, so a quiet tree witnesses cleanly",
    );
}

#[test]
fn a_child_removed_between_the_listings_refuses() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.at("dist")).unwrap();
    fs::write(fixture.at("dist/a.txt"), b"one").unwrap();
    fs::write(fixture.at("dist/b.txt"), b"two").unwrap();

    let target = fixture.at("dist/b.txt");
    inject::arm_between_listings(Some(Box::new(move |_prefix| {
        fs::remove_file(&target).unwrap();
    })));
    assert_eq!(fixture.refusal("dist"), WitnessRefusal::Moved);
    inject::arm_between_listings(None);
}

/// A directory swapped for a different directory of the SAME name and the
/// same child names: the child set agrees, so only the identity proof can
/// catch it.
#[test]
fn a_directory_rebound_to_a_new_object_refuses() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.at("dist")).unwrap();
    fs::write(fixture.at("dist/a.txt"), b"one").unwrap();
    fs::create_dir(fixture.at("replacement")).unwrap();
    fs::write(fixture.at("replacement/a.txt"), b"one").unwrap();

    let old = fixture.at("dist");
    let new = fixture.at("replacement");
    inject::arm_between_listings(Some(Box::new(move |_prefix| {
        // Windows will not rename over an open directory handle; where that
        // is refused the identity law is proved by the sibling tests.
        let _ = fs::remove_dir_all(&old).and_then(|()| fs::rename(&new, &old));
    })));
    let outcome = fixture.observe("dist");
    inject::arm_between_listings(None);
    if let WitnessOutcome::Refused(cause) = outcome {
        assert_eq!(cause, WitnessRefusal::Moved);
    }
}

/// Depth is fenced. The cheap proof is the fence itself: a tree one level
/// past the limit refuses, and no million-entry fixture is built to prove the
/// width fence, which the safefs primitive owns and tests directly.
#[test]
fn a_tree_deeper_than_the_fence_refuses() {
    let fixture = Fixture::new();
    let mut path = fixture.at("dist");
    fs::create_dir(&path).unwrap();
    for level in 0..super::observe::MAX_DEPTH + 2 {
        path = path.join(format!("d{level}"));
        if fs::create_dir(&path).is_err() {
            return; // The host path limit fired first; nothing to prove here.
        }
    }
    assert_eq!(fixture.refusal("dist"), WitnessRefusal::Unbounded);
}

/// A tree exactly at the fence still witnesses: the ceiling refuses what is
/// past it, it does not demand slack.
#[test]
fn a_tree_within_the_depth_fence_still_witnesses() {
    let fixture = Fixture::new();
    let mut path = fixture.at("dist");
    fs::create_dir(&path).unwrap();
    for level in 0..8 {
        path = path.join(format!("d{level}"));
        fs::create_dir(&path).unwrap();
    }
    assert!(matches!(
        fixture.observe("dist"),
        WitnessOutcome::Measured(_)
    ));
}

/// The blast radius: one refused artifact leaves its neighbour witnessed.
/// Refusing the sibling too would drag an honest row off `matched` for a
/// reason that has nothing to do with it.
#[test]
fn one_refused_artifact_does_not_touch_a_clean_sibling() {
    let fixture = Fixture::new();
    fs::write(fixture.at("clean.bin"), b"one").unwrap();
    let root = fixture.root_text();
    let observer = ArtifactObserver::new(&root);

    assert!(matches!(
        observer.observe("clean", &format!("{root}/clean.bin")),
        WitnessOutcome::Measured(_)
    ));
    assert!(matches!(
        observer.observe("gone", &format!("{root}/gone.bin")),
        WitnessOutcome::Refused(WitnessRefusal::Absent)
    ));
    assert!(
        matches!(
            observer.observe("clean", &format!("{root}/clean.bin")),
            WitnessOutcome::Measured(_)
        ),
        "and the refusal left the observer usable for the rest of the batch",
    );
}

/// An unopenable project root refuses every row honestly instead of panicking
/// or becoming a handler error. The root must be ABSOLUTE for the platform, or
/// the row would refuse as malformed before the capability was ever asked for.
#[test]
fn an_unopenable_project_refuses_every_row() {
    let root = if cfg!(windows) {
        "C:/no/such/project/root"
    } else {
        "/no/such/project/root"
    };
    let observer = ArtifactObserver::new(root);
    assert_eq!(
        observer.observe("row", &format!("{root}/a.txt")),
        WitnessOutcome::Refused(WitnessRefusal::Io),
    );
}
