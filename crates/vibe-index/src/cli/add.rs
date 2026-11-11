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

use crate::error::{Error, Result};
use crate::index::memory::{WriteCtx, default_generator};
use crate::journal::{Event, JournalRecord, append, default_dir, project, replay};
use crate::scanner::manifest as mfst;
use crate::scanner::tree_entry::{EntrySource, entry_from_tree};
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
    let entry = add(args)?;
    println!(
        "added {}:{}/{} @ {} ({})",
        entry.kind, entry.group, entry.name, entry.version, entry.content_hash
    );
    Ok(())
}

/// The whole of `add` without its line of output, for a caller that
/// reports in its own words — `vibe registry publish` adds the version it
/// has just pushed to a local index checkout through this, in process, so
/// the entry is born by the same code as one an operator adds by hand.
/// Returns the entry as it went into the journal.
pub fn add(args: Args) -> Result<VersionEntry> {
    // F2-1 — the clock enters here, once per command: the same `at`
    // stamps the entry's `indexed_at` and the written manifest, so two
    // records born of one command never differ by a millisecond.
    let at = Utc::now();
    super::refuse_if_server_running(&args.data_dir)?;

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
    let source_ref = args.r#ref.unwrap_or_else(|| format!("v{}", pkg.version));
    let source_url = args.repo_url.unwrap_or_else(|| {
        compose_default_repo_url(
            &index.registry_url,
            &index.naming,
            &kind,
            &pkg.group,
            &pkg.name,
        )
    });
    let entry = entry_from_tree(
        pkg_root,
        EntrySource {
            source_url,
            source_ref,
            resolved_commit: args.commit,
            registry: index.registry.clone(),
            indexed_at: at,
            indexed_by: default_generator(),
        },
    )?;

    // Truth first (PROP-044 `##LAW-NO-UNRECOVERABLE`), the `init`
    // order: the fact lands in the journal before the derived catalog
    // is written, so a failed `write_to` leaves a journal without a
    // catalog — recoverable by re-running the command — never a
    // catalog whose truth never existed.
    let record = JournalRecord {
        at,
        actor: default_generator(),
        event: Event::Published {
            entry: Box::new(entry.clone()),
        },
    };
    append(&journal_dir, &record)?;
    records.push(record);
    crate::aliases::projected(records, &args.data_dir)?
        .write_to(&args.data_dir, &WriteCtx { at })?;
    Ok(entry)
}

fn compose_default_repo_url(
    registry_url: &str,
    naming: &NamingConvention,
    kind: &PackageKind,
    group: &Group,
    name: &str,
) -> String {
    let trimmed = registry_url.trim_end_matches('/');
    let repo = naming.repo_name(kind, group, name);
    format!("{trimmed}/{repo}.git")
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
