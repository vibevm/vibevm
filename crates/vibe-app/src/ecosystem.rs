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
//! What this module does *not* do is decide, except where the design put
//! the decision here: consent belongs to the caller, and so does the item
//! list of an explicit verb, while the **final list** of an apply is the
//! arithmetic of `#SLOT-EXTRAS` and lives in [`extras`].

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS");

mod extras;
mod listing;
mod record;
mod run;

use specmark::spec;
use vibe_core::manifest::{EcosystemDecl, EcosystemVerb};
use vibe_wire::generated::application_ecosystem as wire;

use crate::error::{AppError, StepError};
use crate::index::Index;
use crate::operations::OperationContext;

pub use extras::{ExtrasApply, ExtrasPolicy, extras_apply, final_list as extras_final_list};
pub use listing::{EXTRAS_PROTOCOL, listing as extras_listing};
pub(crate) use record::summary as ecosystem_summary;
pub use record::{
    ECOSYSTEM_PROTOCOL, kept_runs as kept_ecosystem_work, work_runs as slot_ecosystem_work,
};

/// Which installation, which adapters and which verb one run is about —
/// everything the selection needs and nothing about the items.
#[derive(Debug, Clone, Copy)]
pub(super) struct Target<'a> {
    pub slot: &'a str,
    /// The version or the whole installation key; `None` is the version
    /// the slot's default points at.
    pub version: Option<&'a str>,
    /// One adapter by `id`; `None` is every adapter the package declares.
    pub ecosystem: Option<&'a str>,
    pub verb: EcosystemVerb,
}

/// Where one adapter's item list comes from.
enum Items<'a> {
    /// Already decided by the caller: `vibe app extras remove ITEM…`, and
    /// the two verbs that take no items at all.
    Given(&'a [String]),
    /// Computed per adapter by the arithmetic of `#SLOT-EXTRAS` over the
    /// author's recommendations, the installation's chosen feature set and
    /// the user's own `add` / `drop`.
    Computed(ExtrasPolicy),
}

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

impl<'a> EcosystemRequest<'a> {
    fn target(&self) -> Target<'a> {
        Target {
            slot: self.slot,
            version: self.version,
            ecosystem: self.ecosystem,
            verb: self.verb,
        }
    }
}

impl Target<'_> {
    /// The exact command that runs this verb again, which a failure
    /// reports instead of rolling anything back (PROP-075 `#SLOT-EXTRAS`).
    pub(crate) fn retry(&self, ecosystem: &str, items: &[String]) -> String {
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
            for item in items {
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
    run_all(ctx, &request.target(), Items::Given(request.items))
}

/// One verb over the selected adapters, with `items` deciding what each
/// one is handed. The single runner behind both the explicit verbs and the
/// apply, so there is one lock, one work directory, one record and one
/// publication whichever way the list was arrived at.
fn run_all(
    ctx: &OperationContext<'_>,
    target: &Target<'_>,
    items: Items<'_>,
) -> Result<wire::ApplicationEcosystem, AppError> {
    let held = ctx.settings.lock()?;
    let mut index = Index::load(ctx.settings)?;
    let (key, version) = self::target(&index, target.slot, target.version)?;
    let package = resolve(ctx, target.slot, &key)?;
    let chosen = index.installations[&key].features.clone();
    let chosen_adapters: Vec<EcosystemDecl> =
        selected(&package, target)?.into_iter().cloned().collect();
    let (paths, root) = crate::migrate::installation_paths(ctx, &index, &key, &held)?;
    let record = &index.installations[&key];
    let (platform, arch) = (record.platform.clone(), record.arch.clone());
    let work = record::open(ctx.settings, target.slot, &version)?;
    let mut document = record::empty(target, &key, &work);
    // The user's own list, and every stale line it earned a word about.
    let user = match items {
        Items::Computed(_) => {
            let mut user = extras::user_extras(ctx.user_config, &index, target.slot);
            user.notices.extend(extras::stale_notices(&package, &user));
            user
        }
        Items::Given(_) => extras::UserExtras::default(),
    };
    document.notices = user.notices.clone();
    let session = run::Session {
        ctx,
        target,
        key: &key,
        version: &version,
        work: &work,
        package: &package,
        root,
        paths,
        platform,
        arch,
    };
    for adapter in &chosen_adapters {
        let list = match items {
            Items::Given(given) => given.to_vec(),
            Items::Computed(policy) => {
                extras::computed(&package, adapter, &chosen, &user, policy).items
            }
        };
        document.adapters.push(session.run_verb(adapter, &list)?);
        record::save(&work, &document)?;
    }
    document.ok = document
        .adapters
        .iter()
        .all(|row| row.outcome != wire::Outcome::Failed);
    record::save(&work, &document)?;
    // What each adapter applied and observed is kept with the
    // installation, so a later `vibe app describe` still says what a
    // version carries — and what it did not (`PROP-075#SLOT-EXTRAS`).
    remember(ctx, &mut index, &key, &document, &held)?;
    Ok(document)
}

/// Record this run's rows on the installation and publish the inventory.
/// A record is the first thing in a home that may need the
/// `vibe-user-applications/8` reader, so the publication admits and
/// refreshes the capable proxy image first (PROP-059
/// `#proxy-index-upgrade`), exactly as a migration's outcome does.
fn remember(
    ctx: &OperationContext<'_>,
    index: &mut Index,
    key: &str,
    document: &wire::ApplicationEcosystem,
    lock: &crate::MutationLock,
) -> Result<(), AppError> {
    let Some(record) = index.installations.get_mut(key) else {
        return Ok(());
    };
    if !extras::remember(&mut record.extras, document) {
        return Ok(());
    }
    let image = crate::proxies::for_publication(ctx.settings, index, ctx.proxy_image)?;
    index.publish_locked(
        ctx.settings,
        &crate::catalog::HomeEvent::about(crate::catalog::EventKind::Updated, key.to_owned()),
        lock,
        image.as_ref(),
    )?;
    Ok(())
}

/// That version's package tree, where an adapter's scripts live — a verb
/// is the package's own script, and the inventory does not retain one.
fn resolve(
    ctx: &OperationContext<'_>,
    slot: &str,
    key: &str,
) -> Result<crate::resolve::ResolvedPackage, AppError> {
    let resolver = ctx
        .resolver
        .ok_or_else(|| StepError::EcosystemUnresolvable {
            slot: slot.to_owned(),
            reason: "this command cannot resolve application packages".to_owned(),
        })?;
    resolver(key).map_err(|error| {
        StepError::EcosystemUnresolvable {
            slot: slot.to_owned(),
            reason: error.to_string(),
        }
        .into()
    })
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
    target: &Target<'_>,
) -> Result<Vec<&'a EcosystemDecl>, AppError> {
    let declared = &package.application.ecosystem;
    if declared.is_empty() {
        return Err(StepError::NoEcosystem {
            slot: target.slot.to_owned(),
            verb: target.verb.as_str().to_owned(),
        }
        .into());
    }
    let Some(id) = target.ecosystem else {
        return Ok(declared.iter().collect());
    };
    let Some(adapter) = package.application.ecosystem_adapter(id) else {
        return Err(StepError::UnknownEcosystem {
            slot: target.slot.to_owned(),
            id: id.to_owned(),
            declared: ids(declared),
        }
        .into());
    };
    if adapter.verb(target.verb).is_none() {
        return Err(StepError::EcosystemVerbUndeclared {
            slot: target.slot.to_owned(),
            id: id.to_owned(),
            verb: target.verb.as_str().to_owned(),
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

#[cfg(test)]
#[path = "ecosystem/extras_tests.rs"]
mod extras_tests;
