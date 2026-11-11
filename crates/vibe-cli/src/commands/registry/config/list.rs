//! `vibe registry list` — render the configured `[[registry]]` /
//! `[[mirror]]` / `[[override]]` blocks.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#registry");

use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use vibe_core::manifest::{Manifest, MirrorSection, OverrideSection, RegistrySection};
use vibe_core::{GlobalRegistryConfig, RegistryLayers, RegistryScope};

use crate::cli::RegistryListArgs;
use crate::commands::registry::resolve_project_root;
use crate::output;

use super::scope::{ScopeFile, parse_scope};
use super::{ListReportMirror, ListReportRegistry, adapter_for, display_segments, naming_label};

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
    /// The first registry the effective `[source_policy]` admits — the one
    /// resolution actually asks first (PROP-002 `#registry-layers`, PROP-073
    /// `#SOURCE-POLICY`). `null` when the policy admits none.
    #[serde(skip_serializing_if = "Option::is_none")]
    effective_primary: Option<String>,
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

    // The same policy resolution applies, so the marks below name exactly the
    // registries resolution skips. The rows themselves are untouched: an
    // excluded registry stays declared and is simply not used.
    // A listing must not fail over a project manifest it was not asked to
    // read — `--scope user` never looks at one — so a project layer that
    // cannot be parsed simply does not contribute here.
    let project = resolve_project_root(&args.path)
        .ok()
        .map(|root| root.join(Manifest::FILENAME))
        .filter(|path| path.exists())
        .and_then(|path| Manifest::read(&path).ok());
    let policy = super::super::effective_source_policy(project.as_ref())?;

    let mut registries: Vec<ListReportRegistry> = Vec::with_capacity(view.rows.len());
    for (scope, reg) in &view.rows {
        let kind = reg.effective_type();
        let (host, org) = display_segments(kind, &reg.url);
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
            disabled_by_source_policy: policy.excludes_endpoint(&reg.url),
            disabled_by: policy.excluded_by(&reg.url).map(ToString::to_string),
        });
    }
    // 4b: declared precedence is unchanged, but a registry the policy
    // disables can never be primary in practice, so the effective primary is
    // the first allowed one.
    let effective_primary = vibe_registry::multi_registry_resolver::effective_primary(
        &policy,
        view.rows.iter().map(|(_, reg)| reg),
    )
    .map(|reg| reg.name.clone());

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
            effective_primary,
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
        "Registries ({}; declared order)",
        registries.len()
    ));
    for (i, reg) in registries.iter().enumerate() {
        // The tag goes on the registry resolution actually asks first, not
        // on whichever row happens to be declared first: tagging a row the
        // source policy disables `(primary)` contradicts the effective
        // primary printed below it, and the disabled row is the one a
        // reader is already unsure about.
        let primary_marker = match effective_primary.as_deref() {
            Some(name) if name == reg.name => " (primary)",
            _ => "",
        };
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
        // The declaration stays; the policy switches it off. Nothing here
        // suggests removing the row — leaving it in place is the point.
        if reg.disabled_by_source_policy {
            println!(
                "     status:  {}{}",
                vibe_registry::multi_registry_resolver::DISABLED_BY_POLICY_MARK,
                reg.disabled_by
                    .as_deref()
                    .map(|selector| format!(" [{selector}]"))
                    .unwrap_or_default(),
            );
        }
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

    let disabled = registries
        .iter()
        .filter(|reg| reg.disabled_by_source_policy)
        .count();
    if disabled > 0 {
        println!(
            "\neffective primary: {}",
            effective_primary
                .as_deref()
                .unwrap_or("(none — the source policy disables every configured registry)")
        );
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
        "\nvibe registry list: {} registries{}, {} mirror{}, {} override{}.",
        registries.len(),
        // Said only when the policy disabled something, so the ordinary
        // listing reads exactly as it did before.
        if disabled > 0 {
            format!(" ({disabled} disabled by the source policy)")
        } else {
            String::new()
        },
        mirror_total,
        if mirror_total == 1 { "" } else { "s" },
        overrides.len(),
        if overrides.len() == 1 { "" } else { "s" },
    ));
    Ok(())
}
