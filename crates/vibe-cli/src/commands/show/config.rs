//! `vibe show config` — dump the effective configuration with
//! provenance per value (`VIBEVM-SPEC.md` §9.5).

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use vibe_core::manifest::Manifest;
use vibe_core::user_config::UserConfig;

use crate::cli::ShowConfigArgs;
use crate::output;

use super::resolve_project_root;

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
    /// Detailed resolution block for the agent-context layer. The top-
    /// level `invoked_by: "<agent>"` field stamped on every JSON
    /// envelope by `output::Context` is just the resolved value;
    /// `invoked_by_resolution` adds the provenance ("cli-flag" / "env"
    /// / "default") so an operator can see which layer supplied it.
    invoked_by_resolution: ConfigInvokedBy,
}

#[derive(Debug, Serialize)]
struct ConfigInvokedBy {
    /// Resolved agent identifier — `null` when neither `--invoked-by`
    /// nor `VIBE_INVOKED_BY` is set.
    value: Option<String>,
    /// `"cli-flag"` (passed via `--invoked-by`), `"env"` (`VIBE_INVOKED_BY`),
    /// or `"default"` (unset; no agent context attached to envelopes).
    provenance: &'static str,
    /// Short description for human-readable output.
    description: &'static str,
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

pub(super) fn run_config(ctx: &output::Context, args: ShowConfigArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    let manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;
    let project = manifest
        .require_project()
        .with_context(|| format!("`{}` has no `[project]` table", manifest_path.display()))?;

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

    let invoked_by_block = ConfigInvokedBy {
        value: ctx.invoked_by().map(|s| s.to_string()),
        provenance: ctx.invoked_by_provenance().as_str(),
        description: "Identifier of the agent or harness invoking vibe. \
                      Stamped onto every JSON envelope. Resolved from \
                      `--invoked-by <agent>` (highest), `VIBE_INVOKED_BY` \
                      env-var, or unset. The `vibevm` skill installed by \
                      `vibe mcp install --with-skill` instructs each agent \
                      to pass this flag automatically.",
    };

    if ctx.is_json() {
        let payload = ConfigReport {
            ok: true,
            command: "show:config",
            project: project_root.display().to_string(),
            project_name: project.name.clone(),
            project_version: project.version.clone(),
            registries,
            mirrors,
            overrides,
            env,
            user_config: user_config_summary,
            invoked_by_resolution: invoked_by_block,
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
        project.name,
        project.version,
        project_root.display()
    );
    println!();
    println!("Registries ({}; primary first):", registries.len());
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
        ConfigUserConfigSummary {
            path: Some(p),
            loaded: true,
            ..
        } => {
            println!("User config: {p}  (loaded)");
        }
        ConfigUserConfigSummary {
            path: Some(p),
            loaded: false,
            error: Some(err),
        } => {
            println!("User config: {p}  (parse error — {err})");
        }
        ConfigUserConfigSummary {
            path: Some(p),
            loaded: false,
            error: None,
        } => {
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

    println!();
    println!("Invoked-by:");
    let invoked_value_text = match &invoked_by_block.value {
        Some(v) => format!("`{v}`"),
        None => "(unset; envelopes carry no `invoked_by` field)".to_string(),
    };
    println!(
        "  [source: {}]\n  {}\n  {}",
        invoked_by_block.provenance, invoked_by_block.description, invoked_value_text
    );

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
        vibe_core::manifest::NamingConvention::Fqdn => "fqdn",
        vibe_core::manifest::NamingConvention::KindName => "kind-name",
        vibe_core::manifest::NamingConvention::Name => "name",
        vibe_core::manifest::NamingConvention::KindSlashName => "kind/name",
    }
    .to_string()
}
