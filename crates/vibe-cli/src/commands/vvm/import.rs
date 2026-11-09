//! Local import of a ready-built `vibe` distribution into the immutable VVM
//! inventory. The path is local-only: no resolver, network, or signature
//! machinery participates.
//!
//! The import takes the whole distribution beside the payload, never the
//! executable alone: a release unpacked by hand (its `DISTRIBUTION.json`
//! beside the payload) is imported member by member, digest-verified, as
//! the release ZIP would install it; a bare payload brings the companion
//! executables found beside it under their canonical names, and each one
//! missing is reported with what does not work without it.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::progress::{Progress, ProgressTask};
use vibe_publish::release_manifest::DISTRIBUTION_MANIFEST_FILENAME;

use super::bundle;
use super::model::{InstallRecord, Kind, Origin, Profile, VersionId};
use super::placer;
use super::store::{
    APP_PROXY_BINARY_NAME, BINARY_NAME, INDEX_BINARY_NAME, VersionStore, open_regular_no_follow,
};
use crate::output;

/// The provenance of a bare payload with no explicit `--commit`.
const UNKNOWN_COMMIT: &str = "unknown";

pub(crate) struct ImportRequest<'a> {
    pub executable: &'a Path,
    pub tag: &'a str,
    pub commit: Option<&'a str>,
    pub profile: Profile,
    pub replace_candidate: bool,
    pub now: &'a str,
}

pub(crate) struct ImportOutcome {
    pub record: InstallRecord,
    pub home: std::path::PathBuf,
    pub reused: bool,
}

/// What the payload-verification stage observed, including the commit the
/// record will carry.
struct ObservedPayload {
    id: VersionId,
    dist: Vec<(PathBuf, String)>,
    manifest: placer::Manifest,
    digest: String,
    commit: String,
    existing: Vec<InstallRecord>,
    reused: Option<InstallRecord>,
}

pub(crate) fn perform_import(
    ctx: &output::Context,
    store: &VersionStore,
    req: &ImportRequest<'_>,
) -> Result<ImportOutcome> {
    let progress = ctx.progress();
    if let Some(dir) = unpacked_release_dir(req.executable) {
        return import_unpacked_release(ctx, store, req, &dir, &progress);
    }
    let observed = observed_stage(&progress, "Verifying local import payload", |task| {
        task.detail(format!("payload: {}", req.executable.display()));
        let tag = parse_tag(req.tag)?;
        ensure_payload_file(req.executable)?;
        let id = VersionId::new(Kind::Tag, tag);
        let (dist, missing) = loose_distribution(req.executable);
        for companion in &missing {
            task.detail(format!("missing companion: {}", companion.name));
        }
        report_missing_companions(ctx, req.executable, &missing);
        let manifest = placer::manifest_for(&dist)?;
        let digest = manifest
            .content_hash_for(&format!("bin/{BINARY_NAME}"))
            .context("import payload manifest omitted its required SHA-256")?
            .to_string();
        let commit = req.commit.unwrap_or(UNKNOWN_COMMIT).to_owned();
        task.detail(format!("commit: {commit}"));
        let mut existing = store.instances_of(&id)?;
        existing.sort_by_key(|record| record.instance);
        let reused = existing
            .iter()
            .rev()
            .find(|record| {
                record.payload_sha256.as_deref() == Some(digest.as_str())
                    && record.commit == commit
                    && record.profile == req.profile
                    && placer::read_manifest(&store.instance_dir(&id, record.instance))
                        .is_some_and(|previous| placer::matches(&manifest, &previous))
                    && placer::installed_files_match(store, record)
            })
            .cloned();
        task.detail(format!(
            "version: {id}; existing instances: {}",
            existing.len()
        ));
        Ok(ObservedPayload {
            id,
            dist,
            manifest,
            digest,
            commit,
            existing,
            reused,
        })
    })?;
    px_beside(req.executable, store, &progress)?;
    let ObservedPayload {
        id,
        dist,
        manifest,
        digest,
        commit,
        existing,
        reused,
    } = observed;
    // Kept as a CLI compatibility flag. Remote version labels are mutable;
    // every distinct payload is now admitted as a fresh immutable local #N.
    let _ = req.replace_candidate;

    if let Some(record) = reused {
        let instance_dir = store.instance_dir(&id, record.instance);
        let copying = progress.task("Copying import payload");
        copying.detail(format!("reusing {}", record.selector()));
        copying.skip("matching verified payload already installed");
        let recording = progress.task("Recording imported instance");
        recording.skip("inventory unchanged");
        ctx.summary(&format!(
            "{} payload already imported — reused",
            record.selector()
        ));
        return Ok(ImportOutcome {
            record,
            home: instance_dir,
            reused: true,
        });
    }

    let instance = observed_stage(&progress, "Copying import payload", |task| {
        task.detail(format!("destination version: {id}"));
        let instance = store.alloc_instance()?;
        let previous = existing.last().and_then(|record| {
            let dir = store.instance_dir(&id, record.instance);
            placer::read_manifest(&dir).map(|manifest| (dir, manifest))
        });
        let previous_ref = previous
            .as_ref()
            .map(|(dir, manifest)| (dir.as_path(), manifest));
        placer::place(store, &id, instance, &dist, &manifest, previous_ref)?;
        task.detail(format!("instance: {id}#{instance}"));
        Ok(instance)
    })?;

    let record = InstallRecord {
        kind: Kind::Tag,
        id: id.id.clone(),
        instance,
        commit,
        toolchain: "prebuilt".to_string(),
        profile: req.profile,
        installed_at: req.now.to_string(),
        origin: Origin::Binary,
        source_path: None,
        payload_sha256: Some(digest),
        distribution_manifest_sha256: None,
    };
    observed_stage(&progress, "Recording imported instance", |task| {
        task.detail(format!("selector: {}", record.selector()));
        store.record_install(record.clone())?;
        Ok(())
    })?;

    let instance_dir = store.instance_dir(&id, instance);
    ctx.created(&instance_dir.display().to_string());
    ctx.summary(&format!("imported {id}#{instance}"));
    Ok(ImportOutcome {
        record,
        home: instance_dir,
        reused: false,
    })
}

fn observed_stage<T>(
    progress: &Progress,
    label: &str,
    run: impl FnOnce(&ProgressTask) -> Result<T>,
) -> Result<T> {
    let task = progress.task(label);
    match run(&task) {
        Ok(value) => {
            task.finish();
            Ok(value)
        }
        Err(error) => {
            task.fail(error.to_string());
            Err(error)
        }
    }
}

fn parse_tag(raw: &str) -> Result<String> {
    let tag = raw.trim();
    if tag.starts_with('v') || semver::Version::parse(tag).is_err() {
        bail!("invalid import tag `{raw}`; expected a semantic version such as `1.2.3`");
    }
    Ok(tag.to_string())
}

/// The directory of a release unpacked by hand: the payload's own
/// directory, when it holds the bundle's `DISTRIBUTION.json`. That file
/// makes the directory a release, and the import then takes exactly the
/// release's members — or refuses — instead of the executable alone.
fn unpacked_release_dir(executable: &Path) -> Option<PathBuf> {
    let dir = match executable.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
    std::fs::symlink_metadata(dir.join(DISTRIBUTION_MANIFEST_FILENAME))
        .is_ok()
        .then_some(dir)
}

fn import_unpacked_release(
    ctx: &output::Context,
    store: &VersionStore,
    req: &ImportRequest<'_>,
    dir: &Path,
    progress: &Progress,
) -> Result<ImportOutcome> {
    parse_tag(req.tag)?;
    ensure_payload_file(req.executable)?;
    let outcome = bundle::archive::import_unpacked(
        store,
        &bundle::archive::UnpackedRequest {
            dir,
            executable: req.executable,
            tag: req.tag,
            commit: req.commit,
        },
        progress,
    )?;
    px_beside(req.executable, store, progress)?;
    let selector = outcome.record.selector();
    if outcome.reused {
        ctx.summary(&format!("{selector} release already imported — reused"));
    } else {
        ctx.created(&outcome.home.display().to_string());
        ctx.summary(&format!(
            "imported {selector} from the unpacked release in {}",
            dir.display()
        ));
    }
    Ok(ImportOutcome {
        record: outcome.record,
        home: outcome.home,
        reused: outcome.reused,
    })
}

/// One companion executable a distribution carries beside `vibe`.
struct Companion {
    name: &'static str,
    /// What does not work in an instance imported without it.
    loss: &'static str,
}

const COMPANIONS: [Companion; 2] = [
    Companion {
        name: APP_PROXY_BINARY_NAME,
        loss: "applications cannot be installed (`vibe install -g` refuses with proxy_image_missing)",
    },
    Companion {
        name: INDEX_BINARY_NAME,
        loss: "the `vibe-index` command is unavailable",
    },
];

/// The distribution of a payload with no `DISTRIBUTION.json` beside it:
/// the payload as `bin/vibe`, and each known companion that sits beside it
/// under its canonical name. Nothing else in the directory is read — it
/// may be shared with foreign tools. The companions not found are returned.
fn loose_distribution(executable: &Path) -> (Vec<(PathBuf, String)>, Vec<&'static Companion>) {
    let mut dist = vec![(executable.to_path_buf(), format!("bin/{BINARY_NAME}"))];
    let mut missing = Vec::new();
    for companion in &COMPANIONS {
        let path = executable.with_file_name(companion.name);
        if path != executable && open_regular_no_follow(&path).is_ok() {
            dist.push((path, format!("bin/{}", companion.name)));
        } else {
            missing.push(companion);
        }
    }
    (dist, missing)
}

fn report_missing_companions(ctx: &output::Context, executable: &Path, missing: &[&Companion]) {
    let dir = executable
        .parent()
        .map(|dir| dir.display().to_string())
        .unwrap_or_default();
    for companion in missing {
        ctx.diagnostic(&format!(
            "`{}` is not beside the imported payload in `{dir}`, so in this instance {}; \
             put it beside the payload (or unpack the whole release there) and import again",
            companion.name, companion.loss
        ));
    }
}

/// px from beside the payload (PROP-019 `#companion-tools`).
fn px_beside(executable: &Path, store: &VersionStore, progress: &Progress) -> Result<()> {
    if let Some(dir) = executable
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
    {
        bundle::px_lane::px_beside(store, dir, progress)?;
    }
    Ok(())
}

fn ensure_payload_file(path: &Path) -> Result<()> {
    let (_, metadata) = open_regular_no_follow(path)
        .with_context(|| format!("reading import payload `{}`", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            bail!("import payload `{}` is not executable", path.display());
        }
    }
    #[cfg(not(unix))]
    let _ = metadata;
    Ok(())
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
