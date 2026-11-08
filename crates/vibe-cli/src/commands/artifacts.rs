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
use serde::Serialize;
use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::manifest::{StepKind, steps_of};
use vibe_core::remap::drop::{bundle_file_name, package_lookup, release_dir, url_file_name};

use crate::cli::{ArtifactsArgs, ArtifactsCommand, ArtifactsWhereArgs};
use crate::commands::vvm::bundle::release_source::{self, ReleaseSource};
use crate::output;

/// The `--json` document of `vibe artifacts where`.
#[derive(Debug, Serialize)]
struct WhereReport {
    protocol: &'static str,
    release: ReleaseWhere,
    packages: Option<String>,
    inputs: Vec<InputWhere>,
}

#[derive(Debug, Serialize)]
struct ReleaseWhere {
    source: &'static str,
    location: String,
    from: String,
    /// The directory one release goes into, for the running version.
    example_dir: Option<String>,
    /// The files that directory must hold for this machine.
    files: Vec<String>,
}

#[derive(Debug, Serialize)]
struct InputWhere {
    option: String,
    step: String,
    input: String,
    file: Option<String>,
    sha256: String,
    url: Option<String>,
    searched: Vec<String>,
    status: &'static str,
}

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
    let report = WhereReport {
        protocol: "vibe-artifacts-where/1",
        release,
        packages: drop.as_ref().map(|path| path.display().to_string()),
        inputs,
    };
    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    render(ctx, &report, args.package.as_deref());
    Ok(())
}

fn release_where() -> Result<ReleaseWhere> {
    let effective = release_source::active()?;
    let version = env!("CARGO_PKG_VERSION");
    let target =
        crate::commands::vvm::bundle::release_source::release_target().unwrap_or("<target>");
    let files = vec![
        vibe_publish::release_manifest::DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME.to_owned(),
        bundle_file_name(version, target),
    ];
    let (example_dir, files) = match &effective.source {
        ReleaseSource::Localdir { path } => (
            Some(release_dir(path, version).display().to_string()),
            files,
        ),
        ReleaseSource::Nexus { base } => (Some(format!("{base}/v{version}")), files),
        _ => (None, Vec::new()),
    };
    Ok(ReleaseWhere {
        source: effective.source.kind(),
        location: effective.source.location(),
        from: effective.origin.describe(),
        example_dir,
        files,
    })
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
    let resolved = crate::commands::app::global::resolve_package_with_operation(
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
                    searched: searched.iter().map(|p| p.display().to_string()).collect(),
                    status,
                });
            }
        }
    }
    Ok(inputs)
}

/// `present` when a searched rung holds the declared bytes, `mismatch`
/// when the first file found holds others, `missing` otherwise.
fn status(searched: &[PathBuf], sha256: &str) -> &'static str {
    match searched.iter().find(|rung| rung.is_file()) {
        Some(found) => match std::fs::read(found) {
            Ok(bytes) if hex(&Sha256::digest(&bytes)) == sha256.to_ascii_lowercase() => "present",
            _ => "mismatch",
        },
        None if searched.is_empty() => "no-drop",
        None => "missing",
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn render(ctx: &output::Context, report: &WhereReport, package: Option<&str>) {
    let release = &report.release;
    let mut text = format!(
        "vibe releases: {} {} ({})",
        release.source, release.location, release.from
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
                input.status
            ));
            if let Some(first) = input.searched.first()
                && input.status != "present"
            {
                text.push_str(&format!("\n    put it at `{first}`"));
            }
        }
    }
    ctx.summary(&text);
}
