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
use vibe_core::Group;
use vibe_core::manifest::{
    DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_URL, Lockfile, Manifest, MirrorSection,
    NamingConvention, RegistrySection,
};
use vibe_publish::{
    DirectGitCreator, PublishConfig, Publisher, creator_for_url, extract_host_segment,
    extract_org_segment, load_token_for_host,
};
use vibe_registry::{MultiRegistryResolver, RefreshedVia};

use crate::cli::{
    RegistryAddArgs, RegistryArgs, RegistryListArgs, RegistryPublishArgs, RegistryRedirectArgs,
    RegistryRedirectSyncArgs, RegistryRedirectUpdateArgs, RegistryRemoveArgs,
    RegistryRemoveMirrorArgs, RegistryRemoveRegistryArgs, RegistryRemoveTarget,
    RegistrySetMirrorArgs, RegistrySubcommand, RegistrySyncArgs, RegistryTestArgs,
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
        RegistrySubcommand::Test(sub) => run_test(ctx, sub),
        RegistrySubcommand::Redirect(sub) => run_redirect(ctx, sub),
        RegistrySubcommand::RedirectSync(sub) => run_redirect_sync(ctx, sub),
        RegistrySubcommand::RedirectUpdate(sub) => run_redirect_update(ctx, sub),
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
    group: String,
    name: String,
    via: String, // "registry:<name>" or "override"
    #[serde(rename = "ref")]
    refname: String,
}

#[derive(Debug, Serialize)]
struct SkippedReportEntry {
    group: String,
    name: String,
    reason: String,
}

fn run_sync(ctx: &output::Context, args: RegistrySyncArgs) -> Result<()> {
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

    let mrr =
        MultiRegistryResolver::open(&manifest.registries, &manifest.mirrors, &manifest.overrides)
            .context("opening multi-registry resolver")?;

    ctx.heading(&format!(
        "Syncing {} package clone{} referenced by lockfile",
        lockfile.packages.len(),
        if lockfile.packages.len() == 1 {
            ""
        } else {
            "s"
        }
    ));

    let report = mrr
        .refresh_lockfile_clones(&lockfile)
        .context("refreshing per-package clones")?;

    let json_refreshed: Vec<RefreshedReportEntry> = report
        .refreshed
        .iter()
        .map(|e| RefreshedReportEntry {
            group: e.group.as_str().to_string(),
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
            group: e.group.as_str().to_string(),
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
                "{}/{} @ {} via {}",
                e.group, e.name, e.refname, via_text
            ));
        }
    }
    if !report.skipped.is_empty() {
        for e in &report.skipped {
            ctx.skipped(&format!("{}/{}", e.group, e.name), &e.reason);
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
        "fqdn" => Ok(NamingConvention::Fqdn),
        "kind-name" => Ok(NamingConvention::KindName),
        "name" => Ok(NamingConvention::Name),
        "kind/name" => Ok(NamingConvention::KindSlashName),
        other => Err(anyhow!(
            "unknown naming convention `{other}` — must be one of `fqdn`, `kind-name`, `name`, `kind/name`"
        )),
    }
}

fn run_add(ctx: &output::Context, args: RegistryAddArgs) -> Result<()> {
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
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let mut manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    if args.of.trim().is_empty() {
        bail!("--of (target registry name) must be non-empty; use `*` for any registry");
    }

    // Validate that named `of` targets resolve to a real `[[registry]]`.
    // The wildcard `*` is allowed even when no registries exist — it is
    // a forward-compatible declaration that any future registry should
    // try this mirror.
    if args.of != "*" && manifest.registry_by_name(&args.of).is_none() {
        let known: Vec<&str> = manifest
            .registries
            .iter()
            .map(|r| r.name.as_str())
            .collect();
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
        manifest.registries.iter().map(|r| r.name.clone()).collect()
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

fn run_remove_registry(ctx: &output::Context, args: RegistryRemoveRegistryArgs) -> Result<()> {
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

    if manifest.registry_by_name(&args.name).is_none() {
        let known: Vec<&str> = manifest
            .registries
            .iter()
            .map(|r| r.name.as_str())
            .collect();
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
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let mut manifest = Manifest::read(&manifest_path)
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
    group: String,
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
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = Manifest::read(&manifest_path)
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
            std::fs::remove_dir_all(&out_dir)
                .with_context(|| format!("wiping `{}`", out_dir.display()))?;
        }
    }
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("creating `{}`", out_dir.display()))?;

    let mrr =
        MultiRegistryResolver::open(&manifest.registries, &manifest.mirrors, &manifest.overrides)
            .context("opening multi-registry resolver")?;

    ctx.heading(&format!(
        "Vendoring {} lockfile entr{} into `{}`",
        lockfile.packages.len(),
        if lockfile.packages.len() == 1 {
            "y"
        } else {
            "ies"
        },
        out_dir.display()
    ));

    let mut vendored: Vec<VendoredReportEntry> = Vec::new();
    let mut skipped: Vec<SkippedReportEntry> = Vec::new();

    for entry in &lockfile.packages {
        if entry.overridden {
            skipped.push(SkippedReportEntry {
                group: entry.group.as_str().to_string(),
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
                group: entry.group.as_str().to_string(),
                name: entry.name.clone(),
                reason: "lockfile entry has no `registry` (likely installed via `--registry <path>` or a legacy v1 path)"
                    .to_string(),
            });
            continue;
        };
        let Some(reg) = mrr.registries().iter().find(|r| r.name() == reg_name) else {
            skipped.push(SkippedReportEntry {
                group: entry.group.as_str().to_string(),
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
        reg.refresh_package(&entry.group, &entry.name, &refname)
            .with_context(|| {
                format!(
                    "refreshing per-package clone for `{}/{}` against `{}`",
                    entry.group, entry.name, refname
                )
            })?;

        let clone_dir = reg.package_clone_dir(&entry.group, &entry.name);
        let clone_git = clone_dir.join(".git");
        if !clone_git.is_dir() {
            // Should not happen after a successful `refresh_package`,
            // but guard anyway — `bare_clone_from_clone` reads
            // `.git/` and an explicit error here beats a confusing
            // I/O error two layers down.
            bail!(
                "per-package clone for `{}/{}` lacks a `.git/` after refresh — registry returned without populating the cache (`{}`)",
                entry.group,
                entry.name,
                clone_dir.display()
            );
        }

        let repo_name = reg
            .naming()
            .repo_name(Some(entry.kind), &entry.group, &entry.name)
            .with_context(|| {
                format!(
                    "deriving the vendor repo name for `{}/{}`",
                    entry.group, entry.name
                )
            })?;
        let vendor_repo = out_dir.join(format!("{repo_name}.git"));
        if vendor_repo.exists() {
            std::fs::remove_dir_all(&vendor_repo)
                .with_context(|| format!("wiping stale vendor repo `{}`", vendor_repo.display()))?;
        }
        if let Some(parent) = vendor_repo.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating parent dir `{}`", parent.display()))?;
        }

        bare_clone_from_clone(&clone_git, &vendor_repo).with_context(|| {
            format!(
                "vendoring `{}/{}` into `{}`",
                entry.group,
                entry.name,
                vendor_repo.display()
            )
        })?;

        ctx.step(&format!(
            "{}/{} @ {} → {}",
            entry.group,
            entry.name,
            refname,
            forward_slash_display(&vendor_repo)
        ));
        vendored.push(VendoredReportEntry {
            group: entry.group.as_str().to_string(),
            name: entry.name.clone(),
            registry: reg_name.to_string(),
            repo_dir: forward_slash_display(&vendor_repo),
            refname,
        });
    }

    let suggested_url = file_url_for_dir(&out_dir);
    write_vendor_readme(&out_dir, &suggested_url, &vendored).context("writing vendor README.md")?;

    if !skipped.is_empty() {
        for s in &skipped {
            ctx.skipped(&format!("{}/{}", s.group, s.name), &s.reason);
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
    std::fs::create_dir_all(dst).with_context(|| format!("creating `{}`", dst.display()))?;
    for entry in walkdir::WalkDir::new(src_git)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let rel = entry.path().strip_prefix(src_git).unwrap_or(entry.path());
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
                "- `{}/{}` @ `{}` — `{}` (from registry `{}`)\n",
                v.group, v.name, v.refname, v.repo_dir, v.registry
            ));
        }
    }
    let readme_path = out_dir.join("README.md");
    std::fs::write(&readme_path, body)
        .with_context(|| format!("writing `{}`", readme_path.display()))?;
    Ok(())
}

/// Extract the `(group, …)` half of a pkgref's identity, rejecting an
/// unqualified registry-subcommand argument (PROP-008 §2.4). Registry
/// resolution and repo naming are group-keyed; a bare name has no group.
fn require_group(pkgref: &vibe_core::PackageRef) -> Result<&Group> {
    pkgref.group.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "package reference `{pkgref}` is not group-qualified — write `<group>/<name>`"
        )
    })
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
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = Manifest::read(&manifest_path)
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
        Some(name) => manifest.registry_by_name(name).ok_or_else(|| {
            anyhow!(
                "no `[[registry]]` named `{name}` in `{}`",
                manifest_path.display()
            )
        })?,
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
    let creator =
        creator_for_url(&registry_section.url, org_segment, token).map_err(|e| anyhow!("{e}"))?;

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
            hook_report.url_endpoint.as_deref().unwrap_or("(unknown)"),
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

// ---------------------------------------------------------------------------
// vibe registry test
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct TestReport {
    ok: bool,
    command: &'static str,
    registries: Vec<TestReportRegistry>,
}

#[derive(Debug, Serialize)]
struct TestReportRegistry {
    name: String,
    url: String,
    auth: &'static str,
    /// One of:
    /// - `reachable` — host responded, package layout recognised.
    /// - `auth-required` — host returned 401 / 403; for
    ///   `auth = "none"` registries this means "host policy is
    ///   to demand credentials for missing repos" (GitVerse-style),
    ///   for `auth = "token-env"` / `"credential-helper"` it
    ///   means the credentials presented were rejected.
    /// - `unreachable` — DNS / TCP / cert error.
    /// - `missing-token` — `auth = "token-env"` declared but the
    ///   env-var resolved empty.
    /// - `unknown` — any other shape; details in `note`.
    status: &'static str,
    /// Human-readable elaboration when `status` alone isn't
    /// enough (token env-var name, error tail, etc.). `None` for
    /// the happy `reachable` path.
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

fn run_test(ctx: &output::Context, args: RegistryTestArgs) -> Result<()> {
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

    if manifest.registries.is_empty() {
        ctx.summary("No `[[registry]]` entries to probe. Add one with `vibe registry add` first.");
        if ctx.is_json() {
            ctx.emit_json(&TestReport {
                ok: true,
                command: "registry:test",
                registries: vec![],
            })?;
        }
        return Ok(());
    }

    // Build a `MultiRegistryResolver` so each registry inherits the
    // exact auth configuration the install path would use. We then
    // probe each registry by attempting to resolve a deliberately-
    // unique fake pkgref — every registry will return one of:
    // `UnknownPackage` (host responded, no such repo → reachable),
    // `Git(AuthFailed)` (401 / 403 → auth-required),
    // `Git(NetworkUnreachable)` (DNS / TCP fail → unreachable),
    // `MissingToken` (env-var unset → missing-token), or other
    // (unknown). The resolver runs through `try_lookup` and walks
    // mirrors, so the diagnostic reflects what the install path
    // would actually see.
    use vibe_core::PackageRef;
    use vibe_registry::git_backend::GitError;
    use vibe_registry::{MultiRegistryResolver, RegistryError};

    // The probe pkgref. Using a UUID-like suffix keeps the
    // `(kind, name)` extraordinarily unlikely to clash with any
    // real package — every host should respond
    // `UnknownPackage` for it. Underscores are not valid in
    // package names (kebab-case only), so we use `flow:vibe-probe-XXXX`.
    let probe_pkgref = PackageRef::parse("flow:vibe-probe-99zzqq").unwrap();

    let mut rows: Vec<TestReportRegistry> = Vec::with_capacity(manifest.registries.len());

    // Probe each registry independently — open a single-registry
    // resolver per probe so the walk does not chain across
    // registries (we want per-registry diagnostic, not aggregate).
    for reg in &manifest.registries {
        let row_url = reg.url.clone();
        let row_auth_label = reg.auth.as_str();
        let single = std::slice::from_ref(reg);
        let resolver = match MultiRegistryResolver::open(single, &[], &[]) {
            Ok(r) => r,
            Err(e) => {
                rows.push(TestReportRegistry {
                    name: reg.name.clone(),
                    url: row_url,
                    auth: row_auth_label,
                    status: "unknown",
                    note: Some(format!("could not open resolver: {e}")),
                });
                continue;
            }
        };
        let outcome = resolver.resolve(&probe_pkgref);
        let (status, note) = match outcome {
            Ok(_) => (
                "reachable",
                Some("probe pkgref unexpectedly resolved (treating as reachable)".into()),
            ),
            Err(RegistryError::UnknownPackage { .. }) => ("reachable", None),
            // Aggregate-walk shape from a single-registry resolver
            // collapses to PackageNotFoundEverywhere with one
            // attempt. Same meaning as UnknownPackage above.
            Err(RegistryError::PackageNotFoundEverywhere { .. }) => ("reachable", None),
            Err(RegistryError::MissingToken { env_var, .. }) => (
                "missing-token",
                Some(format!(
                    "set `{env_var}` to a personal access token with read scope"
                )),
            ),
            Err(RegistryError::Git(GitError::AuthFailed { .. })) => {
                let hint = match reg.auth {
                    vibe_core::manifest::AuthKind::None => {
                        "host returned 401/403; if this registry is private, change `auth` to \
                         `token-env` / `credential-helper` / `ssh` and provide credentials"
                    }
                    vibe_core::manifest::AuthKind::TokenEnv => {
                        "host rejected the token from the configured env-var; check token scope and freshness"
                    }
                    vibe_core::manifest::AuthKind::CredentialHelper => {
                        "system credential helper did not produce valid credentials"
                    }
                    vibe_core::manifest::AuthKind::Ssh => {
                        "ssh-agent / keys did not authorise the connection"
                    }
                };
                ("auth-required", Some(hint.to_string()))
            }
            Err(RegistryError::Git(GitError::NetworkUnreachable { .. })) => (
                "unreachable",
                Some("DNS / TCP / cert error reaching the host".to_string()),
            ),
            Err(RegistryError::Git(GitError::NotInstalled)) => {
                ("unknown", Some("`git` is not on PATH".to_string()))
            }
            Err(other) => ("unknown", Some(format!("{other}"))),
        };
        rows.push(TestReportRegistry {
            name: reg.name.clone(),
            url: row_url,
            auth: row_auth_label,
            status,
            note,
        });
    }

    if ctx.is_json() {
        ctx.emit_json(&TestReport {
            ok: true,
            command: "registry:test",
            registries: rows,
        })?;
        return Ok(());
    }

    // Text output: aligned table.
    let name_w = rows.iter().map(|r| r.name.len()).max().unwrap_or(0);
    let url_w = rows.iter().map(|r| r.url.len()).max().unwrap_or(0);
    let status_w = rows.iter().map(|r| r.status.len()).max().unwrap_or(0);
    if !ctx.is_quiet() {
        ctx.heading("Registry test");
        for r in &rows {
            let note = r
                .note
                .as_deref()
                .map(|n| format!(" — {n}"))
                .unwrap_or_default();
            println!(
                "  {:<name_w$}  {:<url_w$}  → {:<status_w$}  (auth={}){note}",
                r.name,
                r.url,
                r.status,
                r.auth,
                name_w = name_w,
                url_w = url_w,
                status_w = status_w,
            );
        }
    }
    let n_reachable = rows.iter().filter(|r| r.status == "reachable").count();
    ctx.summary(&format!(
        "vibe registry test: {n_reachable}/{} reachable",
        rows.len()
    ));
    Ok(())
}

// ---------------------------------------------------------------------------
// vibe registry redirect / redirect-sync (PROP-002 §2.4.2)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct RedirectReport {
    ok: bool,
    command: &'static str,
    registry: String,
    pkgref: String,
    stub_url: String,
    target_url: String,
    ref_policy: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pinned_ref: Option<String>,
    target_auth: &'static str,
    created_repo: bool,
    dry_run: bool,
    /// `Some` when `--sync` is passed and the sync leg ran. `None` when
    /// the operator did not request sync, when the policy is `pinned`
    /// (sync is meaningless), or when this is a dry-run.
    #[serde(skip_serializing_if = "Option::is_none")]
    sync: Option<RedirectSyncReport>,
}

#[derive(Debug, Serialize)]
struct RedirectSyncReport {
    ok: bool,
    command: &'static str,
    registry: String,
    pkgref: String,
    stub_url: String,
    target_url: String,
    /// Tags pushed into the stub on this run. Empty on a no-op sync
    /// (target and stub already agree).
    pushed_tags: Vec<String>,
    /// Tags already present in the stub before this run (informational —
    /// helps a CI run that aggregates sync output across many stubs).
    already_present: Vec<String>,
    dry_run: bool,
}

#[derive(Debug, Serialize)]
struct RedirectUpdateReport {
    ok: bool,
    command: &'static str,
    registry: String,
    pkgref: String,
    stub_url: String,
    /// Target URL on the new marker. Mirrors the post-update marker
    /// content; equals `target_url` of the existing marker when `--to`
    /// was not passed.
    target_url: String,
    ref_policy: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pinned_ref: Option<String>,
    target_auth: &'static str,
    /// Per-field before/after diff for the marker rewrite. Empty only on
    /// dry-runs of trivial edits — in real applies the handler bails
    /// before push when this would be empty.
    changes: Vec<RedirectChangeEntry>,
    /// `true` when the change set carries fields that require
    /// `--trust-redirect` per PROP-002 §2.4.2 (target_url, ref_policy,
    /// or pinned_ref under pinned policy).
    trust_required: bool,
    dry_run: bool,
    /// `Some` when `--resync` was passed and the sync leg ran. `None`
    /// when no resync was requested, when the new policy is `pinned`,
    /// or on dry-run.
    #[serde(skip_serializing_if = "Option::is_none")]
    sync: Option<RedirectSyncReport>,
}

/// Per-field before / after entry produced by
/// [`compute_updated_redirect_section`]. Field values are rendered as
/// canonical strings — `None` for absent optional fields, the kebab-case
/// `RefPolicy` / `AuthKind` discriminant otherwise. This keeps the JSON
/// envelope ergonomic for log aggregators without needing to know the
/// internal enum shapes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RedirectChangeEntry {
    field: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<String>,
}

fn parse_target_auth(s: Option<&str>) -> Result<vibe_core::manifest::AuthKind> {
    match s {
        None | Some("none") => Ok(vibe_core::manifest::AuthKind::None),
        Some("token-env") => Ok(vibe_core::manifest::AuthKind::TokenEnv),
        Some("credential-helper") => Ok(vibe_core::manifest::AuthKind::CredentialHelper),
        Some("ssh") => Ok(vibe_core::manifest::AuthKind::Ssh),
        Some(other) => bail!(
            "unknown --target-auth `{other}` — must be `none`, `token-env`, `credential-helper`, or `ssh`"
        ),
    }
}

/// Resolve the registry to act on for a redirect / redirect-sync command.
fn resolve_target_registry<'m>(
    manifest: &'m Manifest,
    requested: Option<&str>,
    manifest_path: &Path,
) -> Result<&'m RegistrySection> {
    if manifest.registries.is_empty() {
        bail!(
            "no `[[registry]]` entries in `{}`. `vibe registry redirect` needs a registry org \
             where the stub will be created.",
            manifest_path.display()
        );
    }
    match requested {
        Some(name) => manifest.registry_by_name(name).ok_or_else(|| {
            anyhow!(
                "no `[[registry]]` named `{name}` in `{}`",
                manifest_path.display()
            )
        }),
        None => manifest
            .primary_registry()
            .ok_or_else(|| anyhow!("no `[[registry]]` configured")),
    }
}

fn run_redirect(ctx: &output::Context, args: RegistryRedirectArgs) -> Result<()> {
    use vibe_core::PackageRef;
    use vibe_core::manifest::{AuthKind, RedirectFile, RefPolicy};

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

    let pkgref = PackageRef::parse(&args.pkgref)
        .with_context(|| format!("parsing pkgref `{}`", args.pkgref))?;
    let group = require_group(&pkgref)?;

    let registry_section =
        resolve_target_registry(&manifest, args.registry.as_deref(), &manifest_path)?;

    // Validate URL shape early — before any side-effecting work — so the
    // operator gets a fast actionable error instead of a network failure.
    let host = extract_host_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;
    let org_segment = extract_org_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;

    // Validate target URL shape — must at least have a scheme git accepts.
    if args.to.trim().is_empty() {
        bail!("--to must be a non-empty git URL");
    }

    // Validate ref-policy + pinned-ref combination.
    let (ref_policy, pinned_ref) = match args.ref_policy.as_str() {
        "pass-through-tag" => {
            if args.pinned_ref.is_some() {
                bail!(
                    "--pinned-ref is only meaningful with --ref-policy pinned; drop it or \
                     change to --ref-policy pinned"
                );
            }
            (RefPolicy::PassThroughTag, None)
        }
        "pinned" => {
            let r = args.pinned_ref.as_deref().ok_or_else(|| {
                anyhow!("--ref-policy pinned requires --pinned-ref <tag/branch/rev>")
            })?;
            (RefPolicy::Pinned, Some(r.to_string()))
        }
        other => bail!(
            "unknown --ref-policy `{other}` — must be `pass-through-tag` (default) or `pinned`"
        ),
    };

    let target_auth = parse_target_auth(args.target_auth.as_deref())?;
    if matches!(target_auth, AuthKind::TokenEnv) && args.target_token_env.is_none() {
        tracing::debug!(
            target: "vibe_cli::registry::redirect",
            "target-auth=token-env without explicit --target-token-env; will derive from host on resolve"
        );
    }
    if args.target_token_env.is_some() && !matches!(target_auth, AuthKind::TokenEnv) {
        bail!(
            "--target-token-env is only meaningful with --target-auth token-env; got --target-auth {:?}",
            target_auth.as_str()
        );
    }

    // Compute the stub repo name from naming convention.
    let stub_repo_name = registry_section
        .naming
        .repo_name(pkgref.kind, group, &pkgref.name)
        .with_context(|| format!("deriving the stub repo name for `{group}/{}`", pkgref.name))?;
    // Stub URL surfaced in JSON / human output. Construction mirrors what
    // [`MultiRegistryResolver`] does at resolve time.
    let stub_url = format!(
        "{}/{}",
        registry_section.url.trim_end_matches('/'),
        stub_repo_name
    );

    // Build the stub source dir — `vibe-redirect.toml` + README.
    let stub_section = vibe_core::manifest::RedirectSection {
        target_url: args.to.clone(),
        ref_policy,
        pinned_ref: pinned_ref.clone(),
        auth: target_auth,
        token_env: args.target_token_env.clone(),
        description: args.description.clone(),
    };
    let stub_file = RedirectFile {
        redirect: stub_section,
    };

    let staging = tempfile::tempdir().context("creating stub staging dir")?;
    let stub_marker_path = staging.path().join(RedirectFile::FILENAME);
    stub_file
        .write(&stub_marker_path)
        .with_context(|| format!("writing `{}`", stub_marker_path.display()))?;

    // README — operator-friendly summary so a human visiting the stub
    // repo on the host's web UI understands what they're looking at
    // without needing to read the marker file.
    let readme = build_redirect_readme(
        &pkgref.qualified_name(),
        &args.to,
        args.description.as_deref(),
    );
    std::fs::write(staging.path().join("README.md"), readme).with_context(|| {
        format!(
            "writing README into stub staging dir `{}`",
            staging.path().display()
        )
    })?;

    ctx.heading(&format!(
        "Creating redirect stub: {} → {}{}",
        pkgref.qualified_name(),
        args.to,
        if args.dry_run { " [dry-run]" } else { "" }
    ));

    if args.dry_run {
        ctx.step(&format!(
            "Would create repository `{stub_repo_name}` on `{host}` (org `{org_segment}`)"
        ));
        ctx.step(&format!(
            "Would write `{}` and README; would push to `{stub_url}`",
            RedirectFile::FILENAME
        ));
        let report = RedirectReport {
            ok: true,
            command: "registry:redirect",
            registry: registry_section.name.clone(),
            pkgref: pkgref.qualified_name(),
            stub_url: stub_url.clone(),
            target_url: args.to.clone(),
            ref_policy: match ref_policy {
                RefPolicy::PassThroughTag => "pass-through-tag",
                RefPolicy::Pinned => "pinned",
            },
            pinned_ref,
            target_auth: target_auth.as_str(),
            created_repo: false,
            dry_run: true,
            sync: None,
        };
        if ctx.is_json() {
            ctx.emit_json(&report)?;
        } else {
            ctx.summary(
                "\nvibe registry redirect [dry-run]: re-run without `--dry-run` to create the stub.",
            );
        }
        return Ok(());
    }

    // GitVerse publish path is a stub today (PROP-002 §2.10 — GitVerse
    // does not expose org-scoped repo creation). Refuse early with the
    // same shape as `vibe registry publish`.
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "gitverse.ru" || host_lower.ends_with(".gitverse.ru") {
        bail!(
            "GitVerse publish is not implemented yet — the GitVerse public API does not expose \
             org-scoped repository creation. Use a GitHub `[[registry]]` for redirect stubs, or \
             create the stub repo by hand and `vibe registry publish --repo-url` content into it."
        );
    }

    let token = load_token_for_host(&host).context("loading publish token")?;
    ctx.step(&format!(
        "Loaded publish token from {} (value redacted)",
        match token.source() {
            vibe_publish::TokenSource::Explicit => "explicit argument".to_string(),
            vibe_publish::TokenSource::EnvVar(name) => format!("$ {name}"),
            vibe_publish::TokenSource::File(p) => p.display().to_string(),
        }
    ));
    let creator = creator_for_url(&registry_section.url, org_segment.clone(), token)
        .map_err(|e| anyhow!("{e}"))?;

    // Refuse to clobber an existing stub — operators who want to update
    // a stub's marker file should hand-edit it (the M1.16 v0 surface).
    let exists = creator
        .repo_exists(&org_segment, &stub_repo_name)
        .map_err(|e| anyhow!("{e}"))?;
    if exists {
        bail!(
            "stub repository `{stub_repo_name}` already exists in `{org_segment}` on `{host}`. \
             Editing an existing redirect stub is a manual procedure for v0 — clone it, edit \
             `{}`, push back. `vibe registry redirect` only handles fresh-stub creation.",
            RedirectFile::FILENAME
        );
    }

    let opts = vibe_publish::CreateOpts {
        description: Some(format!(
            "vibevm registry stub for {} (delegated to {})",
            pkgref.qualified_name(),
            args.to
        )),
        default_branch: Some("main".to_string()),
        homepage: None,
    };
    let _info = creator
        .create_repo(&org_segment, &stub_repo_name, &opts)
        .map_err(|e| anyhow!("{e}"))?;
    ctx.step(&format!(
        "Created repository `{stub_repo_name}` on `{host}`"
    ));

    // Push the stub contents to `main`. Token embedded only at the
    // moment of git invocation; never in stdout / stderr / logs.
    let push_url = creator.push_url(&org_segment, &stub_repo_name);
    let commit_msg = format!("stub: delegate {} to {}", pkgref.qualified_name(), args.to);
    vibe_publish::git_publish::push_initial(staging.path(), &push_url, &commit_msg)
        .map_err(|e| anyhow!("{e}"))?;
    ctx.step(&format!(
        "Pushed stub `{}` to `main`",
        RedirectFile::FILENAME
    ));

    // Optional: sync target tags into the stub immediately.
    let sync_report = if args.sync && matches!(ref_policy, RefPolicy::PassThroughTag) {
        ctx.step("Synchronising target tags into the freshly-created stub");
        Some(do_redirect_sync(
            ctx,
            registry_section,
            &pkgref.qualified_name(),
            &stub_url,
            &args.to,
            &push_url,
            args.dry_run,
        )?)
    } else if args.sync && matches!(ref_policy, RefPolicy::Pinned) {
        ctx.step(
            "Skipping --sync: pinned-policy stubs do not pass through target tags (every \
             consumer resolves to --pinned-ref regardless of stub tag)",
        );
        None
    } else {
        None
    };

    let report = RedirectReport {
        ok: true,
        command: "registry:redirect",
        registry: registry_section.name.clone(),
        pkgref: pkgref.qualified_name(),
        stub_url: stub_url.clone(),
        target_url: args.to.clone(),
        ref_policy: match ref_policy {
            RefPolicy::PassThroughTag => "pass-through-tag",
            RefPolicy::Pinned => "pinned",
        },
        pinned_ref,
        target_auth: target_auth.as_str(),
        created_repo: true,
        dry_run: false,
        sync: sync_report,
    };

    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    ctx.summary(&format!(
        "\nvibe registry redirect: stub `{stub_url}` delegates `{}` → `{}`. Consumers \
         resolving `{}` will be redirected to the target transparently. Tag the stub with \
         `git tag vX.Y.Z && git push origin vX.Y.Z` to surface a target version, or run \
         `vibe registry redirect-sync {}` to mirror the target's tag list.",
        pkgref.qualified_name(),
        args.to,
        pkgref.qualified_name(),
        pkgref.qualified_name(),
    ));
    Ok(())
}

fn build_redirect_readme(pkgref: &str, target_url: &str, description: Option<&str>) -> String {
    let desc_block = description
        .map(|d| format!("\n> {d}\n"))
        .unwrap_or_default();
    format!(
        "# {pkgref} — registry stub\n\n\
         This repository is a vibevm registry stub that redirects consumers to\n\
         the canonical home of `{pkgref}`:\n\n\
         > {target_url}\n\
         {desc_block}\n\
         Operators reach this package via `vibe install {pkgref}` through the\n\
         org's `[[registry]]` configuration; vibevm follows the\n\
         `vibe-redirect.toml` marker transparently. The actual package content\n\
         (`vibe.toml`, spec files, etc.) lives at the target URL above.\n\n\
         See [PROP-002 §2.4.2](https://example.invalid/spec) for the redirect\n\
         protocol and [`docs/registry-redirect.md`](https://example.invalid/docs)\n\
         for the operator reference.\n"
    )
}

fn run_redirect_sync(ctx: &output::Context, args: RegistryRedirectSyncArgs) -> Result<()> {
    use vibe_core::PackageRef;

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

    let pkgref = PackageRef::parse(&args.pkgref)
        .with_context(|| format!("parsing pkgref `{}`", args.pkgref))?;
    let group = require_group(&pkgref)?;
    let registry_section =
        resolve_target_registry(&manifest, args.registry.as_deref(), &manifest_path)?;
    let host = extract_host_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;
    let org_segment = extract_org_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;
    let stub_repo_name = registry_section
        .naming
        .repo_name(pkgref.kind, group, &pkgref.name)
        .with_context(|| format!("deriving the stub repo name for `{group}/{}`", pkgref.name))?;
    let stub_url = format!(
        "{}/{}",
        registry_section.url.trim_end_matches('/'),
        stub_repo_name
    );

    ctx.heading(&format!(
        "Syncing target tags into stub: {}{}",
        pkgref.qualified_name(),
        if args.dry_run { " [dry-run]" } else { "" }
    ));

    // Load token + build push URL using the same path as `vibe registry
    // redirect`. Read access does not strictly require a token for
    // public registries, but using the credentialed URL when available
    // (e.g. when the registry is `auth = "token-env"`) lets us read
    // private stubs symmetrically.
    let token = load_token_for_host(&host).context("loading publish token")?;
    let creator = creator_for_url(&registry_section.url, org_segment.clone(), token)
        .map_err(|e| anyhow!("{e}"))?;
    let push_url = creator.push_url(&org_segment, &stub_repo_name);

    // Probe stub existence so we fail fast with a clear message.
    let exists = creator
        .repo_exists(&org_segment, &stub_repo_name)
        .map_err(|e| anyhow!("{e}"))?;
    if !exists {
        bail!(
            "stub repository `{stub_repo_name}` does not exist in `{org_segment}` on `{host}`. \
             Run `vibe registry redirect {} --to <target-url>` first to create it.",
            pkgref.qualified_name()
        );
    }

    let report = do_redirect_sync(
        ctx,
        registry_section,
        &pkgref.qualified_name(),
        &stub_url,
        "<read-from-stub>",
        &push_url,
        args.dry_run,
    )?;

    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    if report.pushed_tags.is_empty() {
        ctx.summary(&format!(
            "\nvibe registry redirect-sync: `{}` is in sync with target. {} tag{} already \
             present on stub.",
            pkgref.qualified_name(),
            report.already_present.len(),
            if report.already_present.len() == 1 {
                ""
            } else {
                "s"
            }
        ));
    } else {
        ctx.summary(&format!(
            "\nvibe registry redirect-sync: pushed {} tag{} into stub `{}`. {} tag{} were \
             already present.",
            report.pushed_tags.len(),
            if report.pushed_tags.len() == 1 {
                ""
            } else {
                "s"
            },
            pkgref.qualified_name(),
            report.already_present.len(),
            if report.already_present.len() == 1 {
                ""
            } else {
                "s"
            }
        ));
    }
    Ok(())
}

fn run_redirect_update(ctx: &output::Context, args: RegistryRedirectUpdateArgs) -> Result<()> {
    use vibe_core::PackageRef;
    use vibe_core::manifest::{RedirectFile, RefPolicy};
    use vibe_publish::git_publish;

    // Validate args-level invariants FIRST, before touching the
    // filesystem or network. Operators expect "bad flag combo" to fail
    // with a clear message even when no project is in scope.
    if args.description.is_some() && args.clear_description {
        bail!("--description and --clear-description are mutually exclusive; pass exactly one");
    }

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

    let pkgref = PackageRef::parse(&args.pkgref)
        .with_context(|| format!("parsing pkgref `{}`", args.pkgref))?;
    let group = require_group(&pkgref)?;
    let registry_section =
        resolve_target_registry(&manifest, args.registry.as_deref(), &manifest_path)?;
    let host = extract_host_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;
    let org_segment = extract_org_segment(&registry_section.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", registry_section.url))?;
    let stub_repo_name = registry_section
        .naming
        .repo_name(pkgref.kind, group, &pkgref.name)
        .with_context(|| format!("deriving the stub repo name for `{group}/{}`", pkgref.name))?;
    let stub_url = format!(
        "{}/{}",
        registry_section.url.trim_end_matches('/'),
        stub_repo_name
    );

    ctx.heading(&format!(
        "Updating redirect stub: {}{}",
        pkgref.qualified_name(),
        if args.dry_run { " [dry-run]" } else { "" }
    ));

    // GitVerse host refusal — symmetric to redirect-create. v0 only
    // creates / mutates GitHub stubs via the `RepoCreator` machinery.
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "gitverse.ru" || host_lower.ends_with(".gitverse.ru") {
        bail!(
            "GitVerse publish is not implemented yet — the GitVerse public API does not expose \
             org-scoped repository creation. For redirect-update against a GitVerse stub, \
             clone the stub by hand, edit `{}`, push back.",
            RedirectFile::FILENAME
        );
    }

    let token = load_token_for_host(&host).context("loading publish token")?;
    ctx.step(&format!(
        "Loaded publish token from {} (value redacted)",
        match token.source() {
            vibe_publish::TokenSource::Explicit => "explicit argument".to_string(),
            vibe_publish::TokenSource::EnvVar(name) => format!("$ {name}"),
            vibe_publish::TokenSource::File(p) => p.display().to_string(),
        }
    ));
    let creator = creator_for_url(&registry_section.url, org_segment.clone(), token)
        .map_err(|e| anyhow!("{e}"))?;
    let push_url = creator.push_url(&org_segment, &stub_repo_name);

    let exists = creator
        .repo_exists(&org_segment, &stub_repo_name)
        .map_err(|e| anyhow!("{e}"))?;
    if !exists {
        bail!(
            "stub repository `{stub_repo_name}` does not exist in `{org_segment}` on `{host}`. \
             Run `vibe registry redirect {} --to <target-url>` first to create it.",
            pkgref.qualified_name()
        );
    }

    // Shallow-clone the stub so we have a working tree to write the
    // updated marker into and commit_and_push back onto `main`.
    let stub_clone = git_publish::shallow_clone(&push_url).map_err(|e| anyhow!("{e}"))?;
    let marker_path = stub_clone.path().join(RedirectFile::FILENAME);
    if !marker_path.exists() {
        bail!(
            "stub at `{stub_url}` does not carry `{}` at HEAD — is this actually a redirect \
             stub? `vibe registry redirect-update` only operates on stub repos.",
            RedirectFile::FILENAME
        );
    }
    let existing = RedirectFile::read(&marker_path).with_context(|| {
        format!(
            "parsing `{}` from stub `{stub_url}`",
            RedirectFile::FILENAME
        )
    })?;

    let (new_section, changes) = compute_updated_redirect_section(&existing.redirect, &args)?;

    if changes.is_empty() {
        bail!(
            "no changes requested — the computed `{}` is identical to the stub's current marker. \
             Pass at least one of --to / --ref-policy / --pinned-ref / --target-auth / \
             --target-token-env / --description / --clear-description.",
            RedirectFile::FILENAME
        );
    }

    let trust_required = changes.iter().any(|c| c.requires_trust());
    if trust_required && !args.trust_redirect {
        let fields: Vec<&str> = changes
            .iter()
            .filter(|c| c.requires_trust())
            .map(|c| c.field)
            .collect();
        bail!(
            "this update changes `{}` which alters resolution outcomes for every consumer of \
             `{}`. Pass `--trust-redirect` to confirm a deliberate switch (PROP-002 §2.4.2 trust \
             model — never silent, always operator-initiated).",
            fields.join("`, `"),
            pkgref.qualified_name()
        );
    }

    // Build the report shape (used for --json and the dry-run path
    // before we exit; for a real apply it's emitted at the end).
    let new_target_url = new_section.target_url.clone();
    let new_ref_policy_str: &'static str = match new_section.ref_policy {
        RefPolicy::PassThroughTag => "pass-through-tag",
        RefPolicy::Pinned => "pinned",
    };
    let new_target_auth_str: &'static str = new_section.auth.as_str();
    let new_pinned_ref = new_section.pinned_ref.clone();

    for c in &changes {
        ctx.step(&format!(
            "{}: {} → {}",
            c.field,
            c.before.as_deref().unwrap_or("<unset>"),
            c.after.as_deref().unwrap_or("<unset>"),
        ));
    }

    if args.dry_run {
        let report = RedirectUpdateReport {
            ok: true,
            command: "registry:redirect-update",
            registry: registry_section.name.clone(),
            pkgref: pkgref.qualified_name(),
            stub_url: stub_url.clone(),
            target_url: new_target_url,
            ref_policy: new_ref_policy_str,
            pinned_ref: new_pinned_ref,
            target_auth: new_target_auth_str,
            changes,
            trust_required,
            dry_run: true,
            sync: None,
        };
        if ctx.is_json() {
            ctx.emit_json(&report)?;
        } else {
            ctx.summary(
                "\nvibe registry redirect-update [dry-run]: re-run without `--dry-run` to push \
                 the rewritten marker.",
            );
        }
        return Ok(());
    }

    // Write new marker + regenerate README in the existing clone. Both
    // files are full rewrites; git status -s after `git add -A` tells
    // us whether anything actually changed on disk (commit_and_push
    // bails if not).
    let new_file = RedirectFile {
        redirect: new_section,
    };
    new_file
        .write(&marker_path)
        .with_context(|| format!("writing `{}`", marker_path.display()))?;
    let readme = build_redirect_readme(
        &pkgref.qualified_name(),
        &new_file.redirect.target_url,
        new_file.redirect.description.as_deref(),
    );
    std::fs::write(stub_clone.path().join("README.md"), readme).with_context(|| {
        format!(
            "writing README into stub clone `{}`",
            stub_clone.path().display()
        )
    })?;

    let commit_msg = build_redirect_update_commit_msg(&pkgref.qualified_name(), &changes);
    git_publish::commit_and_push(stub_clone.path(), &push_url, &commit_msg)
        .map_err(|e| anyhow!("{e}"))?;
    ctx.step(&format!(
        "Pushed updated `{}` to `main`",
        RedirectFile::FILENAME
    ));

    let sync_report =
        if args.resync && matches!(new_file.redirect.ref_policy, RefPolicy::PassThroughTag) {
            ctx.step("Re-syncing target tags into the updated stub");
            Some(do_redirect_sync(
                ctx,
                registry_section,
                &pkgref.qualified_name(),
                &stub_url,
                &new_file.redirect.target_url,
                &push_url,
                false,
            )?)
        } else if args.resync && matches!(new_file.redirect.ref_policy, RefPolicy::Pinned) {
            ctx.step("Skipping --resync: pinned-policy stubs do not pass through target tags");
            None
        } else {
            None
        };

    let report = RedirectUpdateReport {
        ok: true,
        command: "registry:redirect-update",
        registry: registry_section.name.clone(),
        pkgref: pkgref.qualified_name(),
        stub_url: stub_url.clone(),
        target_url: new_file.redirect.target_url.clone(),
        ref_policy: new_ref_policy_str,
        pinned_ref: new_file.redirect.pinned_ref.clone(),
        target_auth: new_target_auth_str,
        changes,
        trust_required,
        dry_run: false,
        sync: sync_report,
    };
    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    ctx.summary(&format!(
        "\nvibe registry redirect-update: rewrote `{}` on stub `{stub_url}`. Consumers \
         resolving `{}` now see the new marker.",
        RedirectFile::FILENAME,
        pkgref.qualified_name(),
    ));
    Ok(())
}

/// Merge a [`RegistryRedirectUpdateArgs`] flag set into an existing
/// `[redirect]` section. Returns the new section and a list of changed
/// fields. Validates flag combinations (mutual exclusion already
/// checked by the caller) and any cross-field invariants (pinned policy
/// requires pinned_ref; token_env only meaningful with token-env auth).
///
/// Switching `auth` away from `token-env` clears `token_env`
/// automatically — keeping it would be a parse error on the next read.
/// Switching `ref_policy` to `pass-through-tag` clears `pinned_ref` for
/// the same reason.
fn compute_updated_redirect_section(
    current: &vibe_core::manifest::RedirectSection,
    args: &RegistryRedirectUpdateArgs,
) -> Result<(
    vibe_core::manifest::RedirectSection,
    Vec<RedirectChangeEntry>,
)> {
    use vibe_core::manifest::{AuthKind, RedirectSection, RefPolicy};

    // target_url
    let new_target_url = match &args.to {
        Some(t) => {
            if t.trim().is_empty() {
                bail!("--to must be a non-empty git URL");
            }
            t.clone()
        }
        None => current.target_url.clone(),
    };

    // ref_policy
    let new_ref_policy = match args.ref_policy.as_deref() {
        None => current.ref_policy,
        Some("pass-through-tag") => RefPolicy::PassThroughTag,
        Some("pinned") => RefPolicy::Pinned,
        Some(other) => {
            bail!("unknown --ref-policy `{other}` — must be `pass-through-tag` or `pinned`")
        }
    };

    // pinned_ref — depends on new_ref_policy
    let new_pinned_ref = match new_ref_policy {
        RefPolicy::PassThroughTag => {
            if args.pinned_ref.is_some() {
                bail!(
                    "--pinned-ref is only meaningful with `--ref-policy pinned`; drop it or pass \
                     `--ref-policy pinned`"
                );
            }
            // Switching to (or staying at) pass-through clears any
            // previously-set pinned_ref — the marker would otherwise be
            // rejected at parse.
            None
        }
        RefPolicy::Pinned => {
            // Prefer the explicit flag; fall back to the existing
            // pinned_ref iff the policy already was pinned. Switching
            // from pass-through to pinned without --pinned-ref is a
            // hard error since there is nothing to preserve.
            match args.pinned_ref.as_deref() {
                Some(r) => Some(r.to_string()),
                None => match current.ref_policy {
                    RefPolicy::Pinned => current.pinned_ref.clone(),
                    RefPolicy::PassThroughTag => bail!(
                        "switching to `--ref-policy pinned` requires `--pinned-ref \
                         <tag/branch/rev>` (no existing pinned ref to preserve)"
                    ),
                },
            }
        }
    };

    // auth
    let new_auth = match args.target_auth.as_deref() {
        None => current.auth,
        Some(s) => parse_target_auth(Some(s))?,
    };

    // token_env — only meaningful with TokenEnv
    let new_token_env = match new_auth {
        AuthKind::TokenEnv => match &args.target_token_env {
            Some(name) if name.trim().is_empty() => {
                bail!("--target-token-env must be a non-empty env-var name")
            }
            Some(name) => Some(name.clone()),
            None => current.token_env.clone(),
        },
        _ => {
            if args.target_token_env.is_some() {
                bail!("--target-token-env is only meaningful with --target-auth token-env");
            }
            None
        }
    };

    // description
    let new_description = if args.clear_description {
        None
    } else if let Some(d) = &args.description {
        Some(d.clone())
    } else {
        current.description.clone()
    };

    let new_section = RedirectSection {
        target_url: new_target_url,
        ref_policy: new_ref_policy,
        pinned_ref: new_pinned_ref,
        auth: new_auth,
        token_env: new_token_env,
        description: new_description,
    };

    let changes = diff_redirect_sections(current, &new_section);
    Ok((new_section, changes))
}

fn diff_redirect_sections(
    before: &vibe_core::manifest::RedirectSection,
    after: &vibe_core::manifest::RedirectSection,
) -> Vec<RedirectChangeEntry> {
    use vibe_core::manifest::RefPolicy;

    let mut out: Vec<RedirectChangeEntry> = Vec::new();
    if before.target_url != after.target_url {
        out.push(RedirectChangeEntry {
            field: "target_url",
            before: Some(before.target_url.clone()),
            after: Some(after.target_url.clone()),
        });
    }
    if before.ref_policy != after.ref_policy {
        let pol = |p: RefPolicy| -> &'static str {
            match p {
                RefPolicy::PassThroughTag => "pass-through-tag",
                RefPolicy::Pinned => "pinned",
            }
        };
        out.push(RedirectChangeEntry {
            field: "ref_policy",
            before: Some(pol(before.ref_policy).to_string()),
            after: Some(pol(after.ref_policy).to_string()),
        });
    }
    if before.pinned_ref != after.pinned_ref {
        out.push(RedirectChangeEntry {
            field: "pinned_ref",
            before: before.pinned_ref.clone(),
            after: after.pinned_ref.clone(),
        });
    }
    if before.auth != after.auth {
        out.push(RedirectChangeEntry {
            field: "auth",
            before: Some(before.auth.as_str().to_string()),
            after: Some(after.auth.as_str().to_string()),
        });
    }
    if before.token_env != after.token_env {
        out.push(RedirectChangeEntry {
            field: "token_env",
            before: before.token_env.clone(),
            after: after.token_env.clone(),
        });
    }
    if before.description != after.description {
        out.push(RedirectChangeEntry {
            field: "description",
            before: before.description.clone(),
            after: after.description.clone(),
        });
    }
    out
}

impl RedirectChangeEntry {
    /// Per PROP-002 §2.4.2 trust model — these three fields change what
    /// content a consumer ends up materialising. The other fields
    /// (auth, token_env, description) are operator-side metadata and do
    /// not require `--trust-redirect`.
    fn requires_trust(&self) -> bool {
        matches!(self.field, "target_url" | "ref_policy" | "pinned_ref")
    }
}

fn build_redirect_update_commit_msg(pkgref: &str, changes: &[RedirectChangeEntry]) -> String {
    if let Some(c) = changes.iter().find(|c| c.field == "target_url")
        && let Some(after) = &c.after
    {
        return format!("stub: retarget {pkgref} to {after}");
    }
    let fields: Vec<&str> = changes.iter().map(|c| c.field).collect();
    format!("stub: update marker for {pkgref} ({})", fields.join(", "))
}

/// Inner sync logic — shared by `vibe registry redirect --sync` and
/// `vibe registry redirect-sync`. Reads the stub's `vibe-redirect.toml`,
/// enumerates target tags, pushes the missing ones into the stub.
fn do_redirect_sync(
    ctx: &output::Context,
    registry_section: &RegistrySection,
    pkgref_qualified: &str,
    stub_url: &str,
    target_url_hint: &str,
    push_url: &str,
    dry_run: bool,
) -> Result<RedirectSyncReport> {
    use vibe_core::manifest::{RedirectFile, RefPolicy};
    use vibe_publish::git_publish;

    // Step 1: shallow-clone the stub so we can read the marker file
    // and have a working tree to anchor new tags onto.
    let stub_clone = git_publish::shallow_clone(push_url).map_err(|e| anyhow!("{e}"))?;
    let marker_path = stub_clone.path().join(RedirectFile::FILENAME);
    if !marker_path.exists() {
        bail!(
            "stub at `{stub_url}` does not carry `{}` at HEAD — is this actually a redirect \
             stub? `vibe registry redirect-sync` only operates on stub repos.",
            RedirectFile::FILENAME
        );
    }
    let stub_file = RedirectFile::read(&marker_path).with_context(|| {
        format!(
            "parsing `{}` from stub `{stub_url}`",
            RedirectFile::FILENAME
        )
    })?;

    // Pinned policy — stub tags don't pass through, so syncing is a
    // semantic mistake.
    if matches!(stub_file.redirect.ref_policy, RefPolicy::Pinned) {
        bail!(
            "stub `{stub_url}` uses `ref_policy = \"pinned\"` — every consumer resolves to \
             `pinned_ref = {:?}` regardless of stub tag, so there is nothing to sync. Edit \
             `{}` to change the policy if you want pass-through behaviour.",
            stub_file.redirect.pinned_ref.as_deref().unwrap_or(""),
            RedirectFile::FILENAME
        );
    }

    let target_url = stub_file.redirect.target_url.clone();
    if target_url_hint != "<read-from-stub>" && target_url_hint != target_url {
        // The CLI surface (`--to`) only matches the stub on `redirect`
        // since `redirect-sync` reads from the stub itself. The hint
        // disagreeing is a sanity check, not a hard error — log it.
        tracing::debug!(
            target: "vibe_cli::registry::redirect_sync",
            "target_url hint `{target_url_hint}` disagrees with stub-stored `{target_url}`; using stub"
        );
    }

    // Step 2: build a target-side fetch URL with credentials if the
    // stub declares `auth = "token-env"`. Public targets need no token.
    let target_fetch_url = build_target_fetch_url(&target_url, &stub_file.redirect)?;

    // Step 3: list tags on both sides.
    let target_tags = git_publish::ls_remote_tags(&target_fetch_url).map_err(|e| anyhow!("{e}"))?;
    // For listing stub tags we use `git ls-remote` directly so we do
    // not depend on the shallow clone having all refs (it does, by
    // virtue of `--single-branch`, but ls-remote is the source of truth).
    let stub_tags = git_publish::ls_remote_tags(push_url).map_err(|e| anyhow!("{e}"))?;

    // Step 4: classify.
    let mut to_push: Vec<String> = Vec::new();
    let mut already: Vec<String> = Vec::new();
    for t in &target_tags {
        if stub_tags.iter().any(|s| s == t) {
            already.push(t.clone());
        } else {
            to_push.push(t.clone());
        }
    }
    to_push.sort();
    already.sort();

    if dry_run {
        for t in &to_push {
            ctx.step(&format!(
                "Would push tag `{t}` (target has it; stub does not)"
            ));
        }
        for t in &already {
            ctx.skipped(&format!("tag `{t}`"), "already present on stub");
        }
        return Ok(RedirectSyncReport {
            ok: true,
            command: "registry:redirect-sync",
            registry: registry_section.name.clone(),
            pkgref: pkgref_qualified.to_string(),
            stub_url: stub_url.to_string(),
            target_url,
            pushed_tags: to_push,
            already_present: already,
            dry_run: true,
        });
    }

    // Step 5: push the missing tags. Each tag is annotated, anchored
    // at the stub's `main` commit. Stubs are flat — tag → marker file
    // — so the commit is identical regardless of which target tag the
    // stub tag fronts.
    for t in &to_push {
        git_publish::push_tag_only(stub_clone.path(), push_url, t).map_err(|e| anyhow!("{e}"))?;
        ctx.step(&format!("Pushed tag `{t}` into stub"));
    }
    for t in &already {
        ctx.skipped(&format!("tag `{t}`"), "already present on stub");
    }

    Ok(RedirectSyncReport {
        ok: true,
        command: "registry:redirect-sync",
        registry: registry_section.name.clone(),
        pkgref: pkgref_qualified.to_string(),
        stub_url: stub_url.to_string(),
        target_url,
        pushed_tags: to_push,
        already_present: already,
        dry_run: false,
    })
}

/// Build a fetch URL for the target side of a redirect, applying
/// `[redirect].auth` if it asks for token-based auth. For `auth = "none"`
/// this returns the URL verbatim; for `auth = "token-env"` it injects
/// the resolved token using the same shape M1.14 plumbing applies
/// (`https://x-access-token:<TOKEN>@host/...`). Other auth regimes
/// (`credential-helper`, `ssh`) trust the local git's auth path.
fn build_target_fetch_url(
    target_url: &str,
    redirect: &vibe_core::manifest::RedirectSection,
) -> Result<String> {
    use vibe_core::manifest::AuthKind;
    match redirect.auth {
        AuthKind::None | AuthKind::CredentialHelper | AuthKind::Ssh => Ok(target_url.to_string()),
        AuthKind::TokenEnv => {
            let env_name = redirect
                .token_env
                .clone()
                .or_else(|| derive_target_token_env(target_url))
                .ok_or_else(|| {
                    anyhow!(
                        "target URL `{target_url}` declares auth = \"token-env\" but no \
                         `token_env` is set and the host cannot be derived for a default \
                         env-var name"
                    )
                })?;
            let value = std::env::var(&env_name).map_err(|_| {
                anyhow!(
                    "target URL `{target_url}` declares auth = \"token-env\" with env-var \
                     `{env_name}` but the variable is unset or empty in this shell"
                )
            })?;
            Ok(inject_token_into_url(target_url, &value))
        }
    }
}

fn derive_target_token_env(target_url: &str) -> Option<String> {
    let host = extract_host_segment(target_url).ok()?;
    let upper = host.to_ascii_uppercase().replace(['.', '-'], "_");
    Some(format!("VIBEVM_TARGET_TOKEN_{upper}"))
}

fn inject_token_into_url(url: &str, token: &str) -> String {
    if !url.starts_with("https://") {
        // SSH-form / file:// — token has nowhere to land; pass through.
        return url.to_string();
    }
    let rest = &url[8..]; // past "https://"
    if rest.contains('@') {
        // Already credentialed — caller's choice; do not double-inject.
        return url.to_string();
    }
    format!("https://x-access-token:{token}@{rest}")
}

#[cfg(test)]
mod tests {
    use super::{
        adapter_for_host, bare_clone_from_clone, build_redirect_readme,
        build_redirect_update_commit_msg, build_target_fetch_url, compute_updated_redirect_section,
        derive_target_token_env, diff_redirect_sections, file_url_for_dir, inject_token_into_url,
        parse_naming, parse_target_auth,
    };
    use crate::cli::RegistryRedirectUpdateArgs;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use vibe_core::manifest::{AuthKind, NamingConvention, RedirectSection, RefPolicy};

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
        fs::write(
            src_git.join("config"),
            "[core]\n\trepositoryformatversion = 0\n",
        )
        .unwrap();
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

    // -----------------------------------------------------------------
    // redirect / redirect-sync helpers (PROP-002 §2.4.2)
    // -----------------------------------------------------------------

    #[test]
    fn parse_target_auth_canonical_spellings() {
        assert!(matches!(parse_target_auth(None).unwrap(), AuthKind::None));
        assert!(matches!(
            parse_target_auth(Some("none")).unwrap(),
            AuthKind::None
        ));
        assert!(matches!(
            parse_target_auth(Some("token-env")).unwrap(),
            AuthKind::TokenEnv
        ));
        assert!(matches!(
            parse_target_auth(Some("credential-helper")).unwrap(),
            AuthKind::CredentialHelper
        ));
        assert!(matches!(
            parse_target_auth(Some("ssh")).unwrap(),
            AuthKind::Ssh
        ));
    }

    #[test]
    fn parse_target_auth_rejects_unknown() {
        let err = parse_target_auth(Some("oauth")).unwrap_err();
        assert!(err.to_string().contains("unknown --target-auth"));
    }

    #[test]
    fn build_redirect_readme_includes_pkgref_and_target() {
        let r = build_redirect_readme(
            "flow:internal-helper",
            "https://gitlab.acme.example/flows/internal-helper",
            None,
        );
        assert!(r.contains("flow:internal-helper"));
        assert!(r.contains("https://gitlab.acme.example/flows/internal-helper"));
        assert!(r.contains("vibe-redirect.toml"));
    }

    #[test]
    fn build_redirect_readme_includes_description_when_present() {
        let r = build_redirect_readme(
            "flow:x",
            "https://example.invalid/x",
            Some("delegated to acme-corp"),
        );
        assert!(r.contains("delegated to acme-corp"));
    }

    #[test]
    fn derive_target_token_env_uppercase_and_underscore() {
        assert_eq!(
            derive_target_token_env("https://gitlab.acme.example/x").as_deref(),
            Some("VIBEVM_TARGET_TOKEN_GITLAB_ACME_EXAMPLE")
        );
        assert_eq!(
            derive_target_token_env("https://gitverse.ru/y").as_deref(),
            Some("VIBEVM_TARGET_TOKEN_GITVERSE_RU")
        );
    }

    #[test]
    fn inject_token_passes_through_ssh_form() {
        let url = "git@github.com:vibespecs/flow-wal.git";
        assert_eq!(inject_token_into_url(url, "secret"), url);
    }

    #[test]
    fn inject_token_skips_already_credentialed_https() {
        let url = "https://existing:cred@github.com/x/y";
        assert_eq!(inject_token_into_url(url, "newtoken"), url);
    }

    #[test]
    fn inject_token_embeds_into_https() {
        let url = "https://github.com/vibespecs/flow-wal.git";
        let out = inject_token_into_url(url, "abc123");
        assert_eq!(
            out,
            "https://x-access-token:abc123@github.com/vibespecs/flow-wal.git"
        );
    }

    #[test]
    fn build_target_fetch_url_none_passes_through() {
        let section = RedirectSection {
            target_url: "https://example.invalid/x".into(),
            ref_policy: RefPolicy::PassThroughTag,
            pinned_ref: None,
            auth: AuthKind::None,
            token_env: None,
            description: None,
        };
        let out = build_target_fetch_url("https://example.invalid/x", &section).unwrap();
        assert_eq!(out, "https://example.invalid/x");
    }

    #[test]
    fn build_target_fetch_url_token_env_demands_var_set() {
        let section = RedirectSection {
            target_url: "https://example.invalid/x".into(),
            ref_policy: RefPolicy::PassThroughTag,
            pinned_ref: None,
            auth: AuthKind::TokenEnv,
            token_env: Some("VIBEVM_TEST_DEFINITELY_UNSET_TOKEN_VAR".into()),
            description: None,
        };
        let err = build_target_fetch_url("https://example.invalid/x", &section).unwrap_err();
        assert!(
            err.to_string()
                .contains("VIBEVM_TEST_DEFINITELY_UNSET_TOKEN_VAR")
        );
    }

    // -----------------------------------------------------------------
    // compute_updated_redirect_section + helpers — partial update for
    // `vibe registry redirect-update` (PROP-002 §2.4.2)
    // -----------------------------------------------------------------

    fn baseline_pass_through() -> RedirectSection {
        RedirectSection {
            target_url: "https://github.com/old/flow-wal".into(),
            ref_policy: RefPolicy::PassThroughTag,
            pinned_ref: None,
            auth: AuthKind::None,
            token_env: None,
            description: Some("old description".into()),
        }
    }

    fn baseline_pinned() -> RedirectSection {
        RedirectSection {
            target_url: "https://github.com/old/flow-wal".into(),
            ref_policy: RefPolicy::Pinned,
            pinned_ref: Some("v0.3.0".into()),
            auth: AuthKind::None,
            token_env: None,
            description: None,
        }
    }

    fn empty_update_args() -> RegistryRedirectUpdateArgs {
        RegistryRedirectUpdateArgs {
            pkgref: "flow:wal".into(),
            to: None,
            registry: None,
            ref_policy: None,
            pinned_ref: None,
            target_auth: None,
            target_token_env: None,
            description: None,
            clear_description: false,
            trust_redirect: false,
            resync: false,
            path: PathBuf::from("."),
            dry_run: false,
        }
    }

    #[test]
    fn compute_update_only_description_change_detected() {
        let mut args = empty_update_args();
        args.description = Some("new description".into());
        let (new, changes) =
            compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
        assert_eq!(new.description.as_deref(), Some("new description"));
        assert_eq!(new.target_url, "https://github.com/old/flow-wal");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "description");
        assert!(!changes.iter().any(|c| c.requires_trust()));
    }

    #[test]
    fn compute_update_clear_description_drops_field() {
        let mut args = empty_update_args();
        args.clear_description = true;
        let (new, changes) =
            compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
        assert_eq!(new.description, None);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "description");
        assert_eq!(changes[0].before.as_deref(), Some("old description"));
        assert_eq!(changes[0].after, None);
    }

    #[test]
    fn compute_update_target_url_change_flags_trust() {
        let mut args = empty_update_args();
        args.to = Some("https://forgejo.example/x/y".into());
        let (new, changes) =
            compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
        assert_eq!(new.target_url, "https://forgejo.example/x/y");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "target_url");
        assert!(changes[0].requires_trust());
    }

    #[test]
    fn compute_update_switch_to_pinned_requires_pinned_ref() {
        let mut args = empty_update_args();
        args.ref_policy = Some("pinned".into());
        // No --pinned-ref, no current pinned_ref → reject.
        let err = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap_err();
        assert!(
            err.to_string().contains("requires `--pinned-ref"),
            "expected pinned-ref-required hint, got: {err}"
        );
    }

    #[test]
    fn compute_update_switch_to_pinned_uses_explicit_ref() {
        let mut args = empty_update_args();
        args.ref_policy = Some("pinned".into());
        args.pinned_ref = Some("v1.2.3".into());
        let (new, changes) =
            compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
        assert!(matches!(new.ref_policy, RefPolicy::Pinned));
        assert_eq!(new.pinned_ref.as_deref(), Some("v1.2.3"));
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.requires_trust()));
    }

    #[test]
    fn compute_update_switch_to_pass_through_clears_pinned_ref() {
        let mut args = empty_update_args();
        args.ref_policy = Some("pass-through-tag".into());
        let (new, changes) = compute_updated_redirect_section(&baseline_pinned(), &args).unwrap();
        assert!(matches!(new.ref_policy, RefPolicy::PassThroughTag));
        assert_eq!(new.pinned_ref, None);
        // Two changes: ref_policy + pinned_ref (was Some, now None).
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.requires_trust()));
    }

    #[test]
    fn compute_update_pinned_ref_alone_on_pinned_stub() {
        let mut args = empty_update_args();
        args.pinned_ref = Some("v0.4.0".into());
        // Current is pinned with v0.3.0 — flag bumps to v0.4.0 without
        // touching policy.
        let (new, changes) = compute_updated_redirect_section(&baseline_pinned(), &args).unwrap();
        assert!(matches!(new.ref_policy, RefPolicy::Pinned));
        assert_eq!(new.pinned_ref.as_deref(), Some("v0.4.0"));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "pinned_ref");
        assert!(changes[0].requires_trust());
    }

    #[test]
    fn compute_update_rejects_pinned_ref_on_pass_through() {
        let mut args = empty_update_args();
        args.pinned_ref = Some("v1.0.0".into());
        let err = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap_err();
        assert!(err.to_string().contains("--pinned-ref is only meaningful"));
    }

    #[test]
    fn compute_update_auth_flip_clears_token_env() {
        let with_token = RedirectSection {
            target_url: "https://x/y".into(),
            ref_policy: RefPolicy::PassThroughTag,
            pinned_ref: None,
            auth: AuthKind::TokenEnv,
            token_env: Some("VIBEVM_TARGET_TOKEN_X".into()),
            description: None,
        };
        let mut args = empty_update_args();
        args.target_auth = Some("none".into());
        let (new, changes) = compute_updated_redirect_section(&with_token, &args).unwrap();
        assert!(matches!(new.auth, AuthKind::None));
        assert_eq!(new.token_env, None);
        // Both auth and token_env appear in the diff — operator-side
        // metadata, not trust-required.
        assert!(changes.iter().any(|c| c.field == "auth"));
        assert!(changes.iter().any(|c| c.field == "token_env"));
        assert!(!changes.iter().any(|c| c.requires_trust()));
    }

    #[test]
    fn compute_update_rejects_token_env_without_matching_auth() {
        let mut args = empty_update_args();
        args.target_token_env = Some("WHATEVER".into());
        // Current auth is None; flag not provided → token_env not
        // meaningful.
        let err = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap_err();
        assert!(
            err.to_string()
                .contains("--target-token-env is only meaningful")
        );
    }

    #[test]
    fn compute_update_rejects_empty_to() {
        let mut args = empty_update_args();
        args.to = Some("   ".into());
        let err = compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap_err();
        assert!(err.to_string().contains("--to must be a non-empty"));
    }

    #[test]
    fn compute_update_no_op_returns_empty_changes() {
        let args = empty_update_args();
        let (new, changes) =
            compute_updated_redirect_section(&baseline_pass_through(), &args).unwrap();
        assert_eq!(new, baseline_pass_through());
        assert!(changes.is_empty());
    }

    #[test]
    fn diff_redirect_sections_emits_field_names_in_canonical_order() {
        let before = baseline_pass_through();
        let mut after = baseline_pass_through();
        after.target_url = "new".into();
        after.description = Some("new".into());
        let changes = diff_redirect_sections(&before, &after);
        // Canonical iteration order: target_url, ref_policy, pinned_ref,
        // auth, token_env, description. With two fields touched the
        // order must be target_url first, description last.
        assert_eq!(changes[0].field, "target_url");
        assert_eq!(changes[1].field, "description");
    }

    #[test]
    fn redirect_update_commit_msg_highlights_target_url_change() {
        let changes = vec![
            super::RedirectChangeEntry {
                field: "target_url",
                before: Some("https://old/x".into()),
                after: Some("https://new/x".into()),
            },
            super::RedirectChangeEntry {
                field: "description",
                before: None,
                after: Some("delegated".into()),
            },
        ];
        let msg = build_redirect_update_commit_msg("flow:wal", &changes);
        assert!(msg.contains("retarget flow:wal"));
        assert!(msg.contains("https://new/x"));
    }

    #[test]
    fn redirect_update_commit_msg_lists_fields_when_no_target_change() {
        let changes = vec![
            super::RedirectChangeEntry {
                field: "auth",
                before: Some("none".into()),
                after: Some("token-env".into()),
            },
            super::RedirectChangeEntry {
                field: "token_env",
                before: None,
                after: Some("VAR".into()),
            },
        ];
        let msg = build_redirect_update_commit_msg("flow:wal", &changes);
        assert!(msg.contains("update marker for flow:wal"));
        assert!(msg.contains("auth, token_env"));
    }
}
