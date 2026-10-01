use std::collections::VecDeque;

use super::*;
use crate::github_release::GithubGitObject;

fn commit() -> String {
    "a".repeat(40)
}
fn digest() -> String {
    format!("sha256:{}", "b".repeat(64))
}

fn tag() -> GithubGitRef {
    GithubGitRef {
        reference: REFERENCE.into(),
        object: GithubGitObject {
            sha: commit(),
            kind: "commit".into(),
            url: String::new(),
        },
    }
}

fn release() -> GithubRelease {
    GithubRelease {
        id: 7,
        tag_name: TAG.into(),
        target_commitish: commit(),
        name: Some("v1.1.0".into()),
        body: Some("draft".into()),
        draft: true,
        prerelease: false,
        html_url: "https://github.com/vibevm/vibevm/releases/7".into(),
        upload_url: "https://uploads.github.com/repos/vibevm/vibevm/releases/7/assets".into(),
    }
}

fn asset(id: u64, name: &str) -> GithubReleaseAsset {
    GithubReleaseAsset {
        id,
        name: name.into(),
        size: 3,
        digest: Some(digest()),
        browser_download_url: format!("https://github.com/vibevm/vibevm/releases/download/{name}"),
        content_type: "application/octet-stream".into(),
    }
}

fn snapshot() -> RemoteSnapshot {
    RemoteSnapshot {
        latest: LatestReleaseObservation::Present {
            release_id: 101,
            tag: "v1.0.1".into(),
        },
        tag: Some(tag()),
        release: Some(release()),
        assets: Some(vec![asset(20, "second"), asset(3, "first")]),
        streamed_digests: Vec::new(),
    }
}

struct FakePort(VecDeque<RemoteSnapshot>);
impl RemoteReadPort for FakePort {
    fn read_complete(&mut self) -> Result<RemoteSnapshot, ObservationError> {
        self.0
            .pop_front()
            .ok_or_else(|| ObservationError::Unavailable("fake read absent".into()))
    }
}

#[test]
fn absent_and_reordered_present_observations_are_stable() {
    let absent = RemoteSnapshot {
        latest: LatestReleaseObservation::Absent,
        tag: None,
        release: None,
        assets: None,
        streamed_digests: Vec::new(),
    };
    let first = normalize_fixed(absent.clone(), &commit()).unwrap();
    assert!(matches!(first.tag, PublicationObservedTag::Absent(_)));
    assert!(matches!(
        first.release,
        PublicationObservedRelease::Absent(_)
    ));
    assert!(first.generation_ref.starts_with("sha256:"));
    let one = snapshot();
    let mut two = one.clone();
    two.assets.as_mut().unwrap().reverse();
    let result = normalize_fixed(one.clone(), &commit()).unwrap();
    assert_eq!(result, normalize_fixed(two.clone(), &commit()).unwrap());
    let PublicationObservedRelease::Present(release) = &result.release else {
        panic!("release missing")
    };
    assert_eq!(
        release
            .assets
            .iter()
            .map(|row| row.asset_id.as_str())
            .collect::<Vec<_>>(),
        vec!["3", "20"]
    );
    assert_eq!(release.generation_ref, result.generation_ref);
    let mut fake = FakePort(VecDeque::from([one, two]));
    assert_eq!(observe_twice(&mut fake, &commit()).unwrap(), result);
}

#[test]
fn latest_baseline_is_bound_to_same_epoch_fingerprint() {
    let first = snapshot();
    let mut changed = first.clone();
    changed.latest = LatestReleaseObservation::Present {
        release_id: 102,
        tag: "v1.0.2".into(),
    };
    let before = normalize_fixed(first.clone(), &commit()).unwrap();
    let after = normalize_fixed(changed.clone(), &commit()).unwrap();
    assert_ne!(before.generation_ref, after.generation_ref);
    let mut fake = FakePort(VecDeque::from([first, changed]));
    assert_eq!(
        observe_twice(&mut fake, &commit()),
        Err(ObservationError::Drift)
    );
}

#[test]
fn duplicate_foreign_and_malformed_remote_data_refuse() {
    let mut duplicate_name = snapshot();
    duplicate_name.assets.as_mut().unwrap()[1].name = "second".into();
    assert!(matches!(
        normalize_fixed(duplicate_name, &commit()),
        Err(ObservationError::Refused(_))
    ));
    let mut duplicate_id = snapshot();
    duplicate_id.assets.as_mut().unwrap()[1].id = 20;
    assert!(normalize_fixed(duplicate_id, &commit()).is_err());
    let mut foreign = snapshot();
    foreign.tag.as_mut().unwrap().object.sha = "c".repeat(40);
    assert!(normalize_fixed(foreign, &commit()).is_err());
    let mut malformed = snapshot();
    malformed.assets.as_mut().unwrap()[0].digest = Some("sha256:ABC".into());
    assert!(normalize_fixed(malformed, &commit()).is_err());
    let mut incomplete = snapshot();
    incomplete.assets = None;
    assert!(matches!(
        normalize_fixed(incomplete, &commit()),
        Err(ObservationError::Unavailable(_))
    ));
    let mut absent_with_assets = snapshot();
    absent_with_assets.release = None;
    assert!(normalize_fixed(absent_with_assets, &commit()).is_err());
    let mut oversized = snapshot();
    oversized.assets.as_mut().unwrap()[0].name = "x".repeat(MAX_TEXT + 1);
    assert!(normalize_fixed(oversized, &commit()).is_err());
    let mut notes = snapshot();
    notes.release.as_mut().unwrap().body = Some(format!("notes\n{}", "x".repeat(20 * 1024)));
    assert!(normalize_fixed(notes.clone(), &commit()).is_ok());
    notes.release.as_mut().unwrap().body = Some("x".repeat(MAX_BODY + 1));
    assert!(matches!(
        normalize_fixed(notes, &commit()),
        Err(ObservationError::Refused(_))
    ));
}

#[test]
fn missing_digest_needs_independent_witness_on_both_reads() {
    let mut first = snapshot();
    first.assets.as_mut().unwrap()[0].digest = None;
    assert!(matches!(
        normalize_fixed(first.clone(), &commit()),
        Err(ObservationError::Unavailable(_))
    ));
    first.streamed_digests.push(AssetDigestEvidence {
        asset_id: 20,
        size_bytes: 3,
        sha256: digest(),
    });
    let mut second = first.clone();
    second.assets.as_mut().unwrap().reverse();
    let mut fake = FakePort(VecDeque::from([first.clone(), second]));
    assert!(observe_twice(&mut fake, &commit()).is_ok());
    let mut missing_second = first.clone();
    missing_second.streamed_digests.clear();
    let mut fake = FakePort(VecDeque::from([first.clone(), missing_second]));
    assert!(matches!(
        observe_twice(&mut fake, &commit()),
        Err(ObservationError::Unavailable(_))
    ));
    let mut wrong = first.clone();
    wrong.streamed_digests[0].sha256 = format!("sha256:{}", "c".repeat(64));
    let mut fake = FakePort(VecDeque::from([first, wrong]));
    assert_eq!(
        observe_twice(&mut fake, &commit()),
        Err(ObservationError::Drift)
    );
}

#[test]
fn changed_bookends_refuse_and_do_not_guess_post_effect_success() {
    let first = snapshot();
    let mut second = first.clone();
    second.release.as_mut().unwrap().id = 8;
    let mut fake = FakePort(VecDeque::from([first, second]));
    assert_eq!(
        observe_twice(&mut fake, &commit()),
        Err(ObservationError::Drift)
    );
}
