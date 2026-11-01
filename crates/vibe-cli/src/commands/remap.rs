//! `vibe remap list` and `vibe remap explain` (`PROP-066#commands`).
//!
//! The two read-only views over the rows the composition root installed.
//! Neither touches a registry, a cache or a lock; `list` makes at most one
//! DNS lookup per distinct target host for its diagnostics, and none at
//! all under `--offline`.
//!
//! The remaining verbs are about remap **packages** and the consent that
//! activates them (`PROP-066#commands-packages`): `accept` / `reject` /
//! `enable` / `disable` answer the question
//! ([`verbs`], [`consent`]), `test` probes every declared artifact's
//! winning target without downloading it ([`probe`]), and `export` /
//! `import` carry a package and its artifacts into a closed network
//! ([`transport`], [`bundle`]). The home's three files — installed
//! packages, the acceptance ledger, the derived provider map — are read
//! and written in one place ([`home`]).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#commands");

pub(crate) mod bundle;
mod consent;
mod diagnostics;
mod home;
pub(crate) mod packages;
mod probe;
mod render;
mod transport;
mod uninstall;
mod verbs;

pub(crate) use consent::{ConsentFlags, ConsentRequired, resolve as resolve_consent};
pub(crate) use home::RemapHome;
pub(crate) use packages::{
    adopt_project_requirements, claims as claims_install, install as install_packages,
};
pub(crate) use probe::RemapUnreachable;
pub(crate) use uninstall::{claims as claims_uninstall, run as run_uninstall};
pub(crate) use verbs::RemapPackageMissing;

use anyhow::Result;
use vibe_core::remap::{
    Artifact, ArtifactKind, Layer, Locator, OfferState, RemapKey, RemapRow, RemapTarget,
    RowIdentity, UnmatchedPolicy,
};
use vibe_wire::generated::remap_explanation as explanation;
use vibe_wire::generated::remap_listing as wire;

use crate::cli::{RemapArgs, RemapCommand, RemapExplainArgs, RemapListArgs};
use crate::output;

/// The `vibe-remap/1` documents both views emit.
const PROTOCOL: &str = "vibe-remap/1";

/// The whole `vibe remap` family. `Some(code)` is `remap test`'s exit
/// status — a readiness check whose status IS the answer
/// (`PROP-066#policy`) — and `None` is every other verb's "it printed its
/// report".
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#commands")]
pub(crate) fn dispatch(
    ctx: &output::Context,
    args: RemapArgs,
    offline: bool,
    accept_remap: bool,
) -> Result<Option<i32>> {
    match args.command {
        RemapCommand::Test(test) => {
            probe::run(ctx, vibe_core::remap::active(), test, offline).map(Some)
        }
        command => run(ctx, RemapArgs { command }, offline, accept_remap).map(|()| None),
    }
}

#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#commands")]
pub(crate) fn run(
    ctx: &output::Context,
    args: RemapArgs,
    offline: bool,
    accept_remap: bool,
) -> Result<()> {
    let locator = vibe_core::remap::active();
    match args.command {
        RemapCommand::List(list) => run_list(ctx, locator, list, offline),
        RemapCommand::Explain(explain) => run_explain(ctx, locator, explain, offline),
        RemapCommand::Accept(packages) => verbs::run(ctx, verbs::Verb::Accept, packages),
        RemapCommand::Reject(packages) => verbs::run(ctx, verbs::Verb::Reject, packages),
        RemapCommand::Enable(packages) => verbs::run(ctx, verbs::Verb::Enable, packages),
        RemapCommand::Disable(packages) => verbs::run(ctx, verbs::Verb::Disable, packages),
        // `dispatch` takes `test` before `run` is reached, because it
        // alone answers with a status rather than only a report.
        RemapCommand::Test(test) => probe::run(ctx, locator, test, offline).map(|_| ()),
        RemapCommand::Export(export) => transport::export(ctx, export),
        RemapCommand::Import(import) => transport::import(ctx, import, accept_remap),
    }
}

fn run_list(
    ctx: &output::Context,
    locator: &Locator,
    args: RemapListArgs,
    offline: bool,
) -> Result<()> {
    // The home is read for the two things a row alone cannot say: which
    // packages offer it (`PROP-066#providers`), and what state each of
    // those packages is in (`PROP-066#consent`). A home that will not
    // open is simply no packages, which is what `list` showed before
    // remap packages existed.
    let home = home::RemapHome::open().ok();
    let document = listing(locator, home.as_ref(), offline);
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
    ctx.block(&render::listing(&document));
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
    ctx.block(&render::explain(&document));
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
fn listing(locator: &Locator, home: Option<&home::RemapHome>, offline: bool) -> wire::RemapListing {
    let mut sources: Vec<wire::Source> = Vec::new();
    let mut targets: Vec<(String, wire::Reach, Option<String>)> = Vec::new();
    let mut denies = 0u32;
    let providers = home.map(home::RemapHome::providers);
    let reachability = home.map(|home| vibe_core::remap::Reachability::load(home.root()));
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
                from: row.from_package.clone().or_else(|| {
                    row.target_ref
                        .as_ref()
                        .map(|reference| format!("ref {reference}"))
                }),
                notes: found.notes,
                // Which packages offer this row — a row many accepted
                // packages share reads differently from one a single
                // package brought (`PROP-066#providers`).
                providers: providers
                    .as_ref()
                    .map(|map| map.providers_of(&RowIdentity::of(row)).to_vec())
                    .unwrap_or_default(),
                // What the target answered the last time `remap test` ran
                // (`PROP-066#diagnostics-reachability`) — a record of a
                // past probe, dated, never a live claim.
                probed: reachability
                    .as_ref()
                    .and_then(|record| record.note(&row.target.display())),
            });
        }
        sources.push(wire::Source {
            source: name.clone(),
            layer: wire_layer(*layer),
            policy: wire_policy(*policy),
            // A package source's header carries its state; a settings
            // file's does not, because a file the user wrote needs no
            // acceptance (`PROP-066#layers`).
            state: package_state(home, name),
            rows: block,
        });
    }
    // Every installed package the Locator does NOT carry rows from — one
    // that is offered, rejected or switched off — still belongs in the
    // listing: `PROP-066#consent` asks for `offered` and `rejected` to be
    // visible, and a catalog that showed only what applies would hide
    // exactly the question the user has to answer.
    if let Some(home) = home {
        for offer in home.offers() {
            let label = offer.label();
            if sources.iter().any(|source| source.source == label) {
                continue;
            }
            sources.push(inert_source(&offer, home, offline));
        }
    }
    wire::RemapListing {
        protocol: PROTOCOL.into(),
        policy: wire_policy(locator.policy()),
        sources,
        hosts: diagnostics::host_summary(&targets),
        denies,
    }
}

/// The state of the package a source's rows came from, when the source IS
/// a package (`PROP-066#consent`).
fn package_state(home: Option<&home::RemapHome>, source: &str) -> Option<wire::PackageState> {
    let home = home?;
    let key = source.split('@').next()?;
    Some(match home.state(key)? {
        OfferState::Offered => wire::PackageState::Offered,
        OfferState::Accepted => wire::PackageState::Accepted,
        OfferState::Rejected => wire::PackageState::Rejected,
        OfferState::Disabled => wire::PackageState::Disabled,
    })
}

/// One block for an installed package whose rows do not apply: the same
/// layout, with the state saying why nothing of it is in force.
fn inert_source(
    offer: &vibe_core::remap::RemapPackage,
    home: &home::RemapHome,
    offline: bool,
) -> wire::Source {
    let rows = offer
        .loaded(Layer::UserPackage)
        .map(|loaded| loaded.rows)
        .unwrap_or_default();
    wire::Source {
        source: offer.label(),
        layer: wire::Layer::UserPackage,
        policy: wire_policy(offer.policy.unmatched),
        state: package_state(Some(home), &offer.label()),
        rows: rows
            .iter()
            .enumerate()
            .map(|(position, row)| {
                let found = diagnostics::diagnose(&row.key, &row.target, offline);
                wire::Row {
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
                    from: row.from_package.clone(),
                    notes: found.notes,
                    providers: home
                        .providers()
                        .providers_of(&RowIdentity::of(row))
                        .to_vec(),
                    probed: vibe_core::remap::Reachability::load(home.root())
                        .note(&row.target.display()),
                }
            })
            .collect(),
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
