//! A release bundle unpacked by hand (PROP-019 `#provenance`).
//!
//! In a closed network the release ZIP is often unpacked into a directory —
//! `C:\Work\opt\bin` — and the `vibe` in it is run directly. That directory
//! holds the bundle's `DISTRIBUTION.json` beside the executables, and the
//! manifest names exactly what belongs to the release: every component and
//! the packaged source. Importing from it installs those members and nothing
//! else — the directory may be shared with foreign tools — and verifies each
//! one against the manifest first, refusing by name the first that differs.
//! The instance it publishes has the layout and the record of one installed
//! from the release ZIP, so integrity checks, `doctor` and later reuse treat
//! the two identically.

use super::*;

use crate::commands::vvm::store::open_regular_no_follow;

/// What one import from an unpacked release is asked to admit.
pub(in crate::commands::vvm) struct UnpackedRequest<'a> {
    /// The directory holding `DISTRIBUTION.json` and the members.
    pub dir: &'a Path,
    /// The executable being imported — it must be the manifest's `vibe`.
    pub executable: &'a Path,
    pub tag: &'a str,
    pub commit: Option<&'a str>,
}

/// Verify the release members beside `request.executable` and publish them
/// as one immutable instance, or reuse the instance that already holds
/// exactly this release.
pub(in crate::commands::vvm) fn import_unpacked(
    store: &VersionStore,
    request: &UnpackedRequest<'_>,
    progress: &Progress,
) -> Result<InstallOutcome> {
    let verify = progress.task("Verifying unpacked release");
    verify.detail(request.dir.display().to_string());
    let (manifest, manifest_bytes) = match verify_unpacked(request) {
        Ok(verified) => verified,
        Err(error) => {
            verify.fail("the unpacked release does not verify");
            return Err(error);
        }
    };
    verify.finish();

    let id = VersionId::new(Kind::Tag, manifest.version.clone());
    let root = format!("{:x}", Sha256::digest(&manifest_bytes));
    let reused = store
        .instances_of(&id)?
        .into_iter()
        .filter(|record| record.distribution_manifest_sha256.as_deref() == Some(root.as_str()))
        .max_by_key(|record| record.instance)
        .filter(|record| verified_existing_instance(store, record, &manifest, progress));
    if let Some(record) = reused {
        let home = store.instance_dir(&id, record.instance);
        return Ok(InstallOutcome {
            record,
            home,
            reused: true,
        });
    }

    let instance = store.alloc_instance()?;
    let staging = store
        .version_id_dir(&id)
        .join(format!(".import-staging-{instance}"));
    let staging_parent = staging.parent().context("import staging has no parent")?;
    store.guard_mutation_path(staging_parent)?;
    fs::create_dir_all(staging_parent)?;
    store.guard_mutation_tree(&staging)?;
    fs::create_dir(&staging)
        .with_context(|| format!("creating import staging `{}`", staging.display()))?;
    let mut cleanup = StagingCleanup(Some(staging.clone()));
    stage_members(request.dir, &manifest, &manifest_bytes, &staging, progress)?;
    let home = placer::publish_staged_instance(store, &id, instance, &staging)?;
    cleanup.0 = None;

    let source_path = store.instance_source_dir(&id, instance);
    let source_path = source_path
        .canonicalize()
        .unwrap_or(source_path)
        .display()
        .to_string();
    let vibe = vibe_component(&manifest)?;
    let record = InstallRecord {
        kind: Kind::Tag,
        id: manifest.version.clone(),
        instance,
        commit: manifest.source_commit.clone(),
        toolchain: format!("prebuilt:{}", manifest.target),
        profile: Profile::Release,
        installed_at: chrono::Utc::now().to_rfc3339(),
        origin: Origin::Binary,
        source_path: Some(source_path),
        payload_sha256: Some(vibe.digest.trim_start_matches("sha256:").to_string()),
        distribution_manifest_sha256: Some(root),
    };
    store.record_install(record.clone())?;
    Ok(InstallOutcome {
        record,
        home,
        reused: false,
    })
}

/// Read the manifest and check every member it names against the bytes
/// in the directory, before anything is allocated or copied.
fn verify_unpacked(request: &UnpackedRequest<'_>) -> Result<(BundleDistributionManifest, Vec<u8>)> {
    let manifest_path = request.dir.join(DISTRIBUTION_MANIFEST_FILENAME);
    let bytes = read_regular_bounded(&manifest_path, DISTRIBUTION_MANIFEST_MAX_BYTES)
        .with_context(|| refusal(&manifest_path, "cannot be read as a bounded regular file"))?;
    let manifest = BundleDistributionManifest::from_json_slice(&bytes)
        .with_context(|| refusal(&manifest_path, "is not a release distribution manifest"))?;
    validate_platform_paths(&manifest)
        .with_context(|| refusal(&manifest_path, "names non-canonical members"))?;
    let target = super::super::current_target()?;
    if manifest.target != target {
        bail!(refusal(
            &manifest_path,
            &format!(
                "describes target `{}`, this vibe runs on `{target}`",
                manifest.target
            ),
        ));
    }
    let tag = request.tag.trim();
    if manifest.version != tag {
        bail!(refusal(
            &manifest_path,
            &format!(
                "describes version {}, the import asked for {tag}",
                manifest.version
            ),
        ));
    }
    if let Some(commit) = request.commit
        && commit != manifest.source_commit
    {
        bail!(refusal(
            &manifest_path,
            &format!(
                "was built from commit {}, `--commit` says {commit}",
                manifest.source_commit
            ),
        ));
    }
    let vibe = vibe_component(&manifest)?;
    check_member(
        request.executable,
        vibe.size,
        &vibe.digest,
        DISTRIBUTION_COMPONENT_MAX_BYTES,
        &manifest_path,
    )?;
    for component in &manifest.components {
        check_member(
            &request.dir.join(&component.path),
            component.size,
            &component.digest,
            DISTRIBUTION_COMPONENT_MAX_BYTES,
            &manifest_path,
        )?;
    }
    check_member(
        &request.dir.join(&manifest.source_archive.path),
        manifest.source_archive.size,
        &manifest.source_archive.digest,
        DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES,
        &manifest_path,
    )?;
    Ok((manifest, bytes))
}

fn vibe_component(manifest: &BundleDistributionManifest) -> Result<&DistributionComponent> {
    manifest
        .components
        .iter()
        .find(|component| component.name == DistributionComponentName::Vibe)
        .context("the release manifest declares no `vibe` component")
}

/// The refusal sentence every failed check of the unpacked release shares.
fn refusal(manifest: &Path, why: &str) -> String {
    format!(
        "the unpacked release is refused: `{}` {why} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#provenance; \
          fix: unpack the release archive again into the directory, or remove \
          the stale `{DISTRIBUTION_MANIFEST_FILENAME}` to import the executables alone)",
        manifest.display()
    )
}

/// One member must be a regular file of exactly the declared bytes.
fn check_member(path: &Path, size: u64, digest: &str, maximum: u64, manifest: &Path) -> Result<()> {
    let (actual_size, actual_digest) = hash_member(path, maximum).with_context(|| {
        refusal(
            manifest,
            &format!("names `{}`, which cannot be read", path.display()),
        )
    })?;
    if actual_size != size || actual_digest != digest {
        bail!(refusal(
            manifest,
            &format!(
                "declares `{}` as {size} bytes/{digest}, the file is {actual_size} bytes/{actual_digest}",
                path.display()
            ),
        ));
    }
    Ok(())
}

fn hash_member(path: &Path, maximum: u64) -> Result<(u64, String)> {
    let (mut file, _) = open_regular_no_follow(path)?;
    copy_and_hash_bounded(&mut file, &mut std::io::sink(), maximum)
}

/// Copy each member into `staging` in the bundle layout, verifying the
/// bytes written — not only the bytes checked earlier — and extract the
/// packaged source.
fn stage_members(
    dir: &Path,
    manifest: &BundleDistributionManifest,
    manifest_bytes: &[u8],
    staging: &Path,
    progress: &Progress,
) -> Result<()> {
    let manifest_path = dir.join(DISTRIBUTION_MANIFEST_FILENAME);
    let bin_dir = staging.join("bin");
    fs::create_dir_all(&bin_dir)?;
    for component in &manifest.components {
        let task = progress.task(format!("Copying {}", component.name.as_str()));
        task.set_progress(0, Some(component.size), "bytes");
        let destination = match component.name {
            DistributionComponentName::Vibe => bin_dir.join(BINARY_NAME),
            DistributionComponentName::VibeIndex => bin_dir.join(INDEX_BINARY_NAME),
            DistributionComponentName::VibeAppProxy => bin_dir.join(APP_PROXY_BINARY_NAME),
        };
        if let Err(error) = copy_member(
            &dir.join(&component.path),
            component.size,
            &component.digest,
            DISTRIBUTION_COMPONENT_MAX_BYTES,
            &destination,
            &manifest_path,
        ) {
            task.fail("member copy failed");
            return Err(error);
        }
        make_executable(&destination)?;
        task.set_progress(component.size, Some(component.size), "bytes");
        task.finish();
    }

    let source_task = progress.task("Copying packaged source");
    source_task.set_progress(0, Some(manifest.source_archive.size), "bytes");
    let source_zip = staging.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME);
    let source_dir = staging.join("source");
    let copied = copy_member(
        &dir.join(&manifest.source_archive.path),
        manifest.source_archive.size,
        &manifest.source_archive.digest,
        DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES,
        &source_zip,
        &manifest_path,
    )
    .and_then(|()| extract_source_archive(&source_zip, &source_dir));
    if let Err(error) = copied {
        source_task.fail("packaged source could not be installed");
        return Err(error);
    }
    source_task.set_progress(
        manifest.source_archive.size,
        Some(manifest.source_archive.size),
        "bytes",
    );
    source_task.finish();
    if !source_dir.join("Cargo.toml").is_file()
        || !source_dir.join("crates").join("vibe-cli").is_dir()
    {
        bail!(refusal(
            &manifest_path,
            "names a source archive that does not contain the vibevm repository root",
        ));
    }
    fs::write(staging.join(DISTRIBUTION_MANIFEST_FILENAME), manifest_bytes)?;
    Ok(())
}

fn copy_member(
    source: &Path,
    size: u64,
    digest: &str,
    maximum: u64,
    destination: &Path,
    manifest: &Path,
) -> Result<()> {
    let (mut file, _) = open_regular_no_follow(source)
        .with_context(|| format!("reading release member `{}`", source.display()))?;
    let mut out = fs::File::create(destination)
        .with_context(|| format!("writing `{}`", destination.display()))?;
    let (actual_size, actual_digest) = copy_and_hash_bounded(&mut file, &mut out, maximum)?;
    if actual_size != size || actual_digest != digest {
        bail!(refusal(
            manifest,
            &format!(
                "declares `{}` as {size} bytes/{digest}, it changed to {actual_size} bytes/{actual_digest} while it was copied",
                source.display()
            ),
        ));
    }
    Ok(())
}
