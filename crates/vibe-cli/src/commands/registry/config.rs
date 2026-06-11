//! `vibe registry list / add / set-mirror / remove / test` —
//! `[[registry]]` and `[[mirror]]` configuration management.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#registry");

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use vibe_core::manifest::{Manifest, MirrorSection, NamingConvention, RegistrySection};
use vibe_publish::{extract_host_segment, extract_org_segment};

use crate::cli::{
    RegistryAddArgs, RegistryListArgs, RegistryRemoveArgs, RegistryRemoveMirrorArgs,
    RegistryRemoveRegistryArgs, RegistryRemoveTarget, RegistrySetMirrorArgs, RegistryTestArgs,
};
use crate::output;

use super::resolve_project_root;

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

pub(super) fn run_list(ctx: &output::Context, args: RegistryListArgs) -> Result<()> {
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

pub(super) fn run_add(ctx: &output::Context, args: RegistryAddArgs) -> Result<()> {
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

pub(super) fn run_set_mirror(ctx: &output::Context, args: RegistrySetMirrorArgs) -> Result<()> {
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

pub(super) fn run_remove(ctx: &output::Context, args: RegistryRemoveArgs) -> Result<()> {
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

pub(super) fn run_test(ctx: &output::Context, args: RegistryTestArgs) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::{adapter_for_host, parse_naming};
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
}
