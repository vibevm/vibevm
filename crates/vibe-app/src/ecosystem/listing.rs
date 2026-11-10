//! `vibe app extras list SLOT` (PROP-075 `#SLOT-EXTRAS`): per declared
//! adapter of one installed version, which verbs the package wrote, the
//! author's recommended items with their enabled state, the list that
//! would be installed, and what the adapter's `list` verb observes in that
//! version.
//!
//! The enabled state is read off the **recorded** chosen feature set of
//! the installation, because an `[[application.ecosystem.recommends]]` row
//! is shorthand for one ordinary installer feature and nothing else
//! (PROP-075 `#ECOSYSTEM-RECOMMENDS`). The observation is taken by running
//! the adapter's own `list` verb; a verb that fails leaves the listing
//! standing and names why, because a listing that refuses to answer what
//! it does know would be the wrong trade.
//!
//! The listing answers the same arithmetic the apply runs, under the same
//! two flags: `items` is the **final** list — the enabled recommendations
//! minus the user's drops plus the user's `add`, de-duplicated first-wins
//! — so `vibe app extras list SLOT --no-extras` is an honest preview of
//! `vibe app extras apply SLOT --no-extras` and not a second opinion.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS");

use specmark::spec;
use vibe_core::manifest::{EcosystemDecl, EcosystemVerb};
use vibe_wire::generated::application_extras as wire;

use crate::error::AppError;
use crate::operations::OperationContext;
use crate::resolve::ResolvedPackage;

use super::extras::{self, ExtrasPolicy, UserExtras};
use super::{EcosystemRequest, Target, ecosystem_run};

/// The in-document protocol tag of the extras listing.
pub const EXTRAS_PROTOCOL: &str = "vibe-app-extras/1";

/// The listing of one slot's extras (PROP-075 `#SLOT-EXTRAS`).
///
/// `version` names the installation, or `None` takes the one the slot's
/// default points at; `ecosystem` narrows the listing to one adapter.
/// With `observe`, each adapter that declares a `list` verb runs it and
/// the listing carries what it saw.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
pub fn listing(
    ctx: &OperationContext<'_>,
    slot: &str,
    version: Option<&str>,
    ecosystem: Option<&str>,
    observe: bool,
    policy: ExtrasPolicy,
) -> Result<wire::ApplicationExtras, AppError> {
    let index = crate::index::Index::load(ctx.settings)?;
    let (key, resolved_version) = super::target(&index, slot, version)?;
    let chosen = index.installations[&key].features.clone();
    let resolver = ctx
        .resolver
        .ok_or_else(|| crate::error::StepError::EcosystemUnresolvable {
            slot: slot.to_owned(),
            reason: "this command cannot resolve application packages".to_owned(),
        })?;
    let package =
        resolver(&key).map_err(|error| crate::error::StepError::EcosystemUnresolvable {
            slot: slot.to_owned(),
            reason: error.to_string(),
        })?;
    let adapters = super::selected(
        &package,
        &Target {
            slot,
            version,
            ecosystem,
            // Every adapter declares `install`, so the selection refuses
            // only an `--ecosystem` the package does not declare.
            verb: EcosystemVerb::Install,
        },
    )?;
    let mut user = extras::user_extras(ctx.user_config, &index, slot);
    user.notices.extend(extras::stale_notices(&package, &user));
    let recorded = index.installations[&key].extras.clone();
    let mut document = wire::ApplicationExtras {
        protocol: EXTRAS_PROTOCOL.to_owned(),
        slot: slot.to_owned(),
        key: key.clone(),
        version: resolved_version,
        ecosystems: Vec::new(),
        notices: user.notices.clone(),
    };
    for adapter in adapters {
        let mut row = describe(&package, adapter, &chosen, &user, policy);
        // What the last apply put there, from the installation's own
        // record — the live observation below is a second, better answer
        // when the adapter can be asked (`AV-FMT-029`).
        if let Some(kept) = recorded.iter().find(|kept| kept.id == adapter.id) {
            row.applied = kept.applied.clone();
            row.applied_at = kept.applied_at.clone();
        }
        if observe && adapter.verb(EcosystemVerb::List).is_some() {
            match ecosystem_run(
                ctx,
                &EcosystemRequest {
                    slot,
                    version: Some(&key),
                    ecosystem: Some(&adapter.id),
                    verb: EcosystemVerb::List,
                    items: &[],
                },
            ) {
                Ok(run) => match run.adapters.first() {
                    Some(observed)
                        if observed.outcome
                            == vibe_wire::generated::application_ecosystem::Outcome::Done =>
                    {
                        row.observed = observed.observed.clone();
                    }
                    Some(observed) => row.observed_failed = observed.reason.clone(),
                    None => {}
                },
                Err(error) => row.observed_failed = Some(error.to_string()),
            }
        }
        document.ecosystems.push(row);
    }
    Ok(document)
}

/// One adapter's row: its verbs, the author's recommendations with their
/// enabled and dropped state, the user's own `add` and `drop`, and the
/// final list the arithmetic of `#SLOT-EXTRAS` makes of all of it.
///
/// The recommendations are the rows **routed to** this adapter by their
/// `ecosystem` field, which is what decides where an item goes
/// (`#P7B-AS-BUILT` (3)) — reading `EcosystemDecl::recommends` would send
/// a row authored in one table and addressed to another to the wrong
/// `install` verb.
pub(super) fn describe(
    package: &ResolvedPackage,
    adapter: &EcosystemDecl,
    chosen: &[String],
    user: &UserExtras,
    policy: ExtrasPolicy,
) -> wire::Ecosystem {
    let routed = package.application.ecosystem_recommends_of(&adapter.id);
    let own = user.by_ecosystem.get(&adapter.id);
    let dropped = |id: &str| own.is_some_and(|extras| extras.drop.iter().any(|row| row == id));
    let recommendations: Vec<wire::Recommendation> = routed
        .iter()
        .map(|recommend| wire::Recommendation {
            id: recommend.id.clone(),
            name: recommend.feature_name().to_owned(),
            item: recommend.item.clone(),
            default: recommend.default,
            enabled: chosen.iter().any(|id| id == &recommend.id),
            dropped: dropped(&recommend.id),
            reason: recommend.reason.clone(),
        })
        .collect();
    let computed = extras::computed(package, adapter, chosen, user, policy);
    wire::Ecosystem {
        id: adapter.id.clone(),
        name: adapter.name.clone(),
        description: adapter.description.clone(),
        verbs: adapter
            .declared_verbs()
            .into_iter()
            .map(|verb| match verb {
                EcosystemVerb::Install => wire::Verb::Install,
                EcosystemVerb::List => wire::Verb::List,
                EcosystemVerb::Remove => wire::Verb::Remove,
                EcosystemVerb::SelfUpgrade => wire::Verb::SelfUpgrade,
            })
            .collect(),
        recommendations,
        add: computed.add,
        drop: computed.drop,
        items: computed.items,
        applied: Vec::new(),
        applied_at: None,
        observed: Vec::new(),
        observed_failed: None,
    }
}
