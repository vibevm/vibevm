//! The px step of `vibe self apply` (PROP-019 `#companion-tools`): a
//! profile that selects px gets px, from the companion archive beside the
//! running executable, into the store of the install root THAT profile
//! chooses — and when there is no archive, the step says so with the
//! repair instead of one grey `skipped` line.
//!
//! The pin is fabricated here: a real px archive cannot be written by a
//! test, because its bytes are checked against the digest the shipped pin
//! carries. Everything else — the two places the archive is read from, the
//! store layout, the verification — is the production lane.

#![cfg(test)]

use std::io::Write;
use std::path::Path;

use vibe_core::progress::Progress;
use vibe_net::px_store::{PinnedAsset, PxPin};
use vibe_wire::generated::self_apply_report::{PxOutcome, PxStep};

use super::px_step_with;
use crate::commands::vvm::bundle::px_lane::px_store_dir;
use crate::commands::vvm::store::VersionStore;

/// A pin for bytes a test may write, shaped like the embedded one.
fn pin_for(bytes: &[u8], executable: &str) -> Option<PxPin> {
    use sha2::{Digest, Sha256};
    let asset = PinnedAsset {
        target: vibe_net::px_store::running_target()?.to_owned(),
        url: "https://example.invalid/px".into(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        archive: "zip".into(),
        executable: executable.into(),
    };
    Some(PxPin {
        assets: vec![asset],
        version: "0.0.1-apply-test".into(),
        ..PxPin::embedded().clone()
    })
}

fn zip_bytes(name: &str, contents: &[u8]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .start_file(name, zip::write::SimpleFileOptions::default())
        .unwrap();
    writer.write_all(contents).unwrap();
    writer.finish().unwrap().into_inner()
}

struct Fixture {
    store: VersionStore,
    pin: PxPin,
    /// The directory the running release was unpacked into.
    release: tempfile::TempDir,
    archive: Vec<u8>,
    asset: String,
    _root: tempfile::TempDir,
}

impl Fixture {
    /// `None` on a platform the pin has no px for — the `no_target` lane,
    /// which has its own assertion below.
    fn new() -> Option<Self> {
        let root = tempfile::tempdir().unwrap();
        let release = tempfile::tempdir().unwrap();
        let archive = zip_bytes(vibe_net::PX_EXE, b"px for the test");
        let pin = pin_for(&archive, vibe_net::PX_EXE)?;
        let asset = pin.release_asset_name(pin.assets.first().unwrap());
        Some(Fixture {
            store: VersionStore::new(root.path().join("opt")),
            pin,
            release,
            archive,
            asset,
            _root: root,
        })
    }

    fn step(&self, needed: bool, dry_run: bool, imported: bool) -> PxStep {
        px_step_with(
            &self.store,
            &self.pin,
            Some(self.release.path()),
            needed,
            dry_run,
            imported,
            &Progress::default(),
        )
        .unwrap()
    }

    /// Put the companion archive where an operator puts it.
    fn place_archive(&self, dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(&self.asset), &self.archive).unwrap();
    }
}

/// The whole sequence the owner walked: apply with no px beside the binary
/// reports the repair; the archive is copied in and the next apply installs
/// px; the one after that changes nothing.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn a_profile_selecting_px_gets_px_on_the_apply_after_the_archive_arrives() {
    let Some(fixture) = Fixture::new() else {
        eprintln!("skipping: the pin has no px for this platform");
        return;
    };
    let stored = px_store_dir(&fixture.store, &fixture.pin);

    // (a) Nothing beside the binary: named, with the fix, not a success.
    let missing = fixture.step(true, false, false);
    assert_eq!(missing.outcome, PxOutcome::Missing);
    assert!(missing.needed && !missing.planned);
    assert_eq!(missing.program, None);
    assert_eq!(missing.asset.as_deref(), Some(fixture.asset.as_str()));
    let fix = missing.fix.expect("a missing px names its repair");
    assert!(fix.contains(&fixture.asset), "{fix}");
    assert!(fix.contains("vibe self px install --from"), "{fix}");
    assert!(fix.contains(&stored.display().to_string()), "{fix}");
    assert!(!stored.exists(), "nothing was written");

    // (b) The operator copies the companion archive beside `vibe`.
    fixture.place_archive(fixture.release.path());
    let planned = fixture.step(true, true, false);
    assert_eq!(planned.outcome, PxOutcome::Installed);
    assert!(planned.planned && planned.program.is_none());
    assert!(planned.fix.is_none());
    assert!(!stored.exists(), "a dry run writes nothing");

    let installed = fixture.step(true, false, false);
    assert_eq!(installed.outcome, PxOutcome::Installed);
    assert!(installed.fix.is_none() && !installed.planned);
    let program = installed.program.expect("the step names the px it placed");
    assert_eq!(
        std::fs::read(&program).unwrap(),
        b"px for the test",
        "the pinned bytes, unpacked"
    );
    assert_eq!(program, stored.join(vibe_net::PX_EXE).display().to_string());

    // (c) The apply after that is a no-op, and says so.
    let again = fixture.step(true, false, false);
    assert_eq!(again.outcome, PxOutcome::AlreadyInstalled);
    assert_eq!(again.program, Some(program.clone()));
    assert!(again.fix.is_none());
    // Also when the import step of the same invocation already ran this
    // lane: the step observes, and never reports twice.
    let after_import = fixture.step(true, false, true);
    assert_eq!(after_import.outcome, PxOutcome::AlreadyInstalled);
    assert_eq!(after_import.program, Some(program));
}

/// `tools/px/` beside the binary is read too — the layout an unpacked
/// release has — and a profile that does not select px is left alone even
/// with the archive sitting right there.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn the_step_reads_the_tools_directory_and_does_nothing_for_another_proxy_mode() {
    let Some(fixture) = Fixture::new() else {
        eprintln!("skipping: the pin has no px for this platform");
        return;
    };
    let stored = px_store_dir(&fixture.store, &fixture.pin);
    fixture.place_archive(&fixture.release.path().join("tools").join("px"));

    // Not selected: no store is consulted and nothing is installed.
    let untouched = fixture.step(false, false, false);
    assert_eq!(untouched.outcome, PxOutcome::NotNeeded);
    assert!(!untouched.needed && untouched.fix.is_none());
    assert!(!stored.exists());

    let installed = fixture.step(true, false, false);
    assert_eq!(installed.outcome, PxOutcome::Installed);
    assert!(stored.join(vibe_net::PX_EXE).is_file());
}

/// Whatever px the operator put beside `vibe` is the px they get: a
/// digest that is not the pin file's is no refusal (owner decision
/// 2026-11-09). Bytes that are not a px at all still stop the step.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn any_px_beside_the_binary_is_installed_and_only_a_non_px_stops_the_step() {
    let Some(fixture) = Fixture::new() else {
        eprintln!("skipping: the pin has no px for this platform");
        return;
    };
    let mine = zip_bytes(vibe_net::PX_EXE, b"a px of my own");
    assert_ne!(mine, fixture.archive);
    std::fs::write(fixture.release.path().join(&fixture.asset), &mine).unwrap();
    let step = fixture.step(true, false, false);
    assert_eq!(step.outcome, PxOutcome::Installed);
    let program = step.program.expect("installed");
    assert_eq!(std::fs::read(&program).unwrap(), b"a px of my own");

    // Not an archive and not a program: refused, and nothing is left.
    let Some(other) = Fixture::new() else { return };
    std::fs::write(
        other.release.path().join(&other.asset),
        b"not an archive at all",
    )
    .unwrap();
    assert!(
        px_step_with(
            &other.store,
            &other.pin,
            Some(other.release.path()),
            true,
            false,
            false,
            &Progress::default(),
        )
        .is_err()
    );
}

/// With no directory to read — a binary whose own path has no parent —
/// the step still reports the repair rather than failing the apply.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn a_step_with_nowhere_to_read_from_reports_the_repair() {
    let Some(fixture) = Fixture::new() else {
        eprintln!("skipping: the pin has no px for this platform");
        return;
    };
    let step = px_step_with(
        &fixture.store,
        &fixture.pin,
        None,
        true,
        false,
        false,
        &Progress::default(),
    )
    .unwrap();
    assert_eq!(step.outcome, PxOutcome::Missing);
    assert!(step.fix.is_some());
}
