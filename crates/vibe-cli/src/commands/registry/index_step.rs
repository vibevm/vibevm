//! The publish step that adds a pushed release to a local checkout of the
//! registry's git-hosted index (PROP-005 `##INT-PUBLISH-INDEX-CHECKOUT`).
//!
//! The default index is a git repository read through a raw-file host, not
//! a server the HTTP hook could post to, and it used to be rebuilt by hand
//! with `vibe-index reindex` after each publish. The entry is added through
//! the index's own writer, in process, so it is born by the same code as one
//! an operator adds with `vibe-index add`; the commit is the checkout's, and
//! the push waits for `--push-index` so a maintainer can look before it ships.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#integration");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use vibe_core::GlobalRegistryConfig;
use vibe_core::manifest::Manifest;
use vibe_index::publish::PublishOutcome;

use crate::output;

/// `index_checkout` in the publish report. `path` is `null` when no
/// checkout is configured; `skipped`, `would_add`, `entry` and `error`
/// appear only when they say something.
#[derive(Debug, Default, Serialize)]
pub(super) struct IndexCheckoutReport {
    path: Option<String>,
    added: bool,
    committed: bool,
    pushed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    would_add: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// What the publish produced, as the index entry needs it.
pub(super) struct Release<'a> {
    pub(super) source_dir: &'a Path,
    /// The repository URL recorded on the entry — never one carrying
    /// credentials.
    pub(super) repo_url: &'a str,
    pub(super) tag: &'a str,
    pub(super) dry_run: bool,
}

/// The checkout this run writes into: `--index-checkout`, else
/// `[index] checkout` of the user-level registry file.
pub(super) fn configured_checkout(flag: Option<&Path>) -> Result<Option<PathBuf>> {
    if let Some(path) = flag {
        return Ok(Some(path.to_path_buf()));
    }
    let global = GlobalRegistryConfig::load().context("loading `~/.vibe/registry.toml`")?;
    Ok(global.index.checkout)
}

/// Run the step and say what it did. Never fails the publish: by the time
/// it runs the release is pushed, and refusing now would only hide that.
/// A failure is reported, with the command that repeats the step by hand.
pub(super) fn run(
    ctx: &output::Context,
    checkout: Option<PathBuf>,
    push: bool,
    release: &Release<'_>,
) -> IndexCheckoutReport {
    let task = ctx
        .progress()
        .task("Adding the release to the index checkout");
    let Some(checkout) = checkout else {
        task.skip("no index checkout is configured");
        return IndexCheckoutReport {
            skipped: Some(
                "no index checkout is configured (`--index-checkout`, or `[index] checkout` \
                 in ~/.vibe/registry.toml)"
                    .to_string(),
            ),
            ..IndexCheckoutReport::default()
        };
    };
    let mut report = IndexCheckoutReport {
        path: Some(checkout.display().to_string()),
        ..IndexCheckoutReport::default()
    };
    let manifest = release.source_dir.join(Manifest::FILENAME);
    if release.dry_run {
        task.skip("dry run");
        match coordinate(&manifest) {
            Ok(entry) => report.would_add = Some(entry),
            Err(error) => report.error = Some(format!("{error:#}")),
        }
        return report;
    }
    match add_and_commit(&checkout, &manifest, push, release, &mut report) {
        Ok(()) => task.finish(),
        Err(error) => {
            task.fail("the index checkout was not updated");
            report.error = Some(format!(
                "{error:#}; repeat the step with `vibe-index add {} --manifest {} --repo-url {} \
                 --ref {}`",
                checkout.display(),
                manifest.display(),
                release.repo_url,
                release.tag
            ));
        }
    }
    report
}

fn add_and_commit(
    checkout: &Path,
    manifest: &Path,
    push: bool,
    release: &Release<'_>,
    report: &mut IndexCheckoutReport,
) -> Result<()> {
    vibe_index::publish::preflight_checkout(checkout).map_err(|e| anyhow!("{e}"))?;
    let entry = vibe_index::cli::add::add(vibe_index::cli::add::Args {
        data_dir: checkout.to_path_buf(),
        manifest: manifest.to_path_buf(),
        repo_url: Some(release.repo_url.to_string()),
        r#ref: Some(release.tag.to_string()),
        commit: None,
    })
    .map_err(|e| anyhow!("adding the release to the index: {e}"))?;
    let coordinate = format!("{}/{}@{}", entry.group, entry.name, entry.version);
    report.added = true;
    report.entry = Some(coordinate.clone());
    let committed = vibe_index::publish::commit(checkout, &format!("index: {coordinate}"))
        .map_err(|e| anyhow!("committing the index checkout: {e}"))?;
    report.committed = committed == PublishOutcome::Published;
    if push {
        // Pushed even when nothing new was committed: an earlier run may
        // have left its commit unpushed, and pushing an up-to-date branch
        // changes nothing.
        vibe_index::publish::push(checkout)
            .map_err(|e| anyhow!("pushing the index checkout: {e}"))?;
        report.pushed = true;
    }
    Ok(())
}

fn coordinate(manifest: &Path) -> Result<String> {
    let parsed =
        Manifest::read(manifest).with_context(|| format!("reading `{}`", manifest.display()))?;
    let package = parsed
        .package
        .as_ref()
        .ok_or_else(|| anyhow!("`{}` has no [package] table", manifest.display()))?;
    Ok(format!(
        "{}/{}@{}",
        package.group, package.name, package.version
    ))
}

/// The step's lines in the human report.
pub(super) fn draw(ctx: &output::Context, report: &IndexCheckoutReport) {
    let path = report.path.as_deref().unwrap_or_default();
    if let Some(error) = &report.error {
        ctx.diagnostic(&format!(
            "the index checkout `{path}` was not updated: {error}"
        ));
    } else if let Some(entry) = &report.would_add {
        ctx.step(&format!("Would add {entry} to the index checkout `{path}`"));
    } else if let Some(entry) = &report.entry {
        let commit = match (report.committed, report.pushed) {
            (true, true) => "committed and pushed",
            (true, false) => "committed; push it with `--push-index` or `git push`",
            (false, true) => "nothing new to commit; pushed",
            (false, false) => "nothing new to commit",
        };
        ctx.step(&format!(
            "Added {entry} to the index checkout `{path}` ({commit})"
        ));
    } else if let Some(why) = &report.skipped {
        ctx.step(&format!("Index checkout step skipped: {why}"));
    }
}
