//! `vibe-index add <data-dir>` — insert/upsert a single index entry
//! from a `vibe.toml` manifest. The package's working
//! directory (containing the manifest) is hashed to populate
//! `content_hash`. Source URL / ref / commit are supplied via flags
//! when the operator has them; otherwise sensible defaults apply.

use std::path::PathBuf;

use chrono::Utc;
use clap::Parser;

use crate::content_hash::compute_content_hash;
use crate::error::{Error, Result};
use crate::index::Index;
use crate::scanner::manifest as mfst;
use crate::server::lock::ServerLock;
use crate::types::{NamingConvention, PackageKind, VersionEntry};

#[derive(Debug, Parser)]
#[command(about = "Insert/upsert a single index entry from a vibe.toml manifest.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Path to the `vibe.toml` whose entry should be inserted.
    /// The directory holding this file is hashed for `content_hash`.
    #[arg(long, value_name = "PATH")]
    pub manifest: PathBuf,

    /// Canonical clone URL recorded on the index entry. Defaults to
    /// composing `<registry-url>/<naming(repo)>` from the existing
    /// `repomd.json`.
    #[arg(long, value_name = "URL")]
    pub repo_url: Option<String>,

    /// Git ref the content was fetched at. Defaults to `v<semver>`.
    #[arg(long, value_name = "REF")]
    pub r#ref: Option<String>,

    /// Commit SHA the ref resolved to.
    #[arg(long, value_name = "SHA")]
    pub commit: Option<String>,
}

pub fn run(args: Args) -> Result<()> {
    refuse_if_server_running(&args.data_dir)?;

    let mut index = Index::load_from(&args.data_dir).map_err(|e| match e {
        Error::Io { .. } | Error::Malformed(_) => Error::InvalidInput(format!(
            "data-dir `{}` does not look like an initialised index. \
             Run `vibe-index init` first.",
            args.data_dir.display()
        )),
        other => other,
    })?;

    let manifest_bytes = std::fs::read(&args.manifest).map_err(|e| Error::Io {
        path: args.manifest.clone(),
        message: e.to_string(),
    })?;
    let raw = mfst::parse_manifest(&manifest_bytes)?;
    let pkg_root = args.manifest.parent().unwrap_or(std::path::Path::new("."));

    let kind = raw.package.kind;
    let name = raw.package.name.clone();
    let version = raw.package.version.clone();

    let content_hash = compute_content_hash(pkg_root)?;
    let source_ref = args.r#ref.unwrap_or_else(|| format!("v{version}"));
    let source_url = args.repo_url.unwrap_or_else(|| {
        compose_default_repo_url(&index.registry_url, index.naming, kind, &name)
    });
    let files_count = walkdir::WalkDir::new(pkg_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count() as u32;

    let entry = VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind,
        name,
        version,
        content_hash,
        source_url,
        source_ref,
        resolved_commit: args.commit,
        registry: index.registry.clone(),
        license: raw.package.license.clone(),
        authors: raw.package.authors.clone(),
        description: raw.package.description.clone(),
        homepage: raw.package.homepage.clone(),
        keywords: raw.package.keywords.clone(),
        describes: raw.package.describes.clone(),
        compatibility: mfst::compatibility_from_raw(&raw.compatibility),
        provides: mfst::provides_from_raw(&raw.provides),
        requires: mfst::requires_from_raw(&raw.requires),
        requires_any: mfst::requires_any_from_raw(&raw.requires_any),
        obsoletes: mfst::obsoletes_from_raw(&raw.obsoletes),
        conflicts: mfst::conflicts_from_raw(&raw.conflicts),
        features: mfst::features_from_raw(&raw.features)?,
        subskills: mfst::collect_subskills(pkg_root)?,
        i18n: mfst::i18n_from_raw(&raw.i18n),
        boot_snippet: mfst::boot_snippet_from_raw(&raw.boot_snippet),
        files_count,
        indexed_at: Utc::now(),
        indexed_by: format!("vibe-index {}", env!("CARGO_PKG_VERSION")),
    };

    println!(
        "adding {}:{} @ {} ({})",
        entry.kind, entry.name, entry.version, entry.content_hash
    );
    index.upsert(entry);
    index.write_to(&args.data_dir)?;
    Ok(())
}

fn compose_default_repo_url(
    registry_url: &str,
    naming: NamingConvention,
    kind: PackageKind,
    name: &str,
) -> String {
    let trimmed = registry_url.trim_end_matches('/');
    let repo = naming.repo_name(kind, name);
    format!("{trimmed}/{repo}.git")
}

fn refuse_if_server_running(data_dir: &std::path::Path) -> Result<()> {
    if let Some(pid) = ServerLock::read_pid(data_dir) {
        return Err(Error::InvalidInput(format!(
            "a vibe-index server is running on this data dir (PID {pid}). \
             Use the HTTP API or stop the server first."
        )));
    }
    Ok(())
}
