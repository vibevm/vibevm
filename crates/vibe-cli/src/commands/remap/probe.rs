//! `vibe remap test` — does every declared artifact still have a source?
//! (`PROP-066#policy`).
//!
//! The command a closed network is checked with before a policy bites. It
//! walks what the project or the home actually declares, resolves each
//! artifact through the same Locator every fetch uses, and asks the
//! winning target whether it is there **without downloading it**: a `HEAD`
//! for an HTTP URL, a `git ls-remote` for a git URL, a `stat` for a path.
//! Exit 1 when any target is unreachable.
//!
//! Nothing here second-guesses the Locator. A `deny` row reports `denied`
//! rather than being probed, because the refusal is the answer, and
//! `--offline` reports every network probe as skipped rather than
//! pretending a target is gone.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#policy");

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use vibe_core::manifest::{Lockfile, Manifest, StepKind};
use vibe_core::remap::{Artifact, ArtifactKind, Locator, RemapTarget, Resolution};
use vibe_wire::generated::remap_test as wire;

use crate::cli::RemapTestArgs;
use crate::output;

/// The `vibe-remap-test/1` protocol.
const PROTOCOL: &str = "vibe-remap-test/1";

/// How long one probe waits before it counts as unreachable.
const PROBE_TIMEOUT: Duration = Duration::from_secs(20);

/// An unreachable target, as the command's refusal names it.
#[derive(Debug, thiserror::Error)]
#[error(
    "remap_unreachable: {count} of {total} declared artifact(s) have no reachable source — \
     {first} \
     (violates spec://org.vibevm.core/vibevm/common/PROP-066#policy; \
      fix: point the row at a target this host can read, or bring the artifacts in with \
      `vibe remap export --with-artifacts` and `vibe cache import`; \
      `vibe remap explain <url>` prints the chain for one of them)"
)]
pub(crate) struct RemapUnreachable {
    /// How many were unreachable.
    pub(crate) count: usize,
    /// How many were walked.
    pub(crate) total: usize,
    /// The first failure, with its error text.
    pub(crate) first: String,
}

/// One artifact the walk found, before it is probed.
struct Declared {
    /// What it is called in the report.
    name: String,
    /// The artifact identity the Locator is asked about.
    artifact: Artifact,
}

#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#policy")]
pub(crate) fn run(
    ctx: &output::Context,
    locator: &Locator,
    args: RemapTestArgs,
    offline: bool,
) -> Result<i32> {
    let root = args
        .path
        .canonicalize()
        .unwrap_or_else(|_| args.path.clone());
    let (scope, declared) = walk(&root)?;
    let task = ctx.progress().task("Probing remap targets");
    let mut probes = Vec::with_capacity(declared.len());
    for (index, entry) in declared.iter().enumerate() {
        probes.push(probe(locator, entry, offline));
        task.set_progress((index + 1) as u64, Some(declared.len() as u64), "artifacts");
    }
    task.finish();
    let unreachable: Vec<&wire::Probe> = probes
        .iter()
        .filter(|probe| probe.outcome == wire::Outcome::Unreachable)
        .collect();
    let document = wire::RemapTest {
        protocol: PROTOCOL.into(),
        scope: scope.clone(),
        offline,
        reachable: probes
            .iter()
            .filter(|probe| probe.outcome == wire::Outcome::Reachable)
            .count() as u32,
        unreachable: unreachable.len() as u32,
        skipped: probes
            .iter()
            .filter(|probe| probe.outcome == wire::Outcome::Skipped)
            .count() as u32,
        artifacts: probes.clone(),
    };
    // Record what this probe found, so every view of a row can state
    // whether its target answered the last time anyone asked
    // (`PROP-066#diagnostics-reachability`). A home that cannot be written
    // costs the note, never the report.
    if let Some(home) = vibe_core::settings::settings_dir() {
        let when = crate::commands::init::current_timestamp_utc();
        let mut record = vibe_core::remap::Reachability::load(&home);
        for probe in &probes {
            let Some(target) = &probe.target else {
                continue;
            };
            record.targets.insert(
                target.clone(),
                vibe_core::remap::Probed {
                    outcome: match probe.outcome {
                        wire::Outcome::Reachable => "reachable",
                        wire::Outcome::Unreachable => "unreachable",
                        wire::Outcome::Denied => "denied",
                        wire::Outcome::Skipped => "skipped",
                    }
                    .to_owned(),
                    when: when.clone(),
                },
            );
        }
        if let Err(error) = record.store(&home) {
            ctx.diagnostic(&format!("the reachability record was not updated: {error}"));
        }
    }
    let failure = (!unreachable.is_empty()).then(|| RemapUnreachable {
        count: unreachable.len(),
        total: probes.len(),
        first: format!(
            "{}: {}",
            unreachable[0].artifact,
            unreachable[0].error.as_deref().unwrap_or("no answer")
        ),
    });
    emit(ctx, args.json, &document, failure.as_ref())?;
    // Exit 1 when any target is unreachable, 0 otherwise
    // (`PROP-066#policy`): the command is a readiness check, so its status
    // is the answer and the report is printed either way.
    Ok(i32::from(failure.is_some()))
}

/// What a project or a home declares (`PROP-066#policy`).
///
/// A project is anything with a `vibe.lock`: its locked packages and their
/// embedded sources. Otherwise the home: every installed application's
/// declared inputs, and the distribution manifest `vibe self` reads.
fn walk(root: &Path) -> Result<(wire::TestScope, Vec<Declared>)> {
    let lock = root.join(Lockfile::FILENAME);
    if lock.exists() {
        return Ok((wire::TestScope::Project, project_artifacts(&lock)?));
    }
    Ok((wire::TestScope::Home, home_artifacts()))
}

/// The lock's rows and their embedded sources.
fn project_artifacts(lock: &Path) -> Result<Vec<Declared>> {
    let lockfile = Lockfile::read(lock).with_context(|| format!("reading `{}`", lock.display()))?;
    let mut declared = Vec::new();
    for row in &lockfile.packages {
        let key = format!("{}/{}@{}", row.group.as_str(), row.name, row.version);
        declared.push(Declared {
            name: key.clone(),
            artifact: Artifact::new(ArtifactKind::RegistryClone)
                .with_url(row.source_url.as_str())
                .with_package(&key),
        });
        for source in &row.embedded_sources {
            declared.push(Declared {
                name: format!("{key} / {}", source.name),
                artifact: Artifact::new(ArtifactKind::EmbeddedSource)
                    .with_url(source.source_url.as_str())
                    .with_digest(source.content_hash.as_str())
                    .with_package(&key),
            });
        }
    }
    Ok(declared)
}

/// The home's installed application inputs and the distribution manifest.
fn home_artifacts() -> Vec<Declared> {
    let mut declared = Vec::new();
    if let Some(settings) = vibe_core::settings::settings_dir() {
        let root = vibe_app::SettingsRoot::new(&settings);
        if let Ok(index) = vibe_app::Index::load(&root) {
            for installation in index.installations.values() {
                declared.extend(inputs_of(installation));
            }
        }
        // Every input this home has ever fetched, by digest. The index
        // alone is not enough: an installation keeps no copy of the
        // package that declared it, and the machine store holds a
        // package tree only when something warmed it. The download cache
        // is the one place a home always remembers its own artifacts,
        // and a digest is exactly what a `sha256` row matches on — which
        // is the row a closed network is built out of.
        declared.extend(cached_inputs(&root.app_sources_cache(), &declared));
    }
    // `vibe self` reads the publisher's manifest before any download, so
    // a closed network that cannot reach it cannot update vibe at all.
    declared.push(Declared {
        name: "vibe self / DISTRIBUTIONS.json".into(),
        artifact: Artifact::new(ArtifactKind::Distribution).with_url(format!(
            "{}/latest/download/DISTRIBUTIONS.json",
            crate::composition::remap::CANONICAL_RELEASE_ORIGIN
        )),
    });
    declared
}

/// The artifacts in the application download cache, as digest-only
/// artifacts the Locator can still answer about. The declared URL is not
/// kept there, so these carry a digest and no origin — which is enough for
/// a `sha256` row and honest about the rest.
fn cached_inputs(cache: &Path, already: &[Declared]) -> Vec<Declared> {
    let Ok(entries) = std::fs::read_dir(cache) else {
        return Vec::new();
    };
    let mut declared = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let digest = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        if already
            .iter()
            .any(|found| found.artifact.digest.as_deref() == Some(digest.as_str()))
        {
            continue;
        }
        let name = std::fs::read_dir(entry.path())
            .ok()
            .and_then(|mut files| files.next()?.ok())
            .map(|file| file.file_name().to_string_lossy().into_owned())
            .unwrap_or_else(|| digest.chars().take(12).collect());
        declared.push(Declared {
            name: format!("cached input {name}"),
            artifact: Artifact::new(ArtifactKind::AppInput).with_digest(&digest),
        });
    }
    declared.sort_by(|left, right| left.name.cmp(&right.name));
    declared
}

/// The declared inputs of one installation, read from the package tree the
/// machine store still holds. An installation whose package is gone from
/// the store declares nothing we can read — which is itself not a remap
/// problem, so it is skipped rather than reported unreachable.
fn inputs_of(installation: &vibe_app::Installation) -> Vec<Declared> {
    let coordinate = &installation.package;
    let Ok(group) = vibe_core::Group::parse(&coordinate.group) else {
        return Vec::new();
    };
    let Ok(version) = semver::Version::parse(&coordinate.version) else {
        return Vec::new();
    };
    let Some(entry) = vibe_registry::lookup(&group, &coordinate.name, &version) else {
        return Vec::new();
    };
    let Ok(manifest) = Manifest::read(entry.join(Manifest::FILENAME)) else {
        return Vec::new();
    };
    let Some(option) = manifest
        .installer_options
        .iter()
        .find(|option| option.id == installation.option)
    else {
        return Vec::new();
    };
    let key = coordinate.key();
    let mut declared = Vec::new();
    for step in vibe_core::manifest::steps_of(option) {
        let base = Artifact::new(ArtifactKind::AppInput)
            .with_package(&key)
            .with_step(step.id.clone());
        match &step.kind {
            StepKind::Sources { sources } => {
                for source in sources {
                    let Some(url) = &source.url else { continue };
                    declared.push(Declared {
                        name: format!("{key} / {} / {}", step.id, source.name),
                        artifact: base
                            .clone()
                            .with_url(url.clone())
                            .with_digest(&source.sha256),
                    });
                }
            }
            StepKind::Archive { source, sha256, .. } | StepKind::Native { source, sha256, .. } => {
                if !source.contains("://") {
                    continue;
                }
                declared.push(Declared {
                    name: format!("{key} / {}", step.id),
                    artifact: base.clone().with_url(source.clone()).with_digest(sha256),
                });
            }
            _ => {}
        }
    }
    declared
}

/// Resolve one artifact and ask its winning target whether it is there.
fn probe(locator: &Locator, entry: &Declared, offline: bool) -> wire::Probe {
    let row = |reference: &vibe_core::remap::RowRef| {
        format!(
            "row {} of `{}` ({})",
            reference.provenance.index + 1,
            reference.provenance.source,
            reference.provenance.layer.label()
        )
    };
    let blank = wire::Probe {
        artifact: entry.name.clone(),
        kind: entry.artifact.kind.label().to_owned(),
        origin: entry.artifact.url.clone(),
        digest: entry.artifact.digest.clone(),
        target: None,
        row: None,
        probe: wire::ProbeKind::None,
        outcome: wire::Outcome::Skipped,
        error: None,
        note: None,
    };
    let candidates = match locator.locate(&entry.artifact) {
        Resolution::Denied { row: refused, .. } => {
            return wire::Probe {
                row: refused.as_ref().map(row),
                outcome: wire::Outcome::Denied,
                note: Some("refused before any connection by the active rows".into()),
                ..blank
            };
        }
        Resolution::Candidates(candidates) => candidates,
    };
    let Some(winner) = candidates.first() else {
        return wire::Probe {
            note: Some("nothing to fetch — the artifact declares no source".into()),
            ..blank
        };
    };
    let chosen = wire::Probe {
        target: Some(winner.target.display()),
        row: winner.row.as_ref().map(row),
        ..blank
    };
    match &winner.target {
        RemapTarget::Deny => wire::Probe {
            outcome: wire::Outcome::Denied,
            ..chosen
        },
        RemapTarget::Path(path) => match stat(path, entry.artifact.digest.as_deref()) {
            Ok(()) => wire::Probe {
                probe: wire::ProbeKind::Stat,
                outcome: wire::Outcome::Reachable,
                ..chosen
            },
            Err(error) => wire::Probe {
                probe: wire::ProbeKind::Stat,
                outcome: wire::Outcome::Unreachable,
                error: Some(error),
                ..chosen
            },
        },
        RemapTarget::Url(url) if url.starts_with("file://") => {
            let path = url.trim_start_matches("file://");
            match stat(Path::new(path), None) {
                Ok(()) => wire::Probe {
                    probe: wire::ProbeKind::Stat,
                    outcome: wire::Outcome::Reachable,
                    ..chosen
                },
                Err(error) => wire::Probe {
                    probe: wire::ProbeKind::Stat,
                    outcome: wire::Outcome::Unreachable,
                    error: Some(error),
                    ..chosen
                },
            }
        }
        RemapTarget::Url(url) if offline => wire::Probe {
            probe: wire::ProbeKind::None,
            outcome: wire::Outcome::Skipped,
            note: Some(format!("--offline: `{url}` was not asked")),
            ..chosen
        },
        RemapTarget::Url(url) if is_git(url) => match ls_remote(url) {
            Ok(()) => wire::Probe {
                probe: wire::ProbeKind::LsRemote,
                outcome: wire::Outcome::Reachable,
                ..chosen
            },
            Err(error) => wire::Probe {
                probe: wire::ProbeKind::LsRemote,
                outcome: wire::Outcome::Unreachable,
                error: Some(error),
                ..chosen
            },
        },
        RemapTarget::Url(url) => match head(url) {
            Ok(()) => wire::Probe {
                probe: wire::ProbeKind::Head,
                outcome: wire::Outcome::Reachable,
                ..chosen
            },
            Err(error) => wire::Probe {
                probe: wire::ProbeKind::Head,
                outcome: wire::Outcome::Unreachable,
                error: Some(error),
                ..chosen
            },
        },
    }
}

/// Whether a URL is a git remote rather than a file to `HEAD`.
fn is_git(url: &str) -> bool {
    url.starts_with("git://")
        || url.starts_with("ssh://")
        || url.starts_with("git+")
        || url.ends_with(".git")
        || (!url.contains("://") && url.contains('@') && url.contains(':'))
}

/// Does the path exist — the file itself, or the `<path>/<sha256>` entry a
/// digest-laid-out directory holds (`PROP-066#row`)?
fn stat(path: &Path, digest: Option<&str>) -> Result<(), String> {
    if path.is_file() {
        return Ok(());
    }
    if let Some(digest) = digest {
        let entry = path.join(digest.to_ascii_lowercase());
        if entry.exists() {
            return Ok(());
        }
        if path.is_dir() {
            return Err(format!(
                "`{}` holds no entry for the digest",
                path.display()
            ));
        }
    }
    if path.is_dir() {
        return Ok(());
    }
    Err(format!("`{}` does not exist", path.display()))
}

/// One `HEAD`, no body.
fn head(url: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(PROBE_TIMEOUT)
        .build()
        .map_err(|error| error.to_string())?;
    client
        .head(url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// One `git ls-remote`, which lists refs and downloads no objects.
fn ls_remote(url: &str) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .args(["ls-remote", "--exit-code", url, "HEAD"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|error| format!("running `git ls-remote`: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(String::from_utf8_lossy(&output.stderr)
        .lines()
        .next()
        .unwrap_or("git ls-remote failed")
        .to_owned())
}

/// The report, in the one layout `--json` and a human share.
fn emit(
    ctx: &output::Context,
    json: bool,
    document: &wire::RemapTest,
    failure: Option<&RemapUnreachable>,
) -> Result<()> {
    if json || ctx.is_json() {
        return ctx.emit_json(document);
    }
    ctx.summary(&format!(
        "{} artifact(s) of the {} — {} reachable, {} unreachable, {} skipped",
        document.artifacts.len(),
        match document.scope {
            wire::TestScope::Project => "project",
            wire::TestScope::Home => "home",
        },
        document.reachable,
        document.unreachable,
        document.skipped,
    ));
    if ctx.is_quiet() {
        return Ok(());
    }
    use crate::output::language::{Block, Glyph, Line, Meta, Role};
    let mut blocks = Vec::new();
    for probe in &document.artifacts {
        let (glyph, word) = match probe.outcome {
            wire::Outcome::Reachable => (Glyph::Done, "reachable"),
            wire::Outcome::Unreachable => (Glyph::Failed, "unreachable"),
            wire::Outcome::Denied => (Glyph::Failed, "denied"),
            wire::Outcome::Skipped => (Glyph::Skipped, "skipped"),
        };
        let mut body = Vec::new();
        let label = |name: &str, value: &str| {
            Line::Label(
                name.to_owned(),
                vec![crate::output::language::LabelValue::Text(value.to_owned())],
            )
        };
        if let Some(target) = &probe.target {
            body.push(label("to", target));
        }
        if let Some(row) = &probe.row {
            body.push(label("by", row));
        }
        if let Some(error) = &probe.error {
            body.push(label("error", error));
        }
        if let Some(note) = &probe.note {
            body.push(label("note", note));
        }
        let mut block = Block::new(glyph, probe.artifact.clone())
            .with_meta(vec![Meta::State(word.to_owned())])
            .with_body(body);
        if glyph == Glyph::Skipped {
            block.headline_role = Role::Dim;
        }
        blocks.push(block.render());
    }
    if !blocks.is_empty() {
        ctx.block(&blocks.join("\n"));
    }
    if let Some(failure) = failure {
        let refusal = crate::output::refusal_of(&anyhow::anyhow!(failure.to_string()));
        ctx.block_err(&refusal.block().render());
    }
    Ok(())
}
