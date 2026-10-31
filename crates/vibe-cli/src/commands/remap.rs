//! `vibe remap list` and `vibe remap explain` (`PROP-066#commands`).
//!
//! The two read-only views over the rows the composition root installed.
//! Neither touches a registry, a cache or a lock; `list` makes at most one
//! DNS lookup per distinct target host for its diagnostics, and none at
//! all under `--offline`.
//!
//! `accept`, `reject`, `test`, `enable`, `disable`, `export` and `import`
//! of `PROP-066#commands` are deliberately absent: they are about remap
//! *packages* and the consent that activates them, which this slice does
//! not build. A verb the CLI cannot honour must not appear in its help.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#commands");

mod diagnostics;
mod render;

use anyhow::Result;
use vibe_core::remap::{
    Artifact, ArtifactKind, Layer, Locator, RemapKey, RemapRow, RemapTarget, UnmatchedPolicy,
};
use vibe_wire::generated::remap_explanation as explanation;
use vibe_wire::generated::remap_listing as wire;

use crate::cli::{RemapArgs, RemapCommand, RemapExplainArgs, RemapListArgs};
use crate::output;

/// The `vibe-remap/1` documents both views emit.
const PROTOCOL: &str = "vibe-remap/1";

#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#commands")]
pub(crate) fn run(ctx: &output::Context, args: RemapArgs, offline: bool) -> Result<()> {
    let locator = vibe_core::remap::active();
    match args.command {
        RemapCommand::List(list) => run_list(ctx, locator, list, offline),
        RemapCommand::Explain(explain) => run_explain(ctx, locator, explain, offline),
    }
}

fn run_list(
    ctx: &output::Context,
    locator: &Locator,
    args: RemapListArgs,
    offline: bool,
) -> Result<()> {
    let document = listing(locator, offline);
    if args.json || ctx.is_json() {
        return ctx.emit_json(&document);
    }
    let rows: u32 = document.sources.iter().map(|s| s.rows.len() as u32).sum();
    ctx.summary(&format!(
        "{rows} remap row(s) from {} source(s); unmatched = {}",
        document.sources.len(),
        match document.policy {
            wire::UnmatchedPolicy::Allow => "allow",
            wire::UnmatchedPolicy::Deny => "deny",
        }
    ));
    if ctx.is_quiet() {
        return Ok(());
    }
    print!("{}", render::listing(&document));
    Ok(())
}

fn run_explain(
    ctx: &output::Context,
    locator: &Locator,
    args: RemapExplainArgs,
    offline: bool,
) -> Result<()> {
    let artifact = asked_about(&args);
    let document = explanation(locator, &artifact, offline);
    if args.json || ctx.is_json() {
        return ctx.emit_json(&document);
    }
    ctx.summary(&format!(
        "{} matching row(s); {}",
        document.matches.len(),
        match (&document.chosen, document.denied) {
            (_, true) => "refused before any connection".to_owned(),
            (Some(chosen), false) => format!("chosen {chosen}"),
            (None, false) => "nothing to fetch".to_owned(),
        }
    ));
    if ctx.is_quiet() {
        return Ok(());
    }
    print!("{}", render::explain(&document));
    Ok(())
}

/// What `explain` was asked about: a `sha256:<hex>` or bare 64-hex digest
/// is an artifact matched by its digest alone; anything else is a URL.
///
/// The artifact kind does not change which rows match — a `sha256` row
/// matches a digest wherever it was declared, and a `prefix` row matches a
/// URL whichever path is fetching it — so the question is answered for
/// every path at once and the document says `any` rather than naming one.
fn asked_about(args: &RemapExplainArgs) -> Artifact {
    let what = args.what.trim();
    let digest = what
        .strip_prefix("sha256:")
        .or_else(|| {
            (what.len() == 64 && what.bytes().all(|b| b.is_ascii_hexdigit())).then_some(what)
        })
        .map(str::to_ascii_lowercase);
    let mut artifact = Artifact::new(ArtifactKind::AppInput);
    match digest {
        Some(digest) => artifact = artifact.with_digest(digest),
        None => artifact = artifact.with_url(what),
    }
    if let Some(package) = &args.package {
        artifact = artifact.with_package(package.clone());
    }
    if let Some(step) = &args.step {
        artifact = artifact.with_step(step.clone());
    }
    artifact
}

/// Build the `vibe remap list` document: one block per source, in the
/// order the layers were loaded, and the per-host footer.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#display")]
fn listing(locator: &Locator, offline: bool) -> wire::RemapListing {
    let mut sources: Vec<wire::Source> = Vec::new();
    let mut targets: Vec<(String, wire::Reach, Option<String>)> = Vec::new();
    let mut denies = 0u32;
    for (layer, name, policy) in locator.sources() {
        let rows: Vec<&RemapRow> = locator
            .rows()
            .iter()
            .filter(|row| row.from.layer == *layer && &row.from.source == name)
            .collect();
        let mut block = Vec::with_capacity(rows.len());
        for (position, row) in rows.iter().enumerate() {
            let found = diagnostics::diagnose(&row.key, &row.target, offline);
            if row.target.is_deny() {
                denies += 1;
            }
            targets.push((
                found.scheme.clone(),
                found.reach.clone(),
                found.host.clone(),
            ));
            block.push(wire::Row {
                number: position as u32 + 1,
                kind: key_kind(&row.key),
                what: row.key.what().to_owned(),
                to: row.target.display(),
                deny: row.target.is_deny(),
                scheme: found.scheme,
                reach: found.reach,
                host: found.host,
                why: row.reason.clone(),
                scope: row.scope.as_ref().map(|scope| wire::Scope {
                    package: scope.package.clone(),
                    step: scope.step.clone(),
                }),
                // `from` carries the detail PROP-002's tables add and a
                // `[[remap]]` row does not: the git ref an `[[override]]`
                // pinned beside its source URL.
                from: row
                    .target_ref
                    .as_ref()
                    .map(|reference| format!("ref {reference}")),
                notes: found.notes,
            });
        }
        sources.push(wire::Source {
            source: name.clone(),
            layer: wire_layer(*layer),
            policy: wire_policy(*policy),
            rows: block,
        });
    }
    wire::RemapListing {
        protocol: PROTOCOL.into(),
        policy: wire_policy(locator.policy()),
        sources,
        hosts: diagnostics::host_summary(&targets),
        denies,
    }
}

/// Build the `vibe remap explain` document from the Locator's own chain.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#precedence")]
fn explanation(
    locator: &Locator,
    artifact: &Artifact,
    offline: bool,
) -> explanation::RemapExplanation {
    let chain = locator.explain(artifact);
    let matches = chain
        .matches
        .iter()
        .enumerate()
        .map(|(position, found)| {
            let diagnosis = diagnostics::diagnose(&found.key, &found.target, offline);
            explanation::Match {
                order: position as u32 + 1,
                layer: explain_layer(found.row.provenance.layer),
                source: found.row.provenance.source.clone(),
                index: found.row.provenance.index as u32,
                kind: explain_kind(&found.key),
                what: found.key.what().to_owned(),
                to: found.target.display(),
                deny: found.target.is_deny(),
                scheme: diagnosis.scheme,
                reach: explain_reach(diagnosis.reach),
                host: diagnosis.host,
                why: found.reason.clone(),
                scope: found.scope.as_ref().map(|scope| explanation::Scope {
                    package: scope.package.clone(),
                    step: scope.step.clone(),
                }),
                notes: diagnosis.notes.into_iter().map(explain_note).collect(),
            }
        })
        .collect();
    explanation::RemapExplanation {
        protocol: PROTOCOL.into(),
        kind: "any".into(),
        origin: chain.origin.clone(),
        digest: chain.digest.clone(),
        policy: explain_policy(chain.policy),
        denied: chain.denied,
        chosen: chain.chosen.as_ref().map(RemapTarget::display),
        matches,
    }
}

fn key_kind(key: &RemapKey) -> wire::KeyKind {
    match key {
        RemapKey::Sha256(_) => wire::KeyKind::Sha256,
        RemapKey::Package(_) => wire::KeyKind::Package,
        RemapKey::Registry(_) => wire::KeyKind::Registry,
        RemapKey::Prefix(_) => wire::KeyKind::Prefix,
    }
}

fn explain_kind(key: &RemapKey) -> explanation::KeyKind {
    match key {
        RemapKey::Sha256(_) => explanation::KeyKind::Sha256,
        RemapKey::Package(_) => explanation::KeyKind::Package,
        RemapKey::Registry(_) => explanation::KeyKind::Registry,
        RemapKey::Prefix(_) => explanation::KeyKind::Prefix,
    }
}

fn wire_layer(layer: Layer) -> wire::Layer {
    match layer {
        Layer::UserPackage => wire::Layer::UserPackage,
        Layer::ProjectPackage => wire::Layer::ProjectPackage,
        Layer::Local => wire::Layer::Local,
        Layer::Invocation => wire::Layer::Invocation,
    }
}

fn explain_layer(layer: Layer) -> explanation::Layer {
    match layer {
        Layer::UserPackage => explanation::Layer::UserPackage,
        Layer::ProjectPackage => explanation::Layer::ProjectPackage,
        Layer::Local => explanation::Layer::Local,
        Layer::Invocation => explanation::Layer::Invocation,
    }
}

fn wire_policy(policy: UnmatchedPolicy) -> wire::UnmatchedPolicy {
    match policy {
        UnmatchedPolicy::Allow => wire::UnmatchedPolicy::Allow,
        UnmatchedPolicy::Deny => wire::UnmatchedPolicy::Deny,
    }
}

fn explain_policy(policy: UnmatchedPolicy) -> explanation::UnmatchedPolicy {
    match policy {
        UnmatchedPolicy::Allow => explanation::UnmatchedPolicy::Allow,
        UnmatchedPolicy::Deny => explanation::UnmatchedPolicy::Deny,
    }
}

fn explain_reach(reach: wire::Reach) -> explanation::Reach {
    match reach {
        wire::Reach::Local => explanation::Reach::Local,
        wire::Reach::Private => explanation::Reach::Private,
        wire::Reach::Public => explanation::Reach::Public,
        wire::Reach::Unknown => explanation::Reach::Unknown,
    }
}

fn explain_note(note: wire::Note) -> explanation::Note {
    match note {
        wire::Note::Unencrypted => explanation::Note::Unencrypted,
        wire::Note::Broad => explanation::Note::Broad,
        wire::Note::Unknown => explanation::Note::Unknown,
    }
}

#[cfg(test)]
#[path = "remap/tests.rs"]
mod tests;
