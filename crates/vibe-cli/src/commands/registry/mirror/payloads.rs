//! Payload projection: the inputs of the selected versions and platforms,
//! written into the package drop at the paths the drop lookup already
//! reads (PROP-075 `#MIRROR-PAYLOAD-PROJECTION`, `#MIRROR-PAYLOAD-BUDGET`).
//!
//! **Nothing here is invented.** The inputs come out of the manifests the
//! snapshot stage already rendered; the drop is the one artifact-byte
//! mechanism (PROP-066 `#drops`, `CORP-FMT-003`); the bytes are fetched
//! through the installer's own Locator fetch, so the digest is verified as
//! it always is and every download goes through the one transfer wrapper;
//! and the file a client finds in the drop is the file a client would have
//! downloaded, at `<into>/<group>/<name>/<version>/<file>` — the rung
//! `vibe_core::remap::drop::package_lookup` tries first.
//!
//! The order is **project, weigh, then write**: the `[payloads] max_total`
//! budget is checked against the bytes to fetch before the first byte is
//! written, so a run that does not fit names what does not fit instead of
//! half-filling a drop.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{InstallerOptionDecl, Manifest, StepKind, steps_of};
use vibe_core::progress::transfer::Purpose;
use vibe_core::remap::drop::url_file_name;
use vibe_registry::templates::FullRender;

use super::selection::{PayloadPlan, Selection, TemplateSelection};
use super::templates::SnapshotOutcome;

const SPEC: &str = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION";
const BUDGET: &str = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-BUDGET";

/// One input of one installer option of one produced version.
#[derive(Debug, Clone)]
pub(super) struct Input {
    /// The installer option that declares it.
    pub option: String,
    /// The input's name: a `sources` entry's `name`, else the step id.
    pub name: String,
    /// The URL the rendered manifest declares, before remap.
    pub declared: String,
    pub sha256: String,
    /// The declared size, when the manifest states one. A template that
    /// declares none makes the projected total a **lower bound**, which
    /// the report says out loud.
    pub size: Option<u64>,
    pub file: String,
    /// `<into>/<group>/<name>/<version>/<file>`.
    pub destination: PathBuf,
    /// Already in the drop with the right digest: counted as present and
    /// never fetched again.
    pub present: bool,
    pub purpose: Purpose,
}

/// The inputs of one produced version, in selection order.
#[derive(Debug, Clone)]
pub(super) struct VersionInputs {
    pub version: semver::Version,
    pub inputs: Vec<Input>,
}

/// One `[[template]]` row's projection.
#[derive(Debug, Clone)]
pub(super) struct RowProjection {
    pub package: String,
    pub versions: Vec<VersionInputs>,
}

/// What a run would write outside the registry target.
#[derive(Debug, Clone, Default)]
pub(super) struct Projection {
    pub rows: Vec<RowProjection>,
}

/// The counts and bytes one `[[template]]` row's payloads come to, which
/// `#MIRROR-DRY-RUN-OUTPUT` asks a dry run to show and `AV-FMT-009` asks
/// the report to carry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct PayloadFacts {
    pub inputs: usize,
    /// Inputs already in the drop with the right digest.
    pub present: usize,
    pub present_bytes: u64,
    pub fetch_bytes: u64,
    pub total_bytes: u64,
    /// Inputs whose size the template does not declare, which is what
    /// makes the total a lower bound.
    pub unknown: usize,
}

impl RowProjection {
    /// The counts and bytes `#MIRROR-DRY-RUN-OUTPUT` asks a row to carry.
    pub(super) fn facts(&self) -> PayloadFacts {
        let inputs = self.versions.iter().flat_map(|v| v.inputs.iter());
        let mut facts = PayloadFacts::default();
        for input in inputs {
            facts.inputs += 1;
            match input.size {
                Some(bytes) if input.present => facts.present_bytes += bytes,
                Some(bytes) => facts.fetch_bytes += bytes,
                None => facts.unknown += 1,
            }
            if input.present {
                facts.present += 1;
            }
        }
        facts.total_bytes = facts.present_bytes + facts.fetch_bytes;
        facts
    }
}

impl Projection {
    pub(super) fn row(&self, package: &str) -> Option<&RowProjection> {
        self.rows.iter().find(|row| row.package == package)
    }

    /// Every input of every row, in selection order — the order the
    /// budget is spent in and the order a refusal names.
    fn all(&self) -> impl Iterator<Item = (&RowProjection, &VersionInputs, &Input)> {
        self.rows.iter().flat_map(|row| {
            row.versions
                .iter()
                .flat_map(move |version| version.inputs.iter().map(move |i| (row, version, i)))
        })
    }

    fn to_fetch(&self) -> usize {
        self.all().filter(|(_, _, input)| !input.present).count()
    }
}

/// Build the projection from the renders the snapshot stage performed.
///
/// A row without `payloads = true` carries no renders and so projects
/// nothing: the snapshot stage keeps a render only for a row that asked
/// for payloads, which is what stops a four-hundred-version selection
/// from holding four hundred manifests for nothing.
pub(super) fn project(selection: &Selection, rows: &[SnapshotOutcome]) -> Result<Projection> {
    let Some(plan) = selection.payloads() else {
        return Ok(Projection::default());
    };
    let mut projected = Vec::new();
    for row in selection.templates().iter().filter(|row| row.payloads) {
        let package = row.package();
        let Some(outcome) = rows.iter().find(|done| done.package == package) else {
            continue;
        };
        let mut versions = Vec::new();
        for render in &outcome.renders {
            versions.push(VersionInputs {
                version: render.version.clone(),
                inputs: inputs_of(row, render, plan, &package)?,
            });
        }
        projected.push(RowProjection { package, versions });
    }
    Ok(Projection { rows: projected })
}

/// The inputs one produced version's selected installer options declare.
fn inputs_of(
    row: &TemplateSelection,
    render: &FullRender,
    plan: &PayloadPlan,
    package: &str,
) -> Result<Vec<Input>> {
    let manifest = Manifest::parse_str(&render.toml).with_context(|| {
        format!(
            "reading the rendered manifest of `{package}@{}` for its payload inputs",
            render.version
        )
    })?;
    let dir = plan
        .into
        .join(&row.group)
        .join(&row.name)
        .join(render.version.to_string());
    let mut inputs = Vec::new();
    for option in manifest.installer_options.iter().filter(|o| kept(row, o)) {
        for (name, declared, sha256, size, purpose) in declared_inputs(option) {
            // Only a URL input is projectable: a package-relative source
            // is already in the package the mirror copies, and a `drop`
            // input is one an administrator places by hand, which is
            // exactly what projection is doing here.
            if !declared.starts_with("https://") && !declared.starts_with("http://") {
                continue;
            }
            let Some(file) = url_file_name(&declared) else {
                continue;
            };
            let destination = dir.join(&file);
            inputs.push(Input {
                option: option.id.clone(),
                name,
                declared,
                present: present(&destination, &sha256)?,
                sha256,
                size,
                file,
                destination,
                purpose,
            });
        }
    }
    Ok(inputs)
}

/// Whether a `[[template]]` row keeps this installer option: `installers`
/// names them outright and wins over `platforms` / `archs`, which filter
/// by what the option declares. None of the three keeps every option.
fn kept(row: &TemplateSelection, option: &InstallerOptionDecl) -> bool {
    if !row.installers.is_empty() {
        return row.installers.contains(&option.id);
    }
    let by_platform = row.platforms.is_empty() || row.platforms.contains(&option.platform);
    let by_arch = row.archs.is_empty() || row.archs.contains(&option.arch);
    by_platform && by_arch
}

/// `(name, declared url, sha256, declared size, journal purpose)` of every
/// input one installer option declares — its steps' own sources and the
/// `sources[]` entries of its `sources` steps, read through the sugar so
/// the method forms are the step lists they stand for.
fn declared_inputs(
    option: &InstallerOptionDecl,
) -> Vec<(String, String, String, Option<u64>, Purpose)> {
    let mut found = Vec::new();
    for step in steps_of(option) {
        match step.kind {
            StepKind::Archive {
                source,
                sha256,
                size,
                ..
            } => found.push((step.id.clone(), source, sha256, size, Purpose::Archive)),
            StepKind::Native { source, sha256, .. } => {
                found.push((step.id.clone(), source, sha256, None, Purpose::NativeInput));
            }
            StepKind::Sources { sources } => {
                for input in sources {
                    let Some(url) = input.url else { continue };
                    found.push((
                        input.name,
                        url,
                        input.sha256,
                        input.size,
                        Purpose::PackageSource,
                    ));
                }
            }
            _ => {}
        }
    }
    found
}

/// Whether the drop already holds this input.
///
/// A file present with the right digest is skipped and counted as present;
/// one present with a **wrong** digest stops the run and is not replaced,
/// because a human put it there.
fn present(destination: &Path, sha256: &str) -> Result<bool> {
    let bytes = match std::fs::read(destination) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error).with_context(|| format!("reading `{}`", destination.display()));
        }
    };
    let found = format!(
        "{:x}",
        <sha2::Sha256 as sha2::Digest>::digest(bytes.as_slice())
    );
    if found != sha256.to_ascii_lowercase() {
        bail!(
            "digest_mismatch: the drop already holds `{}`, and its SHA-256 is {found}, not the \
             {sha256} the rendered manifest declares; nothing was written \
             (violates {SPEC}; fix: a file a human placed is never replaced — check where that \
             file came from, then move it aside and re-run)",
            destination.display()
        )
    }
    Ok(true)
}

/// `[payloads] max_total` against the bytes to fetch, **before the first
/// byte is written**.
///
/// Over budget stops the run naming the budget, the projected total and
/// the versions and platforms that do not fit, in selection order, so
/// narrowing the row is mechanical.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-BUDGET"
)]
pub(super) fn weigh(projection: &Projection, plan: &PayloadPlan) -> Result<()> {
    let Some(budget) = plan.max_total else {
        return Ok(());
    };
    let mut spent = 0u64;
    let mut over: Vec<String> = Vec::new();
    for (row, version, input) in projection.all() {
        if input.present {
            continue;
        }
        spent = spent.saturating_add(input.size.unwrap_or(0));
        if spent > budget {
            over.push(format!(
                "{}@{} ({})",
                row.package, version.version, input.option
            ));
        }
    }
    if over.is_empty() {
        return Ok(());
    }
    bail!(
        "payload_budget_exceeded: `[payloads] max_total` is {budget} byte(s) and this selection \
         projects {spent} byte(s) to fetch; nothing was written. Beyond the budget, in selection \
         order: {} \
         (violates {BUDGET}; fix: raise `[payloads] max_total`, or narrow the `[[template]]` \
         row(s) — fewer `versions`, a `newest = N`, or only the `installers` / `platforms` this \
         network needs)",
        over.join(", ")
    )
}

/// Fetch every input the projection does not already hold, into the drop.
///
/// Each one goes through the installer's own Locator fetch, so a remap row
/// or a drop that already carries the bytes serves them, the digest is
/// verified on whatever did, and an HTTP download is journaled by the one
/// transfer wrapper.
pub(super) fn fetch(
    ctx: &crate::output::Context,
    projection: &Projection,
    operation: &vibe_package_source::SourceOperation,
    offline: bool,
) -> Result<usize> {
    let outstanding = projection.to_fetch();
    if outstanding == 0 {
        return Ok(0);
    }
    let client = operation.http_client(std::time::Duration::from_secs(600));
    let locator = vibe_core::remap::active();
    let task = ctx
        .progress()
        .task(format!("Projecting {outstanding} payload(s) into the drop"));
    let mut written = 0usize;
    for (row, version, input) in projection.all() {
        if input.present {
            continue;
        }
        let coordinate = format!("{}@{}", row.package, version.version);
        let outcome = (|| -> Result<()> {
            if let Some(parent) = input.destination.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating `{}`", parent.display()))?;
            }
            // A partial file is never left where the drop lookup reads:
            // the bytes are verified under a scratch name in the same
            // directory and renamed into place.
            let partial = input.destination.with_file_name(format!(
                ".{}.{}.part",
                input.file,
                std::process::id()
            ));
            let fetched = vibe_app::fetch_declared_input(
                &vibe_app::DeclaredInput {
                    remap: locator,
                    package: &coordinate,
                    input: &input.name,
                    declared: &input.declared,
                    sha256: &input.sha256,
                    size: input.size,
                    file_name: &input.file,
                    http_client: Some(&client),
                    task: Some(&task),
                    purpose: input.purpose,
                },
                &partial,
                offline,
            );
            match fetched {
                Ok(_) => {}
                Err(error) => {
                    let _ = std::fs::remove_file(&partial);
                    return Err(anyhow::Error::from(error));
                }
            }
            std::fs::rename(&partial, &input.destination).with_context(|| {
                format!("placing `{}` in the drop", input.destination.display())
            })?;
            Ok(())
        })();
        if let Err(error) = outcome {
            task.fail(error.to_string());
            return Err(error.context(format!(
                "projecting the payload `{}` of `{coordinate}` into the drop",
                input.file
            )));
        }
        written += 1;
        task.set_progress(written as u64, Some(outstanding as u64), "payloads");
    }
    task.finish();
    Ok(written)
}

/// Both answers a projection owes — the document's `templates[]` and the
/// row's human form — live beside it rather than in it: they are the one
/// place every fact of a `[[template]]` row comes together, and neither
/// decides anything.
#[path = "payloads/report.rs"]
mod report;

pub(super) use report::{rows, show};

#[cfg(test)]
#[path = "payloads/tests.rs"]
mod tests;
