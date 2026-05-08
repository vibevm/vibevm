//! `vibe registry …` — registry cache management.
//!
//! Spec: `VIBEVM-SPEC.md` §8.3 (cache layout, refresh).
//! Decentralized per-package model: PROP-002.
//!
//! `vibe registry sync` walks the lockfile and refreshes the on-disk
//! clone of every installed package. For `[[registry]]`-served entries
//! that means `git fetch` + hard-reset on the per-package clone under
//! `<cache>/<canonical-url-hash>/packages/<kind>-<name>/clone/`. For
//! `[[override]]`-served entries that means the same against the
//! `__overrides__/<kind>-<name>/clone/` subtree. Local-directory
//! registries (`--registry <path>`) and legacy v1 entries are reported
//! as skipped — there is no per-package clone to refresh for them.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use vibe_core::manifest::{
    DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_URL, Lockfile, MirrorSection, NamingConvention,
    ProjectManifest, RegistrySection,
};
use vibe_publish::{
    DirectGitCreator, PublishConfig, Publisher, creator_for_url, extract_host_segment,
    extract_org_segment, load_token_for_host,
};
use vibe_registry::{MultiRegistryResolver, RefreshedVia};

use crate::cli::{
    RegistryAddArgs, RegistryArgs, RegistryListArgs, RegistryPublishArgs,
    RegistryRemoveArgs, RegistryRemoveMirrorArgs, RegistryRemoveRegistryArgs,
    RegistryRemoveTarget, RegistrySetMirrorArgs, RegistrySubcommand, RegistrySyncArgs,
    RegistryVendorArgs,
};
use crate::output;

pub fn run(ctx: &output::Context, args: RegistryArgs) -> Result<()> {
    match args.command {
        RegistrySubcommand::Sync(sub) => run_sync(ctx, sub),
        RegistrySubcommand::Publish(sub) => run_publish(ctx, sub),
        RegistrySubcommand::List(sub) => run_list(ctx, sub),
        RegistrySubcommand::Add(sub) => run_add(ctx, sub),
        RegistrySubcommand::SetMirror(sub) => run_set_mirror(ctx, sub),
        RegistrySubcommand::Remove(sub) => run_remove(ctx, sub),
        RegistrySubcommand::Vendor(sub) => run_vendor(ctx, sub),
    }
}

#[derive(Debug, Serialize)]
struct SyncReport {
    ok: bool,
    command: &'static str,
    refreshed: Vec<RefreshedReportEntry>,
    skipped: Vec<SkippedReportEntry>,
}

#[derive(Debug, Serialize)]
struct RefreshedReportEntry {
    kind: String,
    name: String,
    via: String, // "registry:<name>" or "override"
    #[serde(rename = "ref")]
    refname: String,
}

#[derive(Debug, Serialize)]
struct SkippedReportEntry {
    kind: String,
    name: String,
    reason: String,
}

fn run_sync(ctx: &output::Context, args: RegistrySyncArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(ProjectManifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = ProjectManifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let lockfile_path = project_root.join(Lockfile::FILENAME);
    if !lockfile_path.exists() {
        ctx.summary(
            "vibe registry sync: no `vibe.lock` yet — nothing installed, nothing to refresh.",
        );
        if ctx.is_json() {
            ctx.emit_json(&SyncReport {
                ok: true,
                command: "registry:sync",
                refreshed: Vec::new(),
                skipped: Vec::new(),
            })?;
        }
        return Ok(());
    }
    let lockfile = Lockfile::read(&lockfile_path)
        .with_context(|| format!("reading `{}`", lockfile_path.display()))?;

    if lockfile.packages.is_empty() {
        ctx.summary("vibe registry sync: lockfile is empty — nothing to refresh.");
        if ctx.is_json() {
            ctx.emit_json(&SyncReport {
                ok: true,
                command: "registry:sync",
                refreshed: Vec::new(),
                skipped: Vec::new(),
            })?;
        }
        return Ok(());
    }

    if manifest.registries.is_empty() {
        // Empty `[[registry]]` is legal (e.g., projects that only use
        // `--registry <path>` or `[[override]]`-only setups), but
        // `registry sync` has nothing to do without `[[registry]]`
        // entries to dispatch through. Override-only refresh would
        // need its own flag; for now, surface the situation.
        ctx.summary(
            "vibe registry sync: no `[[registry]]` entries in `vibe.toml` — nothing to refresh.",
        );
        if ctx.is_json() {
            ctx.emit_json(&SyncReport {
                ok: true,
                command: "registry:sync",
                refreshed: Vec::new(),
                skipped: Vec::new(),
            })?;
        }
        return Ok(());
    }

    let mrr = MultiRegistryResolver::open(
        &manifest.registries,
        &manifest.mirrors,
        &manifest.overrides,
    )
    .context("opening multi-registry resolver")?;

    ctx.heading(&format!(
        "Syncing {} package clone{} referenced by lockfile",
        lockfile.packages.len(),
        if lockfile.packages.len() == 1 { "" } else { "s" }
    ));

    let report = mrr
        .refresh_lockfile_clones(&lockfile)
        .context("refreshing per-package clones")?;

    let json_refreshed: Vec<RefreshedReportEntry> = report
        .refreshed
        .iter()
        .map(|e| RefreshedReportEntry {
            kind: e.kind.as_str().to_string(),
            name: e.name.clone(),
            via: match &e.via {
                RefreshedVia::Registry(n) => format!("registry:{n}"),
                RefreshedVia::Override => "override".to_string(),
            },
            refname: e.refname.clone(),
        })
        .collect();
    let json_skipped: Vec<SkippedReportEntry> = report
        .skipped
        .iter()
        .map(|e| SkippedReportEntry {
            kind: e.kind.as_str().to_string(),
            name: e.name.clone(),
            reason: e.reason.clone(),
        })
        .collect();

    if ctx.is_json() {
        ctx.emit_json(&SyncReport {
            ok: true,
            command: "registry:sync",
            refreshed: json_refreshed,
            skipped: json_skipped,
        })?;
        return Ok(());
    }

    if !report.refreshed.is_empty() {
        for e in &report.refreshed {
            let via_text = match &e.via {
                RefreshedVia::Registry(name) => format!("registry `{name}`"),
                RefreshedVia::Override => "override".to_string(),
            };
            ctx.step(&format!(
                "{}:{} @ {} via {}",
                e.kind, e.name, e.refname, via_text
            ));
        }
    }
    if !report.skipped.is_empty() {
        for e in &report.skipped {
            ctx.skipped(&format!("{}:{}", e.kind, e.name), &e.reason);
        }
    }

    ctx.summary(&format!(
        "\nvibe registry sync: {} refreshed, {} skipped.",
        report.refreshed.len(),
        report.skipped.len()
    ));
    Ok(())
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
struct ListReportRegistry {
    name: String,
    url: String,
    #[serde(rename = "ref")]
    refname: String,
    naming: String,
    host: String,
    org: String,
    /// Adapter that `vibe registry publish` would dispatch to for this
    /// registry's host. `null` if the host has no adapter today.
    adapter: Option<String>,
    /// Mirrors that fall through to this registry, in priority order.
    mirrors: Vec<ListReportMirror>,
}

#[derive(Debug, Serialize)]
struct ListReportMirror {
    of: String,
    url: String,
    priority: i32,
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

/// Map a host segment to the `RepoCreator` adapter `creator_for_url`
/// would pick. `None` means there is no adapter and `vibe registry
/// publish` would fail with `UnsupportedHost`. Pure read of the
/// dispatch rule in `vibe-publish::creator_for_url`; kept in sync by
/// hand because the rule is short and keeping it in code-as-data
/// would defer the user-facing label here from the rule there for no
/// real win.
fn adapter_for_host(host: &str) -> Option<&'static str> {
    let h = host.to_ascii_lowercase();
    if h == "github.com" || h.ends_with(".github.com") {
        return Some("github");
    }
    if h == "gitverse.ru" || h.ends_with(".gitverse.ru") {
        return Some("gitverse");
    }
    None
}

fn run_list(ctx: &output::Context, args: RegistryListArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(ProjectManifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = ProjectManifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let mut registries: Vec<ListReportRegistry> = Vec::with_capacity(manifest.registries.len());
    for reg in &manifest.registries {
        let host = extract_host_segment(&reg.url).unwrap_or_else(|_| String::from("?"));
        let org = extract_org_segment(&reg.url).unwrap_or_else(|_| String::from("?"));
        let adapter = adapter_for_host(&host).map(String::from);
        let naming_label = match reg.naming {
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

/// Parse the `--naming` CLI argument. Mirrors the serde `rename`s on
/// `NamingConvention` so what users type matches the `vibe.toml`
/// spelling exactly.
fn parse_naming(s: &str) -> Result<NamingConvention> {
    match s {
        "kind-name" => Ok(NamingConvention::KindName),
        "name" => Ok(NamingConvention::Name),
        "kind/name" => Ok(NamingConvention::KindSlashName),
        other => Err(anyhow!(
            "unknown naming convention `{other}` — must be one of `kind-name`, `name`, `kind/name`"
        )),
    }
}

fn run_add(ctx: &output::Context, args: RegistryAddArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(ProjectManifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let mut manifest = ProjectManifest::read(&manifest_path)
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
    let host = extract_host_segment(&args.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", args.url))?;
    let org = extract_org_segment(&args.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", args.url))?;

    let naming = match args.naming.as_deref() {
        Some(s) => parse_naming(s)?,
        None => NamingConvention::default(),
    };

    let position_label = match args.position.as_str() {
        "primary" | "append" => args.position.as_str(),
        other => bail!(
            "unknown --position `{other}` — must be `primary` or `append`"
        ),
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
        .unwrap_or_else(|| " (adapter: none — `vibe registry publish` won't dispatch here)".to_string());
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

#[derive(Debug, Serialize)]
struct SetMirrorReport {
    ok: bool,
    command: &'static str,
    mirror: ListReportMirror,
    /// Which registries this mirror now attaches to. `*` always
    /// attaches to all; a named `of` attaches to one.
    attached_to: Vec<String>,
    /// Total `[[mirror]]` count after the add.
    total_mirrors: usize,
}

fn run_set_mirror(ctx: &output::Context, args: RegistrySetMirrorArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(ProjectManifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let mut manifest = ProjectManifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    if args.of.trim().is_empty() {
        bail!("--of (target registry name) must be non-empty; use `*` for any registry");
    }

    // Validate that named `of` targets resolve to a real `[[registry]]`.
    // The wildcard `*` is allowed even when no registries exist — it is
    // a forward-compatible declaration that any future registry should
    // try this mirror.
    if args.of != "*" && manifest.registry_by_name(&args.of).is_none() {
        let known: Vec<&str> = manifest.registries.iter().map(|r| r.name.as_str()).collect();
        let known_text = if known.is_empty() {
            "(none configured)".to_string()
        } else {
            known.join(", ")
        };
        bail!(
            "no `[[registry]]` named `{}` in `{}`. Known registries: {}. Use `*` to target every registry.",
            args.of,
            manifest_path.display(),
            known_text
        );
    }

    // Mirror URL validation. A `[[mirror]]` is an availability copy of
    // the same source — consumed only by `git ls-remote` / `git fetch`,
    // never handed to a `RepoCreator` adapter — so the org/host
    // extraction that `[[registry]]` URLs require does not apply here.
    // In particular, `vibe registry vendor` produces `file:///<dir>`
    // mirror URLs that have no host or org segment by construction;
    // refusing them at this gate would self-contradict. Cheap sanity:
    // non-empty after trim. Anything past that is `git`'s job to reject
    // at fetch time (and `MultiRegistryResolver` surfaces the diagnostic
    // to the operator with the failing URL inline).
    let url_trimmed = args.url.trim();
    if url_trimmed.is_empty() {
        bail!("mirror URL must be non-empty");
    }

    // Exact duplicate guard. A repeat add of the same `(of, url)` is
    // almost always a typo — refuse rather than silently double up
    // and let the priority chain end up with two identical entries.
    // Different priority for the same `(of, url)` is also refused —
    // edit the manifest by hand for that case until `set-priority`
    // lands. Different URL with the same `of` is fine; that's the
    // whole point of having a chain.
    if manifest
        .mirrors
        .iter()
        .any(|m| m.of == args.of && m.url == args.url)
    {
        bail!(
            "a `[[mirror]]` with of=`{}` and the same URL already exists in `{}`. Remove or edit the existing block before adding another.",
            args.of,
            manifest_path.display()
        );
    }

    let new = MirrorSection {
        of: args.of.clone(),
        url: args.url.clone(),
        priority: args.priority,
    };
    manifest.mirrors.push(new.clone());

    manifest
        .write(&manifest_path)
        .with_context(|| format!("writing `{}`", manifest_path.display()))?;

    // Compute which registries this mirror now attaches to. `*` →
    // every registry; otherwise the single named registry.
    let attached_to: Vec<String> = if args.of == "*" {
        manifest
            .registries
            .iter()
            .map(|r| r.name.clone())
            .collect()
    } else {
        vec![args.of.clone()]
    };
    let mirror_view = ListReportMirror {
        of: new.of.clone(),
        url: new.url.clone(),
        priority: new.priority,
    };

    if ctx.is_json() {
        ctx.emit_json(&SetMirrorReport {
            ok: true,
            command: "registry:set-mirror",
            mirror: mirror_view,
            attached_to,
            total_mirrors: manifest.mirrors.len(),
        })?;
        return Ok(());
    }

    let attached_text = if args.of == "*" {
        if attached_to.is_empty() {
            "every future registry (no `[[registry]]` configured yet)".to_string()
        } else {
            format!(
                "every registry ({})",
                attached_to
                    .iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    } else {
        format!("registry `{}`", args.of)
    };
    ctx.step(&format!(
        "Added `[[mirror]]` of=`{}` priority={} → {} (attaches to {})",
        new.of, new.priority, new.url, attached_text
    ));
    ctx.summary(&format!(
        "\nvibe registry set-mirror: {} total mirror{} configured.",
        manifest.mirrors.len(),
        if manifest.mirrors.len() == 1 { "" } else { "s" }
    ));
    Ok(())
}

#[derive(Debug, Serialize)]
struct RemoveReport {
    ok: bool,
    command: &'static str,
    /// `"registry"` or `"mirror"` — what was removed.
    target: &'static str,
    /// For `target == "registry"`, the name. For `target == "mirror"`,
    /// `<of>:<url>`.
    identity: String,
    total_registries: usize,
    total_mirrors: usize,
}

fn run_remove(ctx: &output::Context, args: RegistryRemoveArgs) -> Result<()> {
    match args.target {
        RegistryRemoveTarget::Registry(sub) => run_remove_registry(ctx, sub),
        RegistryRemoveTarget::Mirror(sub) => run_remove_mirror(ctx, sub),
    }
}

fn run_remove_registry(
    ctx: &output::Context,
    args: RegistryRemoveRegistryArgs,
) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(ProjectManifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let mut manifest = ProjectManifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    if manifest.registry_by_name(&args.name).is_none() {
        let known: Vec<&str> = manifest.registries.iter().map(|r| r.name.as_str()).collect();
        let known_text = if known.is_empty() {
            "(none configured)".to_string()
        } else {
            known.join(", ")
        };
        bail!(
            "no `[[registry]]` named `{}` in `{}`. Known: {}.",
            args.name,
            manifest_path.display(),
            known_text
        );
    }

    // Refuse to orphan named mirrors. A `[[mirror]] of = "<name>"`
    // referring to a now-removed registry would never be consulted —
    // the manifest would still be parseable but operationally nonsense.
    // Wildcard `of = "*"` mirrors are fine; they apply to whatever
    // registries exist.
    let orphaned: Vec<&MirrorSection> = manifest
        .mirrors
        .iter()
        .filter(|m| m.of == args.name)
        .collect();
    if !orphaned.is_empty() {
        let urls: Vec<String> = orphaned.iter().map(|m| m.url.clone()).collect();
        bail!(
            "cannot remove `[[registry]]` `{}`: {} `[[mirror]]` block(s) target it ({}). Remove those mirrors first with `vibe registry remove mirror <of> <url>`.",
            args.name,
            urls.len(),
            urls.join(", ")
        );
    }

    manifest.registries.retain(|r| r.name != args.name);

    manifest
        .write(&manifest_path)
        .with_context(|| format!("writing `{}`", manifest_path.display()))?;

    if ctx.is_json() {
        ctx.emit_json(&RemoveReport {
            ok: true,
            command: "registry:remove",
            target: "registry",
            identity: args.name.clone(),
            total_registries: manifest.registries.len(),
            total_mirrors: manifest.mirrors.len(),
        })?;
        return Ok(());
    }

    ctx.step(&format!("Removed `[[registry]]` `{}`", args.name));
    ctx.summary(&format!(
        "\nvibe registry remove: {} registr{} remain.",
        manifest.registries.len(),
        if manifest.registries.len() == 1 {
            "y"
        } else {
            "ies"
        }
    ));
    Ok(())
}

fn run_remove_mirror(ctx: &output::Context, args: RegistryRemoveMirrorArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(ProjectManifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let mut manifest = ProjectManifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let before = manifest.mirrors.len();
    manifest
        .mirrors
        .retain(|m| !(m.of == args.of && m.url == args.url));
    let after = manifest.mirrors.len();

    if before == after {
        bail!(
            "no `[[mirror]]` in `{}` matches of=`{}` url=`{}`. Use `vibe registry list` to see what's configured.",
            manifest_path.display(),
            args.of,
            args.url
        );
    }
    if before - after > 1 {
        // Shouldn't happen if `set-mirror` enforces uniqueness on
        // (of, url), but if a hand-edited manifest carries duplicates
        // we drop them all and tell the user.
        eprintln!(
            "warning: removed {} `[[mirror]]` blocks matching of=`{}` url=`{}` (duplicates were present)",
            before - after,
            args.of,
            args.url
        );
    }

    manifest
        .write(&manifest_path)
        .with_context(|| format!("writing `{}`", manifest_path.display()))?;

    if ctx.is_json() {
        ctx.emit_json(&RemoveReport {
            ok: true,
            command: "registry:remove",
            target: "mirror",
            identity: format!("{}:{}", args.of, args.url),
            total_registries: manifest.registries.len(),
            total_mirrors: manifest.mirrors.len(),
        })?;
        return Ok(());
    }

    ctx.step(&format!(
        "Removed `[[mirror]]` of=`{}` url=`{}`",
        args.of, args.url
    ));
    ctx.summary(&format!(
        "\nvibe registry remove: {} mirror{} remain.",
        manifest.mirrors.len(),
        if manifest.mirrors.len() == 1 { "" } else { "s" }
    ));
    Ok(())
}

// ===================== vendor =====================
//
// `vibe registry vendor [--out <dir>] [--force]` — generates a local
// directory that vibe can later use as `[[mirror]] url =
// "file:///abs/path"` for offline / air-gapped installs. Each
// `[[registry]]`-served lockfile entry produces a bare git repo
// `<out>/<naming.repo_name(kind,name)>.git/` populated from the
// matching per-package cache clone.
//
// Spec: PROP-002 §2.3 (mirror layer), §6 (Phase B preview).

#[derive(Debug, Serialize)]
struct VendorReport {
    ok: bool,
    command: &'static str,
    out_dir: String,
    /// Suggested `[[mirror]]` snippet the operator can paste into
    /// `vibe.toml`. The URL is `file://` + the absolute, forward-slash
    /// form of `out_dir`.
    suggested_mirror_url: String,
    vendored: Vec<VendoredReportEntry>,
    skipped: Vec<SkippedReportEntry>,
}

#[derive(Debug, Serialize)]
struct VendoredReportEntry {
    kind: String,
    name: String,
    /// Registry that originally served this package — what `vibe.lock`
    /// records under `registry`.
    registry: String,
    repo_dir: String,
    /// What `vibe.lock` records under `source_ref` — typically
    /// `v<version>`. Vendored repo carries this tag.
    #[serde(rename = "ref")]
    refname: String,
}

fn run_vendor(ctx: &output::Context, args: RegistryVendorArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(ProjectManifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = ProjectManifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let lockfile_path = project_root.join(Lockfile::FILENAME);
    if !lockfile_path.exists() {
        bail!(
            "no `vibe.lock` in `{}`. Run `vibe install` first — vendoring is driven by the lockfile, not the manifest.",
            project_root.display()
        );
    }
    let lockfile = Lockfile::read(&lockfile_path)
        .with_context(|| format!("reading `{}`", lockfile_path.display()))?;

    if manifest.registries.is_empty() {
        bail!(
            "no `[[registry]]` entries in `{}`. Vendor only mirrors registry-served packages; \
             projects using only `--registry <path>` or `[[override]]` have nothing to vendor.",
            manifest_path.display()
        );
    }

    let out_dir = args
        .out
        .as_ref()
        .map(|p| project_root.join(p))
        .unwrap_or_else(|| project_root.join("vendor"));

    // Safety: never silently overwrite operator content. `--force`
    // wipes; without it, a non-empty target dir is a hard error.
    if out_dir.exists() {
        let mut iter = std::fs::read_dir(&out_dir)
            .with_context(|| format!("reading `{}`", out_dir.display()))?;
        let non_empty = iter.next().is_some();
        if non_empty && !args.force {
            bail!(
                "`{}` exists and is not empty. Pass `--force` to wipe and re-vendor, \
                 or pick a different `--out`.",
                out_dir.display()
            );
        }
        if args.force {
            std::fs::remove_dir_all(&out_dir).with_context(|| {
                format!("wiping `{}`", out_dir.display())
            })?;
        }
    }
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("creating `{}`", out_dir.display()))?;

    let mrr = MultiRegistryResolver::open(
        &manifest.registries,
        &manifest.mirrors,
        &manifest.overrides,
    )
    .context("opening multi-registry resolver")?;

    ctx.heading(&format!(
        "Vendoring {} lockfile entr{} into `{}`",
        lockfile.packages.len(),
        if lockfile.packages.len() == 1 { "y" } else { "ies" },
        out_dir.display()
    ));

    let mut vendored: Vec<VendoredReportEntry> = Vec::new();
    let mut skipped: Vec<SkippedReportEntry> = Vec::new();

    for entry in &lockfile.packages {
        if entry.overridden {
            skipped.push(SkippedReportEntry {
                kind: entry.kind.as_str().to_string(),
                name: entry.name.clone(),
                reason: format!(
                    "[[override]]-served (source_url `{}`); vendor it manually if you need offline coverage",
                    entry.source_url
                ),
            });
            continue;
        }
        let Some(reg_name) = entry.registry.as_deref() else {
            skipped.push(SkippedReportEntry {
                kind: entry.kind.as_str().to_string(),
                name: entry.name.clone(),
                reason: "lockfile entry has no `registry` (likely installed via `--registry <path>` or a legacy v1 path)"
                    .to_string(),
            });
            continue;
        };
        let Some(reg) = mrr.registries().iter().find(|r| r.name() == reg_name) else {
            skipped.push(SkippedReportEntry {
                kind: entry.kind.as_str().to_string(),
                name: entry.name.clone(),
                reason: format!(
                    "lockfile names registry `{reg_name}` but no `[[registry]]` with that name exists in `vibe.toml`"
                ),
            });
            continue;
        };

        let refname = entry
            .source_ref
            .clone()
            .unwrap_or_else(|| format!("v{}", entry.version));

        // Make sure the per-package clone is on disk and at the
        // requested ref. `refresh_package` is mirror-aware, so a fresh
        // `vibe registry vendor` against an unreachable primary still
        // works as long as some `[[mirror]]` URL is reachable.
        reg.refresh_package(entry.kind, &entry.name, &refname)
            .with_context(|| {
                format!(
                    "refreshing per-package clone for `{}:{}` against `{}`",
                    entry.kind, entry.name, refname
                )
            })?;

        let clone_dir = reg.package_clone_dir(entry.kind, &entry.name);
        let clone_git = clone_dir.join(".git");
        if !clone_git.is_dir() {
            // Should not happen after a successful `refresh_package`,
            // but guard anyway — `bare_clone_from_clone` reads
            // `.git/` and an explicit error here beats a confusing
            // I/O error two layers down.
            bail!(
                "per-package clone for `{}:{}` lacks a `.git/` after refresh — registry returned without populating the cache (`{}`)",
                entry.kind,
                entry.name,
                clone_dir.display()
            );
        }

        let repo_name = reg.naming().repo_name(entry.kind, &entry.name);
        let vendor_repo = out_dir.join(format!("{repo_name}.git"));
        if vendor_repo.exists() {
            std::fs::remove_dir_all(&vendor_repo).with_context(|| {
                format!("wiping stale vendor repo `{}`", vendor_repo.display())
            })?;
        }
        if let Some(parent) = vendor_repo.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("creating parent dir `{}`", parent.display())
            })?;
        }

        bare_clone_from_clone(&clone_git, &vendor_repo).with_context(|| {
            format!(
                "vendoring `{}:{}` into `{}`",
                entry.kind,
                entry.name,
                vendor_repo.display()
            )
        })?;

        ctx.step(&format!(
            "{}:{} @ {} → {}",
            entry.kind,
            entry.name,
            refname,
            forward_slash_display(&vendor_repo)
        ));
        vendored.push(VendoredReportEntry {
            kind: entry.kind.as_str().to_string(),
            name: entry.name.clone(),
            registry: reg_name.to_string(),
            repo_dir: forward_slash_display(&vendor_repo),
            refname,
        });
    }

    let suggested_url = file_url_for_dir(&out_dir);
    write_vendor_readme(&out_dir, &suggested_url, &vendored)
        .context("writing vendor README.md")?;

    if !skipped.is_empty() {
        for s in &skipped {
            ctx.skipped(&format!("{}:{}", s.kind, s.name), &s.reason);
        }
    }

    if ctx.is_json() {
        ctx.emit_json(&VendorReport {
            ok: true,
            command: "registry:vendor",
            out_dir: forward_slash_display(&out_dir),
            suggested_mirror_url: suggested_url.clone(),
            vendored,
            skipped,
        })?;
        return Ok(());
    }

    ctx.summary(&format!(
        "\nvibe registry vendor: {} vendored, {} skipped. \
         Wire as `[[mirror]] of = \"<registry>\" url = \"{}\"` to enable offline fallback.",
        vendored.len(),
        skipped.len(),
        suggested_url
    ));
    Ok(())
}

/// Produce a `file://` URL for an absolute directory path, forward-slashed
/// so the URL is well-formed on Windows (`file:///C:/Users/...`) and Unix
/// (`file:///path/...`).
fn file_url_for_dir(dir: &Path) -> String {
    let mut s = dir.to_string_lossy().replace('\\', "/");
    // Strip Windows UNC `\\?\` prefix that may survive `canonicalize`.
    if let Some(stripped) = s.strip_prefix("//?/") {
        s = stripped.to_string();
    }
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    format!("file://{s}")
}

fn forward_slash_display(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = s.strip_prefix("//?/") {
        s = stripped.to_string();
    }
    s
}

/// Build a bare repo at `dst` from the contents of a non-bare clone's
/// `.git/` at `src_git`. Implementation: copy every file under `src_git/`
/// recursively into `dst/`, preserving relative paths. The result is a
/// directory whose layout (`HEAD`, `refs/`, `objects/`, …) is what `git
/// clone <dst>` and `git ls-remote <dst>` consume — git auto-detects
/// bare-ness from the layout, the `core.bare` config flag is informational
/// from the consumer's side.
///
/// We deliberately do NOT shell out to `git clone --bare` because (a) it
/// would couple `vibe registry vendor` to git availability at vendor-time,
/// not just install-time, and (b) the copy is straightforward and easier
/// to test without spawning subprocesses. Hard-links would be faster but
/// would tie the vendor dir's lifetime to the source clone's filesystem;
/// a plain `fs::copy` produces a self-contained vendor that survives a
/// `~/.vibe/registries` wipe.
fn bare_clone_from_clone(src_git: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)
        .with_context(|| format!("creating `{}`", dst.display()))?;
    for entry in walkdir::WalkDir::new(src_git).into_iter().filter_map(|e| e.ok()) {
        let rel = entry
            .path()
            .strip_prefix(src_git)
            .unwrap_or(entry.path());
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)
                .with_context(|| format!("creating `{}`", target.display()))?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating `{}`", parent.display()))?;
            }
            std::fs::copy(entry.path(), &target)
                .with_context(|| format!("copying to `{}`", target.display()))?;
        }
    }
    Ok(())
}

/// Generate a small `README.md` at the root of the vendor directory
/// explaining what it is and how to wire it as `[[mirror]]`. Idempotent:
/// any prior README is overwritten as part of `--force` / first vendor.
fn write_vendor_readme(
    out_dir: &Path,
    suggested_url: &str,
    vendored: &[VendoredReportEntry],
) -> Result<()> {
    let mut body = String::new();
    body.push_str("# vibe vendor\n\n");
    body.push_str(
        "Local mirror directory generated by `vibe registry vendor`. Each entry \
        below is a bare git repository populated from the per-package cache clone \
        for the package referenced by `vibe.lock`.\n\n\
        Wire it into your `vibe.toml` as a `[[mirror]]` for offline / air-gapped \
        installs:\n\n",
    );
    body.push_str("```toml\n");
    body.push_str("[[mirror]]\n");
    body.push_str("of = \"<registry-name>\"  # or \"*\" to mirror every registry\n");
    body.push_str(&format!("url = \"{suggested_url}\"\n"));
    body.push_str("priority = 0\n");
    body.push_str("```\n\n");
    body.push_str(
        "When the primary registry is reachable, `vibe install` walks it first per \
        PROP-002 §2.3; the file:// mirror takes over only if the primary is \
        unavailable, which is the offline / air-gapped path.\n\n",
    );
    if vendored.is_empty() {
        body.push_str("_(No registry-served lockfile entries were vendored on this run.)_\n");
    } else {
        body.push_str("## Contents\n\n");
        for v in vendored {
            body.push_str(&format!(
                "- `{}:{}` @ `{}` — `{}` (from registry `{}`)\n",
                v.kind, v.name, v.refname, v.repo_dir, v.registry
            ));
        }
    }
    let readme_path = out_dir.join("README.md");
    std::fs::write(&readme_path, body)
        .with_context(|| format!("writing `{}`", readme_path.display()))?;
    Ok(())
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .map_err(|e| anyhow!("canonicalizing `{}`: {e}", path.display()))?;
    Ok(super::init::strip_unc_public(canonical))
}

#[derive(Debug, Serialize)]
struct PublishReport {
    ok: bool,
    command: &'static str,
    host: String,
    org_url: String,
    repo_name: String,
    repo_url: String,
    tag: String,
    created_repo: bool,
    dry_run: bool,
    /// Status of the optional post-publish index hook. Always
    /// present; `fired = false` + `error = None` means the hook was
    /// dormant (no env config) and the operator wanted no index update.
    #[serde(skip_serializing_if = "Option::is_none")]
    index_hook: Option<vibe_publish::HookReport>,
}

/// Envelope emitted when the operator targets a host whose publish path
/// is intentionally a stub (today: GitVerse — see `run_publish`). Marked
/// `ok: false` so CI / scripting can distinguish stub-paths from a
/// successful publish without parsing the message.
#[derive(Debug, Serialize)]
struct PublishStubReport {
    ok: bool,
    command: &'static str,
    host: String,
    org_url: String,
    registry: String,
    stub: bool,
    reason: String,
}

/// Envelope emitted on the `--repo-url` no-API path. The shape mirrors
/// [`PublishReport`] minus fields that don't apply (no `org_url` because
/// the URL is repo-level; no `created_repo` because direct-push never
/// provisions). `mode = "direct-git"` lets consumers distinguish this
/// path from the registry path without parsing host strings.
#[derive(Debug, Serialize)]
struct DirectPublishReport {
    ok: bool,
    command: &'static str,
    mode: &'static str,
    host: String,
    repo_url: String,
    repo_name: String,
    tag: String,
    dry_run: bool,
}

fn run_publish(ctx: &output::Context, args: RegistryPublishArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(ProjectManifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = ProjectManifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    // `--repo-url <url>`: bypass registries, host adapters, tokens, and
    // every host API. Operator supplied an SSH/HTTPS URL pointing at an
    // already-provisioned repo; we just push the package contents +
    // tag. Local git resolves credentials however it normally does
    // (SSH agent / credential.helper / netrc). No `[[registry]]` entry
    // is consulted; `vibe.toml` need only exist (asserted above so the
    // command behaves consistently w.r.t. project-root lookup).
    if let Some(direct_url) = args.repo_url.as_deref() {
        return run_publish_direct(ctx, &args, direct_url);
    }

    if manifest.registries.is_empty() {
        bail!(
            "no `[[registry]]` entries in `{}`. `vibe registry publish` needs a target registry, \
             or pass `--repo-url <git-url>` to push directly to a known repo without an API call.",
            manifest_path.display()
        );
    }

    let registry_section = match &args.registry {
        Some(name) => manifest
            .registry_by_name(name)
            .ok_or_else(|| anyhow!("no `[[registry]]` named `{name}` in `{}`", manifest_path.display()))?,
        None => manifest
            .primary_registry()
            .ok_or_else(|| anyhow!("no `[[registry]]` configured"))?,
    };

    // Canonicalise the source dir.
    let source_dir = args
        .source
        .canonicalize()
        .with_context(|| format!("source path `{}`", args.source.display()))?;
    let source_dir = super::init::strip_unc_public(source_dir);

    // Pick the host adapter from the registry URL's host segment per
    // PROP-002 §2.10. `creator_for_url` returns a boxed `RepoCreator`
    // already scoped to the configured org; that's the boundary that
    // enforces "never operate outside the configured organization"
    // per PROP-000 §20. Each adapter additionally validates the org
    // at every method call as defence in depth.
    let host = extract_host_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;
    let org_segment = extract_org_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;

    // GitVerse publish is currently a stub. The GitVerse public REST API
    // does not yet expose org-scoped repo creation (`POST /orgs/<org>/repos`
    // returns no parity for the GitHub flow `vibe registry publish`
    // depends on). Short-circuit here with a clear console message and
    // a JSON envelope marked `stub: true`, so consumers learn the
    // limitation before any token is loaded or any network call is made.
    // GitHub stays the canonical publish target; resolve-time reads
    // against GitVerse continue to work via `MultiRegistryResolver`.
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "gitverse.ru" || host_lower.ends_with(".gitverse.ru") {
        let reason = format!(
            "GitVerse publishing is not implemented yet — the GitVerse public API does not \
             expose org-scoped repository creation, so `vibe registry publish` cannot drive \
             the create-repo + push-tag flow end to end. Publish to a GitHub `[[registry]]` \
             instead (default: `{}` → `{}`), or run with `--registry <name>` to pick a \
             different target. Resolve-time reads against `{}` are unaffected.",
            DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_URL, registry_section.name
        );
        if ctx.is_json() {
            ctx.emit_json(&PublishStubReport {
                ok: false,
                command: "registry:publish",
                host: host.clone(),
                org_url: registry_section.url.clone(),
                registry: registry_section.name.clone(),
                stub: true,
                reason,
            })?;
        } else {
            ctx.heading(&format!(
                "Publishing {} → registry `{}` (`{}`)",
                source_dir.display(),
                registry_section.name,
                registry_section.url,
            ));
            ctx.summary(&format!("\nvibe registry publish: {reason}"));
        }
        return Ok(());
    }

    ctx.heading(&format!(
        "Publishing {} → registry `{}` (`{}`){}",
        source_dir.display(),
        registry_section.name,
        registry_section.url,
        if args.dry_run { " [dry-run]" } else { "" },
    ));

    let token = load_token_for_host(&host).context("loading publish token")?;
    // The CLI surfaces the *source* of the token (env var, file path),
    // never the value. Token::Display redacts to `***` defensively in
    // case any future code path reaches for it.
    ctx.step(&format!(
        "Loaded publish token from {} (value redacted)",
        match token.source() {
            vibe_publish::TokenSource::Explicit => "explicit argument".to_string(),
            vibe_publish::TokenSource::EnvVar(name) => format!("$ {name}"),
            vibe_publish::TokenSource::File(p) => p.display().to_string(),
        }
    ));
    let creator = creator_for_url(&registry_section.url, org_segment, token)
        .map_err(|e| anyhow!("{e}"))?;

    let config = PublishConfig {
        source_dir: source_dir.clone(),
        org_url: registry_section.url.clone(),
        naming: registry_section.naming,
        tag_prefix: "v".to_string(),
        dry_run: args.dry_run,
    };

    let outcome = Publisher::new(creator.as_ref())
        .publish(&config)
        .map_err(|e| anyhow!("{e}"))?;

    // Optional post-publish hook — POST the freshly-built entry to a
    // configured vibevm-index server. Activation is per-registry via
    // env vars; the hook stays dormant when either VIBEVM_INDEX_URL_<R>
    // or VIBEVM_INDEX_TOKEN_<R> is unset. Hook failures are warnings,
    // never fail the publish itself (PROP-005 §2.14).
    let hook_report = if outcome.dry_run {
        // Dry-runs do not push real bytes; suppress the hook.
        vibe_publish::HookReport::dormant()
    } else {
        vibe_publish::fire_index_hook(&outcome, &source_dir, &registry_section.name)
    };

    if ctx.is_json() {
        ctx.emit_json(&PublishReport {
            ok: true,
            command: "registry:publish",
            host: outcome.host.clone(),
            org_url: registry_section.url.clone(),
            repo_name: outcome.repo_name.clone(),
            repo_url: outcome.repo_url.clone(),
            tag: outcome.tag.clone(),
            created_repo: outcome.created_repo,
            dry_run: outcome.dry_run,
            index_hook: Some(hook_report),
        })?;
        return Ok(());
    }
    if hook_report.fired {
        ctx.step(&format!(
            "Index hook posted to {} (status {})",
            hook_report
                .url_endpoint
                .as_deref()
                .unwrap_or("(unknown)"),
            hook_report.status.unwrap_or(0)
        ));
    } else if let Some(err) = &hook_report.error {
        tracing::warn!(target: "vibe_cli::registry::publish", "index hook skipped: {err}");
    }

    let action_verb = if outcome.dry_run {
        if outcome.created_repo {
            "Would create"
        } else {
            "Would reuse existing"
        }
    } else if outcome.created_repo {
        "Created"
    } else {
        "Reusing existing"
    };
    ctx.step(&format!(
        "{} repository `{}` on `{}`",
        action_verb, outcome.repo_name, outcome.host
    ));
    if outcome.dry_run {
        ctx.summary(&format!(
            "\nvibe registry publish [dry-run]: would push to `{}` and tag `{}`. \
             Re-run without `--dry-run` to apply.",
            outcome.repo_url, outcome.tag
        ));
    } else {
        ctx.summary(&format!(
            "\nvibe registry publish: pushed `{}:{}` @ {} → `{}` (tag `{}`).",
            outcome.kind, outcome.name, outcome.version, outcome.repo_url, outcome.tag
        ));
    }
    Ok(())
}

/// Execute the no-API publish path. Builds a [`DirectGitCreator`] for
/// the supplied URL, threads it through the regular [`Publisher`] flow
/// — which short-circuits at `direct_repo_url` — and renders the
/// outcome. No token loading, no host-API call.
fn run_publish_direct(
    ctx: &output::Context,
    args: &RegistryPublishArgs,
    direct_url: &str,
) -> Result<()> {
    let url = direct_url.trim();
    if url.is_empty() {
        bail!("--repo-url must be a non-empty git URL");
    }

    let source_dir = args
        .source
        .canonicalize()
        .with_context(|| format!("source path `{}`", args.source.display()))?;
    let source_dir = super::init::strip_unc_public(source_dir);

    ctx.heading(&format!(
        "Publishing {} → direct git URL `{}`{}",
        source_dir.display(),
        url,
        if args.dry_run { " [dry-run]" } else { "" },
    ));
    ctx.step("No host API in play — pushing with local git credentials.");

    let creator = DirectGitCreator::new(url.to_string());
    // `org_url` and `naming` are irrelevant on the direct path —
    // [`Publisher::publish`] short-circuits before consulting them.
    // Pass through harmless placeholders so the config validates.
    let config = PublishConfig {
        source_dir: source_dir.clone(),
        org_url: url.to_string(),
        naming: NamingConvention::default(),
        tag_prefix: "v".to_string(),
        dry_run: args.dry_run,
    };

    let outcome = Publisher::new(&creator)
        .publish(&config)
        .map_err(|e| anyhow!("{e}"))?;

    if ctx.is_json() {
        ctx.emit_json(&DirectPublishReport {
            ok: true,
            command: "registry:publish",
            mode: "direct-git",
            host: outcome.host.clone(),
            repo_url: outcome.repo_url.clone(),
            repo_name: outcome.repo_name.clone(),
            tag: outcome.tag.clone(),
            dry_run: outcome.dry_run,
        })?;
        return Ok(());
    }

    if outcome.dry_run {
        ctx.summary(&format!(
            "\nvibe registry publish [dry-run]: would push `{}:{}` @ {} → `{}` (tag `{}`). \
             Re-run without `--dry-run` to apply.",
            outcome.kind, outcome.name, outcome.version, outcome.repo_url, outcome.tag,
        ));
    } else {
        ctx.summary(&format!(
            "\nvibe registry publish: pushed `{}:{}` @ {} → `{}` (tag `{}`).",
            outcome.kind, outcome.name, outcome.version, outcome.repo_url, outcome.tag,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        adapter_for_host, bare_clone_from_clone, file_url_for_dir, parse_naming,
    };
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use vibe_core::manifest::NamingConvention;

    #[test]
    fn adapter_for_host_picks_github() {
        assert_eq!(adapter_for_host("github.com"), Some("github"));
        assert_eq!(adapter_for_host("api.github.com"), Some("github"));
        assert_eq!(adapter_for_host("GITHUB.com"), Some("github"));
    }

    #[test]
    fn adapter_for_host_picks_gitverse() {
        assert_eq!(adapter_for_host("gitverse.ru"), Some("gitverse"));
        assert_eq!(adapter_for_host("api.gitverse.ru"), Some("gitverse"));
    }

    #[test]
    fn adapter_for_host_returns_none_for_unknown_host() {
        assert_eq!(adapter_for_host("example.invalid"), None);
        assert_eq!(adapter_for_host(""), None);
    }

    #[test]
    fn parse_naming_accepts_canonical_spellings() {
        assert!(matches!(
            parse_naming("kind-name").unwrap(),
            NamingConvention::KindName
        ));
        assert!(matches!(
            parse_naming("name").unwrap(),
            NamingConvention::Name
        ));
        assert!(matches!(
            parse_naming("kind/name").unwrap(),
            NamingConvention::KindSlashName
        ));
    }

    #[test]
    fn parse_naming_rejects_unknown_value() {
        let err = parse_naming("KindName").unwrap_err();
        // Spelling mismatch — must match the serde rename exactly.
        assert!(err.to_string().contains("unknown naming convention"));
    }

    #[test]
    fn bare_clone_copies_git_tree_recursively() {
        // Synthesize a minimal `.git/`-shape tree and verify
        // `bare_clone_from_clone` reproduces the layout at the target.
        // No real git; just files + directories in the shape git would
        // produce.
        let src = tempdir().unwrap();
        let src_git = src.path().join(".git");
        fs::create_dir_all(src_git.join("refs/heads")).unwrap();
        fs::create_dir_all(src_git.join("refs/tags")).unwrap();
        fs::create_dir_all(src_git.join("objects/pack")).unwrap();
        fs::write(src_git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(src_git.join("config"), "[core]\n\trepositoryformatversion = 0\n").unwrap();
        fs::write(src_git.join("refs/heads/main"), "abc123\n").unwrap();
        fs::write(src_git.join("refs/tags/v0.1.0"), "def456\n").unwrap();
        fs::write(src_git.join("objects/pack/pack-x.idx"), b"binary").unwrap();

        let dst_root = tempdir().unwrap();
        let dst = dst_root.path().join("flow-wal.git");
        bare_clone_from_clone(&src_git, &dst).unwrap();

        // Every file the helper saw is present at the same relative
        // path under `dst`.
        assert_eq!(
            fs::read_to_string(dst.join("HEAD")).unwrap(),
            "ref: refs/heads/main\n"
        );
        assert_eq!(
            fs::read_to_string(dst.join("refs/heads/main")).unwrap(),
            "abc123\n"
        );
        assert_eq!(
            fs::read_to_string(dst.join("refs/tags/v0.1.0")).unwrap(),
            "def456\n"
        );
        assert_eq!(
            fs::read(dst.join("objects/pack/pack-x.idx")).unwrap(),
            b"binary".to_vec()
        );
        // Empty directories survive the copy too — `objects/` is
        // implicitly preserved by walking, even when only `pack/`
        // contains files.
        assert!(dst.join("objects/pack").is_dir());
    }

    #[test]
    fn bare_clone_creates_dst_when_absent() {
        let src = tempdir().unwrap();
        let src_git = src.path().join(".git");
        fs::create_dir_all(&src_git).unwrap();
        fs::write(src_git.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let dst_root = tempdir().unwrap();
        let dst = dst_root.path().join("nested/flow-wal.git");
        // Caller (run_vendor) guarantees parent exists; the helper
        // creates the leaf. Pre-create the parent here to mirror that
        // contract.
        fs::create_dir_all(dst.parent().unwrap()).unwrap();
        bare_clone_from_clone(&src_git, &dst).unwrap();
        assert!(dst.join("HEAD").is_file());
    }

    #[test]
    fn file_url_for_dir_unix_absolute() {
        let url = file_url_for_dir(&PathBuf::from("/abs/path/to/vendor"));
        assert_eq!(url, "file:///abs/path/to/vendor");
    }

    #[test]
    fn file_url_for_dir_windows_drive_letter() {
        // PathBuf::from on a non-Windows platform won't drive-letter-
        // canonicalize, but the helper just transforms the string —
        // platform-independent.
        let url = file_url_for_dir(&PathBuf::from(r"C:\Users\foo\vendor"));
        assert_eq!(url, "file:///C:/Users/foo/vendor");
    }

    #[test]
    fn file_url_for_dir_strips_unc_prefix() {
        // `canonicalize` on Windows returns paths with the `\\?\`
        // prefix; the helper drops that so the URL is portable.
        let url = file_url_for_dir(&PathBuf::from(r"\\?\C:\Users\foo\vendor"));
        assert_eq!(url, "file:///C:/Users/foo/vendor");
    }
}
