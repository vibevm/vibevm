//! `vibe registry vendor` — generate a local `file://` mirror directory
//! from the per-package cache clones.
//!
//! Argument parsing, the `--force` / empty-dir safety policy, and
//! rendering live here; the vendoring domain (the per-package loop, the
//! bare-clone copy, the README generation, the `file://` URL derivation)
//! lives in [`vibe_registry::vendor`] (CONVERT-PLAN v0.1 §4.2).
//!
//! Spec: PROP-002 §2.3 (mirror layer), §6 (Phase B preview).

specmark::scope!("spec://vibevm/VIBEVM-SPEC#registry");

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_registry::MultiRegistryResolver;
use vibe_registry::vendor::{self, VendorEvent, VendorObserver, VendorSummary};

use crate::cli::RegistryVendorArgs;
use crate::output;

use super::{SkippedReportEntry, resolve_project_root};

#[derive(Debug, Serialize)]
struct VendorReport {
    ok: bool,
    command: &'static str,
    out_dir: String,
    /// Suggested `[[mirror]]` snippet the operator can paste into
    /// `vibe.toml`. The URL is `file://` + the absolute, forward-slash
    /// form of `out_dir`.
    suggested_mirror_url: String,
    vendored: Vec<VendoredReportEntry>,
    skipped: Vec<SkippedReportEntry>,
}

#[derive(Debug, Serialize)]
struct VendoredReportEntry {
    group: String,
    name: String,
    /// Registry that originally served this package — what `vibe.lock`
    /// records under `registry`.
    registry: String,
    repo_dir: String,
    /// What `vibe.lock` records under `source_ref` — typically
    /// `v<version>`. Vendored repo carries this tag.
    #[serde(rename = "ref")]
    refname: String,
}

/// Renders [`VendorEvent`]s as progressive `ctx.step` lines — the live
/// per-package output the domain used to print inline.
struct CliVendorObserver<'a>(&'a output::Context);

impl VendorObserver for CliVendorObserver<'_> {
    fn on(&self, event: VendorEvent) {
        match event {
            VendorEvent::Vendored {
                group,
                name,
                refname,
                repo_dir,
            } => {
                self.0
                    .step(&format!("{group}/{name} @ {refname} → {repo_dir}"));
            }
        }
    }
}

pub(super) fn run_vendor(ctx: &output::Context, args: RegistryVendorArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let lockfile_path = project_root.join(Lockfile::FILENAME);
    if !lockfile_path.exists() {
        bail!(
            "no `vibe.lock` in `{}`. Run `vibe install` first — vendoring is driven by the lockfile, not the manifest.",
            project_root.display()
        );
    }
    let lockfile = Lockfile::read(&lockfile_path)
        .with_context(|| format!("reading `{}`", lockfile_path.display()))?;

    if manifest.registries.is_empty() {
        bail!(
            "no `[[registry]]` entries in `{}`. Vendor only mirrors registry-served packages; \
             projects using only `--registry <path>` or `[[override]]` have nothing to vendor.",
            manifest_path.display()
        );
    }

    let out_dir = args
        .out
        .as_ref()
        .map(|p| project_root.join(p))
        .unwrap_or_else(|| project_root.join("vendor"));

    // Safety: never silently overwrite operator content. `--force`
    // wipes; without it, a non-empty target dir is a hard error.
    if out_dir.exists() {
        let mut iter = std::fs::read_dir(&out_dir)
            .with_context(|| format!("reading `{}`", out_dir.display()))?;
        let non_empty = iter.next().is_some();
        if non_empty && !args.force {
            bail!(
                "`{}` exists and is not empty. Pass `--force` to wipe and re-vendor, \
                 or pick a different `--out`.",
                out_dir.display()
            );
        }
        if args.force {
            std::fs::remove_dir_all(&out_dir)
                .with_context(|| format!("wiping `{}`", out_dir.display()))?;
        }
    }
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("creating `{}`", out_dir.display()))?;

    let mrr =
        MultiRegistryResolver::open(&manifest.registries, &manifest.mirrors, &manifest.overrides)
            .context("opening multi-registry resolver")?;

    ctx.heading(&format!(
        "Vendoring {} lockfile entr{} into `{}`",
        lockfile.packages.len(),
        if lockfile.packages.len() == 1 {
            "y"
        } else {
            "ies"
        },
        out_dir.display()
    ));

    let observer = CliVendorObserver(ctx);
    let summary = vendor::vendor_packages(&mrr, &lockfile, &out_dir, &observer)?;

    // Skipped entries are reported after the vendoring pass, matching the
    // pre-extraction ordering (vendored lines stream live via the
    // observer; skips are batched here).
    for s in &summary.skipped {
        ctx.skipped(&format!("{}/{}", s.group, s.name), &s.reason);
    }

    let VendorSummary {
        out_dir: out_dir_display,
        suggested_mirror_url,
        vendored,
        skipped,
    } = summary;

    let vendored_rows: Vec<VendoredReportEntry> = vendored
        .into_iter()
        .map(|v| VendoredReportEntry {
            group: v.group,
            name: v.name,
            registry: v.registry,
            repo_dir: v.repo_dir,
            refname: v.refname,
        })
        .collect();
    let skipped_rows: Vec<SkippedReportEntry> = skipped
        .into_iter()
        .map(|s| SkippedReportEntry {
            group: s.group,
            name: s.name,
            reason: s.reason,
        })
        .collect();

    if ctx.is_json() {
        ctx.emit_json(&VendorReport {
            ok: true,
            command: "registry:vendor",
            out_dir: out_dir_display,
            suggested_mirror_url: suggested_mirror_url.clone(),
            vendored: vendored_rows,
            skipped: skipped_rows,
        })?;
        return Ok(());
    }

    ctx.summary(&format!(
        "\nvibe registry vendor: {} vendored, {} skipped. \
         Wire as `[[mirror]] of = \"<registry>\" url = \"{}\"` to enable offline fallback.",
        vendored_rows.len(),
        skipped_rows.len(),
        suggested_mirror_url
    ));
    Ok(())
}
