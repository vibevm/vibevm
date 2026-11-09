//! `vibe registry list` — render the configured `[[registry]]` /
//! `[[mirror]]` / `[[override]]` blocks.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#registry");

use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use vibe_core::manifest::{Manifest, MirrorSection, OverrideSection, RegistrySection};
use vibe_core::{GlobalRegistryConfig, RegistryLayers, RegistryScope};
use vibe_publish::{extract_host_segment, extract_org_segment};

use crate::cli::RegistryListArgs;
use crate::commands::registry::resolve_project_root;
use crate::output;

use super::scope::{ScopeFile, parse_scope};
use super::{ListReportMirror, ListReportRegistry, adapter_for, naming_label};

/// The rows one `vibe registry list` renders.
struct View {
    label: String,
    rows: Vec<(RegistryScope, RegistrySection)>,
    mirrors: Vec<MirrorSection>,
    overrides: Vec<OverrideSection>,
}

fn one_layer(file: ScopeFile) -> View {
    View {
        label: format!("`{}`", file.path().display()),
        rows: file
            .registries()
            .iter()
            .map(|row| (file.scope(), row.clone()))
            .collect(),
        mirrors: file.mirrors().to_vec(),
        overrides: match &file {
            ScopeFile::Project { manifest, .. } => manifest.overrides.clone(),
            ScopeFile::Config { config, .. } => config.overrides.clone(),
        },
    }
}

/// The applied site profile's own rows (`PROP-019#site-profile`).
fn site_layer() -> Result<View> {
    let site = vibe_core::site_profile::installed().context("reading the applied site profile")?;
    let (label, config) = match site {
        Some(site) => (
            format!("the site profile `{}`", site.path.display()),
            site.profile.registry_layer(),
        ),
        None => (
            "the site profile (none applied)".to_string(),
            GlobalRegistryConfig::default(),
        ),
    };
    Ok(View {
        label,
        rows: config
            .registries
            .iter()
            .map(|row| (RegistryScope::Site, row.clone()))
            .collect(),
        mirrors: config.mirrors,
        overrides: config.overrides,
    })
}

/// The effective view across every layer (PROP-002 `#registry-layers`):
/// the project-local override and `vibe.toml` when run inside a project,
/// then `~/.vibe/registry.toml`, then the applied site profile.
fn merged(path: &Path) -> Result<View> {
    let root = resolve_project_root(path)?;
    let manifest_path = root.join(Manifest::FILENAME);
    let project = if manifest_path.exists() {
        Some(
            Manifest::read(&manifest_path)
                .with_context(|| format!("reading `{}`", manifest_path.display()))?,
        )
    } else {
        None
    };
    let local = match &project {
        Some(_) => Some(GlobalRegistryConfig::load_project_local(&root)?),
        None => None,
    };
    let user = match vibe_core::settings::registry_config_path() {
        Some(path) => GlobalRegistryConfig::load_from(&path)?,
        None => GlobalRegistryConfig::default(),
    };
    // The applied site profile (`PROP-019#site-profile`) is the layer
    // below the user's own file; its rows are listed with scope `site`.
    let site = vibe_core::site_profile::installed()
        .context("reading the applied site profile")?
        .map(|site| site.profile.registry_layer());
    let layers = RegistryLayers {
        local: local.as_ref(),
        project: project.as_ref(),
        user: &user,
        site: site.as_ref(),
    };
    let effective = layers.merge();
    Ok(View {
        label: "any declaration layer".to_string(),
        rows: layers.origins(),
        mirrors: effective.mirrors,
        overrides: effective.overrides,
    })
}

#[derive(Debug, Serialize)]
struct ListReport {
    ok: bool,
    command: &'static str,
    registries: Vec<ListReportRegistry>,
    mirrors: Vec<ListReportMirror>,
    overrides: Vec<ListReportOverride>,
}

#[derive(Debug, Serialize)]
struct ListReportOverride {
    pkgref: String,
    source_url: String,
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    refname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

pub(in crate::commands::registry) fn run_list(
    ctx: &output::Context,
    args: RegistryListArgs,
) -> Result<()> {
    // `--scope site` is read-only: listing it is allowed, editing is not.
    let view = if args
        .scope
        .as_deref()
        .and_then(RegistryScope::parse)
        .is_some_and(|scope| scope == RegistryScope::Site)
    {
        site_layer()?
    } else {
        match parse_scope(args.scope.as_deref())? {
            Some(scope) => one_layer(ScopeFile::open(scope, &args.path)?),
            None => merged(&args.path)?,
        }
    };

    let mut registries: Vec<ListReportRegistry> = Vec::with_capacity(view.rows.len());
    for (scope, reg) in &view.rows {
        let kind = reg.effective_type();
        let (host, org) = if kind == vibe_core::manifest::RegistryType::Localdir {
            (String::from("(local directory)"), String::new())
        } else {
            (
                extract_host_segment(&reg.url).unwrap_or_else(|_| String::from("?")),
                extract_org_segment(&reg.url).unwrap_or_else(|_| String::from("?")),
            )
        };
        let adapter = adapter_for(kind, &host).map(String::from);
        // Mirrors targeted at this registry by name, plus the wildcard
        // `*` mirrors that apply to any registry, sorted by priority.
        let mut targeted: Vec<&MirrorSection> = view
            .mirrors
            .iter()
            .filter(|m| m.of == reg.name || m.of == "*")
            .collect();
        targeted.sort_by_key(|m| m.priority);
        let mirrors: Vec<ListReportMirror> = targeted
            .into_iter()
            .map(|m| ListReportMirror {
                of: m.of.clone(),
                url: m.url.clone(),
                priority: m.priority,
            })
            .collect();
        registries.push(ListReportRegistry {
            name: reg.name.clone(),
            registry_type: kind.as_str().to_string(),
            scope: Some(scope.as_str().to_string()),
            url: reg.url.clone(),
            refname: reg.r#ref.clone(),
            naming: naming_label(reg.naming).to_string(),
            host,
            org,
            adapter,
            mirrors,
        });
    }

    let all_mirrors: Vec<ListReportMirror> = view
        .mirrors
        .iter()
        .map(|m| ListReportMirror {
            of: m.of.clone(),
            url: m.url.clone(),
            priority: m.priority,
        })
        .collect();
    let overrides: Vec<ListReportOverride> = view
        .overrides
        .iter()
        .map(|o| ListReportOverride {
            pkgref: o.pkgref.clone(),
            source_url: o.source_url.clone(),
            refname: o.r#ref.clone(),
            reason: o.reason.clone(),
        })
        .collect();

    if ctx.is_json() {
        ctx.emit_json(&ListReport {
            ok: true,
            command: "registry:list",
            registries,
            mirrors: all_mirrors,
            overrides,
        })?;
        return Ok(());
    }

    if registries.is_empty() {
        ctx.summary(&format!(
            "No `[[registry]]` entries in {}. Use `--registry <path>` on \
             `vibe install` for a local-directory source, or `vibe registry add`.",
            view.label
        ));
        return Ok(());
    }

    ctx.heading(&format!(
        "Registries ({}; primary listed first)",
        registries.len()
    ));
    for (i, reg) in registries.iter().enumerate() {
        let primary_marker = if i == 0 { " (primary)" } else { "" };
        let adapter_label = reg
            .adapter
            .as_deref()
            .map(|a| format!("adapter: {a}"))
            .unwrap_or_else(|| "adapter: none (publish unsupported)".to_string());
        println!(
            "  {}. {}{}\n     type:    {}\n     scope:   {}\n     url:     {}\n     org:     {}\n     host:    {} ({})\n     naming:  {}\n     ref:     {}",
            i + 1,
            reg.name,
            primary_marker,
            reg.registry_type,
            reg.scope.as_deref().unwrap_or("-"),
            reg.url,
            reg.org,
            reg.host,
            adapter_label,
            reg.naming,
            reg.refname,
        );
        if reg.mirrors.is_empty() {
            println!("     mirrors: (none)");
        } else {
            println!("     mirrors:");
            for m in &reg.mirrors {
                println!(
                    "       - of=`{}` priority={} url={}",
                    m.of, m.priority, m.url
                );
            }
        }
    }

    if !overrides.is_empty() {
        println!();
        ctx.heading(&format!("Overrides ({})", overrides.len()));
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
                "  {} → {}{}{}",
                o.pkgref, o.source_url, ref_part, reason_part
            );
        }
    }

    let mirror_total = registries.iter().map(|r| r.mirrors.len()).sum::<usize>();
    ctx.summary(&format!(
        "\nvibe registry list: {} registries, {} mirror{}, {} override{}.",
        registries.len(),
        mirror_total,
        if mirror_total == 1 { "" } else { "s" },
        overrides.len(),
        if overrides.len() == 1 { "" } else { "s" },
    ));
    Ok(())
}
