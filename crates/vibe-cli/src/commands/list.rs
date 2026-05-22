//! `vibe list` — show installed packages from the lockfile.
//!
//! Spec: `VIBEVM-SPEC.md` §9.1.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use vibe_core::PackageKind;
use vibe_core::manifest::{LockedPackage, Lockfile};

use crate::cli::ListArgs;
use crate::output;

pub fn run(ctx: &output::Context, args: ListArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let lockfile = load_lockfile(&project_root)?;

    let kind_filter = match &args.kind {
        None => None,
        Some(s) => Some(PackageKind::from_str(s).map_err(|e| anyhow!("{e}"))?),
    };

    let filtered: Vec<&LockedPackage> = lockfile
        .packages
        .iter()
        .filter(|p| kind_filter.map(|k| k == p.kind).unwrap_or(true))
        .collect();

    if ctx.is_json() {
        #[derive(Serialize)]
        struct LockedSubskillJson<'a> {
            path: &'a str,
            delivery: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            describes: Option<&'a str>,
        }
        #[derive(Serialize)]
        struct JsonEntry<'a> {
            kind: &'a str,
            name: &'a str,
            version: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            registry: Option<&'a str>,
            source_url: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            source_ref: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            resolved_commit: Option<&'a str>,
            content_hash: &'a str,
            boot_snippet: Option<&'a str>,
            files_written: Vec<String>,
            #[serde(skip_serializing_if = "std::ops::Not::not")]
            overridden: bool,
            // PROP-003 r2 lockfile-v3 fields. Always emitted in JSON
            // (not text — text-mode shows them only with `--verbose`)
            // so machine consumers see the full state regardless of
            // human-output formatting.
            #[serde(skip_serializing_if = "Vec::is_empty")]
            features: Vec<&'a str>,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            subskills_active: Vec<LockedSubskillJson<'a>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            describes: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            language: Option<&'a str>,
        }
        let entries: Vec<JsonEntry<'_>> = filtered
            .iter()
            .map(|p| JsonEntry {
                kind: p.kind.as_str(),
                name: &p.name,
                version: p.version.to_string(),
                registry: p.registry.as_deref(),
                source_url: &p.source_url,
                source_ref: p.source_ref.as_deref(),
                resolved_commit: p.resolved_commit.as_deref(),
                content_hash: &p.content_hash,
                boot_snippet: p.boot_snippet.as_deref(),
                files_written: p
                    .files_written
                    .iter()
                    .map(|f| f.to_string_lossy().to_string())
                    .collect(),
                overridden: p.overridden,
                features: p.features.iter().map(|s| s.as_str()).collect(),
                subskills_active: p
                    .subskills_active
                    .iter()
                    .map(|s| LockedSubskillJson {
                        path: &s.path,
                        delivery: &s.delivery,
                        describes: s.describes.as_deref(),
                    })
                    .collect(),
                describes: p.describes.as_deref(),
                language: p.language.as_deref(),
            })
            .collect();
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "list",
            "project": project_root.display().to_string(),
            "count": entries.len(),
            "packages": entries,
        }))?;
        return Ok(());
    }

    if filtered.is_empty() {
        ctx.summary("(no packages installed)");
        return Ok(());
    }

    if ctx.is_quiet() {
        let joined: Vec<String> = filtered
            .iter()
            .map(|p| format!("{}:{}@{}", p.kind, p.name, p.version))
            .collect();
        ctx.summary(&joined.join(", "));
        return Ok(());
    }

    // Pretty table.
    let mut k_w = "KIND".len();
    let mut n_w = "NAME".len();
    let mut v_w = "VERSION".len();
    for p in &filtered {
        k_w = k_w.max(p.kind.as_str().len());
        n_w = n_w.max(p.name.len());
        v_w = v_w.max(p.version.to_string().len());
    }
    println!(
        "{:<k_w$}  {:<n_w$}  {:<v_w$}  BOOT SNIPPET",
        "KIND", "NAME", "VERSION"
    );
    for p in &filtered {
        println!(
            "{:<k_w$}  {:<n_w$}  {:<v_w$}  {}",
            p.kind.as_str(),
            p.name,
            p.version.to_string(),
            p.boot_snippet.as_deref().unwrap_or("—"),
        );
        if args.verbose {
            if !p.features.is_empty() {
                println!("    features:  {}", p.features.join(", "));
            }
            if !p.subskills_active.is_empty() {
                let subs: Vec<String> = p
                    .subskills_active
                    .iter()
                    .map(|s| format!("{} ({})", s.path, s.delivery))
                    .collect();
                println!("    subskills: {}", subs.join(", "));
            }
            if let Some(d) = &p.describes {
                println!("    describes: {d}");
            }
            if let Some(l) = &p.language {
                println!("    language:  {l}");
            }
        }
    }
    println!(
        "\n{} package{} installed.",
        filtered.len(),
        if filtered.len() == 1 { "" } else { "s" }
    );
    Ok(())
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join("vibe.toml").exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

fn load_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if !path.exists() {
        return Ok(Lockfile::empty("vibe (no-lockfile)", "0"));
    }
    Ok(Lockfile::read(&path)?)
}
