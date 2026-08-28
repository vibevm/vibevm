//! The one-walk declared-input REDs (R7.5 P2/A4a, extended by A4b).
//!
//! The refactor replaced HEAD's one-walk-PER-PATTERN input collection with a
//! single walk serving two projections: the legacy pattern-major fingerprint
//! replay and the deduplicated `sha256:vibe-input-manifest-v1` witness. These
//! cases pin BOTH halves — byte-compatibility against a test-only verbatim
//! copy of the old algorithm, the one-walk/zero-fallback observers, the
//! overlap/duplicate semantics, the `None` vs `Some([])` boundary, the E1/E4
//! mutation axes, order determinism, the frozen manifest framing against an
//! independent longhand recompute plus pinned vectors, and the checked
//! overflow helpers.

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use vibe_core::manifest::ExtensionKey;
use vibe_wire::generated::lifecycle::e1::context::SlotTarget;

use super::support::{context, row};
use crate::HandlerExecution;
use crate::state::fingerprint::inputs::{checked_file_count, checked_total_bytes, observe};
use crate::state::fingerprint::legacy;
use crate::state::{
    fingerprint_execution, fingerprint_handler_execution_with, prepare_handler_execution_with,
};

/// A minimal project the fixture contexts expect, with no selected input yet.
fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname='demo'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::write(dir.path().join("vibe.lock"), "lock").unwrap();
    dir
}

fn prepared(
    root: &Path,
    message: &str,
    inputs: Option<Vec<String>>,
) -> crate::state::PreparedFingerprint {
    let subject = row(root, message, "0.1.0", inputs);
    let ctx = context(root, subject.effective_config().unwrap());
    prepare_handler_execution_with(&HandlerExecution::from_row(&subject), &ctx, None).unwrap()
}

/// The measured half of a prepared manifest — every case in this file
/// measures a clean tree, so a refusal here is a bug, not an outcome.
fn measured(
    manifest: &crate::state::PreparedInputManifest,
) -> &vibe_wire::generated::lifecycle_state::StateDigestWitness {
    manifest.measured().unwrap()
}

/// The one-walk production fingerprint must be byte-identical to HEAD's
/// per-pattern walk across every declared-input shape, including the
/// historical repeat when patterns overlap or duplicate.
#[test]
fn one_walk_fingerprints_match_the_legacy_per_pattern_reference_exactly() {
    let shapes: [&[&str]; 5] = [
        &["*.txt"],          // one pattern
        &["**", "*.txt"],    // overlapping
        &["*.txt", "*.txt"], // duplicate
        &["*.none"],         // no match
        &[],                 // authored empty list
    ];
    for patterns in shapes {
        let dir = project();
        fs::write(dir.path().join("a.txt"), "one").unwrap();
        fs::create_dir_all(dir.path().join("deep/nest/ed")).unwrap();
        fs::write(dir.path().join("deep/nest/ed/file.txt"), "nested").unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        for excluded in [".git", ".vibe", "target", "node_modules"] {
            fs::create_dir_all(dir.path().join(excluded)).unwrap();
            fs::write(dir.path().join(excluded).join("ignored.txt"), "ignored").unwrap();
        }
        let owned: Vec<String> = patterns.iter().map(|pattern| pattern.to_string()).collect();
        let subject = row(dir.path(), "one", "0.1.0", Some(owned.clone()));
        let ctx = context(dir.path(), subject.effective_config().unwrap());
        assert_eq!(
            fingerprint_execution(&subject, &ctx).unwrap(),
            legacy::execution_fingerprint_with(&subject, &ctx, None).unwrap(),
            "one-walk fingerprint must be byte-identical to HEAD for {owned:?}",
        );
    }

    // The absent list: no declared inputs at all, still byte-identical.
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let absent = row(dir.path(), "one", "0.1.0", None);
    let ctx = context(dir.path(), absent.effective_config().unwrap());
    assert_eq!(
        fingerprint_execution(&absent, &ctx).unwrap(),
        legacy::execution_fingerprint_with(&absent, &ctx, None).unwrap(),
    );
}

/// The observers prove the physical law the two-read detection law leaves
/// intact: ONE `WalkDir` construction for a multi-pattern call, ZERO raw
/// fallback reads on a clean tree (the certified observation feeds both
/// projections), and none at all for an absent or authored-empty
/// declaration. The retired A4a "one raw read per union file" claim is
/// deliberately NOT re-asserted: a clean accepted file now costs two
/// bounded capability reads by normative design.
#[test]
fn one_walk_and_zero_raw_fallbacks_on_a_clean_tree() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    fs::write(dir.path().join("b.log"), "log").unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    let subject = row(
        dir.path(),
        "one",
        "0.1.0",
        Some(vec!["**".into(), "*.txt".into(), "src/**".into()]),
    );
    let ctx = context(dir.path(), subject.effective_config().unwrap());

    observe::reset();
    fingerprint_execution(&subject, &ctx).unwrap();
    assert_eq!(observe::walks(), 1, "three patterns must share ONE walk");
    assert_eq!(
        observe::raw_fallbacks(),
        0,
        "a clean stable file never needs its raw fallback"
    );

    observe::reset();
    let absent = row(dir.path(), "absent", "0.1.0", None);
    let absent_ctx = context(dir.path(), absent.effective_config().unwrap());
    fingerprint_execution(&absent, &absent_ctx).unwrap();
    assert_eq!(observe::walks(), 0);
    assert_eq!(observe::raw_fallbacks(), 0);

    observe::reset();
    let empty = row(dir.path(), "empty", "0.1.0", Some(vec![]));
    let empty_ctx = context(dir.path(), empty.effective_config().unwrap());
    fingerprint_execution(&empty, &empty_ctx).unwrap();
    assert_eq!(observe::walks(), 0, "an authored empty list walks nothing");
    assert_eq!(observe::raw_fallbacks(), 0);
}

/// Overlap and duplication repeat bytes in the legacy fingerprint stream
/// (pinned by the reference test above) but the manifest counts the union
/// once; a duplicated pattern still changes the manifest's pattern list.
#[test]
fn overlap_repeats_legacy_bytes_but_the_manifest_counts_the_union_once() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let single = prepared(dir.path(), "one", Some(vec!["*.txt".into()]));
    let overlap = prepared(dir.path(), "one", Some(vec!["*.txt".into(), "a.*".into()]));
    let duplicate = prepared(
        dir.path(),
        "one",
        Some(vec!["*.txt".into(), "*.txt".into()]),
    );

    let single_manifest = single.input_manifest.as_ref().unwrap();
    let overlap_manifest = overlap.input_manifest.as_ref().unwrap();
    let duplicate_manifest = duplicate.input_manifest.as_ref().unwrap();

    assert_eq!(measured(single_manifest).files, Some(1));
    assert_eq!(measured(single_manifest).bytes, Some("3".to_string()));
    assert_eq!(
        measured(overlap_manifest).files,
        Some(1),
        "two patterns selecting one file witness ONE file",
    );
    assert_eq!(
        measured(overlap_manifest).bytes,
        Some("3".to_string()),
        "overlapping patterns count the bytes once, not twice",
    );
    assert_eq!(measured(duplicate_manifest).files, Some(1));
    assert_eq!(measured(duplicate_manifest).bytes, Some("3".to_string()));
    assert_ne!(
        measured(overlap_manifest).digest,
        measured(single_manifest).digest,
        "the pattern list is part of the manifest's identity, so an added \
         pattern moves the digest even when the file union is unchanged",
    );
    assert_ne!(
        measured(duplicate_manifest).digest,
        measured(single_manifest).digest,
        "a duplicated pattern is retained in the pattern list",
    );
    assert_eq!(
        duplicate_manifest.patterns,
        vec!["*.txt".to_string(), "*.txt".to_string()],
    );
}

/// `None` is `unavailable` — no measurement at all. `Some([])` is a complete
/// empty scope with a REAL digest over the header frames alone, pinned here
/// as a literal vector alongside the one-file content vector.
#[test]
fn absent_inputs_have_no_manifest_and_authored_empty_has_a_real_empty_one() {
    let dir = project();
    let absent = prepared(dir.path(), "absent", None);
    assert!(absent.input_manifest.is_none());

    let empty = prepared(dir.path(), "empty", Some(vec![]));
    let manifest = empty.input_manifest.as_ref().unwrap();
    assert!(manifest.patterns.is_empty());
    assert_eq!(
        measured(manifest).algorithm,
        "sha256:vibe-input-manifest-v1"
    );
    assert_eq!(measured(manifest).files, Some(0));
    assert_eq!(measured(manifest).bytes, Some("0".to_string()));
    // Pinned vector: seed + pattern_count "0" + file_count "0" + total "0".
    assert_eq!(
        measured(manifest).digest,
        "sha256:50124f0168f6b08e9c0ea72a080cbcdcb2295927a9acfa73381d4ffc20c4dda8",
    );

    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let content = prepared(dir.path(), "content", Some(vec!["*.txt".into()]));
    let manifest = content.input_manifest.as_ref().unwrap();
    assert_eq!(measured(manifest).files, Some(1));
    assert_eq!(measured(manifest).bytes, Some("3".to_string()));
    // Pinned vector: ["*.txt"] selecting a.txt = "one".
    assert_eq!(
        measured(manifest).digest,
        "sha256:b676f0a870baaf83ad39375642ef43fa0e328bba0d59712779e9570d3a284060",
    );
}

/// E1: a requested/chain/config/provider-only change moves only the execution
/// fingerprint; a selected input byte change moves both identities.
#[test]
fn command_chain_and_provider_move_only_the_fingerprint_while_input_bytes_move_both() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let before = prepared(dir.path(), "one", Some(vec!["*.txt".into()]));

    let subject = row(dir.path(), "one", "0.1.0", Some(vec!["*.txt".into()]));
    let mut moved_ctx = context(dir.path(), subject.effective_config().unwrap());
    moved_ctx.run.requested = "test".into();
    moved_ctx.run.chain.push("test".into());
    let moved =
        prepare_handler_execution_with(&HandlerExecution::from_row(&subject), &moved_ctx, None)
            .unwrap();
    assert_ne!(moved.fingerprint, before.fingerprint);
    assert_eq!(moved.input_manifest, before.input_manifest);

    let config_changed = prepared(dir.path(), "two", Some(vec!["*.txt".into()]));
    assert_ne!(config_changed.fingerprint, before.fingerprint);
    assert_eq!(config_changed.input_manifest, before.input_manifest);

    let provider_changed = row(dir.path(), "one", "0.2.0", Some(vec!["*.txt".into()]));
    let provider_ctx = context(dir.path(), provider_changed.effective_config().unwrap());
    let provider = prepare_handler_execution_with(
        &HandlerExecution::from_row(&provider_changed),
        &provider_ctx,
        None,
    )
    .unwrap();
    assert_ne!(provider.fingerprint, before.fingerprint);
    assert_eq!(provider.input_manifest, before.input_manifest);

    fs::write(dir.path().join("a.txt"), "two").unwrap();
    let after = prepared(dir.path(), "one", Some(vec!["*.txt".into()]));
    assert_ne!(after.fingerprint, before.fingerprint);
    assert_ne!(after.input_manifest, before.input_manifest);
}

/// E4: writes under the shippable exclusions move neither identity — the
/// fingerprint already ignored them and the manifest inherits that scope.
#[test]
fn excluded_directory_writes_move_neither_identity() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let before = prepared(dir.path(), "one", Some(vec!["**".into()]));
    for excluded in [".git", ".vibe", "target", "node_modules"] {
        fs::create_dir_all(dir.path().join(excluded)).unwrap();
        fs::write(dir.path().join(excluded).join("ignored.txt"), "ignored").unwrap();
    }
    let after = prepared(dir.path(), "one", Some(vec!["**".into()]));
    assert_eq!(after.fingerprint, before.fingerprint);
    assert_eq!(after.input_manifest, before.input_manifest);
}

/// The union and its order are derived from sorted relative paths, so
/// creation/enumeration permutations cannot move either projection.
#[test]
fn the_union_and_order_are_stable_under_creation_order_permutation() {
    let names = ["z.txt", "a.txt", "m/n.txt", "m/b.txt"];
    let first = project();
    for name in names {
        if let Some(parent) = Path::new(name).parent() {
            fs::create_dir_all(first.path().join(parent)).unwrap();
        }
        fs::write(first.path().join(name), format!("bytes of {name}")).unwrap();
    }
    let second = project();
    for name in names.iter().rev() {
        if let Some(parent) = Path::new(name).parent() {
            fs::create_dir_all(second.path().join(parent)).unwrap();
        }
        fs::write(second.path().join(name), format!("bytes of {name}")).unwrap();
    }
    let digest_of = |root: &Path| {
        prepared(root, "one", Some(vec!["**/*.txt".into()]))
            .input_manifest
            .as_ref()
            .map(measured)
            .unwrap()
            .digest
            .clone()
    };
    assert_eq!(digest_of(first.path()), digest_of(second.path()));

    // Same root, files recreated in a different order: byte-identical too.
    let subject = row(first.path(), "one", "0.1.0", Some(vec!["**/*.txt".into()]));
    let ctx = context(first.path(), subject.effective_config().unwrap());
    let before = fingerprint_execution(&subject, &ctx).unwrap();
    for name in names.iter().rev() {
        fs::remove_file(first.path().join(name)).unwrap();
    }
    for name in names {
        fs::write(first.path().join(name), format!("bytes of {name}")).unwrap();
    }
    assert_eq!(fingerprint_execution(&subject, &ctx).unwrap(), before);
}

/// The manifest framing is frozen: an independent longhand SHA-256 over the
/// recipe must reproduce the production digest for a non-trivial fixture.
#[test]
fn the_manifest_framing_matches_an_independent_longhand_recompute() {
    fn frame(hash: &mut Sha256, label: &str, value: &[u8]) {
        hash.update((label.len() as u64).to_be_bytes());
        hash.update(label.as_bytes());
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value);
    }

    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    fs::create_dir_all(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub/b.txt"), "two\n").unwrap();
    let manifest = prepared(
        dir.path(),
        "one",
        Some(vec!["*.txt".into(), "sub/*.txt".into()]),
    )
    .input_manifest
    .unwrap();

    let mut hash = Sha256::new();
    hash.update(b"sha256:vibe-input-manifest-v1\0epoch=1\0");
    frame(&mut hash, "pattern_count", b"2");
    frame(&mut hash, "pattern", b"*.txt");
    frame(&mut hash, "pattern", b"sub/*.txt");
    frame(&mut hash, "file_count", b"2");
    frame(&mut hash, "path", b"a.txt");
    frame(&mut hash, "size", b"3");
    frame(&mut hash, "bytes", b"one");
    frame(&mut hash, "path", b"sub/b.txt");
    frame(&mut hash, "size", b"4");
    frame(&mut hash, "bytes", b"two\n");
    frame(&mut hash, "total_bytes", b"7");
    assert_eq!(
        measured(&manifest).digest,
        format!("sha256:{:x}", hash.finalize()),
        "the production framing must equal the frozen recipe",
    );
    assert_eq!(measured(&manifest).files, Some(2));
    assert_eq!(measured(&manifest).bytes, Some("7".to_string()));
    assert_eq!(
        manifest.patterns,
        vec!["*.txt".to_string(), "sub/*.txt".to_string()],
    );
}

/// The checked narrowing helpers refuse overflow with the named error rather
/// than saturating — provable with plain numbers, no huge fixtures.
#[test]
fn file_count_and_total_bytes_overflow_refuse_rather_than_saturate() {
    let key = ExtensionKey::authored("__host__/demo#x");
    let error = checked_file_count(u32::MAX as usize + 1, &key).unwrap_err();
    assert!(matches!(
        error,
        crate::state::FingerprintError::InputManifestOverflow { .. }
    ));
    assert!(
        error
            .to_string()
            .contains("beyond the witness `files` bound")
    );

    let error = checked_total_bytes([u128::MAX, 1_u128], &key).unwrap_err();
    assert!(matches!(
        error,
        crate::state::FingerprintError::InputManifestOverflow { .. }
    ));
    assert!(error.to_string().contains("total byte count overflows"));

    assert_eq!(checked_file_count(0, &key).unwrap(), 0);
    assert_eq!(
        checked_file_count(u32::MAX as usize, &key).unwrap(),
        u32::MAX
    );
    assert_eq!(
        checked_total_bytes(std::iter::empty::<u128>(), &key).unwrap(),
        0
    );
    assert_eq!(
        checked_total_bytes([u64::MAX as u128, 1_u128], &key).unwrap(),
        u64::MAX as u128 + 1,
    );
}

/// Slot-target wrapping changes only the final execution fingerprint — the
/// manifest is a tree observation, never a per-slot identity — and the public
/// surface agrees with the prepared one.
#[test]
fn slot_target_wrapping_moves_only_the_fingerprint_never_the_manifest() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let subject = row(dir.path(), "one", "0.1.0", Some(vec!["*.txt".into()]));
    let ctx = context(dir.path(), subject.effective_config().unwrap());
    let plain =
        prepare_handler_execution_with(&HandlerExecution::from_row(&subject), &ctx, None).unwrap();

    let root = dir.path().to_string_lossy().replace('\\', "/");
    let targeted = HandlerExecution::from_row(&subject).with_slot_target(SlotTarget {
        group: "org.demo".into(),
        kind: "stack".into(),
        name: "rust-stack".into(),
        root: format!("{root}/slot"),
        version: "1.0.0".into(),
    });
    let wrapped = prepare_handler_execution_with(&targeted, &ctx, None).unwrap();
    assert_ne!(wrapped.fingerprint, plain.fingerprint);
    assert_eq!(wrapped.input_manifest, plain.input_manifest);
    assert_eq!(
        fingerprint_handler_execution_with(&targeted, &ctx, None).unwrap(),
        wrapped.fingerprint,
    );
}
