//! Handing the rendered trees to the site's static build
//! (PROP-057 `##SITE-TWO-CONTAINERS`, `##STACK-PAGE-COUNT-GATE`).
//!
//! The documentation pipeline writes trees; the site package turns them
//! into the domain — the landing, the shell around every island, the
//! root machine files. That division is the whole architecture
//! (`##PIPE-LIBRARY`: everything with content in it is in Rust, the
//! shell parses nothing), so this module is the join and nothing more:
//! it names the trees, names the domain, runs the build and moves the
//! result into place.
//!
//! ## The environment is set on the child, never through a shell
//!
//! `VIBE_DOC_OUT` is a list of paths with the platform's separator in
//! it, and a shell that thinks it recognises such a list rewrites it —
//! measured on this machine, where MSYS cut a `;`-separated list at
//! every `:`. Setting it on the process removes the shell from the
//! question entirely.
//!
//! ## The previous render stands until the new one has finished
//!
//! The static build writes into its own `dist`. Only a build that
//! succeeded is moved into the output, so the site a visitor is reading
//! is never the half-written one — which is what «one current render
//! plus the previous until the new one completes» asks for
//! (`##SITE-HOST-POLL`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-TWO-CONTAINERS");

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, bail};
use vibe_doc::site::Site;

/// Where the site package lives inside a checkout of the host.
pub(crate) const WEB_PACKAGE: &str = "vibevm/vibepacks/org.vibevm.doc/web/v1.0.0";

/// The build script, relative to the package.
const BUILD_SCRIPT: &str = "tools/build.mjs";

/// Where the static build leaves its output, relative to the package.
const DIST: &str = "site/dist";

/// The environment names the site package reads. They are its contract
/// and not this module's invention — `site/src/config.ts` and
/// `tools/doc-surfaces.mjs` name the same ones.
///
/// Every value of `[site]` that the shell can act on travels this way,
/// and all of them travel together: a configuration where one key
/// reached the pages and the next was silently dropped would be a
/// configuration nobody could reason about from reading it (X-058).
pub(super) const TREES: &str = "VIBE_DOC_OUT";
pub(super) const ORIGIN: &str = "VITE_SITE_ORIGIN";
pub(super) const WEBSITE_ID: &str = "VITE_UMAMI_WEBSITE_ID";
pub(super) const HOST_URL: &str = "VITE_UMAMI_HOST_URL";
pub(super) const DEFAULT_THEME: &str = "VITE_SITE_DEFAULT_THEME";
pub(super) const FEATURED: &str = "VITE_SITE_FEATURED";

/// How the featured coordinates travel: one variable, comma-separated,
/// no spaces. An environment carries strings and not lists, and a
/// separator that never appears inside a coordinate is what lets the
/// shell split the value without parsing anything
/// (`##PIPE-SHELL-PARSES-NOTHING`).
const FEATURED_SEPARATOR: &str = ",";

/// Run the static build over `trees` and move its output into `out`.
pub(crate) fn build(web: &Path, trees: &[PathBuf], site: &Site, out: &Path) -> Result<String> {
    let script = web.join(BUILD_SCRIPT);
    if !script.is_file() {
        bail!(
            "the site package is not at `{}`: it carries `{BUILD_SCRIPT}`, and the \
             renderer builds the domain with it. Point `--web` at the package, or pass \
             `--no-web` to stop after the documentation trees.",
            web.display()
        );
    }
    let list = std::env::join_paths(trees)
        .map_err(|e| anyhow::anyhow!("the rendered trees cannot be named in one variable: {e}"))?;
    let run = Command::new("node")
        .arg(BUILD_SCRIPT)
        .arg("static")
        .current_dir(web)
        .env(TREES, &list)
        .env(ORIGIN, &site.origin)
        .env(WEBSITE_ID, &site.analytics.website_id)
        .env(HOST_URL, &site.analytics.host_url)
        .env(DEFAULT_THEME, site.default_theme.as_str())
        .env(FEATURED, site.featured.join(FEATURED_SEPARATOR))
        .output()
        .map_err(|e| {
            anyhow::anyhow!(
                "running `node {BUILD_SCRIPT} static` in `{}`: {e} — the renderer \
                 needs Node and the package's dependencies installed",
                web.display()
            )
        })?;
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    if !run.status.success() {
        bail!(
            "the site's static build failed ({}). Its output:\n{report}",
            run.status
        );
    }
    publish(&web.join(DIST), out)?;
    Ok(report)
}

/// Move a finished build into the output directory, keeping the
/// builder's own state.
///
/// Everything the output holds is replaced except `.vibe-site`, which is
/// how the NEXT run knows what is rendered. Replacing rather than
/// merging is deliberate: a file the site no longer publishes must stop
/// being served, and a merge would keep it for ever.
fn publish(dist: &Path, out: &Path) -> Result<()> {
    if !dist.is_dir() {
        bail!(
            "the static build reported success and wrote no `{}`",
            dist.display()
        );
    }
    std::fs::create_dir_all(out)?;
    for entry in std::fs::read_dir(out)? {
        let entry = entry?;
        if entry.file_name() == vibe_doc::site::state::STATE_DIR {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    copy_tree(dist, out)
}

/// Copy a directory tree, creating what it needs.
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Where the site package is when nobody named it: beside the host's
/// checkout, which is the only place a renderer has one.
pub(crate) fn beside(site: &Site) -> Option<PathBuf> {
    let host = site.host.as_ref()?;
    let web = host.checkout.join(WEB_PACKAGE);
    web.is_dir().then_some(web)
}
