//! `vibe artifacts where [<package>]` (`PROP-066#drops`): where a person
//! puts the files a closed network cannot download.
//!
//! The answer is computed from the same owners the fetches use — the
//! release source resolver and the Locator's package-drop rows, and the
//! package convention of `vibe_core::remap::drop` — so what this prints is
//! exactly where `vibe self update` and `vibe install -g` will look.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#drops");

use std::path::{Path, PathBuf};

use anyhow::Result;
use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::manifest::{StepKind, steps_of};
use vibe_core::remap::drop::{
    bundle_file_name, package_lookup, release_dir, shown_path, url_file_name,
};
use vibe_wire::generated::artifacts_where_report::{
    ArtifactsWhereReport, DropStatus, InputWhere, ReleaseWhere,
};

use crate::cli::{ArtifactsArgs, ArtifactsCommand, ArtifactsWhereArgs};
use crate::commands::vvm::bundle::release_source::{self, ReleaseSource};
use crate::output;

/// The JSON protocol name of the report (`cli-artifacts-where-report`).
const PROTOCOL: &str = "vibe-artifacts-where/1";

#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#drops")]
pub fn run(ctx: &output::Context, args: ArtifactsArgs, offline: bool) -> Result<()> {
    match args.command {
        ArtifactsCommand::Where(args) => run_where(ctx, args, offline),
    }
}

fn run_where(ctx: &output::Context, args: ArtifactsWhereArgs, offline: bool) -> Result<()> {
    let release = release_where()?;
    let drop = vibe_core::remap::active()
        .package_drops()
        .first()
        .map(|path| path.to_path_buf());
    let inputs = match &args.package {
        Some(spelling) => {
            package_inputs(spelling, args.registry.as_deref(), drop.as_deref(), offline)?
        }
        None => Vec::new(),
    };
    let report = ArtifactsWhereReport {
        protocol: PROTOCOL.to_owned(),
        release,
        packages: drop.as_deref().map(shown_path),
        inputs,
    };
    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    render(ctx, &report, args.package.as_deref());
    Ok(())
}

fn release_where() -> Result<ReleaseWhere> {
    Ok(release_where_for(
        &release_source::active()?,
        env!("CARGO_PKG_VERSION"),
    ))
}

/// Where one release `version` of this platform goes for the release
/// source `effective` — the answer `vibe artifacts where` prints and
/// `vibe self apply` repeats after applying a profile.
pub(crate) fn release_where_for(
    effective: &release_source::EffectiveReleaseSource,
    version: &str,
) -> ReleaseWhere {
    let target =
        crate::commands::vvm::bundle::release_source::release_target().unwrap_or("<target>");
    let files = vec![
        vibe_publish::release_manifest::DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME.to_owned(),
        bundle_file_name(version, target),
    ];
    let (example_dir, files) = match &effective.source {
        ReleaseSource::Localdir { path } => (Some(shown_path(&release_dir(path, version))), files),
        ReleaseSource::Nexus { base } => (Some(format!("{base}/v{version}")), files),
        _ => (None, Vec::new()),
    };
    ReleaseWhere {
        source: effective.source.wire_kind(),
        location: effective.source.location(),
        from: effective.origin.describe(),
        example_dir,
        files,
    }
}

/// Every downloaded input of every installer option of the package.
fn package_inputs(
    spelling: &str,
    registry: Option<&Path>,
    drop: Option<&Path>,
    offline: bool,
) -> Result<Vec<InputWhere>> {
    let settings = crate::commands::app::global::settings_root()?;
    let operation = crate::commands::app::global::source_operation(Default::default())?;
    // **Deliberately not the policy path.** `vibe artifacts where`
    // answers "where does this payload belong", and under
    // `[versions] select = "drop-only"` the version the operator needs the
    // answer for is exactly the one the drop does not hold yet — a policy
    // applied here would refuse the question instead of answering it, and
    // would name a version whose files are already in place (PROP-075
    // `#VERSIONS-SELECT`, PROP-066 `#drops`).
    let resolved = crate::commands::app::global::resolve_package_newest(
        &settings, registry, None, spelling, offline, &operation,
    )?;
    let package = resolved.package.key();
    let mut inputs = Vec::new();
    for option in &resolved.options {
        for step in steps_of(option) {
            let declared: Vec<(String, Option<String>, Option<String>, String)> = match &step.kind {
                StepKind::Sources { sources } => sources
                    .iter()
                    .filter_map(|source| {
                        // A downloaded input, or an author-side drop input
                        // (`PROP-066#drops-author`) named by its `file`.
                        let file = match (&source.url, &source.drop) {
                            (Some(url), _) => url_file_name(url),
                            (None, Some(_)) => source.file.clone(),
                            (None, None) => return None,
                        };
                        Some((
                            source.name.clone(),
                            source.url.clone(),
                            file,
                            source.sha256.clone(),
                        ))
                    })
                    .collect(),
                StepKind::Archive { source, sha256, .. }
                | StepKind::Native { source, sha256, .. }
                    if source.starts_with("https://") || source.starts_with("http://") =>
                {
                    vec![(
                        step.id.clone(),
                        Some(source.clone()),
                        url_file_name(source),
                        sha256.clone(),
                    )]
                }
                _ => Vec::new(),
            };
            for (input, url, file, sha256) in declared {
                let searched = drop
                    .map(|drop| package_lookup(drop, Some(&package), &sha256, file.as_deref()))
                    .unwrap_or_default();
                let status = status(&searched, &sha256);
                inputs.push(InputWhere {
                    option: option.id.clone(),
                    step: step.id.clone(),
                    input,
                    file,
                    sha256: sha256.to_ascii_lowercase(),
                    url,
                    searched: searched.iter().map(|p| shown_path(p)).collect(),
                    status,
                });
            }
        }
    }
    Ok(inputs)
}

/// `present` when a searched rung holds the declared bytes, `mismatch`
/// when the first file found holds others, `missing` otherwise.
fn status(searched: &[PathBuf], sha256: &str) -> DropStatus {
    match searched.iter().find(|rung| rung.is_file()) {
        Some(found) => match std::fs::read(found) {
            Ok(bytes) if hex(&Sha256::digest(&bytes)) == sha256.to_ascii_lowercase() => {
                DropStatus::Present
            }
            _ => DropStatus::Mismatch,
        },
        None if searched.is_empty() => DropStatus::NoDrop,
        None => DropStatus::Missing,
    }
}

/// The wire value spelled for a human; the one mapping, so the text and the
/// JSON can never disagree.
fn status_label(status: &DropStatus) -> &'static str {
    match status {
        DropStatus::Present => "present",
        DropStatus::Mismatch => "mismatch",
        DropStatus::Missing => "missing",
        DropStatus::NoDrop => "no-drop",
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn render(ctx: &output::Context, report: &ArtifactsWhereReport, package: Option<&str>) {
    let release = &report.release;
    let mut text = format!(
        "vibe releases: {} {} ({})",
        release_source::kind_label(release.source.clone()),
        release.location,
        release.from
    );
    match &release.example_dir {
        Some(dir) => {
            text.push_str(&format!(
                "\n  put each release's files into `<drop>/v<version>/`, e.g. `{dir}`:"
            ));
            for file in &release.files {
                text.push_str(&format!("\n    {file}"));
            }
            text.push_str("\n  (a bare `vibevm-<version>-<target>.zip` is accepted too)");
        }
        None => text.push_str("\n  no release drop: releases are downloaded from the source"),
    }
    match &report.packages {
        Some(drop) => text.push_str(&format!(
            "\npackage payloads: {drop}\n  an input of `G/N@V` named `F` (sha256 `D`) is looked for at \
             `<drop>/G/N/V/F`, `<drop>/G/N/F`, `<drop>/D/F`, `<drop>/F`"
        )),
        None => text.push_str(
            "\npackage payloads: no drop — set `[artifacts] packages = \"<dir>\"` in ~/.vibe/config.toml",
        ),
    }
    if let Some(package) = package {
        if report.inputs.is_empty() {
            text.push_str(&format!("\n{package}: no downloaded inputs"));
        }
        for input in &report.inputs {
            text.push_str(&format!(
                "\n{package} [{}] {} `{}` sha256 {}: {}",
                input.option,
                input.input,
                input.file.as_deref().unwrap_or("<unnamed>"),
                input.sha256,
                status_label(&input.status)
            ));
            if let Some(first) = input.searched.first()
                && input.status != DropStatus::Present
            {
                text.push_str(&format!("\n    put it at `{first}`"));
            }
        }
    }
    ctx.summary(&text);
}
