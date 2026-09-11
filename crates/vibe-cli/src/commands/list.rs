//! `vibe list` — show installed packages from the lockfile.
//!
//! Spec: `VIBEVM-SPEC.md` §9.1.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use vibe_core::manifest::{LockedPackage, Lockfile};
use vibe_core::{PackageKind, machine_json_path};

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
        struct EmbeddedSourceJson<'a> {
            name: &'a str,
            kind: &'static str,
            source_url: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            source_ref: Option<&'a str>,
            resolved_commit: &'a str,
            tree_oid: &'a str,
            content_hash: &'a str,
            upstream_license: &'a str,
            license_path: String,
            license_url: &'a str,
            license_file_sha256: &'a str,
        }
        #[derive(Serialize)]
        struct JsonEntry<'a> {
            kind: &'a str,
            name: &'a str,
            version: String,
            bridge: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            registry: Option<&'a str>,
            source_url: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            source_ref: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            resolved_commit: Option<&'a str>,
            // The stub URL a redirect-resolved package came through
            // (PROP-002 §2.4.2); `source_url` above carries the target.
            // Diagnostic / auditing only — omitted for the common
            // non-redirected case.
            #[serde(skip_serializing_if = "Option::is_none")]
            via_redirect: Option<&'a str>,
            content_hash: &'a str,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            embedded_sources: Vec<EmbeddedSourceJson<'a>>,
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
                bridge: p.bridge,
                registry: p.registry.as_deref(),
                source_url: &p.source_url,
                source_ref: p.source_ref.as_deref(),
                resolved_commit: p.resolved_commit.as_deref(),
                via_redirect: p.via_redirect.as_deref(),
                content_hash: &p.content_hash,
                embedded_sources: p
                    .embedded_sources
                    .iter()
                    .map(|source| EmbeddedSourceJson {
                        name: &source.name,
                        kind: "git",
                        source_url: source.source_url.as_str(),
                        source_ref: source.source_ref.as_deref(),
                        resolved_commit: &source.resolved_commit,
                        tree_oid: &source.tree_oid,
                        content_hash: source.content_hash.as_str(),
                        upstream_license: &source.upstream_license,
                        license_path: machine_json_path(&source.license_path),
                        license_url: &source.license_url,
                        license_file_sha256: source.license_file_sha256.as_str(),
                    })
                    .collect(),
                boot_snippet: p.boot_snippet.as_deref(),
                files_written: p
                    .files_written
                    .iter()
                    .map(|path| machine_json_path(path))
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
    let mut r_w = "ROLE".len();
    for p in &filtered {
        k_w = k_w.max(p.kind.as_str().len());
        n_w = n_w.max(p.name.len());
        v_w = v_w.max(p.version.to_string().len());
        r_w = r_w.max(if p.bridge {
            "BRIDGE".len()
        } else {
            "package".len()
        });
    }
    println!(
        "{:<k_w$}  {:<n_w$}  {:<v_w$}  {:<r_w$}  BOOT SNIPPET",
        "KIND", "NAME", "VERSION", "ROLE"
    );
    for p in &filtered {
        println!(
            "{:<k_w$}  {:<n_w$}  {:<v_w$}  {:<r_w$}  {}",
            p.kind.as_str(),
            p.name,
            p.version.to_string(),
            if p.bridge { "BRIDGE" } else { "package" },
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
            for source in &p.embedded_sources {
                println!("    embedded source {} (git):", source.name);
                println!("      url:              {}", source.source_url);
                if let Some(reference) = &source.source_ref {
                    println!("      ref hint:         {reference}");
                }
                println!("      resolved commit:  {}", source.resolved_commit);
                println!("      tree oid:         {}", source.tree_oid);
                println!("      content hash:     {}", source.content_hash);
                println!("      upstream license: {}", source.upstream_license);
                println!("      license path:     {}", source.license_path.display());
                println!("      license url:      {}", source.license_url);
                println!("      license hash:     {}", source.license_file_sha256);
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
