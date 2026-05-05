//! `vibe show <subcommand>` — inspect computed project state.
//!
//! v0 ships two pure-inspection subcommands; the M1.5+ runner-aware
//! ones (`graph`, `node`, `plan`) land alongside the LLM-build
//! pipeline.
//!
//! - `vibe show effective` — concatenate `spec/boot/*.md` (sorted by
//!   the canonical `NN-` prefix) and every installed package's
//!   `files_written` (in lockfile order), each preceded by a
//!   `spec://` provenance header so a cold reader knows which
//!   package contributed which content.
//! - `vibe show config` — dump the effective configuration: every
//!   `[[registry]]`, `[[mirror]]`, `[[override]]` from `vibe.toml`,
//!   plus the runtime knobs read from environment variables, each
//!   tagged with `provenance` so the operator sees where a value
//!   actually came from.
//!
//! Spec: `VIBEVM-SPEC.md` §9.5 (configuration sources / provenance),
//! §4.6 (effective spec), ROADMAP §M1.4.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::manifest::{Lockfile, ProjectManifest};
use vibe_core::user_config::UserConfig;

use crate::cli::{
    ShowArgs, ShowConfigArgs, ShowEffectiveArgs, ShowFeaturesArgs, ShowPurlsArgs,
    ShowSubcommand, ShowSubskillsArgs,
};
use crate::output;

pub fn run(ctx: &output::Context, args: ShowArgs) -> Result<()> {
    match args.command {
        ShowSubcommand::Effective(sub) => run_effective(ctx, sub),
        ShowSubcommand::Config(sub) => run_config(ctx, sub),
        ShowSubcommand::Features(sub) => run_features(ctx, sub),
        ShowSubcommand::Subskills(sub) => run_subskills(ctx, sub),
        ShowSubcommand::Purls(sub) => run_purls(ctx, sub),
    }
}

// ===================== show effective =====================

#[derive(Debug, Serialize)]
struct EffectiveReport {
    ok: bool,
    command: &'static str,
    project: String,
    sections: Vec<EffectiveSection>,
}

#[derive(Debug, Serialize)]
struct EffectiveSection {
    /// `spec://` URI for this section. Composed from the originating
    /// package's `(kind, name)` plus the project-relative path.
    /// User-owned files (the boot foundation, WAL) get
    /// `spec://project/...`.
    spec_uri: String,
    /// Project-relative path of the file that produced this section.
    path: String,
    /// Origin of the section: `"package:<kind>:<name>@<version>"`,
    /// `"user"`, or `"wal"`.
    origin: String,
    /// File content, verbatim.
    body: String,
}

fn run_effective(ctx: &output::Context, args: ShowEffectiveArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let lockfile_path = project_root.join(Lockfile::FILENAME);
    let lockfile = if lockfile_path.exists() {
        Some(Lockfile::read(&lockfile_path).with_context(|| {
            format!("reading `{}`", lockfile_path.display())
        })?)
    } else {
        None
    };

    let mut sections: Vec<EffectiveSection> = Vec::new();

    // 1. Boot dir — sorted by NN- prefix. Each file gets a
    // user-or-package origin: the lockfile's `boot_snippet` field
    // names which package contributed which `NN-…` file. Files not
    // claimed by any lockfile entry (00-core / 90-user / hand-edited)
    // surface as `user`.
    let boot_dir = project_root.join("spec/boot");
    if boot_dir.is_dir() {
        let mut entries: Vec<PathBuf> = fs::read_dir(&boot_dir)
            .with_context(|| format!("reading `{}`", boot_dir.display()))?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type()
                    .map(|t| t.is_file())
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .map(|x| x == "md")
                    .unwrap_or(false)
            })
            .collect();
        entries.sort();
        for path in entries {
            let filename = path.file_name().unwrap().to_string_lossy().into_owned();
            let rel = format!("spec/boot/{filename}");
            let origin = boot_origin(&filename, lockfile.as_ref());
            let spec_uri = format!("spec://project/boot/{filename}");
            let body = fs::read_to_string(&path)
                .with_context(|| format!("reading `{}`", path.display()))?;
            sections.push(EffectiveSection {
                spec_uri,
                path: rel,
                origin,
                body,
            });
        }
    }

    // 2. WAL — always one section, distinct origin.
    let wal = project_root.join("spec/WAL.md");
    if wal.is_file() {
        let body = fs::read_to_string(&wal)
            .with_context(|| format!("reading `{}`", wal.display()))?;
        sections.push(EffectiveSection {
            spec_uri: "spec://project/WAL".to_string(),
            path: "spec/WAL.md".to_string(),
            origin: "wal".to_string(),
            body,
        });
    }

    // 3. Per package, in lockfile order: every file in `files_written`
    // that we haven't already emitted (skip the boot snippet — it
    // landed in step 1). Lockfile order is the install order, which
    // is the same order the resolver pinned the graph in. Stable
    // enough for cold-reader use.
    if let Some(lockfile) = &lockfile {
        for entry in &lockfile.packages {
            let pkg_uri_root = format!(
                "spec://{}/{}/{}",
                entry.kind, entry.name, entry.version
            );
            let mut paths: Vec<PathBuf> = entry
                .files_written
                .iter()
                .map(|p| normalize_rel_path(p))
                .collect();
            paths.sort();
            for rel in paths {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if rel_str.starts_with("spec/boot/") {
                    // Already emitted under step 1.
                    continue;
                }
                let abs = project_root.join(&rel);
                if !abs.is_file() {
                    // Missing file — surface as a section with empty
                    // body and a warning header instead of crashing.
                    // `vibe check` exists for the dedicated linter
                    // path; `vibe show effective` is best-effort by
                    // design.
                    sections.push(EffectiveSection {
                        spec_uri: format!(
                            "{}/{}",
                            pkg_uri_root,
                            rel_str.trim_start_matches("spec/")
                        ),
                        path: rel_str.clone(),
                        origin: format!(
                            "package:{}:{}@{} (MISSING ON DISK)",
                            entry.kind, entry.name, entry.version
                        ),
                        body: String::new(),
                    });
                    continue;
                }
                let body = fs::read_to_string(&abs)
                    .with_context(|| format!("reading `{}`", abs.display()))?;
                let suffix = rel_str.trim_start_matches("spec/");
                sections.push(EffectiveSection {
                    spec_uri: format!("{pkg_uri_root}/{suffix}"),
                    path: rel_str,
                    origin: format!(
                        "package:{}:{}@{}",
                        entry.kind, entry.name, entry.version
                    ),
                    body,
                });
            }
        }
    }

    if ctx.is_json() {
        let payload = EffectiveReport {
            ok: true,
            command: "show:effective",
            project: project_root.display().to_string(),
            sections,
        };
        ctx.emit_json(&payload)?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe show effective: {} section{} from `{}`",
            sections.len(),
            if sections.len() == 1 { "" } else { "s" },
            project_root.display()
        ));
        return Ok(());
    }
    if sections.is_empty() {
        ctx.summary(&format!(
            "vibe show effective: nothing to materialise — `{}` has no spec/boot files, no WAL, and an empty lockfile",
            project_root.display()
        ));
        return Ok(());
    }
    for section in &sections {
        println!("--- {} ({})", section.spec_uri, section.origin);
        println!("--- path: {}", section.path);
        println!();
        // Trim trailing newline so we don't double up before the next
        // separator. The original file's content is preserved
        // verbatim modulo that trailing trim.
        if section.body.ends_with('\n') {
            print!("{}", section.body);
        } else {
            println!("{}", section.body);
        }
        println!();
    }
    ctx.summary(&format!(
        "vibe show effective: {} sections, project `{}`",
        sections.len(),
        project_root.display()
    ));
    Ok(())
}

fn boot_origin(filename: &str, lockfile: Option<&Lockfile>) -> String {
    if filename == "00-core.md" || filename == "90-user.md" {
        return "user".to_string();
    }
    let Some(lockfile) = lockfile else {
        return "user".to_string();
    };
    if let Some(pkg) = lockfile
        .packages
        .iter()
        .find(|p| p.boot_snippet.as_deref() == Some(filename))
    {
        return format!("package:{}:{}@{}", pkg.kind, pkg.name, pkg.version);
    }
    "user".to_string()
}

fn normalize_rel_path(p: &Path) -> PathBuf {
    PathBuf::from(p.to_string_lossy().replace('\\', "/"))
}

// ===================== show config =====================

#[derive(Debug, Serialize)]
struct ConfigReport {
    ok: bool,
    command: &'static str,
    project: String,
    project_name: String,
    project_version: String,
    registries: Vec<ConfigRegistry>,
    mirrors: Vec<ConfigMirror>,
    overrides: Vec<ConfigOverride>,
    env: Vec<ConfigEnvEntry>,
    user_config: ConfigUserConfigSummary,
}

#[derive(Debug, Serialize)]
struct ConfigRegistry {
    name: String,
    url: String,
    #[serde(rename = "ref")]
    refname: String,
    naming: String,
    /// `"vibe.toml"` for v0; future config layers (user-level,
    /// CLI overrides) will surface here.
    provenance: &'static str,
}

#[derive(Debug, Serialize)]
struct ConfigMirror {
    of: String,
    url: String,
    priority: i32,
    provenance: &'static str,
}

#[derive(Debug, Serialize)]
struct ConfigOverride {
    pkgref: String,
    source_url: String,
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    refname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    provenance: &'static str,
}

#[derive(Debug, Serialize)]
struct ConfigEnvEntry {
    name: &'static str,
    value: Option<String>,
    /// `"env"` — set in the live environment;
    /// `"redacted"` — set in env, sensitive (token-shaped) — bytes never printed;
    /// `"user-config"` — defaulted via `~/.config/vibe/config.toml [env]`;
    /// `"default"` — unset, built-in fallback applies.
    provenance: &'static str,
    /// Short description of what the variable controls.
    description: &'static str,
}

#[derive(Debug, Serialize)]
struct ConfigUserConfigSummary {
    /// Path the loader consulted. `null` if no path could be resolved
    /// (e.g. `HOME` unset on a misconfigured CI).
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    /// `true` when the file exists and parses; `false` if missing
    /// (the layer is optional and that is the common case).
    loaded: bool,
    /// When `loaded = false` and the file IS present but failed to
    /// parse, the error is surfaced here so the operator sees that
    /// their config layer is silently inert.
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

const CONFIG_ENV_VARS: &[(&str, &str, bool /* sensitive */)] = &[
    (
        "VIBE_REGISTRY_CACHE",
        "Override the default `~/.vibe/registries/` cache root.",
        false,
    ),
    (
        "VIBE_LOG",
        "Tracing filter (reads `tracing-subscriber::EnvFilter`).",
        false,
    ),
    (
        "VIBEVM_PUBLISH_TOKEN_GITHUB",
        "Publish token for `vibe registry publish` against GitHub. Wins over the legacy `VIBEVM_PUBLISH_TOKEN` and over `~/.vibevm/github.publish.token`.",
        true,
    ),
    (
        "VIBEVM_PUBLISH_TOKEN_GITVERSE",
        "Publish token for `vibe registry publish` against GitVerse (publishing is currently a stub; reserved for when the GitVerse public API gains parity).",
        true,
    ),
    (
        "VIBEVM_PUBLISH_TOKEN",
        "Legacy host-agnostic publish token. Used only when no `VIBEVM_PUBLISH_TOKEN_<HOST>` is set; prefer the per-host form in new setups.",
        true,
    ),
];

fn run_config(ctx: &output::Context, args: ShowConfigArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(ProjectManifest::FILENAME);
    let manifest = ProjectManifest::read(&manifest_path).with_context(|| {
        format!("reading `{}`", manifest_path.display())
    })?;

    let registries: Vec<ConfigRegistry> = manifest
        .registries
        .iter()
        .map(|r| ConfigRegistry {
            name: r.name.clone(),
            url: r.url.clone(),
            refname: r.r#ref.clone(),
            naming: naming_label(r.naming),
            provenance: "vibe.toml",
        })
        .collect();
    let mirrors: Vec<ConfigMirror> = manifest
        .mirrors
        .iter()
        .map(|m| ConfigMirror {
            of: m.of.clone(),
            url: m.url.clone(),
            priority: m.priority,
            provenance: "vibe.toml",
        })
        .collect();
    let overrides: Vec<ConfigOverride> = manifest
        .overrides
        .iter()
        .map(|o| ConfigOverride {
            pkgref: o.pkgref.clone(),
            source_url: o.source_url.clone(),
            refname: o.r#ref.clone(),
            reason: o.reason.clone(),
            provenance: "vibe.toml",
        })
        .collect();

    // Read the user-level config layer (VIBEVM-SPEC §9.5 step 4).
    // Errors here are surfaced via `user_config_summary` rather than
    // failing the whole `vibe show config` invocation — the layer
    // is optional, and even a malformed file should not block
    // inspection of the other layers.
    let user_config_path = UserConfig::default_path();
    let (user_config, user_config_summary) = match &user_config_path {
        Some(path) => match UserConfig::load_from(path) {
            Ok(cfg) => {
                let loaded = path.exists();
                (
                    cfg,
                    ConfigUserConfigSummary {
                        path: Some(forward_slash_display(path)),
                        loaded,
                        error: None,
                    },
                )
            }
            Err(e) => (
                UserConfig::default(),
                ConfigUserConfigSummary {
                    path: Some(forward_slash_display(path)),
                    loaded: false,
                    error: Some(e.to_string()),
                },
            ),
        },
        None => (
            UserConfig::default(),
            ConfigUserConfigSummary {
                path: None,
                loaded: false,
                error: None,
            },
        ),
    };

    // After startup promotion, the live env contains both
    // operator-set and user-config-defaulted values. To keep
    // provenance honest, we cross-reference the names that
    // `main::promote_user_config_env` actually wrote — anything in
    // that set is sourced from user-config; anything else with a
    // live value is operator-set.
    let promoted = crate::promoted_env_names();
    let env: Vec<ConfigEnvEntry> = CONFIG_ENV_VARS
        .iter()
        .map(|(name, desc, sensitive)| {
            let live = std::env::var(*name).ok();
            let from_user_config = promoted.contains(*name);
            let (rendered, provenance) = match (&live, from_user_config, sensitive) {
                (Some(_), false, true) => (
                    Some("(redacted; set in environment)".to_string()),
                    "redacted",
                ),
                (Some(_), true, true) => (
                    Some("(redacted; defaulted in user config)".to_string()),
                    "redacted",
                ),
                (Some(v), false, false) => (Some(v.clone()), "env"),
                (Some(v), true, false) => (Some(v.clone()), "user-config"),
                (None, _, _) => (None, "default"),
            };
            ConfigEnvEntry {
                name,
                value: rendered,
                provenance,
                description: desc,
            }
        })
        .collect();
    // `user_config` (the parsed file) is no longer needed for env
    // resolution — promotion at startup baked its values into the
    // process env. We still load it for the summary block so the
    // operator can see whether the file is loaded / where the
    // loader looked.
    let _ = user_config;

    if ctx.is_json() {
        let payload = ConfigReport {
            ok: true,
            command: "show:config",
            project: project_root.display().to_string(),
            project_name: manifest.project.name.clone(),
            project_version: manifest.project.version.clone(),
            registries,
            mirrors,
            overrides,
            env,
            user_config: user_config_summary,
        };
        ctx.emit_json(&payload)?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe show config: {} registries, {} mirror{}, {} override{}, {} env entries",
            registries.len(),
            mirrors.len(),
            if mirrors.len() == 1 { "" } else { "s" },
            overrides.len(),
            if overrides.len() == 1 { "" } else { "s" },
            env.len(),
        ));
        return Ok(());
    }

    println!(
        "Project: {} {} ({})",
        manifest.project.name,
        manifest.project.version,
        project_root.display()
    );
    println!();
    println!(
        "Registries ({}; primary first):",
        registries.len()
    );
    if registries.is_empty() {
        println!("  (none configured)");
    } else {
        for (i, r) in registries.iter().enumerate() {
            let primary = if i == 0 { " (primary)" } else { "" };
            println!(
                "  {}. {}{}\n     url:    {}\n     ref:    {}\n     naming: {}\n     source: {}",
                i + 1,
                r.name,
                primary,
                r.url,
                r.refname,
                r.naming,
                r.provenance
            );
        }
    }
    println!();
    println!("Mirrors ({}):", mirrors.len());
    if mirrors.is_empty() {
        println!("  (none configured)");
    } else {
        for m in &mirrors {
            println!(
                "  - of=`{}` priority={} url={} (source: {})",
                m.of, m.priority, m.url, m.provenance
            );
        }
    }
    println!();
    println!("Overrides ({}):", overrides.len());
    if overrides.is_empty() {
        println!("  (none configured)");
    } else {
        for o in &overrides {
            let ref_part = o
                .refname
                .as_deref()
                .map(|r| format!("@{r}"))
                .unwrap_or_default();
            let reason_part = o
                .reason
                .as_deref()
                .map(|r| format!(" — {r}"))
                .unwrap_or_default();
            println!(
                "  - {} → {}{}{} (source: {})",
                o.pkgref, o.source_url, ref_part, reason_part, o.provenance
            );
        }
    }
    println!();
    match &user_config_summary {
        ConfigUserConfigSummary { path: Some(p), loaded: true, .. } => {
            println!("User config: {p}  (loaded)");
        }
        ConfigUserConfigSummary { path: Some(p), loaded: false, error: Some(err) } => {
            println!("User config: {p}  (parse error — {err})");
        }
        ConfigUserConfigSummary { path: Some(p), loaded: false, error: None } => {
            println!("User config: {p}  (not present)");
        }
        ConfigUserConfigSummary { path: None, .. } => {
            println!(
                "User config: (no path resolved — set HOME / XDG_CONFIG_HOME / VIBEVM_USER_CONFIG)"
            );
        }
    }
    println!();
    println!("Environment:");
    for e in &env {
        let value_part = match (&e.value, e.provenance) {
            (Some(v), "redacted") => v.clone(),
            (Some(v), _) => format!("`{v}`"),
            (None, _) => "(unset; using built-in default)".to_string(),
        };
        println!(
            "  {}  [source: {}]\n    {}\n    {}",
            e.name, e.provenance, e.description, value_part
        );
    }

    ctx.summary(&format!(
        "\nvibe show config: {} registries, {} mirrors, {} overrides, {} env entries",
        registries.len(),
        mirrors.len(),
        overrides.len(),
        env.len()
    ));
    Ok(())
}

fn forward_slash_display(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = s.strip_prefix("//?/") {
        s = stripped.to_string();
    }
    s
}

fn naming_label(naming: vibe_core::manifest::NamingConvention) -> String {
    match naming {
        vibe_core::manifest::NamingConvention::KindName => "kind-name",
        vibe_core::manifest::NamingConvention::Name => "name",
        vibe_core::manifest::NamingConvention::KindSlashName => "kind/name",
    }
    .to_string()
}

// ===================== shared =====================

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(ProjectManifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first or pass `--path <dir>` pointing at a project root",
            stripped.display()
        );
    }
    Ok(stripped)
}

// ===================== show features =====================

#[derive(Debug, Serialize)]
struct FeaturesEntry {
    package: String,
    features: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FeaturesReport {
    ok: bool,
    command: &'static str,
    project: String,
    packages: Vec<FeaturesEntry>,
    /// Total active feature lines, project-wide.
    total: usize,
}

fn run_features(ctx: &output::Context, args: ShowFeaturesArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let lockfile_path = project_root.join(vibe_core::manifest::Lockfile::FILENAME);
    let lockfile = if lockfile_path.exists() {
        vibe_core::manifest::Lockfile::read(&lockfile_path)?
    } else {
        vibe_core::manifest::Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            super::init::current_timestamp_utc(),
        )
    };

    let mut entries: Vec<FeaturesEntry> = Vec::new();
    let mut total = 0usize;
    for p in &lockfile.packages {
        if p.features.is_empty() {
            continue;
        }
        total += p.features.len();
        entries.push(FeaturesEntry {
            package: format!("{}:{}", p.kind, p.name),
            features: p.features.clone(),
        });
    }

    if ctx.is_json() {
        ctx.emit_json(&FeaturesReport {
            ok: true,
            command: "show:features",
            project: project_root.display().to_string(),
            packages: entries,
            total,
        })?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe show features: {} active feature{} across {} package{}",
            total,
            if total == 1 { "" } else { "s" },
            entries.len(),
            if entries.len() == 1 { "" } else { "s" },
        ));
        return Ok(());
    }
    if entries.is_empty() {
        ctx.summary("(no features active in this project)");
        return Ok(());
    }
    for e in &entries {
        ctx.heading(&e.package);
        for f in &e.features {
            ctx.step(f);
        }
    }
    ctx.summary(&format!(
        "\n{} active feature{} across {} package{}",
        total,
        if total == 1 { "" } else { "s" },
        entries.len(),
        if entries.len() == 1 { "" } else { "s" },
    ));
    Ok(())
}

// ===================== show subskills =====================

#[derive(Debug, Serialize)]
struct SubskillEntry {
    path: String,
    delivery: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    describes: Option<String>,
}

#[derive(Debug, Serialize)]
struct SubskillsPackageEntry {
    package: String,
    subskills: Vec<SubskillEntry>,
}

#[derive(Debug, Serialize)]
struct SubskillsReport {
    ok: bool,
    command: &'static str,
    project: String,
    packages: Vec<SubskillsPackageEntry>,
    total: usize,
}

fn run_subskills(ctx: &output::Context, args: ShowSubskillsArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let lockfile_path = project_root.join(vibe_core::manifest::Lockfile::FILENAME);
    let lockfile = if lockfile_path.exists() {
        vibe_core::manifest::Lockfile::read(&lockfile_path)?
    } else {
        vibe_core::manifest::Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            super::init::current_timestamp_utc(),
        )
    };

    let mut entries: Vec<SubskillsPackageEntry> = Vec::new();
    let mut total = 0usize;
    for p in &lockfile.packages {
        if p.subskills_active.is_empty() {
            continue;
        }
        total += p.subskills_active.len();
        entries.push(SubskillsPackageEntry {
            package: format!("{}:{}", p.kind, p.name),
            subskills: p
                .subskills_active
                .iter()
                .map(|s| SubskillEntry {
                    path: s.path.clone(),
                    delivery: s.delivery.clone(),
                    describes: s.describes.clone(),
                })
                .collect(),
        });
    }

    if ctx.is_json() {
        ctx.emit_json(&SubskillsReport {
            ok: true,
            command: "show:subskills",
            project: project_root.display().to_string(),
            packages: entries,
            total,
        })?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe show subskills: {} active subskill{} across {} package{}",
            total,
            if total == 1 { "" } else { "s" },
            entries.len(),
            if entries.len() == 1 { "" } else { "s" },
        ));
        return Ok(());
    }
    if entries.is_empty() {
        ctx.summary("(no subskills active in this project)");
        return Ok(());
    }
    for e in &entries {
        ctx.heading(&e.package);
        for s in &e.subskills {
            let mut line = format!("{} ({})", s.path, s.delivery);
            if let Some(d) = &s.describes {
                line.push_str(&format!("  describes: {d}"));
            }
            ctx.step(&line);
        }
    }
    ctx.summary(&format!(
        "\n{} active subskill{} across {} package{}",
        total,
        if total == 1 { "" } else { "s" },
        entries.len(),
        if entries.len() == 1 { "" } else { "s" },
    ));
    Ok(())
}

// ===================== show purls =====================

#[derive(Debug, Serialize)]
struct PurlEntry {
    package: String,
    purl: String,
}

#[derive(Debug, Serialize)]
struct PurlsReport {
    ok: bool,
    command: &'static str,
    project: String,
    bindings: Vec<PurlEntry>,
}

fn run_purls(ctx: &output::Context, args: ShowPurlsArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let lockfile_path = project_root.join(vibe_core::manifest::Lockfile::FILENAME);
    let lockfile = if lockfile_path.exists() {
        vibe_core::manifest::Lockfile::read(&lockfile_path)?
    } else {
        vibe_core::manifest::Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            super::init::current_timestamp_utc(),
        )
    };
    let mut bindings: Vec<PurlEntry> = Vec::new();
    for p in &lockfile.packages {
        if let Some(purl) = &p.describes {
            bindings.push(PurlEntry {
                package: format!("{}:{}", p.kind, p.name),
                purl: purl.clone(),
            });
        }
        for s in &p.subskills_active {
            if let Some(purl) = &s.describes {
                bindings.push(PurlEntry {
                    package: format!("{}:{}/{}", p.kind, p.name, s.path),
                    purl: purl.clone(),
                });
            }
        }
    }
    if ctx.is_json() {
        ctx.emit_json(&PurlsReport {
            ok: true,
            command: "show:purls",
            project: project_root.display().to_string(),
            bindings,
        })?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe show purls: {} binding{}",
            bindings.len(),
            if bindings.len() == 1 { "" } else { "s" },
        ));
        return Ok(());
    }
    if bindings.is_empty() {
        ctx.summary("(no PURL bindings in this project)");
        return Ok(());
    }
    for b in &bindings {
        ctx.step(&format!("{}  →  {}", b.package, b.purl));
    }
    ctx.summary(&format!(
        "\n{} PURL binding{}",
        bindings.len(),
        if bindings.len() == 1 { "" } else { "s" },
    ));
    Ok(())
}
