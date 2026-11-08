//! The release lane's per-release reads: which manifest describes a
//! release, where a remapped release base points, and what the offline
//! posture refuses (`PROP-019#release-sources`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#release-sources");

use std::path::Path;

use anyhow::Result;
use vibe_core::remap::ByteLocation;
use vibe_publish::release_manifest::{
    DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME, PlatformDistributionFragment,
};
use vibe_publish::token::Token;

use super::super::model::InstallRecord;
use super::archive;
use super::releases::{self, ReleaseDir};
use super::report::{ActivationReport, emit_outcome};
use super::{RemoteContext, current_target, fetch_aggregate, select_platform};

/// The release base or `DISTRIBUTIONS.json` location to actually read
/// (`PROP-066#locator`): the Locator's winning candidate for the declared
/// one — a `path` target as a `file://` URL (`PROP-066#locations`) — or a
/// refusal before any connection. The second half of the answer is
/// whether a row produced it — `validated_release_base` keeps its strict
/// GitHub check only for a base nobody remapped.
pub(super) fn located_distribution(declared: &str) -> Result<(String, bool)> {
    let locator = vibe_core::remap::active();
    let artifact = vibe_core::remap::Artifact::new(vibe_core::remap::ArtifactKind::Distribution)
        .with_url(declared);
    let candidate = locator.choose_candidate(&artifact)?;
    let chosen = ByteLocation::of_target(&candidate.target)?.to_url();
    let remapped = candidate.row.is_some() && chosen != declared;
    Ok((chosen, remapped))
}

/// One release's manifest: the published aggregate, or — for a bare
/// platform zip in a local drop — the fragment its own embedded manifest
/// describes.
pub(super) enum ReleaseManifest {
    Aggregate(vibe_publish::release_manifest::AggregateDistributionManifest),
    Bare(Box<PlatformDistributionFragment>),
}

impl ReleaseManifest {
    pub(super) fn platform(
        &self,
        version: &str,
        target: &str,
    ) -> Result<&PlatformDistributionFragment> {
        match self {
            ReleaseManifest::Aggregate(aggregate) => select_platform(aggregate, version, target),
            ReleaseManifest::Bare(platform) => Ok(platform.as_ref()),
        }
    }
}

/// Read the manifest of one release. A local release directory without
/// `DISTRIBUTIONS.json` may still hold the bare platform zip (in it, or at
/// the drop root beside it): its embedded `DISTRIBUTION.json` describes it,
/// and the directory the zip is in becomes the release directory.
pub(super) fn read_release(
    remote: &RemoteContext<'_>,
    dir: &ReleaseDir,
    version: &semver::Version,
    token: Option<&Token>,
) -> Result<(ReleaseManifest, ReleaseDir)> {
    let target = current_target()?;
    if let Some(local) = dir.local()
        && !local
            .join(DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME)
            .is_file()
        && let Some(zip) = releases::bare_bundle(&local, &version.to_string(), target)
    {
        let platform = releases::synthesize_platform(&zip, &version.to_string(), target)?;
        remote.ctx.hint(&format!(
            "`{}` has no {DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME} beside it: verified against \
             its own embedded manifest only",
            zip.display()
        ));
        let parent = zip.parent().map(Path::to_path_buf).unwrap_or(local);
        return Ok((
            ReleaseManifest::Bare(Box::new(platform)),
            ReleaseDir::Base(ByteLocation::Local(parent).to_url()),
        ));
    }
    let url = dir.asset(DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME)?;
    let aggregate = fetch_aggregate(remote, &url, token)?;
    Ok((ReleaseManifest::Aggregate(aggregate), dir.clone()))
}

/// The newest release is older than the installed one: nothing is fetched and
/// nothing moves. Human output is the one line that explains it; a `--json`
/// caller still gets the ordinary envelope, reporting the running instance as
/// the reused, active one — because that is exactly what it now is.
pub(super) fn report_withdrawn_release(
    remote: &RemoteContext<'_>,
    record: &InstallRecord,
    installed: &semver::Version,
    newest: &semver::Version,
) -> Result<()> {
    remote.ctx.summary(&format!(
        "newest release is {newest}, older than the installed {installed} — staying on \
         {installed}; `vibe self install {newest}` goes back deliberately"
    ));
    if !remote.ctx.is_json() {
        return Ok(());
    }
    let outcome = archive::InstallOutcome {
        record: record.clone(),
        home: remote
            .store
            .instance_dir(&record.version_id(), record.instance),
        reused: true,
    };
    emit_outcome(
        remote.ctx,
        remote.command,
        &outcome,
        &ActivationReport {
            path_on_current_process: super::super::path_has_dir(
                remote.env.path_var.as_deref(),
                &remote.store.shim_dir(),
            ),
            durable_path_changed: false,
            advisory_home_warning: None,
        },
    )
}

/// The offline posture refuses a remote release request; a release drop on
/// this machine needs no network and is read under it (`PROP-019##CMD-OFFLINE`,
/// `PROP-066#locations`).
pub(super) fn refuse_remote_offline(remote: &RemoteContext<'_>, url: &str) -> Result<()> {
    if matches!(ByteLocation::parse(url), Ok(ByteLocation::Local(_))) {
        return Ok(());
    }
    remote.env.refuse_offline(remote.command, url)?;
    Ok(())
}
