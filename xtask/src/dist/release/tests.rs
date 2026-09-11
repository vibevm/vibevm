use super::*;
use std::cell::{Cell, RefCell};
use std::fs;

use crate::dist::archive::{ArchiveEntry, write_zip};
use vibe_publish::{
    BundleDistributionManifest, DistributionAsset, DistributionComponent,
    DistributionComponentName, DistributionSourceArchive, GithubGitObject,
};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const TREE: &str = "89abcdef0123456789abcdef0123456789abcdef";

struct MockHost {
    release: RefCell<Option<GithubRelease>>,
    assets: RefCell<Vec<GithubReleaseAsset>>,
    bytes: RefCell<BTreeMap<u64, Vec<u8>>>,
    moved_to: RefCell<Option<String>>,
    tag_kind: RefCell<String>,
    operations: RefCell<Vec<String>>,
    next_id: Cell<u64>,
}

impl MockHost {
    fn draft() -> Self {
        Self {
            release: RefCell::new(Some(release(true))),
            assets: RefCell::new(Vec::new()),
            bytes: RefCell::new(BTreeMap::new()),
            moved_to: RefCell::new(None),
            tag_kind: RefCell::new("commit".to_string()),
            operations: RefCell::new(Vec::new()),
            next_id: Cell::new(10),
        }
    }

    fn seed(&self, name: String, bytes: Vec<u8>) {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.assets.borrow_mut().push(GithubReleaseAsset {
            id,
            name,
            size: bytes.len() as u64,
            digest: Some(sha256_digest(&bytes)),
            browser_download_url: String::new(),
            content_type: String::new(),
        });
        self.bytes.borrow_mut().insert(id, bytes);
    }
}

impl ReleaseHost for MockHost {
    fn find_release(&self, _tag: &str) -> Result<Option<GithubRelease>> {
        self.operations.borrow_mut().push("find".to_string());
        Ok(self.release.borrow().clone())
    }

    fn get_tag_ref(&self, tag: &str) -> Result<GithubGitRef> {
        Ok(GithubGitRef {
            reference: format!("refs/tags/{tag}"),
            object: GithubGitObject {
                sha: self
                    .moved_to
                    .borrow()
                    .clone()
                    .unwrap_or_else(|| COMMIT.to_string()),
                kind: self.tag_kind.borrow().clone(),
                url: String::new(),
            },
        })
    }

    fn create_release(&self, request: &CreateGithubRelease) -> Result<GithubRelease> {
        self.operations.borrow_mut().push("create".to_string());
        let mut value = release(request.draft);
        value.id = 8;
        value.tag_name = request.tag_name.clone();
        value.target_commitish = request.target_commitish.clone();
        value.draft = request.draft;
        *self.release.borrow_mut() = Some(value.clone());
        Ok(value)
    }

    fn delete_release(&self, release_id: u64) -> Result<()> {
        self.operations
            .borrow_mut()
            .push(format!("delete:{release_id}"));
        *self.release.borrow_mut() = None;
        self.assets.borrow_mut().clear();
        self.bytes.borrow_mut().clear();
        Ok(())
    }

    fn force_move_or_create_tag(&self, tag: &str, source_commit: &str) -> Result<GithubGitRef> {
        self.operations.borrow_mut().push("tag".to_string());
        *self.moved_to.borrow_mut() = Some(source_commit.to_string());
        Ok(GithubGitRef {
            reference: format!("refs/tags/{tag}"),
            object: GithubGitObject {
                sha: source_commit.to_string(),
                kind: self.tag_kind.borrow().clone(),
                url: String::new(),
            },
        })
    }

    fn list_assets(&self, _release_id: u64) -> Result<Vec<GithubReleaseAsset>> {
        Ok(self.assets.borrow().clone())
    }

    fn download_asset(&self, asset_id: u64, expected_size: u64, max_size: u64) -> Result<Vec<u8>> {
        let bytes = self
            .bytes
            .borrow()
            .get(&asset_id)
            .cloned()
            .with_context(|| format!("mock asset {asset_id} is absent"))?;
        anyhow::ensure!(bytes.len() as u64 == expected_size);
        anyhow::ensure!(bytes.len() as u64 <= max_size);
        Ok(bytes)
    }

    fn cleanup_temporary_assets(&self, _release_id: u64, canonical_names: &[&str]) -> Result<()> {
        let stale_ids = self
            .assets
            .borrow()
            .iter()
            .filter(|asset| {
                asset.name.starts_with(".vibe-upload-")
                    && canonical_names
                        .iter()
                        .any(|name| asset.name.ends_with(&format!("-{name}")))
            })
            .map(|asset| asset.id)
            .collect::<Vec<_>>();
        self.assets
            .borrow_mut()
            .retain(|asset| !stale_ids.contains(&asset.id));
        for id in stale_ids {
            self.bytes.borrow_mut().remove(&id);
        }
        Ok(())
    }

    fn publish_asset(
        &self,
        _release_id: u64,
        name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<GithubReleaseAsset> {
        let old = self
            .assets
            .borrow()
            .iter()
            .filter(|asset| asset.name == name)
            .map(|asset| asset.id)
            .collect::<Vec<_>>();
        for id in old {
            self.assets.borrow_mut().retain(|asset| asset.id != id);
            self.bytes.borrow_mut().remove(&id);
        }
        self.seed(name.to_string(), bytes);
        let mut asset = self.assets.borrow().last().unwrap().clone();
        asset.content_type = content_type.to_string();
        Ok(asset)
    }
}

fn release(draft: bool) -> GithubRelease {
    GithubRelease {
        id: 7,
        tag_name: "v1.0.0".to_string(),
        target_commitish: COMMIT.to_string(),
        name: Some("vibevm 1.0.0".to_string()),
        body: None,
        draft,
        prerelease: false,
        html_url: String::new(),
        upload_url: String::new(),
    }
}

fn platform(target: &str) -> (PlatformDistributionFragment, Vec<u8>, Vec<u8>, Vec<u8>) {
    let version = Version::parse("1.0.0").unwrap();
    let vibe = format!("vibe:{target}").into_bytes();
    let index = format!("vibe-index:{target}").into_bytes();
    let source_zip = write_zip(&[ArchiveEntry {
        name: "Cargo.toml".to_string(),
        bytes: b"[workspace]\n".to_vec(),
        mode: 0o644,
    }])
    .unwrap();
    let windows = target == "x86_64-pc-windows-msvc";
    let vibe_path = if windows { "vibe.exe" } else { "vibe" };
    let index_path = if windows {
        "vibe-index.exe"
    } else {
        "vibe-index"
    };
    let manifest = BundleDistributionManifest::new(
        version.clone(),
        COMMIT,
        target,
        vec![
            DistributionComponent {
                name: DistributionComponentName::Vibe,
                path: vibe_path.to_string(),
                size: vibe.len() as u64,
                digest: sha256_digest(&vibe),
            },
            DistributionComponent {
                name: DistributionComponentName::VibeIndex,
                path: index_path.to_string(),
                size: index.len() as u64,
                digest: sha256_digest(&index),
            },
        ],
        DistributionSourceArchive {
            path: DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.to_string(),
            size: source_zip.len() as u64,
            digest: sha256_digest(&source_zip),
            tree_oid: TREE.to_string(),
        },
    )
    .unwrap();
    let manifest_bytes = manifest.to_json_bytes().unwrap();
    let bundle = write_zip(&[
        ArchiveEntry {
            name: vibe_path.to_string(),
            bytes: vibe.clone(),
            mode: 0o755,
        },
        ArchiveEntry {
            name: index_path.to_string(),
            bytes: index,
            mode: 0o755,
        },
        ArchiveEntry {
            name: DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.to_string(),
            bytes: source_zip,
            mode: 0o644,
        },
        ArchiveEntry {
            name: DISTRIBUTION_MANIFEST_FILENAME.to_string(),
            bytes: manifest_bytes,
            mode: 0o644,
        },
        ArchiveEntry {
            name: "LICENSE.md".to_string(),
            bytes: b"license".to_vec(),
            mode: 0o644,
        },
        ArchiveEntry {
            name: "README.md".to_string(),
            bytes: b"readme".to_vec(),
            mode: 0o644,
        },
    ])
    .unwrap();
    let fragment = PlatformDistributionFragment::new(
        DistributionAsset {
            name: bundle_asset_name(&version, target),
            size: bundle.len() as u64,
            digest: sha256_digest(&bundle),
        },
        DistributionAsset {
            name: bootstrap_asset_name(target),
            size: vibe.len() as u64,
            digest: sha256_digest(&vibe),
        },
        manifest,
    )
    .unwrap();
    let fragment_bytes = fragment.to_json_bytes().unwrap();
    (fragment, bundle, vibe, fragment_bytes)
}

#[test]
fn expected_remote_set_is_three_assets_per_target() {
    let version = Version::parse("1.0.0").unwrap();
    let assets = SUPPORTED_DISTRIBUTION_TARGETS
        .iter()
        .flat_map(|target| {
            [
                bundle_asset_name(&version, target),
                bootstrap_asset_name(target),
                fragment_asset_name(&version, target),
            ]
        })
        .enumerate()
        .map(|(index, name)| {
            (
                name.clone(),
                GithubReleaseAsset {
                    id: index as u64,
                    name,
                    size: 1,
                    digest: None,
                    browser_download_url: String::new(),
                    content_type: String::new(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    validate_remote_asset_set(&assets, &version).unwrap();
    assert_eq!(assets.len(), 12);
}

#[test]
fn remote_set_rejects_missing_and_unknown_assets() {
    let version = Version::parse("1.0.0").unwrap();
    let mut assets = BTreeMap::new();
    assets.insert(
        "surprise.zip".to_string(),
        GithubReleaseAsset {
            id: 1,
            name: "surprise.zip".to_string(),
            size: 1,
            digest: None,
            browser_download_url: String::new(),
            content_type: String::new(),
        },
    );
    let message = validate_remote_asset_set(&assets, &version)
        .expect_err("invalid set")
        .to_string();
    assert!(message.contains("missing"));
    assert!(message.contains("surprise.zip"));
}

#[test]
fn oversized_remote_metadata_is_rejected_before_download() {
    let declared = DistributionAsset {
        name: "bundle.zip".to_string(),
        size: DISTRIBUTION_BUNDLE_MAX_BYTES + 1,
        digest: sha256_digest(b"irrelevant"),
    };
    let remote = GithubReleaseAsset {
        id: 1,
        name: declared.name.clone(),
        size: declared.size,
        digest: Some(declared.digest.clone()),
        browser_download_url: String::new(),
        content_type: String::new(),
    };
    assert!(validate_remote_metadata(&remote, &declared, DISTRIBUTION_BUNDLE_MAX_BYTES).is_err());
}

#[test]
fn local_asset_reader_bounds_metadata_before_allocating() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("asset");
    fs::write(&path, b"12345").unwrap();
    assert!(read_bounded_regular_file(&path, Some(4), 10, "test asset").is_err());
    assert!(read_bounded_regular_file(&path, None, 4, "test asset").is_err());
    assert_eq!(
        read_bounded_regular_file(&path, Some(5), 5, "test asset").unwrap(),
        b"12345"
    );
    assert!(read_bounded_regular_file(temp.path(), None, 10, "test asset").is_err());
}

#[test]
fn prepare_deletes_any_existing_release_moves_the_tag_and_creates_a_fresh_draft() {
    let host = MockHost::draft();
    host.release.borrow_mut().as_mut().unwrap().draft = false;
    host.seed("old.zip".to_string(), b"old".to_vec());
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };
    let result = prepare_with(&host, &Version::parse("1.0.0").unwrap(), &identity).unwrap();
    assert!(result.draft);
    assert_eq!(result.id, 8);
    assert_eq!(host.moved_to.borrow().as_deref(), Some(COMMIT));
    assert!(host.assets.borrow().is_empty());
    assert_eq!(
        host.operations.borrow().as_slice(),
        ["find", "delete:7", "tag", "create"]
    );
}

#[test]
fn prepare_without_a_release_sets_the_tag_before_creating_the_draft() {
    let host = MockHost::draft();
    *host.release.borrow_mut() = None;
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };

    let result = prepare_with(&host, &Version::parse("1.0.0").unwrap(), &identity).unwrap();

    assert!(result.draft);
    assert_eq!(result.id, 8);
    assert_eq!(host.moved_to.borrow().as_deref(), Some(COMMIT));
    assert_eq!(
        host.operations.borrow().as_slice(),
        ["find", "tag", "create"]
    );
}

#[test]
fn prepare_refuses_a_tag_update_response_with_wrong_provenance() {
    let host = MockHost::draft();
    *host.release.borrow_mut() = None;
    *host.tag_kind.borrow_mut() = "tag".to_string();
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };
    let error = prepare_with(&host, &Version::parse("1.0.0").unwrap(), &identity)
        .expect_err("an indirect or wrong tag object must be refused")
        .to_string();
    assert!(error.contains("tag provenance mismatch"));
    assert!(host.release.borrow().is_none());
}

#[test]
fn tag_ref_validation_rejects_name_and_sha_drift() {
    for (reference, sha) in [("refs/tags/v2.0.0", COMMIT), ("refs/tags/v1.0.0", TREE)] {
        let invalid = GithubGitRef {
            reference: reference.to_string(),
            object: GithubGitObject {
                sha: sha.to_string(),
                kind: "commit".to_string(),
                url: String::new(),
            },
        };
        assert!(validate_tag_ref(&invalid, "v1.0.0", COMMIT).is_err());
    }
}

#[test]
fn upload_replaces_exactly_bundle_bootstrap_and_fragment() {
    let host = MockHost::draft();
    let (fragment, asset_bytes, bootstrap_bytes, fragment_bytes) =
        platform(SUPPORTED_DISTRIBUTION_TARGETS[0]);
    let mut local = LocalPlatform {
        fragment: fragment.clone(),
        fragment_name: fragment_asset_name(
            &Version::parse(&fragment.version).unwrap(),
            &fragment.target,
        ),
        asset_bytes,
        bootstrap_bytes,
        fragment_bytes,
    };
    upload_with(&host, &mut local).unwrap();
    let names = host
        .assets
        .borrow()
        .iter()
        .map(|asset| asset.name.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        names,
        BTreeSet::from([
            fragment.asset.name,
            fragment.bootstrap.name,
            local.fragment_name,
        ])
    );
}

#[test]
fn finalize_downloads_and_verifies_all_twelve_target_assets() {
    let host = MockHost::draft();
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let (fragment, bundle, bootstrap, fragment_bytes) = platform(target);
        host.seed(fragment.asset.name.clone(), bundle);
        host.seed(fragment.bootstrap.name.clone(), bootstrap);
        host.seed(
            fragment_asset_name(&Version::parse("1.0.0").unwrap(), target),
            fragment_bytes,
        );
    }
    for name in [
        DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME,
        DISTRIBUTION_BASH_INSTALLER_FILENAME,
        DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
    ] {
        host.seed(
            format!(".vibe-upload-stale-0123456789ab-{name}"),
            b"interrupted temporary upload".to_vec(),
        );
    }
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };
    let verified = finalize_with(&host, &Version::parse("1.0.0").unwrap(), &identity).unwrap();
    assert_eq!(verified.release_id, 7);
    assert_eq!(verified.asset_ids.len(), 12);
    assert!(
        host.assets
            .borrow()
            .iter()
            .all(|asset| !asset.name.starts_with(".vibe-upload-"))
    );
    assert_eq!(verified.aggregate.platforms.len(), 4);
    let json = verified.aggregate.to_json_bytes().unwrap();
    let text = String::from_utf8(json).unwrap();
    assert!(text.contains("\"bootstrap\""));
    assert!(text.contains("vibe-bootstrap-aarch64-apple-darwin"));
}

#[test]
fn final_publication_refuses_a_concurrently_reprepared_release_id() {
    let host = MockHost::draft();
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let (fragment, bundle, bootstrap, fragment_bytes) = platform(target);
        host.seed(fragment.asset.name.clone(), bundle);
        host.seed(fragment.bootstrap.name.clone(), bootstrap);
        host.seed(
            fragment_asset_name(&Version::parse("1.0.0").unwrap(), target),
            fragment_bytes,
        );
    }
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };
    let verified = finalize_with(&host, &Version::parse("1.0.0").unwrap(), &identity).unwrap();
    let mut replacement = release(true);
    replacement.id = 99;
    *host.release.borrow_mut() = Some(replacement);

    let error = ensure_release_ready_for_publish(&host, &verified)
        .expect_err("a different release ID must never be published")
        .to_string();
    assert!(error.contains("identity changed"));
    assert_eq!(host.assets.borrow().len(), 12, "the guard is read-only");
}

#[test]
fn final_publication_refuses_a_changed_verified_asset_id() {
    let host = MockHost::draft();
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let (fragment, bundle, bootstrap, fragment_bytes) = platform(target);
        host.seed(fragment.asset.name.clone(), bundle);
        host.seed(fragment.bootstrap.name.clone(), bootstrap);
        host.seed(
            fragment_asset_name(&Version::parse("1.0.0").unwrap(), target),
            fragment_bytes,
        );
    }
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };
    let verified = finalize_with(&host, &Version::parse("1.0.0").unwrap(), &identity).unwrap();
    host.assets.borrow_mut()[0].id += 1_000;
    let error = ensure_release_ready_for_publish(&host, &verified)
        .expect_err("asset identity drift must refuse publication")
        .to_string();
    assert!(error.contains("asset set changed"));
}

#[test]
fn final_publication_refuses_a_moved_release_tag() {
    let host = MockHost::draft();
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let (fragment, bundle, bootstrap, fragment_bytes) = platform(target);
        host.seed(fragment.asset.name.clone(), bundle);
        host.seed(fragment.bootstrap.name.clone(), bootstrap);
        host.seed(
            fragment_asset_name(&Version::parse("1.0.0").unwrap(), target),
            fragment_bytes,
        );
    }
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };
    let verified = finalize_with(&host, &Version::parse("1.0.0").unwrap(), &identity).unwrap();
    *host.moved_to.borrow_mut() = Some("f".repeat(40));

    let error = ensure_release_ready_for_publish(&host, &verified)
        .expect_err("a moved tag must never be published")
        .to_string();
    assert!(error.contains("tag provenance mismatch"));
}
