//! `vibe install <kind>:<name>[@version] …` — plan → confirm → apply.
//!
//! Spec: `VIBEVM-SPEC.md` §5.6, §9.1, §11.1.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use dialoguer::Confirm;
use serde::Serialize;
use vibe_core::PackageRef;
use vibe_core::manifest::{Lockfile, ProjectManifest};
use vibe_install::{
    InstallError, InstallPlan, WriteKind, apply_install, plan_install, register_installed,
};
use vibe_registry::{GitRegistry, LocalRegistry, Registry};

use crate::cli::InstallArgs;
use crate::output;

pub fn run(ctx: &output::Context, args: InstallArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest = load_project_manifest(&project_root)?;
    let mut lockfile = load_or_empty_lockfile(&project_root)?;
    let registry: Box<dyn Registry> = resolve_registry(&args, &manifest)?;

    // Cache layout matches §8.3: `.vibe/cache/<kind>/<name>/<version>/`.
    let cache_root = project_root.join(".vibe/cache");
    fs::create_dir_all(&cache_root)
        .with_context(|| format!("creating cache dir `{}`", cache_root.display()))?;

    // Plan every package before asking the user for approval. If any
    // package's plan fails, we abort without prompting.
    let mut plans: Vec<InstallPlan> = Vec::new();
    for raw in &args.packages {
        let pkgref = PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`"))?;
        ctx.heading(&format!("Resolving {pkgref}…"));
        let resolved = registry.resolve(&pkgref)?;
        let cached = registry.fetch(&resolved, &cache_root)?;
        // Each plan must observe the updated view: if we install two packages
        // in one invocation and both contribute a boot snippet, the second
        // plan must see the first one's intended writes. We do the cheap
        // thing: call plan_install against the lockfile + a shadow of the
        // already-planned targets.
        let plan = plan_install(&project_root, &lockfile, cached)?;
        check_cross_plan_conflicts(&plans, &plan)?;
        plans.push(plan);
    }

    // Show combined plan.
    present_plans(ctx, &project_root, &plans);

    // Confirm (unless --assume-yes or --json or not a TTY).
    let approved = if args.assume_yes || ctx.is_json() {
        true
    } else if !console::user_attended() {
        // No TTY → refuse to apply without explicit --assume-yes. This matches
        // the book's "ask a human" discipline for any destructive action.
        bail!(
            "no TTY available for confirmation; re-run with `--assume-yes` to apply this plan non-interactively"
        );
    } else {
        let prompt = format!(
            "Apply this install plan ({} file{} across {} package{})?",
            total_writes(&plans),
            if total_writes(&plans) == 1 { "" } else { "s" },
            plans.len(),
            if plans.len() == 1 { "" } else { "s" },
        );
        Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()
            .context("reading user confirmation")?
    };

    if !approved {
        return Err(InstallError::UserDeclined.into());
    }

    // Apply each plan in turn; update lockfile after each success.
    let mut applied: Vec<AppliedReport> = Vec::new();
    for plan in &plans {
        let label = plan.package_label();
        ctx.step(&format!("Installing {label}"));
        let written = apply_install(plan)?;
        let written_count = written.len();
        register_installed(
            &mut lockfile,
            plan,
            written.clone(),
            crate::commands::init::current_timestamp_utc(),
        );
        applied.push(AppliedReport {
            package: label,
            files_written: written_count,
            paths: written
                .into_iter()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .collect(),
        });
    }

    // Save lockfile on disk.
    lockfile.write(project_root.join(Lockfile::FILENAME))?;

    emit_report(ctx, &applied, &project_root)?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct AppliedReport {
    package: String,
    files_written: usize,
    paths: Vec<String>,
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(ProjectManifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

fn load_project_manifest(root: &Path) -> Result<ProjectManifest> {
    let path = root.join(ProjectManifest::FILENAME);
    Ok(ProjectManifest::read(&path)?)
}

fn load_or_empty_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if path.exists() {
        Ok(Lockfile::read(&path)?)
    } else {
        Ok(Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            crate::commands::init::current_timestamp_utc(),
        ))
    }
}

/// Build the concrete [`Registry`] for this invocation.
///
/// Precedence (matches `VIBEVM-SPEC.md` §9.1 and
/// [`spec://vibevm/common/PROP-000#registry`]):
/// 1. `--registry <path>` — always a local directory (M0 behaviour).
/// 2. `[registry].url` in `vibe.toml`:
///    - `file://<abs>` — local directory.
///    - anything else (`git+ssh://…`, `ssh://…`, `https://…`,
///      `git@host:…`) — git-backed registry under
///      `~/.vibe/registries/<hash>/`.
pub(crate) fn resolve_registry(
    args: &InstallArgs,
    manifest: &ProjectManifest,
) -> Result<Box<dyn Registry>> {
    if let Some(explicit) = &args.registry {
        let p = explicit
            .canonicalize()
            .with_context(|| format!("registry path `{}`", explicit.display()))?;
        let p = super::init::strip_unc_public(p);
        return Ok(Box::new(
            LocalRegistry::new(p.clone())
                .map_err(|e| anyhow!("failed to open registry at `{}`: {e}", p.display()))?,
        ));
    }
    let Some(reg) = manifest.primary_registry() else {
        bail!(
            "no registry configured. Pass `--registry <path>` or add a `[[registry]]` entry to `vibe.toml`."
        );
    };
    if let Some(path) = parse_file_uri(&reg.url) {
        let p = path
            .canonicalize()
            .with_context(|| format!("registry `{}`", path.display()))?;
        let p = super::init::strip_unc_public(p);
        return Ok(Box::new(
            LocalRegistry::new(p.clone())
                .map_err(|e| anyhow!("failed to open registry at `{}`: {e}", p.display()))?,
        ));
    }
    // Anything else is a git URL. GitRegistry::open handles clone +
    // freshness TTL internally.
    let git = GitRegistry::open(&reg.url, &reg.r#ref)
        .with_context(|| format!("opening git registry `{}`", reg.url))?;
    Ok(Box::new(git))
}

pub(crate) fn parse_file_uri(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    // Accept both `file:///C:/...` (tri-slash) and `file:///home/...`.
    let mut trimmed = rest.trim_start_matches('/');
    // On Windows paths of the form `C:/Users/...` have the drive letter at the
    // start. Keep them as-is.
    let looks_like_windows = trimmed
        .chars()
        .nth(1)
        .map(|c| c == ':')
        .unwrap_or(false);
    if !looks_like_windows {
        // It's a Unix absolute path; re-prepend the `/`.
        trimmed = rest; // restore leading slashes
    }
    Some(PathBuf::from(trimmed))
}

fn check_cross_plan_conflicts(prior: &[InstallPlan], new: &InstallPlan) -> Result<()> {
    use std::collections::HashSet;
    let prior_targets: HashSet<&PathBuf> =
        prior.iter().flat_map(|p| p.writes.iter().map(|w| &w.target_rel)).collect();
    for w in &new.writes {
        if prior_targets.contains(&w.target_rel) {
            bail!(
                "two packages in this install would write to the same path `{}`",
                w.target_rel.display()
            );
        }
    }
    let prior_snippets: HashSet<&str> = prior
        .iter()
        .filter_map(|p| p.boot_snippet_filename.as_deref())
        .collect();
    if let Some(snippet) = new.boot_snippet_filename.as_deref()
        && prior_snippets.contains(snippet)
    {
        bail!("two packages in this install share boot snippet filename `{snippet}`");
    }
    Ok(())
}

fn total_writes(plans: &[InstallPlan]) -> usize {
    plans.iter().map(|p| p.writes.len()).sum()
}

fn present_plans(ctx: &output::Context, project_root: &Path, plans: &[InstallPlan]) {
    if ctx.is_json() {
        #[derive(Serialize)]
        struct JsonPlanEntry<'a> {
            package: String,
            version: String,
            source_url: &'a str,
            content_hash: &'a str,
            writes: Vec<String>,
            boot_snippet: Option<&'a str>,
        }
        let payload: Vec<JsonPlanEntry<'_>> = plans
            .iter()
            .map(|p| JsonPlanEntry {
                package: format!(
                    "{}:{}",
                    p.cached.resolved.kind, p.cached.resolved.name
                ),
                version: p.cached.resolved.version.to_string(),
                source_url: p.cached.source_uri.as_str(),
                content_hash: p.cached.content_hash.as_str(),
                writes: p
                    .writes
                    .iter()
                    .map(|w| w.target_rel.to_string_lossy().to_string())
                    .collect(),
                boot_snippet: p.boot_snippet_filename.as_deref(),
            })
            .collect();
        let envelope = serde_json::json!({
            "command": "install:plan",
            "plans": payload,
        });
        let _ = ctx.emit_json(&envelope);
        return;
    }
    if ctx.is_quiet() {
        return;
    }
    for plan in plans {
        ctx.heading(&format!("\nPlan for {}", plan.package_label()));
        for w in &plan.writes {
            let prefix = match w.kind {
                WriteKind::Regular => "create",
                WriteKind::BootSnippet => "boot  ",
            };
            let rel = w
                .target_abs
                .strip_prefix(project_root)
                .unwrap_or(&w.target_abs);
            let rel_s = rel.to_string_lossy().replace('\\', "/");
            println!("  {prefix}  {}", rel_s);
        }
    }
    println!();
}

fn emit_report(
    ctx: &output::Context,
    applied: &[AppliedReport],
    project_root: &Path,
) -> Result<()> {
    if ctx.is_json() {
        let payload = serde_json::json!({
            "ok": true,
            "command": "install",
            "project": project_root.display().to_string(),
            "installed": applied,
        });
        ctx.emit_json(&payload)?;
        return Ok(());
    }
    let total_files: usize = applied.iter().map(|a| a.files_written).sum();
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe install: {} package{}, {total_files} file{} written",
            applied.len(),
            if applied.len() == 1 { "" } else { "s" },
            if total_files == 1 { "" } else { "s" },
        ));
        return Ok(());
    }
    for a in applied {
        for p in &a.paths {
            ctx.created(p);
        }
    }
    ctx.summary(&format!(
        "\nInstalled {} package{} ({} file{} written).",
        applied.len(),
        if applied.len() == 1 { "" } else { "s" },
        total_files,
        if total_files == 1 { "" } else { "s" },
    ));
    Ok(())
}
