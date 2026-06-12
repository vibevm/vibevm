//! `vibe registry list` — render the configured `[[registry]]` /
//! `[[mirror]]` / `[[override]]` blocks.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#registry");

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::manifest::Manifest;
use vibe_publish::{extract_host_segment, extract_org_segment};

use crate::cli::RegistryListArgs;
use crate::commands::registry::resolve_project_root;
use crate::output;

use super::{ListReportMirror, ListReportRegistry, adapter_for_host};

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
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let mut registries: Vec<ListReportRegistry> = Vec::with_capacity(manifest.registries.len());
    for reg in &manifest.registries {
        let host = extract_host_segment(&reg.url).unwrap_or_else(|_| String::from("?"));
        let org = extract_org_segment(&reg.url).unwrap_or_else(|_| String::from("?"));
        let adapter = adapter_for_host(&host).map(String::from);
        let naming_label = match reg.naming {
            vibe_core::manifest::NamingConvention::Fqdn => "fqdn",
            vibe_core::manifest::NamingConvention::KindName => "kind-name",
            vibe_core::manifest::NamingConvention::Name => "name",
            vibe_core::manifest::NamingConvention::KindSlashName => "kind/name",
        }
        .to_string();
        // Mirrors targeted at this registry by name, plus the wildcard
        // `*` mirrors that apply to any registry. `mirrors_for` already
        // returns them sorted by priority ascending.
        let mirrors: Vec<ListReportMirror> = manifest
            .mirrors_for(&reg.name)
            .into_iter()
            .map(|m| ListReportMirror {
                of: m.of.clone(),
                url: m.url.clone(),
                priority: m.priority,
            })
            .collect();
        registries.push(ListReportRegistry {
            name: reg.name.clone(),
            url: reg.url.clone(),
            refname: reg.r#ref.clone(),
            naming: naming_label,
            host,
            org,
            adapter,
            mirrors,
        });
    }

    let all_mirrors: Vec<ListReportMirror> = manifest
        .mirrors
        .iter()
        .map(|m| ListReportMirror {
            of: m.of.clone(),
            url: m.url.clone(),
            priority: m.priority,
        })
        .collect();
    let overrides: Vec<ListReportOverride> = manifest
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
        ctx.summary(
            "No `[[registry]]` entries in `vibe.toml`. Use `--registry <path>` on \
             `vibe install` for a local-directory source, or add a `[[registry]]` block.",
        );
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
            "  {}. {}{}\n     url:     {}\n     org:     {}\n     host:    {} ({})\n     naming:  {}\n     ref:     {}",
            i + 1,
            reg.name,
            primary_marker,
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
