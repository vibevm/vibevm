//! `vibe registry add` — append or prepend a `[[registry]]` block.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#registry");

use anyhow::{Result, anyhow, bail};
use serde::Serialize;
use vibe_core::RegistryScope;
use vibe_core::manifest::{NamingConvention, RegistrySection, RegistryType};
use vibe_publish::{extract_host_segment, extract_org_segment};

use crate::cli::RegistryAddArgs;
use crate::output;

use super::scope::{ScopeFile, parse_scope, parse_type};
use super::{ListReportMirror, ListReportRegistry, adapter_for, naming_label, parse_naming};

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
    let scope = parse_scope(args.scope.as_deref())?.unwrap_or(RegistryScope::Project);
    let declared_type = parse_type(args.registry_type.as_deref())?;
    let mut file = ScopeFile::open(scope, &args.path)?;

    // Validation: name must not collide with an existing registry.
    if file.registries().iter().any(|r| r.name == args.name) {
        bail!(
            "a `[[registry]]` named `{}` already exists in `{}`. Pick a different name or remove the existing entry first.",
            args.name,
            file.path().display()
        );
    }
    if args.name.trim().is_empty() {
        bail!("registry name must be non-empty");
    }

    let naming = match args.naming.as_deref() {
        Some(s) => parse_naming(s)?,
        None => NamingConvention::default(),
    };

    let position_label = match args.position.as_str() {
        "primary" | "append" => args.position.as_str(),
        other => bail!("unknown --position `{other}` — must be `primary` or `append`"),
    };

    let auth = match args.auth.as_deref() {
        None => None,
        Some("none") => Some(vibe_core::manifest::AuthKind::None),
        Some("token-env") => Some(vibe_core::manifest::AuthKind::TokenEnv),
        Some("credential-helper") => Some(vibe_core::manifest::AuthKind::CredentialHelper),
        Some("ssh") => Some(vibe_core::manifest::AuthKind::Ssh),
        Some(other) => bail!(
            "unknown --auth `{other}` — must be `none`, `token-env`, `credential-helper`, or `ssh`"
        ),
    };
    if matches!(auth, Some(vibe_core::manifest::AuthKind::TokenEnv)) && args.token_env.is_none() {
        // No --token-env supplied: that's fine, the resolver will derive
        // the default name from the registry's host. But warn the
        // operator so they don't get a confusing "env-var not set" error
        // later if they meant to point at a specific name.
        tracing::debug!(
            target: "vibe_cli::registry::add",
            "auth=token-env without explicit --token-env; will derive from host on resolve"
        );
    }
    if args.token_env.is_some() && !matches!(auth, Some(vibe_core::manifest::AuthKind::TokenEnv)) {
        bail!(
            "--token-env is only meaningful with --auth token-env; got --auth {:?}",
            auth.map(vibe_core::manifest::AuthKind::as_str)
        );
    }

    let new = RegistrySection {
        name: args.name.clone(),
        r#type: declared_type,
        url: args.url.clone(),
        r#ref: args.registry_ref.unwrap_or_else(|| "main".to_string()),
        naming,
        auth,
        token_env: args.token_env.clone(),
        enabled: true,
        index_url: None,
    };
    let kind = new.checked_type()?;
    if args.naming.is_some() && matches!(kind, RegistryType::Monorepo | RegistryType::Localdir) {
        bail!(
            "--naming applies only to per-package registries; a `{kind}` registry lays packages \
             out as `<group>/<name>/v<version>/`"
        );
    }

    // Validation: a Git URL must shape-parse for both org and host
    // segmentation; a local directory has neither.
    let (host, org) = if kind == RegistryType::Localdir {
        (String::from("(local directory)"), String::new())
    } else {
        (
            extract_host_segment(&args.url)
                .map_err(|e| anyhow!("registry URL `{}`: {e}", args.url))?,
            extract_org_segment(&args.url)
                .map_err(|e| anyhow!("registry URL `{}`: {e}", args.url))?,
        )
    };

    vibe_registry::resolve_registry_auth(vibe_registry::RegistryAuthInput::SelectedRegistry(&new))?;
    match position_label {
        "primary" => file.registries_mut().insert(0, new.clone()),
        _ => file.registries_mut().push(new.clone()),
    }
    file.save()?;

    let adapter = adapter_for(kind, &host).map(String::from);
    let registry_view = ListReportRegistry {
        name: new.name.clone(),
        registry_type: kind.as_str().to_string(),
        scope: Some(file.scope().as_str().to_string()),
        url: new.url.clone(),
        refname: new.r#ref.clone(),
        naming: naming_label(new.naming).to_string(),
        host: host.clone(),
        org: org.clone(),
        adapter: adapter.clone(),
        mirrors: file
            .mirrors_for(&new.name)
            .into_iter()
            .map(|m| ListReportMirror {
                of: m.of.clone(),
                url: m.url.clone(),
                priority: m.priority,
            })
            .collect(),
    };
    let total = file.registries().len();

    if ctx.is_json() {
        ctx.emit_json(&AddReport {
            ok: true,
            command: "registry:add",
            registry: registry_view,
            position: position_label.to_string(),
            total_registries: total,
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
        .map(|a| format!(" (publish: {a})"))
        .unwrap_or_else(|| " (publish: unsupported)".to_string());
    ctx.step(&format!(
        "Added `[[registry]]` `{}` (type {kind}, scope {}){} → {}{}",
        new.name,
        file.scope().as_str(),
        position_text,
        new.url,
        adapter_text
    ));
    ctx.summary(&format!(
        "\nvibe registry add: `{}` registered in `{}` ({} total registr{}).",
        new.name,
        file.path().display(),
        total,
        if total == 1 { "y" } else { "ies" },
    ));
    Ok(())
}
