//! Local, five-target preparation from one pinned output capability.
//!
//! It may persist a machine report locally. It never contacts a release host
//! and unavailable native platforms remain unavailable in the final subject.

use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use vibe_publish::{
    DISTRIBUTION_BOOTSTRAP_MAX_BYTES, DISTRIBUTION_BUNDLE_MAX_BYTES,
    DISTRIBUTION_MANIFEST_MAX_BYTES, PlatformDistributionFragment, SUPPORTED_DISTRIBUTION_TARGETS,
    sha256_digest,
};
use vibe_safefs::Project;
use vibe_wire::generated::publication_subject::{
    PublicationAsset, PublicationSmoke, PublicationSmokeProbe, PublicationSmokeProbeStatus,
};

use crate::dist::build;
use crate::dist::capture::{PreparedSubjectCapture, capture_prepared_subject_in};

use super::report::{PreparedSmokeReport, prepare_report, summary_from_stored_report};
use super::report_store::{StoredSmokeReport, store_report};
use super::{host_identity, run_native_probe, stage_verified_bundle};

const REPORT_MAX_BYTES: usize = 256 * 1024;

#[allow(dead_code)]
pub(crate) struct PreparedLocalSmoke {
    pub(crate) capture: PreparedSubjectCapture,
    pub(crate) report: StoredSmokeReport,
    pub(crate) report_bytes: Vec<u8>,
}

#[derive(Debug, Default)]
pub(crate) struct SmokeSelection {
    skipped: BTreeSet<String>,
    reason: Option<String>,
}

impl SmokeSelection {
    /// Validate the entire skip request before opening any output path.
    pub(crate) fn new(targets: Vec<String>, reason: Option<String>) -> Result<Self> {
        if targets.is_empty() != reason.is_none() {
            bail!("--skip-smoke-target and --skip-reason must be supplied together");
        }
        if let Some(reason) = reason.as_deref()
            && (reason.is_empty()
                || reason.len() > 480
                || reason.trim() != reason
                || reason.chars().any(char::is_control))
        {
            bail!("--skip-reason must be nonblank, unpadded and at most 480 bytes");
        }
        let mut skipped = BTreeSet::new();
        for target in targets {
            if !SUPPORTED_DISTRIBUTION_TARGETS.contains(&target.as_str()) {
                bail!("--skip-smoke-target `{target}` is unsupported");
            }
            if !skipped.insert(target.clone()) {
                bail!("--skip-smoke-target `{target}` is duplicated");
            }
        }
        Ok(Self { skipped, reason })
    }

    fn skips(&self, target: &str) -> bool {
        self.skipped.contains(target)
    }
}

fn provisional_smoke() -> PublicationSmoke {
    let host = host_identity();
    PublicationSmoke {
        policy_id: "distribution-smoke/e1".into(),
        policy_epoch: 1,
        probes: SUPPORTED_DISTRIBUTION_TARGETS
            .into_iter()
            .map(|target| PublicationSmokeProbe {
                probe_id: "launch".into(),
                target: target.into(),
                status: PublicationSmokeProbeStatus::Unavailable,
                tested_sha256: None,
                runner_ref: None,
                evidence_ref: Some(format!("reason=preparation-pending;host={host}")),
            })
            .collect(),
    }
}

fn read_subject_asset(output: &Project, asset: &PublicationAsset, cap: u64) -> Result<Vec<u8>> {
    let cap = usize::try_from(cap).context("distribution asset cap exceeds this host's usize")?;
    let bytes = output
        .read_file_bounded(&asset.name, cap)
        .with_context(|| format!("reading exact distribution asset `{}`", asset.name))?
        .with_context(|| format!("exact distribution asset `{}` disappeared", asset.name))?;
    if bytes.len().to_string() != asset.size_bytes || sha256_digest(&bytes) != asset.sha256 {
        bail!(
            "distribution asset `{}` changed after provisional capture",
            asset.name
        );
    }
    Ok(bytes)
}

/// Production-local entry point. `out_dir` is opened once; both captures,
/// exact asset reads and report publication stay below that held capability.
#[allow(dead_code)]
pub(crate) fn prepare_local_subject(
    repo_root: &Path,
    out_dir: &Path,
) -> Result<PreparedLocalSmoke> {
    prepare_local_subject_selected(repo_root, out_dir, &SmokeSelection::default())
}

pub(crate) fn prepare_local_subject_selected(
    repo_root: &Path,
    out_dir: &Path,
    selection: &SmokeSelection,
) -> Result<PreparedLocalSmoke> {
    let output_root = build::absolute_out_dir(repo_root, out_dir);
    let output = Project::open(&output_root)
        .with_context(|| format!("pinning distribution output `{}`", output_root.display()))?;
    prepare_local_with_selection(repo_root, &output, selection, || Ok(()))
}

/// Reconstruct a subject from an existing content-addressed report. This is
/// observation only: no probes, report publication or remote calls occur.
pub(crate) fn load_local_subject(
    repo_root: &Path,
    out_dir: &Path,
    report_relative_path: &str,
) -> Result<PreparedLocalSmoke> {
    if !report_relative_path.starts_with("smoke/e1/") || !report_relative_path.ends_with(".json") {
        bail!("smoke report must be a relative smoke/e1/<digest>.json path");
    }
    let output_root = build::absolute_out_dir(repo_root, out_dir);
    let output = Project::open(&output_root)
        .with_context(|| format!("pinning distribution output `{}`", output_root.display()))?;
    let provisional = capture_prepared_subject_in(repo_root, &output, provisional_smoke())
        .context("capturing committed 1.1.0 source and five exact local distributions")?;
    let bytes = output
        .read_file_bounded(report_relative_path, REPORT_MAX_BYTES)
        .context("reading bounded local smoke report")?
        .context("local smoke report is absent")?;
    let digest = vibe_publish::distribution_smoke_report::validate_distribution_smoke_report_bytes(
        &bytes,
        &provisional.subject,
    )
    .context("validating local smoke report against exact subject")?;
    let stored = StoredSmokeReport {
        digest: digest.clone(),
        relative_path: report_relative_path.to_owned(),
    };
    let prepared = PreparedSmokeReport { bytes, digest };
    let summary = summary_from_stored_report(&provisional.subject, &prepared, &stored)?;
    let final_capture = capture_prepared_subject_in(repo_root, &output, summary)
        .context("recapturing final exact publication subject")?;
    let mut expected_subject = provisional.subject;
    expected_subject.smoke = final_capture.subject.smoke.clone();
    if provisional.identity != final_capture.identity
        || provisional.aggregate_bytes != final_capture.aggregate_bytes
        || expected_subject != final_capture.subject
    {
        bail!("source or distribution identity changed between smoke captures");
    }
    let visible = output
        .read_file_bounded(report_relative_path, REPORT_MAX_BYTES)
        .context("rechecking held smoke report after final capture")?;
    if visible.as_deref() != Some(prepared.bytes.as_slice()) {
        bail!("stored smoke report changed after final subject capture");
    }
    Ok(PreparedLocalSmoke {
        capture: final_capture,
        report: stored,
        report_bytes: prepared.bytes,
    })
}

/// Hook is internal and only a deterministic test seam. It runs after the
/// local report is stored so the second capture must catch changed inputs.
pub(crate) fn prepare_local_with<F>(
    repo_root: &Path,
    output: &Project,
    after_store: F,
) -> Result<PreparedLocalSmoke>
where
    F: FnOnce() -> Result<()>,
{
    prepare_local_with_selection(repo_root, output, &SmokeSelection::default(), after_store)
}

pub(crate) fn prepare_local_with_selection<F>(
    repo_root: &Path,
    output: &Project,
    selection: &SmokeSelection,
    after_store: F,
) -> Result<PreparedLocalSmoke>
where
    F: FnOnce() -> Result<()>,
{
    let provisional = capture_prepared_subject_in(repo_root, output, provisional_smoke())
        .context("capturing committed 1.1.0 source and five exact local distributions")?;
    let mut records = Vec::with_capacity(SUPPORTED_DISTRIBUTION_TARGETS.len());
    for (index, target) in SUPPORTED_DISTRIBUTION_TARGETS.iter().enumerate() {
        let subject_target = &provisional.subject.targets[index];
        if subject_target.target != *target {
            bail!("provisional subject target order changed at `{target}`");
        }
        let fragment_bytes = read_subject_asset(
            output,
            &subject_target.fragment,
            DISTRIBUTION_MANIFEST_MAX_BYTES,
        )?;
        let fragment = PlatformDistributionFragment::from_json_slice(&fragment_bytes)
            .with_context(|| format!("decoding exact fragment for `{target}`"))?;
        let bundle = read_subject_asset(
            output,
            &subject_target.bundle,
            DISTRIBUTION_BUNDLE_MAX_BYTES,
        )?;
        // The raw bootstrap is an independent published asset; bind it again
        // even though the stage verifies its matching embedded component.
        read_subject_asset(
            output,
            &subject_target.bootstrap,
            DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
        )?;
        let stage = stage_verified_bundle(&bundle, target, &fragment, subject_target)
            .with_context(|| format!("staging exact bundle `{target}`"))?;
        if selection.skips(target) {
            records.push(super::NativeProbeRecord {
                target: (*target).into(),
                host: host_identity(),
                status: PublicationSmokeProbeStatus::Skipped,
                bundle_digest: stage.bundle_digest.clone(),
                tested_sha256: None,
                runner_ref: None,
                executable_sha256: None,
                reason: format!(
                    "explicit operator skip: {}",
                    selection.reason.as_deref().unwrap()
                ),
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                elapsed_ms: 0,
                timed_out: false,
                output_truncated: false,
            });
        } else {
            records.push(run_native_probe(&stage, Duration::from_secs(30)));
        }
    }
    let prepared = prepare_report(&provisional.subject, records)
        .context("assembling five truthful local smoke rows")?;
    let stored = store_report(output, &prepared.bytes, &provisional.subject)
        .context("publishing content-addressed local smoke report")?;
    let summary = summary_from_stored_report(&provisional.subject, &prepared, &stored)
        .context("deriving report-bound subject smoke summary")?;
    after_store()?;
    let final_capture = capture_prepared_subject_in(repo_root, output, summary)
        .context("recapturing final exact publication subject")?;
    let mut expected_subject = provisional.subject;
    expected_subject.smoke = final_capture.subject.smoke.clone();
    if provisional.identity != final_capture.identity
        || provisional.aggregate_bytes != final_capture.aggregate_bytes
        || expected_subject != final_capture.subject
    {
        bail!("source or distribution identity changed between smoke captures");
    }
    let visible = output
        .read_file_bounded(&stored.relative_path, REPORT_MAX_BYTES)
        .context("rechecking held smoke report after final capture")?;
    if visible.as_deref() != Some(prepared.bytes.as_slice()) {
        bail!("stored smoke report changed after final subject capture");
    }
    Ok(PreparedLocalSmoke {
        capture: final_capture,
        report: stored,
        report_bytes: prepared.bytes,
    })
}
