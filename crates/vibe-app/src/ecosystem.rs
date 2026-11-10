//! Ecosystem adapters (PROP-075 `#ECOSYSTEM-ADAPTER`): the four verbs by
//! which a runtime package installs, lists, removes and self-upgrades the
//! packages of its **own** ecosystem, run inside one installed version.
//!
//! Nothing in this module knows one ecosystem word. An adapter `id` is an
//! opaque token this crate only ever echoes, and an item string is handed
//! on **verbatim**: never parsed, split, quoted, escaped, validated or
//! reordered. The `install` and `remove` verbs receive the items as
//! separate argv elements of a `recipe` script — the manifest layer
//! refuses a `shell` spelling of either, which is what makes that
//! structural rather than hopeful.
//!
//! What this module does *not* do is decide. Consent and the composition
//! of the list belong to the caller: the library runs exactly the verb it
//! is handed with exactly the items it is given.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS");

mod listing;
mod record;
mod run;

use specmark::spec;
use vibe_core::manifest::{EcosystemDecl, EcosystemVerb};
use vibe_wire::generated::application_ecosystem as wire;

use crate::error::{AppError, StepError};
use crate::index::Index;
use crate::operations::OperationContext;

pub use listing::{EXTRAS_PROTOCOL, listing as extras_listing};
pub(crate) use record::summary as ecosystem_summary;
pub use record::{
    ECOSYSTEM_PROTOCOL, kept_runs as kept_ecosystem_work, work_runs as slot_ecosystem_work,
};

/// What one adapter run was asked to do. The selection is already
/// decided: `items` is the exact, ordered, opaque list the verb receives,
/// and an empty list for a verb that takes items means the adapter is not
/// run at all.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
pub struct EcosystemRequest<'a> {
    pub slot: &'a str,
    /// The version or the whole installation key; `None` is the version
    /// the slot's default points at.
    pub version: Option<&'a str>,
    /// One adapter by `id`; `None` is every adapter the package declares.
    pub ecosystem: Option<&'a str>,
    pub verb: EcosystemVerb,
    /// The items of an `install` or `remove` verb, verbatim and in order.
    pub items: &'a [String],
}

impl EcosystemRequest<'_> {
    /// The exact command that runs this verb again, which a failure
    /// reports instead of rolling anything back (PROP-075 `#SLOT-EXTRAS`).
    pub(crate) fn retry(&self, ecosystem: &str) -> String {
        let mut text = match self.verb {
            // The retry of a failed install is the apply command, because
            // what failed is "put my extras in this version".
            EcosystemVerb::Install => format!("vibe app extras apply {}", self.slot),
            EcosystemVerb::List => format!("vibe app extras list {}", self.slot),
            EcosystemVerb::Remove => format!("vibe app extras remove {}", self.slot),
            EcosystemVerb::SelfUpgrade => format!("vibe app extras self-upgrade {}", self.slot),
        };
        text.push_str(&format!(" --ecosystem {ecosystem}"));
        if self.verb == EcosystemVerb::Remove {
            for item in self.items {
                text.push(' ');
                text.push_str(item);
            }
        }
        text
    }
}

/// Run one verb of one or every declared adapter of an installed version
/// (PROP-075 `#ECOSYSTEM-ADAPTER`).
///
/// Every adapter runs independently and nothing is ever rolled back: a
/// failing verb leaves the version installed, its slot default exactly
/// where the user put it and the installation record written. The work
/// directory is kept either way, and the returned document names, per
/// adapter, the items asked for, what a `list` verb observed, and the
/// command that retries it.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
pub fn ecosystem_run(
    ctx: &OperationContext<'_>,
    request: &EcosystemRequest<'_>,
) -> Result<wire::ApplicationEcosystem, AppError> {
    let held = ctx.settings.lock()?;
    let index = Index::load(ctx.settings)?;
    let (key, version) = target(&index, request.slot, request.version)?;
    let resolver = ctx
        .resolver
        .ok_or_else(|| StepError::EcosystemUnresolvable {
            slot: request.slot.to_owned(),
            reason: "this command cannot resolve application packages".to_owned(),
        })?;
    let package = resolver(&key).map_err(|error| StepError::EcosystemUnresolvable {
        slot: request.slot.to_owned(),
        reason: error.to_string(),
    })?;
    let chosen = selected(&package, request)?;
    let (paths, root) = crate::migrate::installation_paths(ctx, &index, &key, &held)?;
    let record = &index.installations[&key];
    let work = record::open(ctx.settings, request.slot, &version)?;
    let mut document = record::empty(request, &key, &work);
    let session = run::Session {
        ctx,
        request,
        key: &key,
        version: &version,
        work: &work,
        package: &package,
        root,
        paths,
        platform: record.platform.clone(),
        arch: record.arch.clone(),
    };
    for adapter in chosen {
        document.adapters.push(session.run_verb(adapter)?);
        record::save(&work, &document)?;
    }
    document.ok = document
        .adapters
        .iter()
        .all(|row| row.outcome != wire::Outcome::Failed);
    record::save(&work, &document)?;
    Ok(document)
}

/// The installation a verb runs inside: the named version or key, else the
/// one the slot's default points at.
fn target(index: &Index, slot: &str, version: Option<&str>) -> Result<(String, String), AppError> {
    if !index.installations.values().any(|row| row.slot == slot) {
        return Err(AppError::UnknownSlot {
            slot: slot.to_owned(),
        });
    }
    if let Some(spelling) = version {
        let (key, record) = crate::migrate::installation_in_slot(index, slot, spelling)?;
        let version = record.package.version.clone();
        return Ok((key, version));
    }
    let selection = crate::activation::slot_default(index, slot);
    match selection.key.zip(selection.version) {
        Some((key, version)) => Ok((key, version)),
        None => Err(AppError::NotInstalled {
            package: slot.to_owned(),
            home: "the selected application home".to_owned(),
            fix: format!(
                "the slot has no default version, so there is none to work in; choose one with \
                 `vibe app use {slot} <version>`, or name a version from \
                 `vibe app alternatives {slot}`"
            ),
        }),
    }
}

/// The adapters this run touches, in declaration order. An explicitly
/// named adapter that does not exist, or does not declare the verb, is a
/// refusal; with no `--ecosystem` the adapters that do not declare it are
/// reported `skipped` by the runner instead, because only `install` is
/// required of an adapter and asking every one of them is not an error.
fn selected<'a>(
    package: &'a crate::resolve::ResolvedPackage,
    request: &EcosystemRequest<'_>,
) -> Result<Vec<&'a EcosystemDecl>, AppError> {
    let declared = &package.application.ecosystem;
    if declared.is_empty() {
        return Err(StepError::NoEcosystem {
            slot: request.slot.to_owned(),
            verb: request.verb.as_str().to_owned(),
        }
        .into());
    }
    let Some(id) = request.ecosystem else {
        return Ok(declared.iter().collect());
    };
    let Some(adapter) = package.application.ecosystem_adapter(id) else {
        return Err(StepError::UnknownEcosystem {
            slot: request.slot.to_owned(),
            id: id.to_owned(),
            declared: ids(declared),
        }
        .into());
    };
    if adapter.verb(request.verb).is_none() {
        return Err(StepError::EcosystemVerbUndeclared {
            slot: request.slot.to_owned(),
            id: id.to_owned(),
            verb: request.verb.as_str().to_owned(),
            declared: verbs(adapter),
        }
        .into());
    }
    Ok(vec![adapter])
}

/// The declared adapter ids, for a refusal that has to name them.
fn ids(declared: &[EcosystemDecl]) -> String {
    declared
        .iter()
        .map(|row| row.id.clone())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The verbs one adapter declares, for a refusal that has to name them.
fn verbs(adapter: &EcosystemDecl) -> String {
    adapter
        .declared_verbs()
        .into_iter()
        .map(|verb| verb.as_str().to_owned())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The wire spelling of a verb.
pub(crate) fn wire_verb(verb: EcosystemVerb) -> wire::Verb {
    match verb {
        EcosystemVerb::Install => wire::Verb::Install,
        EcosystemVerb::List => wire::Verb::List,
        EcosystemVerb::Remove => wire::Verb::Remove,
        EcosystemVerb::SelfUpgrade => wire::Verb::SelfUpgrade,
    }
}

#[cfg(test)]
#[path = "ecosystem/tests.rs"]
mod tests;
