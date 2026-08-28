//! The exact `sha256:file-v1` / `sha256:tree-v1` vectors.
//!
//! Two independent checks per vector, because either alone is weak. The
//! **longhand** rebuilds the frame here from the published recipe, so a
//! transposed field or a dropped count fails; the **pinned hex** is a constant
//! computed outside this codebase entirely, so a longhand that drifts with the
//! implementation still fails. A test that only called the observer twice
//! would bless any self-consistent wrong answer.

use std::fs;

use sha2::{Digest, Sha256};

use super::observe::{ArtifactObserver, WitnessOutcome, WitnessRefusal};

const FILE_SEED: &[u8] = b"sha256:file-v1\0epoch=1\0";
const TREE_SEED: &[u8] = b"sha256:tree-v1\0epoch=1\0";

/// Vectors computed independently of this crate. They pin the wire, not the
/// code: changing the recipe must break them.
const FILE_ONE: &str = "sha256:9dc81049576a346fd6525807d8d4af0e07649e0d2c59653abeb391a85356f7b2";
const TREE_EMPTY_ROOT: &str =
    "sha256:a808e74827154a7804851eb34136c5c25e69c7fe978133af3cfbdbaf8146a4e0";
const TREE_ONE_EMPTY_CHILD: &str =
    "sha256:f060727f4cdd734ced84147d58b308f51ed6c6951992ba7b5fea9ebd5f308c09";
const TREE_ONE_FILE: &str =
    "sha256:ad9e8d64fc4d234bac856c0f9c469d0df5d8d8ead7ee25163e517b68ff62b1fd";

pub(super) fn frame(hash: &mut Sha256, label: &str, value: impl AsRef<[u8]>) {
    let value = value.as_ref();
    hash.update((label.len() as u64).to_be_bytes());
    hash.update(label.as_bytes());
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value);
}

fn inner(bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(bytes);
    hash.finalize().into()
}

fn finish(hash: Sha256) -> String {
    format!("sha256:{:x}", hash.finalize())
}

pub(super) struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    pub(super) fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    pub(super) fn root(&self) -> std::path::PathBuf {
        // The capability walk canonicalises nothing, and macOS hands out a
        // symlinked temp root, so the fixture resolves it once up front.
        self.dir.path().canonicalize().unwrap()
    }

    pub(super) fn root_text(&self) -> String {
        self.root().to_string_lossy().replace('\\', "/")
    }

    pub(super) fn at(&self, relative: &str) -> std::path::PathBuf {
        self.root().join(relative)
    }

    pub(super) fn observe(&self, relative: &str) -> WitnessOutcome {
        let root = self.root_text();
        ArtifactObserver::new(&root).observe("artifact", &format!("{root}/{relative}"))
    }

    pub(super) fn witness(
        &self,
        relative: &str,
    ) -> vibe_wire::generated::lifecycle_state::StateDigestWitness {
        match self.observe(relative) {
            WitnessOutcome::Measured(witness) => witness,
            WitnessOutcome::Refused(cause) => {
                panic!("expected a witness for `{relative}`, got {cause:?}")
            }
        }
    }

    pub(super) fn refusal(&self, relative: &str) -> WitnessRefusal {
        match self.observe(relative) {
            WitnessOutcome::Refused(cause) => cause,
            WitnessOutcome::Measured(witness) => {
                panic!("expected a refusal for `{relative}`, got {witness:?}")
            }
        }
    }
}

/// A regular file: size then the raw inner digest, and no counts at all — the
/// count pair belongs to the input-manifest form, and on an artifact form the
/// wire reads its presence as a defect.
#[test]
fn a_file_witness_matches_longhand_and_the_pinned_vector() {
    let fixture = Fixture::new();
    fs::write(fixture.at("a.txt"), b"one").unwrap();
    let witness = fixture.witness("a.txt");

    let mut longhand = Sha256::new();
    longhand.update(FILE_SEED);
    frame(&mut longhand, "size", "3");
    frame(&mut longhand, "content_sha256", inner(b"one"));

    assert_eq!(witness.algorithm, "sha256:file-v1");
    assert_eq!(witness.digest, finish(longhand));
    assert_eq!(witness.digest, FILE_ONE);
    assert_eq!(witness.files, None, "artifact forms carry no count pair");
    assert_eq!(witness.bytes, None);
}

/// An empty directory artifact is a real witness, not a refusal: three zero
/// counts over an empty stream. Confusing it with absence would make a
/// deliberately empty output indistinguishable from a missing one.
#[test]
fn an_empty_tree_witness_matches_longhand_and_the_pinned_vector() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.at("dist")).unwrap();
    let witness = fixture.witness("dist");

    let mut longhand = Sha256::new();
    longhand.update(TREE_SEED);
    frame(&mut longhand, "directory_count", "0");
    frame(&mut longhand, "file_count", "0");
    frame(&mut longhand, "total_bytes", "0");

    assert_eq!(witness.algorithm, "sha256:tree-v1");
    assert_eq!(witness.digest, finish(longhand));
    assert_eq!(witness.digest, TREE_EMPTY_ROOT);
    assert_eq!(witness.files, None);
    assert_eq!(witness.bytes, None);
}

/// The empty-descendant vector, and the reason it exists: `{}` and `{empty/}`
/// must not be the same tree. A recipe that framed only files would make
/// deleting every empty directory in a declared output invisible.
#[test]
fn an_empty_descendant_is_an_entry_and_matches_the_pinned_vector() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.at("dist/empty")).unwrap();
    let witness = fixture.witness("dist");

    let mut longhand = Sha256::new();
    longhand.update(TREE_SEED);
    frame(&mut longhand, "entry_kind", "directory");
    frame(&mut longhand, "path", "empty");
    frame(&mut longhand, "directory_count", "1");
    frame(&mut longhand, "file_count", "0");
    frame(&mut longhand, "total_bytes", "0");

    assert_eq!(witness.digest, finish(longhand));
    assert_eq!(witness.digest, TREE_ONE_EMPTY_CHILD);
    assert_ne!(
        witness.digest, TREE_EMPTY_ROOT,
        "`{{}}` and `{{empty/}}` are different trees",
    );

    fs::remove_dir(fixture.at("dist/empty")).unwrap();
    assert_eq!(
        fixture.witness("dist").digest,
        TREE_EMPTY_ROOT,
        "and deleting only that empty directory moves the digest",
    );
}

#[test]
fn a_one_file_tree_frames_kind_path_size_and_inner_digest() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.at("dist")).unwrap();
    fs::write(fixture.at("dist/a.txt"), b"one").unwrap();
    let witness = fixture.witness("dist");

    let mut longhand = Sha256::new();
    longhand.update(TREE_SEED);
    frame(&mut longhand, "entry_kind", "file");
    frame(&mut longhand, "path", "a.txt");
    frame(&mut longhand, "size", "3");
    frame(&mut longhand, "content_sha256", inner(b"one"));
    frame(&mut longhand, "directory_count", "0");
    frame(&mut longhand, "file_count", "1");
    frame(&mut longhand, "total_bytes", "3");

    assert_eq!(witness.digest, finish(longhand));
    assert_eq!(witness.digest, TREE_ONE_FILE);
}

/// Order is the digest's, not the filesystem's: the same tree built in the
/// opposite creation order must witness identically, and byte-wise name order
/// is what makes that true.
#[test]
fn the_walk_order_is_deterministic_regardless_of_creation_order() {
    let forward = Fixture::new();
    fs::create_dir(forward.at("dist")).unwrap();
    for name in ["a.txt", "b.txt", "c.txt"] {
        fs::write(forward.at(&format!("dist/{name}")), name.as_bytes()).unwrap();
    }

    let reverse = Fixture::new();
    fs::create_dir(reverse.at("dist")).unwrap();
    for name in ["c.txt", "b.txt", "a.txt"] {
        fs::write(reverse.at(&format!("dist/{name}")), name.as_bytes()).unwrap();
    }

    assert_eq!(
        forward.witness("dist").digest,
        reverse.witness("dist").digest
    );
}

/// Preorder with a directory entry framed before its subtree: a nested tree
/// and a flat one holding the same leaf names are different trees.
#[test]
fn nesting_changes_the_tree_because_paths_are_root_relative() {
    let nested = Fixture::new();
    fs::create_dir_all(nested.at("dist/sub")).unwrap();
    fs::write(nested.at("dist/sub/a.txt"), b"one").unwrap();

    let flat = Fixture::new();
    fs::create_dir(flat.at("dist")).unwrap();
    fs::write(flat.at("dist/a.txt"), b"one").unwrap();

    assert_ne!(nested.witness("dist").digest, flat.witness("dist").digest);
}

/// The same bytes under two different artifact roots witness identically: a
/// tree digest is a claim about content, and the row carries the location.
#[test]
fn identical_content_at_two_roots_witnesses_identically() {
    let fixture = Fixture::new();
    for root in ["one", "two"] {
        fs::create_dir(fixture.at(root)).unwrap();
        fs::write(fixture.at(&format!("{root}/a.txt")), b"one").unwrap();
    }
    assert_eq!(fixture.witness("one").digest, fixture.witness("two").digest);
    assert_eq!(fixture.witness("one").digest, TREE_ONE_FILE);
}

/// A path that becomes the other kind of object moves both the algorithm and
/// the digest, which is what lets a comparison call it changed rather than
/// unwitnessable.
#[test]
fn a_file_replaced_by_a_directory_changes_algorithm_and_digest() {
    let fixture = Fixture::new();
    fs::write(fixture.at("output"), b"one").unwrap();
    let before = fixture.witness("output");
    assert_eq!(before.algorithm, "sha256:file-v1");

    fs::remove_file(fixture.at("output")).unwrap();
    fs::create_dir(fixture.at("output")).unwrap();
    let after = fixture.witness("output");
    assert_eq!(after.algorithm, "sha256:tree-v1");
    assert_ne!(before.digest, after.digest);
}

/// There is no input-side exclusion inside a declared artifact: a `target/`
/// or an empty hook directory is part of the bytes the producer claimed.
#[test]
fn no_input_side_exclusion_applies_inside_a_declared_artifact() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.at("dist")).unwrap();
    fs::write(fixture.at("dist/a.txt"), b"one").unwrap();
    let bare = fixture.witness("dist").digest;

    fs::create_dir(fixture.at("dist/target")).unwrap();
    let with_target = fixture.witness("dist").digest;
    assert_ne!(
        bare, with_target,
        "`target/` is not skipped inside an artifact"
    );

    fs::create_dir(fixture.at("dist/node_modules")).unwrap();
    assert_ne!(
        with_target,
        fixture.witness("dist").digest,
        "and neither is `node_modules/`",
    );
}

/// The declared semantic kind never reaches the physical decision: the OS
/// object picks the algorithm, and an unknown kind is carried, not judged.
#[test]
fn an_unknown_declared_kind_never_gates_physical_hashing() {
    let fixture = Fixture::new();
    fs::write(fixture.at("app.wheel"), b"one").unwrap();
    let root = fixture.root_text();
    let observer = ArtifactObserver::new(&root);
    let WitnessOutcome::Measured(witness) =
        observer.observe("wheel-row", &format!("{root}/app.wheel"))
    else {
        panic!("an unknown kind is not a reason to refuse a regular file");
    };
    assert_eq!(witness.algorithm, "sha256:file-v1");
    assert_eq!(witness.digest, FILE_ONE);
}
