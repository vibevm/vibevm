//! Maintainer tooling for reference-backed bridge packages.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use serde::Serialize;
use sha2::{Digest, Sha256};
use vibe_registry::{GitBackend, ShellGit, compute_portable_content_hash};

#[derive(Debug, Subcommand)]
pub enum BridgeCommand {
    /// Resolve and hash one immutable public Git source, then print a ready
    /// `[[embedded_source]]` TOML row. The checkout exists only in a tempdir.
    Pin(PinArgs),
}

/// The `bridge pin` inputs as one named record. They are the fields of a
/// single `[[embedded_source]]` row and travel together everywhere, so the
/// resolver below reads this record instead of eight positional borrows of
/// its parts.
#[derive(Debug, Args)]
pub struct PinArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    url: String,
    #[arg(long)]
    commit: String,
    #[arg(long)]
    ref_hint: Option<String>,
    /// Author of the referenced upstream bytes. Repeat for each author;
    /// this is intentionally distinct from the package's own authors.
    #[arg(long, required = true)]
    upstream_author: Vec<String>,
    #[arg(long)]
    upstream_license: String,
    #[arg(long)]
    license_path: PathBuf,
    #[arg(long)]
    license_url: String,
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
    upstream_authors: &'a [String],
    upstream_license: &'a str,
    license_path: String,
    license_url: &'a str,
}

pub fn run_bridge(command: BridgeCommand) -> Result<()> {
    match command {
        BridgeCommand::Pin(args) => pin(&args),
    }
}

fn pin(args: &PinArgs) -> Result<()> {
    let upstream_authors = args.upstream_author.as_slice();
    if upstream_authors.is_empty()
        || upstream_authors.iter().any(|author| {
            author.is_empty() || author.trim() != author || author.chars().any(char::is_control)
        })
    {
        bail!(
            "--upstream-author must be repeated at least once and every value must be trimmed, non-empty, and control-free"
        );
    }

    let commit = args.commit.as_str();
    let scratch = tempfile::tempdir().context("creating temporary bridge checkout")?;
    let checkout = scratch.path().join("upstream");
    let shell = ShellGit::new();
    let git: Arc<dyn GitBackend> = shell
        .anonymized_for_public()
        .unwrap_or_else(|| Arc::new(ShellGit::new()));
    git.bootstrap_embedded(&args.url, commit, &checkout)
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

    let checked_license = vibe_core::manifest::declarant_path(&args.license_path)
        .map_err(|fault| anyhow::anyhow!("unsafe --license-path: {}", fault.reason()))?;
    let license_file = checkout.join(checked_license);
    let bytes = std::fs::read(&license_file)
        .with_context(|| format!("reading upstream licence `{}`", license_file.display()))?;
    let license_hash = format!("sha256:{}", hex(Sha256::digest(&bytes)));

    let row = EmbeddedSourceRow {
        name: args.name.as_str(),
        kind: "git",
        url: args.url.as_str(),
        commit,
        content_hash: hash,
        ref_hint: args.ref_hint.as_deref(),
        auth: "none",
        upstream_authors,
        upstream_license: args.upstream_license.as_str(),
        license_path: checked_license.to_string(),
        license_url: args.license_url.as_str(),
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
