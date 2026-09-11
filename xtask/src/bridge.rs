//! Maintainer tooling for reference-backed bridge packages.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde::Serialize;
use sha2::{Digest, Sha256};
use vibe_registry::{GitBackend, ShellGit, compute_portable_content_hash};

#[derive(Debug, Subcommand)]
pub enum BridgeCommand {
    /// Resolve and hash one immutable public Git source, then print a ready
    /// `[[embedded_source]]` TOML row. The checkout exists only in a tempdir.
    Pin {
        #[arg(long)]
        name: String,
        #[arg(long)]
        url: String,
        #[arg(long)]
        commit: String,
        #[arg(long)]
        ref_hint: Option<String>,
        #[arg(long)]
        upstream_license: String,
        #[arg(long)]
        license_path: PathBuf,
        #[arg(long)]
        license_url: String,
    },
}

#[derive(Serialize)]
struct EmbeddedSourceRow<'a> {
    name: &'a str,
    kind: &'static str,
    url: &'a str,
    commit: &'a str,
    content_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ref_hint: Option<&'a str>,
    auth: &'static str,
    upstream_license: &'a str,
    license_path: String,
    license_url: &'a str,
}

pub fn run_bridge(command: BridgeCommand) -> Result<()> {
    match command {
        BridgeCommand::Pin {
            name,
            url,
            commit,
            ref_hint,
            upstream_license,
            license_path,
            license_url,
        } => pin(
            &name,
            &url,
            &commit,
            ref_hint.as_deref(),
            &upstream_license,
            &license_path,
            &license_url,
        ),
    }
}

fn pin(
    name: &str,
    url: &str,
    commit: &str,
    ref_hint: Option<&str>,
    upstream_license: &str,
    license_path: &std::path::Path,
    license_url: &str,
) -> Result<()> {
    let scratch = tempfile::tempdir().context("creating temporary bridge checkout")?;
    let checkout = scratch.path().join("upstream");
    let shell = ShellGit::new();
    let git: Arc<dyn GitBackend> = shell
        .anonymized_for_public()
        .unwrap_or_else(|| Arc::new(ShellGit::new()));
    git.bootstrap_embedded(url, commit, &checkout)
        .context("fetching exact upstream commit without submodules")?;
    let actual = git
        .head_commit(&checkout)?
        .context("Git backend did not report HEAD")?;
    if actual != commit {
        bail!("upstream resolved to `{actual}`, expected exact commit `{commit}`");
    }
    let tree_oid = git
        .head_tree(&checkout)?
        .context("Git backend did not report HEAD^{tree}")?;
    let hash = compute_portable_content_hash(&checkout)?;

    let checked_license = vibe_core::manifest::declarant_path(license_path)
        .map_err(|fault| anyhow::anyhow!("unsafe --license-path: {}", fault.reason()))?;
    let license_file = checkout.join(checked_license);
    let bytes = std::fs::read(&license_file)
        .with_context(|| format!("reading upstream licence `{}`", license_file.display()))?;
    let license_hash = format!("sha256:{}", hex(Sha256::digest(&bytes)));

    let row = EmbeddedSourceRow {
        name,
        kind: "git",
        url,
        commit,
        content_hash: hash,
        ref_hint,
        auth: "none",
        upstream_license,
        license_path: checked_license.to_string(),
        license_url,
    };
    println!("# resolved_tree_oid = {tree_oid}");
    println!("# license_file_sha256 = {license_hash}");
    println!("[[embedded_source]]");
    print!("{}", toml::to_string_pretty(&row)?);
    Ok(())
}

fn hex(bytes: impl IntoIterator<Item = u8>) -> String {
    use std::fmt::Write;
    bytes.into_iter().fold(String::new(), |mut out, byte| {
        let _ = write!(&mut out, "{byte:02x}");
        out
    })
}
