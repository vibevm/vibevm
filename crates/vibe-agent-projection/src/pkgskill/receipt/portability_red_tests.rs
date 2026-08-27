//! Portability RED oracles: exact ownership, the unsupported portable
//! rename, the write-side receipt law, and the one portable component law
//! as the strict reader applies it.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use vibe_wire::generated::package_skill_receipt::PackageSkillFile as ReceiptFile;

use super::nofollow::Project;
use super::stage::Stage;
use super::state::{digest, fresh_nonce, read_receipt, write_receipt};
use super::tests::{lower, provider, seed};
use super::transaction::{Plan, execute_plan};
use crate::pkgskill::reconcile_project_skill_binding;

const RECEIPT: &str = ".vibe/package-skills.toml";
const STAGED: &str = ".vibe/package-skills/staged";

/// Exact ownership is not collision identity: a prior-owned `SKILL.md` must
/// never authorize publishing over a distinct `skill.md`. Driven at the
/// transaction level with the plan-level rename guard deliberately bypassed,
/// and with a real durable stage present — so a fold-keyed owner set would
/// genuinely overwrite the foreign bytes here.
#[test]
fn prior_ownership_never_authorizes_a_differently_spelled_file() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    let alpha = &bindings[0];
    reconcile_project_skill_binding(project.path(), alpha).unwrap();
    let target = Path::new(&alpha.targets[0].path);
    // A never-owned `skill.md` holding foreign bytes: the very same file as
    // the owned `SKILL.md` on a case-insensitive host, its neighbour on a
    // case-sensitive one.
    fs::write(target.join("skill.md"), "someone-elses").unwrap();

    let project_cap = Project::open(project.path()).unwrap();
    let _guard = project_cap.lock(super::nofollow::LOCK_FILE).unwrap();
    let receipt = read_receipt(&project_cap).unwrap().unwrap();
    let desired = BTreeMap::from([("skill.md".to_string(), b"renamed-bytes".to_vec())]);
    let stage = Stage::create(&project_cap, &desired).unwrap();
    let mut after = receipt.binding.clone();
    after[0].target[0].file[0].path = "skill.md".into();
    after[0].target[0].file[0].sha256 = digest(b"renamed-bytes");
    let plan = Plan {
        key: alpha.identity(),
        nonce: stage.nonce.clone(),
        before: receipt.binding.clone(),
        after,
    };

    let error = execute_plan(&project_cap, project.path(), Some(&stage), &plan).unwrap_err();
    let error = format!("{error:#}");
    assert!(
        error.contains("refusing unowned pre-existing file"),
        "{error}"
    );
    assert_eq!(
        fs::read_to_string(target.join("skill.md")).unwrap(),
        "someone-elses",
        "foreign bytes at the differently spelled path are preserved"
    );
    // On a case-sensitive host the old owned bytes survive beside them.
    #[cfg(unix)]
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "body-one"
    );
    let current = read_receipt(&project_cap).unwrap().unwrap();
    assert_eq!(current, receipt, "the committed receipt is untouched");
    assert!(current.applying.is_none());
}

/// An ASCII case-only rename of an owned file refuses **before** the stage
/// and the durable intent: no staged bytes, a byte-identical receipt, and
/// the previously published bytes preserved.
#[test]
fn ascii_case_rename_refuses_before_stage_and_durable_intent() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    reconcile_project_skill_binding(project.path(), &bindings[0]).unwrap();
    let target = Path::new(&bindings[0].targets[0].path);
    let committed = fs::read(project.path().join(RECEIPT)).unwrap();

    let body = one.path().join("skills/body");
    fs::rename(body.join("SKILL.md"), body.join("skill.md")).unwrap();
    let renamed = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    assert!(
        renamed[0]
            .selected_files
            .as_ref()
            .unwrap()
            .contains_key("skill.md"),
        "the source now spells the file `skill.md`"
    );

    let error = reconcile_project_skill_binding(project.path(), &renamed[0]).unwrap_err();
    let error = format!("{error:#}");
    assert!(error.contains("portable rename is unsupported"), "{error}");
    assert!(
        !project.path().join(STAGED).exists(),
        "nothing was staged for the refused rename"
    );
    assert_eq!(fs::read(project.path().join(RECEIPT)).unwrap(), committed);
    let receipt = read_receipt(&Project::open(project.path()).unwrap())
        .unwrap()
        .unwrap();
    assert!(
        receipt.applying.is_none(),
        "no durable intent was published"
    );
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "body-one"
    );
}

/// The same law under full Unicode folding, where both spellings are
/// genuinely distinct files on every host: `Maße.md` stays owned, a foreign
/// `MASSE.md` beside it is preserved, and the receipt stays non-applying.
#[test]
fn unicode_fold_rename_refuses_and_preserves_both_spellings() {
    let project = tempfile::tempdir().unwrap();
    let package = tempfile::tempdir().unwrap();
    let body = package.path().join("skills/body");
    fs::create_dir_all(&body).unwrap();
    fs::write(body.join("Maße.md"), "eszett").unwrap();
    let bindings = lower(
        project.path(),
        vec![provider(package.path(), "one", "alpha", &["claude"])],
    );
    reconcile_project_skill_binding(project.path(), &bindings[0]).unwrap();
    let target = Path::new(&bindings[0].targets[0].path);
    assert_eq!(
        fs::read_to_string(target.join("Maße.md")).unwrap(),
        "eszett"
    );
    fs::write(target.join("MASSE.md"), "foreign").unwrap();
    let committed = fs::read(project.path().join(RECEIPT)).unwrap();

    fs::remove_file(body.join("Maße.md")).unwrap();
    fs::write(body.join("MASSE.md"), "upper").unwrap();
    let renamed = lower(
        project.path(),
        vec![provider(package.path(), "one", "alpha", &["claude"])],
    );
    let error = reconcile_project_skill_binding(project.path(), &renamed[0]).unwrap_err();
    let error = format!("{error:#}");
    assert!(error.contains("portable rename is unsupported"), "{error}");
    assert_eq!(
        fs::read_to_string(target.join("Maße.md")).unwrap(),
        "eszett",
        "the owned spelling keeps its published bytes"
    );
    assert_eq!(
        fs::read_to_string(target.join("MASSE.md")).unwrap(),
        "foreign",
        "the foreign alias is preserved, never adopted or overwritten"
    );
    assert_eq!(fs::read(project.path().join(RECEIPT)).unwrap(), committed);
    assert!(
        read_receipt(&Project::open(project.path()).unwrap())
            .unwrap()
            .unwrap()
            .applying
            .is_none()
    );
    assert!(!project.path().join(STAGED).exists());
}

/// The canonical identity is composition-aware: an owned NFC spelling and a
/// desired NFD one are one physical file on macOS, so the rename refuses
/// before the durable intent and both visible spellings plus the receipt
/// survive byte-identical.
#[test]
fn canonical_composition_rename_refuses_before_intent() {
    let project = tempfile::tempdir().unwrap();
    let package = tempfile::tempdir().unwrap();
    let body = package.path().join("skills/body");
    fs::create_dir_all(&body).unwrap();
    // Composed `é` (U+00E9) …
    fs::write(body.join("caf\u{e9}.md"), "composed").unwrap();
    let bindings = lower(
        project.path(),
        vec![provider(package.path(), "one", "alpha", &["claude"])],
    );
    reconcile_project_skill_binding(project.path(), &bindings[0]).unwrap();
    let target = Path::new(&bindings[0].targets[0].path);
    assert_eq!(
        fs::read_to_string(target.join("caf\u{e9}.md")).unwrap(),
        "composed"
    );
    // … and its decomposed `e` + U+0301 neighbour, a distinct file here.
    fs::write(target.join("cafe\u{301}.md"), "foreign").unwrap();
    let committed = fs::read(project.path().join(RECEIPT)).unwrap();

    fs::remove_file(body.join("caf\u{e9}.md")).unwrap();
    fs::write(body.join("cafe\u{301}.md"), "decomposed").unwrap();
    let renamed = lower(
        project.path(),
        vec![provider(package.path(), "one", "alpha", &["claude"])],
    );
    assert!(
        renamed[0]
            .selected_files
            .as_ref()
            .unwrap()
            .contains_key("cafe\u{301}.md"),
        "the source keeps the exact decomposed spelling"
    );

    let error = reconcile_project_skill_binding(project.path(), &renamed[0]).unwrap_err();
    let error = format!("{error:#}");
    assert!(error.contains("portable rename is unsupported"), "{error}");
    assert_eq!(
        fs::read_to_string(target.join("caf\u{e9}.md")).unwrap(),
        "composed",
        "the owned composed spelling keeps its published bytes"
    );
    assert_eq!(
        fs::read_to_string(target.join("cafe\u{301}.md")).unwrap(),
        "foreign",
        "the decomposed neighbour is preserved, never adopted"
    );
    assert_eq!(fs::read(project.path().join(RECEIPT)).unwrap(), committed);
    assert!(
        read_receipt(&Project::open(project.path()).unwrap())
            .unwrap()
            .unwrap()
            .applying
            .is_none()
    );
    assert!(!project.path().join(STAGED).exists());
}

/// Defense in depth: the writer validates the value it built under the same
/// strict semantic law as the reader, so no path can persist a receipt (or
/// an `applying` intent) this build could never read back.
#[test]
fn write_receipt_refuses_a_value_the_strict_reader_would_reject() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    reconcile_project_skill_binding(project.path(), &bindings[0]).unwrap();
    let receipt_path = project.path().join(RECEIPT);
    let committed = fs::read(&receipt_path).unwrap();
    let project_cap = Project::open(project.path()).unwrap();
    let receipt = read_receipt(&project_cap).unwrap().unwrap();

    let mut alias = receipt.clone();
    alias.binding[0].target[0].file.push(ReceiptFile {
        path: "skill.md".into(),
        sha256: digest(b"alias"),
    });
    let error = format!("{:#}", write_receipt(&project_cap, &alias).unwrap_err());
    assert!(
        error.contains("cannot read back") && error.contains("one file on a case-insensitive host"),
        "{error}"
    );

    let mut invalid = receipt.clone();
    invalid.binding[0].target[0].file[0].path = "a<b.md".into();
    let error = format!("{:#}", write_receipt(&project_cap, &invalid).unwrap_err());
    assert!(
        error.contains("cannot read back") && error.contains("unsafe file path `a<b.md`"),
        "{error}"
    );

    let mut intent = receipt.clone();
    let mut applying = receipt.binding.clone();
    applying[0].target[0].file[0].path = "trailing.".into();
    intent.applying = Some(
        vibe_wire::generated::package_skill_receipt::PackageSkillApplying {
            binding: applying,
            key: bindings[0].identity(),
            nonce: fresh_nonce(),
        },
    );
    let error = format!("{:#}", write_receipt(&project_cap, &intent).unwrap_err());
    assert!(
        error.contains("cannot read back") && error.contains("unsafe file path `trailing.`"),
        "{error}"
    );

    assert_eq!(
        fs::read(&receipt_path).unwrap(),
        committed,
        "the durable receipt never changed"
    );
}

/// A refused write mutates **nothing**: the whole value is canonicalized,
/// schema-checked, semantically validated and encoded before the writer may
/// acquire any directory capability, so on a fresh project `.vibe` is not
/// even created.
#[test]
fn a_refused_write_never_creates_the_project_directory() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    let files = bindings[0].selected_files.clone().unwrap();
    let row = super::state::receipt_binding(&bindings[0], &files);
    let vibe = project.path().join(".vibe");
    let project_cap = Project::open(project.path()).unwrap();

    let mut invalid_schema = super::state::empty_receipt();
    invalid_schema.binding = vec![row.clone()];
    invalid_schema.schema = 1;

    let mut unsafe_path = super::state::empty_receipt();
    let mut unsafe_row = row.clone();
    unsafe_row.target[0].file[0].path = "a<b.md".into();
    unsafe_path.binding = vec![unsafe_row];

    let mut invalid_intent = super::state::empty_receipt();
    invalid_intent.binding = vec![row.clone()];
    invalid_intent.applying = Some(
        vibe_wire::generated::package_skill_receipt::PackageSkillApplying {
            binding: vec![row.clone()],
            key: bindings[0].identity(),
            nonce: "not-a-hex-nonce".into(),
        },
    );

    for (label, candidate, needle) in [
        (
            "invalid schema",
            &invalid_schema,
            "refusing to persist package-skill receipt schema 1",
        ),
        ("unsafe path", &unsafe_path, "unsafe file path `a<b.md`"),
        (
            "invalid applying intent",
            &invalid_intent,
            "invalid package-skill applying nonce",
        ),
    ] {
        let error = format!("{:#}", write_receipt(&project_cap, candidate).unwrap_err());
        assert!(error.contains(needle), "{label}: {error}");
        assert!(
            !vibe.exists(),
            "{label}: a refused write must not create `.vibe`"
        );
    }

    // The very same writer accepts the valid value and only then creates it.
    let mut valid = super::state::empty_receipt();
    valid.binding = vec![row];
    write_receipt(&project_cap, &valid).unwrap();
    assert!(vibe.join("package-skills.toml").is_file());
}

/// The one portable component law as the strict reader applies it: every
/// Windows-invalid character, every control character, dot/space endings and
/// device stems refuse; legal Unicode is preserved.
#[test]
fn receipt_owned_paths_obey_the_one_portable_component_law() {
    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    reconcile_project_skill_binding(project.path(), &bindings[0]).unwrap();
    let receipt_path = project.path().join(RECEIPT);
    let text = fs::read_to_string(&receipt_path).unwrap();
    assert!(text.contains("\"SKILL.md\""), "{text}");

    for spelling in [
        "a<b.md",
        "a>b.md",
        "a:b.md",
        "a\\\"b.md",
        "a/../b.md",
        "a\\\\b.md",
        "a|b.md",
        "a?b.md",
        "a*b.md",
        // Every control block, spelled with TOML escapes: C0, DEL, and C1.
        "a\\u0000b.md",
        "a\\u0001b.md",
        "a\\u001Fb.md",
        "a\\u007Fb.md",
        "a\\u0085b.md",
        "a\\u009Fb.md",
        // Prefixes, separators, empty and dot components.
        "/lead.md",
        "a//b.md",
        "..",
        "a/./b.md",
        // Trailing dot or space, and device stems with extensions.
        "trailing.",
        "trailing ",
        "references/CON.txt",
        "references/NUL.md",
        "references/COM1.json",
        "references/CONIN$",
        "references/COM²",
    ] {
        fs::write(
            &receipt_path,
            text.replace("\"SKILL.md\"", &format!("\"{spelling}\"")),
        )
        .unwrap();
        let error = read_receipt(&Project::open(project.path()).unwrap()).unwrap_err();
        let error = format!("{error:#}");
        assert!(
            error.contains("invalid or duplicate owned file set"),
            "{spelling}: {error}"
        );
    }

    // Legal Unicode survives the same law untouched, and the reader hands
    // back the **exact** spelling — including a decomposed one, which only
    // the collision key normalizes.
    for spelling in [
        "références/Maße.md",
        "スキル/説明.md",
        "com10.md",
        "cafe\u{301}.md",
        "a\u{a0}b.md",
    ] {
        fs::write(
            &receipt_path,
            text.replace("\"SKILL.md\"", &format!("\"{spelling}\"")),
        )
        .unwrap();
        let receipt = read_receipt(&Project::open(project.path()).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(receipt.binding[0].target[0].file[0].path, *spelling);
    }
    fs::write(&receipt_path, &text).unwrap();
}

/// A real non-UTF-8 filename in the source refuses at selection time, before
/// the map, the stage, the intent, or any target mutation — and the already
/// committed receipt and target stay byte-identical. Unix-only: Windows
/// cannot create such a name through the ordinary filesystem API.
#[cfg(unix)]
#[test]
fn non_utf8_source_filename_refuses_and_leaves_the_commit_untouched() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let project = tempfile::tempdir().unwrap();
    let one = seed(project.path(), "one");
    let bindings = lower(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    );
    reconcile_project_skill_binding(project.path(), &bindings[0]).unwrap();
    let target = Path::new(&bindings[0].targets[0].path);
    let committed = fs::read(project.path().join(RECEIPT)).unwrap();

    let body = one.path().join("skills/body");
    let name = OsString::from_vec(vec![b'b', 0xff, 0xfe, b'.', b'm', b'd']);
    fs::write(body.join(&name), "unrepresentable").unwrap();

    let error = crate::pkgskill::lower_project_skill_bindings(
        project.path(),
        vec![provider(one.path(), "one", "alpha", &["claude"])],
    )
    .unwrap_err();
    let error = format!("{error:#}");
    assert!(error.contains("is not valid UTF-8"), "{error}");
    assert!(
        error.contains("b\\xFF\\xFE.md"),
        "the diagnostic names the exact raw bytes: {error}"
    );
    assert!(
        !error.contains('\u{fffd}'),
        "no replacement character reaches the diagnostic: {error}"
    );
    assert!(
        !error.contains(&*name.to_string_lossy()),
        "the lossy rendering must not appear anywhere: {error}"
    );

    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "body-one"
    );
    assert_eq!(fs::read(project.path().join(RECEIPT)).unwrap(), committed);
    assert!(
        read_receipt(&Project::open(project.path()).unwrap())
            .unwrap()
            .unwrap()
            .applying
            .is_none()
    );
    assert!(!project.path().join(STAGED).exists());
    assert_eq!(
        fs::read_dir(target).unwrap().count(),
        1,
        "no extra file reached the target"
    );
}
