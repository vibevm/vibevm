//! `vibe registry set-mirror` — attach a `[[mirror]]` to one registry
//! or to every registry (`of = "*"`).

specmark::scope!("spec://vibevm/VIBEVM-SPEC#registry");

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::manifest::{Manifest, MirrorSection};

use crate::cli::RegistrySetMirrorArgs;
use crate::commands::registry::resolve_project_root;
use crate::output;

use super::ListReportMirror;

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

pub(in crate::commands::registry) fn run_set_mirror(
    ctx: &output::Context,
    args: RegistrySetMirrorArgs,
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
