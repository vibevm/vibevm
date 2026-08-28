//! The certified-observation REDs (R7.5 P2/A4b).
//!
//! The two-read detection law: a selected regular path is evidence only when
//! its no-follow single-link proof and length stayed equal around TWO
//! identical bounded capability reads. Every refusal here must leave the
//! LEGACY execution fingerprint byte-identical to HEAD's raw-read reference
//! (hardlinks included) while refusing the WHOLE evidence manifest — never a
//! partial digest, never a veto on the execution itself. Physical races are
//! injected through the deterministic `stable::inject` seams.

use std::fs;
use std::path::Path;

use super::support::{context, row};
use crate::HandlerExecution;
use crate::state::fingerprint::inputs::observe;
use crate::state::fingerprint::legacy;
use crate::state::fingerprint::stable::{InputRefusal, inject};
use crate::state::{fingerprint_execution, prepare_handler_execution_with};

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

fn prepared_with_patterns(root: &Path, patterns: &[&str]) -> crate::state::PreparedFingerprint {
    let owned: Vec<String> = patterns.iter().map(|p| (*p).to_string()).collect();
    let subject = row(root, "one", "0.1.0", Some(owned));
    let ctx = context(root, subject.effective_config().unwrap());
    prepare_handler_execution_with(&HandlerExecution::from_row(&subject), &ctx, None).unwrap()
}

fn legacy_matches(root: &Path, patterns: &[&str]) {
    let owned: Vec<String> = patterns.iter().map(|p| (*p).to_string()).collect();
    let subject = row(root, "one", "0.1.0", Some(owned));
    let ctx = context(root, subject.effective_config().unwrap());
    assert_eq!(
        fingerprint_execution(&subject, &ctx).unwrap(),
        legacy::execution_fingerprint_with(&subject, &ctx, None).unwrap(),
        "the legacy execution fingerprint keeps HEAD's exact raw-read bytes",
    );
}

fn refusal_of(prepared: &crate::state::PreparedFingerprint) -> InputRefusal {
    match &prepared.input_manifest.as_ref().unwrap().outcome {
        crate::state::fingerprint::inputs::ManifestOutcome::Refused(cause) => *cause,
        crate::state::fingerprint::inputs::ManifestOutcome::Measured(witness) => {
            panic!("expected a refusal, measured {witness:?}")
        }
    }
}

/// The decisive hardlink case, cross-platform: BOTH names of one inode are
/// legacy-regular, so each keeps feeding the legacy fingerprint through its
/// raw fallback — while the whole evidence manifest is refused.
#[test]
fn a_hardlinked_regular_input_feeds_the_legacy_fingerprint_but_refuses_evidence() {
    let dir = project();
    fs::write(dir.path().join("real.txt"), "shared body").unwrap();
    fs::hard_link(dir.path().join("real.txt"), dir.path().join("hard.txt")).unwrap();

    observe::reset();
    let prepared = prepared_with_patterns(dir.path(), &["*.txt"]);
    let (walks, fallbacks) = (observe::walks(), observe::raw_fallbacks());
    assert_eq!(
        refusal_of(&prepared),
        InputRefusal::Open,
        "a hard link fails the single-link inspect, so the manifest is refused",
    );
    assert_eq!(
        prepared.input_manifest.as_ref().unwrap().patterns,
        vec!["*.txt"]
    );
    assert_eq!(walks, 1);
    assert_eq!(
        fallbacks, 2,
        "one ordinary raw read per hardlinked regular path"
    );
    legacy_matches(dir.path(), &["*.txt"]);
}

/// The capability itself is evidence-only. A legacy-valid relative project
/// root cannot be pinned by `Project::open`, but its selected regular bytes
/// still take HEAD's raw path and produce the exact old execution fingerprint;
/// only the evidence manifest is refused.
#[test]
fn a_capability_open_refusal_never_vetoes_the_legacy_fingerprint() {
    let cwd = std::env::current_dir().unwrap();
    let dir = tempfile::Builder::new()
        .prefix("vibe-relative-evidence-")
        .tempdir_in(&cwd)
        .unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname='demo'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::write(dir.path().join("vibe.lock"), "lock").unwrap();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let relative = dir.path().strip_prefix(&cwd).unwrap();
    assert!(!relative.is_absolute());

    observe::reset();
    let prepared = prepared_with_patterns(relative, &["*.txt"]);
    assert_eq!(refusal_of(&prepared), InputRefusal::Open);
    assert_eq!(observe::raw_fallbacks(), 1);
    legacy_matches(relative, &["*.txt"]);
}

/// A SELECTED symlink stays unread and unfollowed by the legacy projection
/// (HEAD skipped it as a non-file) while its selection alone refuses the
/// whole evidence manifest — a clean sibling file cannot rescue it.
#[cfg(unix)]
#[test]
fn a_selected_symlink_refuses_evidence_and_is_never_followed() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    std::os::unix::fs::symlink("a.txt", dir.path().join("link.txt")).unwrap();

    observe::reset();
    let prepared = prepared_with_patterns(dir.path(), &["*.txt"]);
    assert_eq!(
        refusal_of(&prepared),
        InputRefusal::NonRegular,
        "the selected link itself refuses the manifest",
    );
    legacy_matches(dir.path(), &["*.txt"]);
    assert_eq!(
        observe::raw_fallbacks(),
        0,
        "a link the old scanner never read gets no raw fallback either",
    );
}

/// The Windows reparse-point twin: a junction selected by the declared
/// pattern refuses evidence while the legacy projection keeps ignoring it.
#[cfg(windows)]
#[test]
fn a_selected_junction_refuses_evidence_and_is_never_followed() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let target = dir.path().join("junction-target");
    fs::create_dir(&target).unwrap();
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(dir.path().join("link.d"))
        .arg(&target)
        .status()
        .unwrap();
    assert!(status.success(), "the junction fixture must be created");

    observe::reset();
    let prepared = prepared_with_patterns(dir.path(), &["*"]);
    assert_eq!(refusal_of(&prepared), InputRefusal::NonRegular);
    legacy_matches(dir.path(), &["*"]);
    assert_eq!(observe::raw_fallbacks(), 0);
}

/// An UNSELECTED link changes nothing: the manifest measures the clean scope
/// and neither identity differs from a tree without the link. (On Windows
/// the link is a junction — creatable without developer mode — and equally
/// unselected by the `*.txt` pattern.)
#[test]
fn an_unselected_link_changes_nothing() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let clean = prepared_with_patterns(dir.path(), &["*.txt"]);

    #[cfg(unix)]
    std::os::unix::fs::symlink("nowhere.txt", dir.path().join("side.dat")).unwrap();
    #[cfg(windows)]
    {
        let target = dir.path().join("side-target");
        fs::create_dir(&target).unwrap();
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(dir.path().join("side.dat"))
            .arg(&target)
            .status()
            .unwrap();
        assert!(status.success(), "the junction fixture must be created");
    }
    let polluted = prepared_with_patterns(dir.path(), &["*.txt"]);
    assert_eq!(polluted.fingerprint, clean.fingerprint);
    assert_eq!(polluted.input_manifest, clean.input_manifest);
}

/// The shared portable identity judgment: two selected spellings that fold
/// to one physical file (NFC versus NFD `é`) refuse the manifest AFTER both
/// legacy rows retained their old bytes.
#[test]
fn a_portable_identity_collision_refuses_the_manifest_after_legacy_rows() {
    let dir = project();
    // `caf\u{E9}` (NFC) and `cafe\u{301}` (NFD) are two distinct directory
    // entries on NTFS and on the usual Unix filesystems, and one physical
    // identity under the shared case/normalisation fold.
    fs::write(dir.path().join("caf\u{E9}.txt"), "composed").unwrap();
    fs::write(dir.path().join("cafe\u{301}.txt"), "decomposed").unwrap();

    let prepared = prepared_with_patterns(dir.path(), &["*.txt"]);
    assert_eq!(refusal_of(&prepared), InputRefusal::Aliased);
    legacy_matches(dir.path(), &["*.txt"]);
}

/// An injected same-length rewrite BETWEEN the two reads: both reads succeed
/// with equal proof and length but different bytes — `Disagree`, and the raw
/// fallback still feeds the legacy fingerprint with the settled bytes.
#[test]
fn an_injected_between_reads_same_length_change_refuses() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let file = dir.path().join("a.txt");
    inject::arm_between_reads(Some(Box::new(move |relative| {
        assert_eq!(relative, "a.txt");
        fs::write(&file, "TWO").unwrap();
    })));

    observe::reset();
    let prepared = prepared_with_patterns(dir.path(), &["*.txt"]);
    let fallbacks = observe::raw_fallbacks();
    inject::arm_between_reads(None);
    assert_eq!(refusal_of(&prepared), InputRefusal::Disagree);
    assert_eq!(fallbacks, 1);
    legacy_matches(dir.path(), &["*.txt"]);
}

/// Growth between the reads refuses at the second read's own cap fence: the
/// pre-proof length is the cap, so a longer file cannot be read as a
/// truncated prefix.
#[test]
fn an_injected_growth_between_reads_refuses() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let file = dir.path().join("a.txt");
    inject::arm_between_reads(Some(Box::new(move |relative| {
        assert_eq!(relative, "a.txt");
        fs::write(&file, "one that grew").unwrap();
    })));

    let prepared = prepared_with_patterns(dir.path(), &["*.txt"]);
    inject::arm_between_reads(None);
    assert_eq!(refusal_of(&prepared), InputRefusal::Read);
    legacy_matches(dir.path(), &["*.txt"]);
}

/// A whole-object swap AFTER both reads (before the post-inspect): the bytes
/// in hand came from an object the name no longer denotes — `Unstable` by
/// proof inequality, never certified bytes over a swapped file.
#[test]
fn an_injected_post_reads_identity_swap_refuses() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    let file = dir.path().join("a.txt");
    inject::arm_after_reads(Some(Box::new(move |relative| {
        assert_eq!(relative, "a.txt");
        fs::remove_file(&file).unwrap();
        fs::write(&file, "TWO").unwrap();
    })));

    let prepared = prepared_with_patterns(dir.path(), &["*.txt"]);
    inject::arm_after_reads(None);
    assert_eq!(refusal_of(&prepared), InputRefusal::Unstable);
    legacy_matches(dir.path(), &["*.txt"]);
}

/// The clean baseline: a stable file certifies on two identical bounded
/// reads, measures the declared scope, and never touches its raw fallback.
#[test]
fn a_clean_stable_two_read_measurement_certifies_the_declared_scope() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();

    observe::reset();
    let prepared = prepared_with_patterns(dir.path(), &["*.txt"]);
    let (walks, fallbacks) = (observe::walks(), observe::raw_fallbacks());
    let manifest = prepared.input_manifest.as_ref().unwrap();
    let witness = manifest.measured().expect("a clean stable file measures");
    assert_eq!(witness.files, Some(1));
    assert_eq!(witness.bytes, Some("3".to_string()));
    assert_eq!(
        witness.digest, "sha256:b676f0a870baaf83ad39375642ef43fa0e328bba0d59712779e9570d3a284060",
        "the certified bytes are exactly the bytes the A4a manifest pinned",
    );
    assert_eq!(fallbacks, 0);
    assert_eq!(walks, 1);
    legacy_matches(dir.path(), &["*.txt"]);
}

/// A refusal is total for the execution: a clean sibling cannot donate a
/// partial manifest, and the declared patterns survive as a fact either way.
#[test]
fn a_refusal_is_total_never_a_partial_manifest() {
    let dir = project();
    fs::write(dir.path().join("a.txt"), "one").unwrap();
    fs::write(dir.path().join("real.md"), "shared").unwrap();
    fs::hard_link(dir.path().join("real.md"), dir.path().join("hard.md")).unwrap();

    let prepared = prepared_with_patterns(dir.path(), &["*.txt", "*.md"]);
    assert_eq!(refusal_of(&prepared), InputRefusal::Open);
    assert!(
        prepared
            .input_manifest
            .as_ref()
            .unwrap()
            .measured()
            .is_none()
    );
    assert_eq!(
        prepared.input_manifest.as_ref().unwrap().patterns,
        vec!["*.txt".to_string(), "*.md".to_string()],
        "the declared scope is a fact even when its witness is refused",
    );
    legacy_matches(dir.path(), &["*.txt", "*.md"]);
}

/// The declaration destructuring fence: this file's own fixtures cannot
/// construct an `ExtensionDecl` with an unclassified field, and the recipe
/// module fails compilation the day a new field appears. The pinned
/// companion is that the observed digest above depends on every selected
/// byte — the manifest's own law, already pinned in `inputs.rs`.
#[test]
fn refused_scopes_still_declare_their_patterns_to_the_state_conversion() {
    let dir = project();
    fs::write(dir.path().join("real.txt"), "shared").unwrap();
    fs::hard_link(dir.path().join("real.txt"), dir.path().join("hard.txt")).unwrap();
    let prepared = prepared_with_patterns(dir.path(), &["*.txt"]);
    assert!(
        prepared
            .state_measurement(
                "__host__/demo#announce",
                "build",
                "00112233445566778899aabbccddeeff"
            )
            .is_none(),
        "a refused observation yields NO state measurement — never a partial one",
    );
}
