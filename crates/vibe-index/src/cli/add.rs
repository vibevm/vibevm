//! `vibe-index add <data-dir>` — insert/upsert a single index entry
//! from a `vibe.toml` manifest. The package's working
//! directory (containing the manifest) is hashed to populate
//! `content_hash`. Source URL / ref / commit are supplied via flags
//! when the operator has them; otherwise sensible defaults apply.
//!
//! Ф3.2 journal form: the published catalog is never this writer's
//! input (PROP-044 §4.4). The mutation is `validate → append →
//! project → write_to` — the entry's registry identity comes from
//! folding the journal, the fact lands in the journal first, and only
//! the re-folded projection is written back as the catalog.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use chrono::Utc;
use clap::Parser;

use vibe_core::Group;

use crate::content_hash::compute_content_hash;
use crate::error::{Error, Result};
use crate::index::memory::{WriteCtx, default_generator};
use crate::journal::{Event, JournalRecord, append, default_dir, project, replay};
use crate::lock::ServerLock;
use crate::scanner::manifest as mfst;
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
    // F2-1 — the clock enters here, once per command: the same `at`
    // stamps the entry's `indexed_at` and the written manifest, so two
    // records born of one command never differ by a millisecond.
    let at = Utc::now();
    refuse_if_server_running(&args.data_dir)?;

    // Ф3.2 — the catalog is never this writer's input (PROP-044 §4.4):
    // the registry identity the entry below is stamped with comes from
    // folding the journal, not from reading back a published catalog.
    // The journal is read from disk exactly once; the record list then
    // carries the appended fact in memory and is re-folded below.
    let journal_dir = default_dir(&args.data_dir);
    let mut records = replay(&journal_dir)?;
    // An uninitialised data-dir refuses here, from the truth layer: a
    // journal without an `Initialised` record folds into
    // `Error::Unprojectable`, whose recipe names `vibe-index init` —
    // the same guidance the failed catalog read used to give.
    let index = project(records.iter().cloned())?;

    let manifest_bytes = std::fs::read(&args.manifest).map_err(|e| Error::Io {
        path: args.manifest.clone(),
        message: e.to_string(),
    })?;
    let manifest = mfst::parse_manifest(&manifest_bytes)?;
    let pkg = mfst::require_package(&manifest)?;
    let pkg_root = args.manifest.parent().unwrap_or(std::path::Path::new("."));

    let kind = mfst::package_kind(pkg.kind);
    let group = pkg.group.clone();
    let name = pkg.name.clone();
    let version = pkg.version.clone();

    let content_hash = compute_content_hash(pkg_root)?;
    let source_ref = args.r#ref.unwrap_or_else(|| format!("v{version}"));
    let source_url = args.repo_url.unwrap_or_else(|| {
        compose_default_repo_url(&index.registry_url, index.naming, kind, &group, &name)
    });
    let files_count = walkdir::WalkDir::new(pkg_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count() as u32;

    let entry = VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind,
        group,
        name,
        version,
        content_hash,
        source_url,
        source_ref,
        resolved_commit: args.commit,
        registry: index.registry.clone(),
        workspace_origin: mfst::workspace_origin_from(&manifest.origin),
        license: pkg.license.clone(),
        authors: pkg.authors.clone(),
        description: pkg.description.clone(),
        homepage: pkg.homepage.clone(),
        keywords: pkg.keywords.clone(),
        describes: pkg.describes.as_ref().map(|p| p.to_string()),
        compatibility: mfst::compatibility_from(&manifest.compatibility),
        provides: mfst::provides_from(&manifest.provides),
        requires: mfst::requires_from(&manifest.requires),
        requires_any: mfst::requires_any_from(&manifest.requires_any),
        obsoletes: mfst::obsoletes_from(&manifest.obsoletes),
        conflicts: mfst::conflicts_from(&manifest.conflicts),
        features: mfst::features_from(&manifest.features),
        subskills: mfst::collect_subskills(pkg_root)?,
        i18n: mfst::i18n_from(&manifest.i18n),
        boot_snippet: mfst::boot_snippet_from(&manifest.boot_snippet),
        files_count,
        must_understand: Vec::new(),
        yanked: false,
        frozen: pkg.frozen,
        indexed_at: at,
        indexed_by: format!("vibe-index {}", env!("CARGO_PKG_VERSION")),
    };

    println!(
        "adding {}:{}/{} @ {} ({})",
        entry.kind, entry.group, entry.name, entry.version, entry.content_hash
    );
    // Truth first (PROP-044 `##LAW-NO-UNRECOVERABLE`), the `init`
    // order: the fact lands in the journal before the derived catalog
    // is written, so a failed `write_to` leaves a journal without a
    // catalog — recoverable by re-running the command — never a
    // catalog whose truth never existed.
    let record = JournalRecord {
        at,
        actor: default_generator(),
        event: Event::Published {
            entry: Box::new(entry),
        },
    };
    append(&journal_dir, &record)?;
    records.push(record);
    project(records)?.write_to(&args.data_dir, &WriteCtx { at })?;
    Ok(())
}

fn compose_default_repo_url(
    registry_url: &str,
    naming: NamingConvention,
    kind: PackageKind,
    group: &Group,
    name: &str,
) -> String {
    let trimmed = registry_url.trim_end_matches('/');
    let repo = naming.repo_name(kind, group, name);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// PROP-044 §2a — the manifest's `frozen` reaches the catalog entry
    /// through the `add` projection path (one of the two disjoint paths
    /// an entry is born from; the other is the org scanner).
    #[test]
    fn add_projects_manifest_frozen_into_the_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        // Setup writes the FACT an `init` would (Ф3.2: the journal is
        // the truth), not a hand-built catalog — `add` no longer reads
        // the catalog, so only an `Initialised` journal record makes
        // this data-dir look initialised to it.
        append(
            &default_dir(&data),
            &JournalRecord {
                at: Utc::now(),
                actor: default_generator(),
                event: Event::Initialised {
                    registry: "vibespecs".to_string(),
                    registry_url: "https://example.invalid/vibespecs".to_string(),
                    naming: NamingConvention::Fqdn,
                },
            },
        )
        .unwrap();

        let pkg_dir = tmp.path().join("pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"frozen-pkg\"\nkind = \"flow\"\nversion = \"0.1.0\"\nfrozen = true\n",
        )
        .unwrap();

        run(Args {
            data_dir: data.clone(),
            manifest: pkg_dir.join("vibe.toml"),
            repo_url: None,
            r#ref: None,
            commit: None,
        })
        .unwrap();

        // Read the PUBLISHED artifact (the by-name catalog file), the
        // same way consumers do — `add` itself no longer reads it, so
        // the test must not either.
        let by_name = data.join("by-name/frozen-pkg.json");
        let parsed: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&by_name).unwrap()).unwrap();
        let versions = parsed["packages"][0]["versions"].as_array().unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(
            versions[0]
                .get("frozen")
                .and_then(serde_json::Value::as_bool),
            Some(true),
            "manifest `frozen = true` must reach the catalog entry"
        );
    }
}
