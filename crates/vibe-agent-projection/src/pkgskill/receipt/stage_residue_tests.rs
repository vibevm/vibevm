//! Staging failure leaves no orphan nonce, and deletes nothing it did not make.
//!
//! `Stage::create` runs before any durable intent exists, so a nonce directory
//! left behind is an orphan nobody ever collects. The nonce is created
//! exclusively, which is what licenses cleaning it *and* what bounds the
//! cleanup: only the files this invocation published, and only its own two
//! directories.
//!
//! The licence is to *attempt* a cleanup, never to delete unverified. Every
//! removal reopens no-follow first, so a name that has been swapped since is
//! preserved and named as residue rather than followed or deleted — and the
//! diagnostics say only what was proved, which is why the swapped case and the
//! ordinary reopen failure render the same sentence.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::nofollow::Project;
use super::stage::Stage;
use super::state::digest;

fn project() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    (dir, project)
}

fn staged_root(dir: &Path) -> PathBuf {
    dir.join(".vibe/package-skills/staged")
}

/// A neighbour nonce from another transaction: cleanup is bounded to what this
/// invocation created, so it must survive byte-for-byte.
fn plant_neighbour(dir: &Path) -> PathBuf {
    let neighbour = staged_root(dir).join("neighbour-nonce/files");
    fs::create_dir_all(&neighbour).unwrap();
    fs::write(neighbour.join("sha256-other"), b"not ours").unwrap();
    neighbour
}

fn desired() -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([
        ("a".to_string(), b"alpha".to_vec()),
        ("b".to_string(), b"beta".to_vec()),
    ])
}

fn nonce_dirs(dir: &Path) -> Vec<String> {
    let root = staged_root(dir);
    if !root.is_dir() {
        return Vec::new();
    }
    let mut names: Vec<String> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn staged_file_name(bytes: &[u8]) -> String {
    let sha = digest(bytes);
    sha.strip_prefix("sha256:").unwrap_or(&sha).to_string()
}

#[test]
fn a_successful_stage_keeps_its_nonce() {
    let (dir, project) = project();
    plant_neighbour(dir.path());
    let stage = Stage::create(&project, &desired()).expect("staging succeeds");
    assert!(nonce_dirs(dir.path()).contains(&stage.nonce));
}

/// A failure *before* publication, where cleanup itself cannot finish because
/// the tree holds something this invocation did not create.
///
/// Two rules meet here and neither may bend: cleanup removes only what it
/// owns, so the planted directory survives — and because it survives, the
/// nonce cannot be removed either, so the error must **name** the residue
/// instead of implying a clean tree.
#[test]
fn a_pre_publication_failure_names_residue_it_does_not_own() {
    let (dir, project) = project();
    let neighbour = plant_neighbour(dir.path());

    // Fire in the window before `files` is created: make it, and put a
    // directory where the first staged file must land. The publication then
    // refuses at its destination check — before any rename.
    let blocked = staged_file_name(b"alpha");
    let planted = blocked.clone();
    vibe_safefs::arm_before_create_dir(Some(Box::new(move |parent, name| {
        if name != "files" {
            return;
        }
        if let Ok(files) = parent.ensure_child("files") {
            let _ = files.ensure_child(&planted);
        }
    })));
    let outcome = Stage::create(&project, &desired());
    vibe_safefs::arm_before_create_dir(None);

    let error = outcome.expect_err("an unpublishable staged destination fails staging");
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("could not be removed and remains under"),
        "surviving residue must be named, not implied away: {rendered}"
    );
    assert!(
        rendered.contains("files (not empty)"),
        "and named precisely: {rendered}"
    );
    let surviving: Vec<String> = nonce_dirs(dir.path())
        .into_iter()
        .filter(|name| name != "neighbour-nonce")
        .collect();
    assert_eq!(
        surviving.len(),
        1,
        "the named nonce is the one that remains"
    );
    assert!(
        staged_root(dir.path())
            .join(&surviving[0])
            .join("files")
            .join(&blocked)
            .is_dir(),
        "the entry this invocation did not create must survive untouched",
    );
    assert_eq!(
        fs::read_to_string(neighbour.join("sha256-other")).unwrap(),
        "not ours",
        "and nothing outside the owned nonce was deleted",
    );
}

/// The window the old bare `?` fell through: this invocation's exclusive create
/// succeeded, and *then* the no-follow reopen failed. Nothing has been
/// published, so the guarded cleanup can complete — and it must, or the tree
/// keeps a nonce no intent will ever reference and no later run can attribute.
#[test]
fn a_nonce_created_but_not_reopened_leaves_nothing_behind() {
    let (dir, project) = project();
    let neighbour = plant_neighbour(dir.path());

    // Exactly after the exclusive create, exactly before the reopen. Deciding
    // the reopen failed — rather than arranging a real failure — is what makes
    // this branch reachable on every host.
    vibe_safefs::arm_after_create_dir(Some(Box::new(|_parent, _name| {
        Some(std::io::Error::other("injected reopen failure"))
    })));
    let outcome = Stage::create(&project, &desired());
    vibe_safefs::arm_after_create_dir(None);

    let error = outcome.expect_err("a nonce that cannot be reopened fails staging");
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("injected reopen failure"),
        "the cause survives: {rendered}"
    );
    // Only the fact `create_dir` proved, and only the doubt the failed reopen
    // proved. The same sentence is rendered when the entry has been swapped
    // (below), so a claim of continuing ownership would be false there.
    assert!(
        rendered.contains("this call created")
            && rendered.contains(
                "but the entry now at that name could not be reopened no-follow and may have \
                 been replaced since"
            ),
        "the diagnostic must state exactly the proved facts: {rendered}"
    );
    assert!(
        !rendered.contains("is still owned"),
        "and must never claim the entry is still the caller's: {rendered}"
    );
    assert!(
        !rendered.contains("could not be removed and remains under"),
        "cleanup succeeded, so nothing may be claimed as residue: {rendered}"
    );
    assert_eq!(
        nonce_dirs(dir.path()),
        vec!["neighbour-nonce".to_string()],
        "no unreferenced nonce may survive: {rendered}",
    );
    assert_eq!(
        fs::read_to_string(neighbour.join("sha256-other")).unwrap(),
        "not ours",
    );
}

/// The same window, but the entry is swapped for something this invocation did
/// not create. The reopen refuses because it does not follow; cleanup must then
/// **preserve and name** what it found rather than following it or deleting it.
#[test]
fn a_nonce_swapped_before_reopen_is_preserved_and_named() {
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("bystander.txt"), "untouched").unwrap();
    let (dir, project) = project();
    let neighbour = plant_neighbour(dir.path());
    let root = staged_root(dir.path());
    let target = outside.path().to_path_buf();

    // Swap the freshly created directory for a link to somewhere else, then
    // let the *real* reopen run: no-follow must refuse it.
    vibe_safefs::arm_after_create_dir(Some(Box::new(move |_parent, name| {
        let planted = root.join(name);
        fs::remove_dir(&planted).unwrap();
        plant_link(&planted, &target);
        None
    })));
    let outcome = Stage::create(&project, &desired());
    vibe_safefs::arm_after_create_dir(None);

    let error = outcome.expect_err("a swapped nonce fails staging");
    let rendered = format!("{error:#}");
    let swapped: Vec<String> = nonce_dirs(dir.path())
        .into_iter()
        .filter(|name| name != "neighbour-nonce")
        .collect();
    assert_eq!(swapped.len(), 1, "the planted entry survives: {rendered}");
    assert!(
        rendered.contains("could not be removed and remains under")
            && rendered.contains(&swapped[0]),
        "and is named as residue rather than implied away: {rendered}"
    );
    // The wording that makes this case and the one above *the same sentence*:
    // here the name holds a foreign junction, so "still owned" would be a lie
    // and "may have been replaced" is exactly right.
    assert!(
        rendered.contains("this call created") && rendered.contains("may have been replaced since"),
        "the same proved-facts clause must render here: {rendered}"
    );
    assert!(
        !rendered.contains("is still owned"),
        "the residue named here is not the caller's directory: {rendered}"
    );
    assert_eq!(
        fs::read_to_string(outside.path().join("bystander.txt")).unwrap(),
        "untouched",
        "the link was never followed: {rendered}",
    );
    assert_eq!(
        fs::read_to_string(neighbour.join("sha256-other")).unwrap(),
        "not ours",
    );
}

#[cfg(windows)]
fn plant_link(planted: &Path, target: &Path) {
    // `cmd` reads a forward slash as the start of a switch, and these paths are
    // built with `/` separators for the Rust APIs that accept either.
    let planted = PathBuf::from(planted.to_string_lossy().replace('/', "\\"));
    let output = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&planted)
        .arg(target)
        .output()
        .expect("mklink is available on Windows");
    assert!(
        output.status.success(),
        "planting the junction at `{}` -> `{}`: {}{}",
        planted.display(),
        target.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[cfg(unix)]
fn plant_link(planted: &Path, target: &Path) {
    std::os::unix::fs::symlink(target, planted).unwrap();
}

/// A failure *after* a rename: the message names the file whose publication
/// could not be ruled out, and cleanup still removes only what we own.
#[test]
fn a_post_publication_failure_names_the_possibly_published_file() {
    let (dir, project) = project();
    let neighbour = plant_neighbour(dir.path());

    let target = staged_file_name(b"alpha");
    vibe_safefs::fail_after_publish(Some(&target));
    let outcome = Stage::create(&project, &desired());
    vibe_safefs::fail_after_publish(None);

    let error = outcome.expect_err("an injected post-publication failure fails staging");
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("may have been published") && rendered.contains(&target),
        "the hedge must name the file: {rendered}"
    );
    assert_eq!(
        nonce_dirs(dir.path()),
        vec!["neighbour-nonce".to_string()],
        "the owned nonce is still cleaned up: {rendered}",
    );
    assert_eq!(
        fs::read_to_string(neighbour.join("sha256-other")).unwrap(),
        "not ours",
    );
}
