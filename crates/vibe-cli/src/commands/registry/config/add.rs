//! `vibe registry add` — append or prepend a `[[registry]]` block.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#registry");

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use vibe_core::manifest::{Manifest, NamingConvention, RegistrySection};
use vibe_publish::{extract_host_segment, extract_org_segment};

use crate::cli::RegistryAddArgs;
use crate::commands::registry::resolve_project_root;
use crate::output;

use super::{ListReportMirror, ListReportRegistry, adapter_for_host, parse_naming};

#[derive(Debug, Serialize)]
struct AddReport {
    ok: bool,
    command: &'static str,
    registry: ListReportRegistry,
    /// `"primary"` if inserted at index 0, `"append"` if at the tail.
    /// Mirrors `--position` from the CLI.
    position: String,
    /// Total number of `[[registry]]` blocks after the add.
    total_registries: usize,
}

pub(in crate::commands::registry) fn run_add(
    ctx: &output::Context,
    args: RegistryAddArgs,
) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let mut manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    // Validation: name must not collide with an existing registry.
    if manifest.registry_by_name(&args.name).is_some() {
        bail!(
            "a `[[registry]]` named `{}` already exists in `{}`. Pick a different name or remove the existing entry first.",
            args.name,
            manifest_path.display()
        );
    }
    if args.name.trim().is_empty() {
        bail!("registry name must be non-empty");
    }

    // Validation: URL must shape-parse for both org and host
    // segmentation. If either fails, the URL is unusable as a
    // `[[registry]].url` regardless of host adapter availability.
    let host =
        extract_host_segment(&args.url).map_err(|e| anyhow!("registry URL `{}`: {e}", args.url))?;
    let org =
        extract_org_segment(&args.url).map_err(|e| anyhow!("registry URL `{}`: {e}", args.url))?;

    let naming = match args.naming.as_deref() {
        Some(s) => parse_naming(s)?,
        None => NamingConvention::default(),
    };

    let position_label = match args.position.as_str() {
        "primary" | "append" => args.position.as_str(),
        other => bail!("unknown --position `{other}` — must be `primary` or `append`"),
    };

    let auth = match args.auth.as_deref() {
        None | Some("none") => vibe_core::manifest::AuthKind::None,
        Some("token-env") => vibe_core::manifest::AuthKind::TokenEnv,
        Some("credential-helper") => vibe_core::manifest::AuthKind::CredentialHelper,
        Some("ssh") => vibe_core::manifest::AuthKind::Ssh,
        Some(other) => bail!(
            "unknown --auth `{other}` — must be `none`, `token-env`, `credential-helper`, or `ssh`"
        ),
    };
    if matches!(auth, vibe_core::manifest::AuthKind::TokenEnv) && args.token_env.is_none() {
        // No --token-env supplied: that's fine, the resolver will derive
        // the default name from the registry's host. But warn the
        // operator so they don't get a confusing "env-var not set" error
        // later if they meant to point at a specific name.
        tracing::debug!(
            target: "vibe_cli::registry::add",
            "auth=token-env without explicit --token-env; will derive from host on resolve"
        );
    }
    if args.token_env.is_some() && !matches!(auth, vibe_core::manifest::AuthKind::TokenEnv) {
        bail!(
            "--token-env is only meaningful with --auth token-env; got --auth {:?}",
            auth.as_str()
        );
    }

    let new = RegistrySection {
        name: args.name.clone(),
        url: args.url.clone(),
        r#ref: args.registry_ref.unwrap_or_else(|| "main".to_string()),
        naming,
        auth,
        token_env: args.token_env.clone(),
    };

    match position_label {
        "primary" => manifest.registries.insert(0, new.clone()),
        "append" => manifest.registries.push(new.clone()),
        _ => unreachable!("validated above"),
    }

    manifest
        .write(&manifest_path)
        .with_context(|| format!("writing `{}`", manifest_path.display()))?;

    let naming_label = match new.naming {
        NamingConvention::Fqdn => "fqdn",
        NamingConvention::KindName => "kind-name",
        NamingConvention::Name => "name",
        NamingConvention::KindSlashName => "kind/name",
    }
    .to_string();
    let adapter = adapter_for_host(&host).map(String::from);

    let registry_view = ListReportRegistry {
        name: new.name.clone(),
        url: new.url.clone(),
        refname: new.r#ref.clone(),
        naming: naming_label.clone(),
        host: host.clone(),
        org: org.clone(),
        adapter: adapter.clone(),
        mirrors: manifest
            .mirrors_for(&new.name)
            .into_iter()
            .map(|m| ListReportMirror {
                of: m.of.clone(),
                url: m.url.clone(),
                priority: m.priority,
            })
            .collect(),
    };

    if ctx.is_json() {
        ctx.emit_json(&AddReport {
            ok: true,
            command: "registry:add",
            registry: registry_view,
            position: position_label.to_string(),
            total_registries: manifest.registries.len(),
        })?;
        return Ok(());
    }

    let position_text = if position_label == "primary" {
        " as primary"
    } else {
        ""
    };
    let adapter_text = adapter
        .as_deref()
        .map(|a| format!(" (adapter: {a})"))
        .unwrap_or_else(|| {
            " (adapter: none — `vibe registry publish` won't dispatch here)".to_string()
        });
    ctx.step(&format!(
        "Added `[[registry]]` `{}`{} → {} on host {}{}",
        new.name, position_text, new.url, host, adapter_text
    ));
    ctx.summary(&format!(
        "\nvibe registry add: `{}` registered ({} total registr{}).",
        new.name,
        manifest.registries.len(),
        if manifest.registries.len() == 1 {
            "y"
        } else {
            "ies"
        },
    ));
    Ok(())
}
