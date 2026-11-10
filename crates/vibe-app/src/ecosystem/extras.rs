//! The user's own per-slot extras list, the final-list arithmetic over it,
//! and the apply that puts that list into a newly installed version
//! (PROP-075 `#SLOT-EXTRAS`).
//!
//! **The arithmetic, in one place, in this order**: the items of the author
//! recommendations that are in the installation's chosen feature set, in
//! declaration order; minus every recommendation the user dropped; plus the
//! user's `add` items, in their own order; de-duplicated by exact string
//! with the first occurrence winning.
//!
//! **Two flags, and they are not the same flag.** `--no-recommends` skips
//! the *author's* recommendations; `--no-extras` skips the user's own `add`
//! list. A `drop` is honoured in **every** case, `--no-extras` included: a
//! flag that says "do not install my extras" must never cause *more* to be
//! installed than the user asked for, and treating a withdrawal of consent
//! as part of the skipped list would do exactly that. With both flags, or
//! with an empty final list, the adapter is not run at all.
//!
//! **A stale line is reported, never fatal.** An unknown slot, an unknown
//! ecosystem id or an unknown recommendation id is reported once and
//! otherwise ignored: a settings file the user wrote by hand outlives the
//! packages it mentions, and refusing an install over one of its lines
//! would be the wrong trade.
//!
//! **Failure never touches the runtime.** A failing `install` verb leaves
//! the new version installed, its slot default exactly where the user put
//! it and the installation record written; the failure is a warning naming
//! the retry command, and it is kept in the installation's adapter record
//! so a later `vibe app describe` still says what was not carried.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS");

use std::collections::BTreeMap;

use specmark::spec;
use vibe_core::manifest::{EcosystemDecl, EcosystemRecommendDecl, EcosystemVerb};
use vibe_core::user_config::{SlotExtras, UserConfig};
use vibe_wire::generated::application_ecosystem as wire;

use crate::error::AppError;
use crate::index::{ExtrasRecord, ExtrasResult, Index};
use crate::operations::OperationContext;

use crate::resolve::ResolvedPackage;

/// Which of the two lists an invocation declined (PROP-075
/// `#SLOT-EXTRAS`). The default carries both, which is what an
/// `install -g` the user typed does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
pub struct ExtrasPolicy {
    /// `--no-recommends`: skip the **author's** recommended items. One
    /// idea with two spellings — the vibe packages of PROP-077
    /// `#APP-RECOMMENDS` and the ecosystem items here — so a user who
    /// declines it declines both, which is why the flag is global and is
    /// not redefined for this plane.
    pub no_recommends: bool,
    /// `--no-extras`: skip the user's own `add` list. Never their `drop`
    /// list.
    pub no_extras: bool,
}

/// What this user's settings say about one slot, and what reading them had
/// to report. Not public API: the arithmetic is, the shape of the read is
/// not.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct UserExtras {
    /// `[slot.<slot>.extras.<ecosystem>]` for this slot, by ecosystem id.
    pub(super) by_ecosystem: BTreeMap<String, SlotExtras>,
    /// What was reported once and otherwise ignored.
    pub(super) notices: Vec<String>,
}

/// Read `[slot.<slot>.extras.*]` out of the user's own `config.toml`
/// (PROP-075 `#SLOT-EXTRAS`), the **one** read of `UserConfig` this crate
/// performs.
///
/// The path comes from the caller, because this crate never resolves the
/// environment itself; with no path the answer is empty, which is exactly
/// how a home with no `[slot]` table behaves. A file that will not parse
/// is a notice and an empty list, never an error: the extras list must not
/// be able to fail an install.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
pub(super) fn user_extras(file: Option<&std::path::Path>, index: &Index, slot: &str) -> UserExtras {
    let Some(path) = file else {
        return UserExtras::default();
    };
    let config = match UserConfig::load_from(path) {
        Ok(config) => config,
        Err(error) => {
            return UserExtras {
                notices: vec![format!(
                    "`{}` could not be read, so no extras list was taken from it: {error}",
                    path.display()
                )],
                ..UserExtras::default()
            };
        }
    };
    let mut answer = UserExtras::default();
    // Every `[slot.X]` the file names, so a slot nothing installs is
    // reported once — here, and not only when that slot is the one being
    // installed, because the user's typo is worth naming at all.
    for named in config.slot.keys() {
        if !index.installations.values().any(|row| &row.slot == named) {
            answer.notices.push(format!(
                "`{}` names the slot `{named}`, which nothing installs; its extras are ignored",
                path.display()
            ));
        }
    }
    if let Some(table) = config.slot.get(slot) {
        answer.by_ecosystem = table.extras.clone();
    }
    answer
}

/// The notices the slot's own tables earn against the package that
/// declares the adapters: an ecosystem id no adapter answers to, and a
/// `drop` naming no recommendation of that adapter. Reported once and
/// otherwise ignored.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
pub(super) fn stale_notices(package: &ResolvedPackage, user: &UserExtras) -> Vec<String> {
    let mut notices = Vec::new();
    for (id, extras) in &user.by_ecosystem {
        if package.application.ecosystem_adapter(id).is_none() {
            notices.push(format!(
                "the ecosystem `{id}` is not one `{}` declares; its extras are ignored",
                package.package.key()
            ));
            continue;
        }
        let declared = package.application.ecosystem_recommends_of(id);
        for dropped in &extras.drop {
            if !declared.iter().any(|row| &row.id == dropped) {
                notices.push(format!(
                    "the ecosystem `{id}` of `{}` recommends nothing called `{dropped}`; that \
                     drop is ignored",
                    package.package.key()
                ));
            }
        }
    }
    notices
}

/// The final list for one adapter (PROP-075 `#SLOT-EXTRAS`), the whole
/// arithmetic in one pure function.
///
/// `recommends` are the rows **routed to** this adapter by their
/// `ecosystem` field; `chosen` is the feature set the installation
/// recorded, because a recommendation is shorthand for one ordinary
/// installer feature and nothing else.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
pub fn final_list(
    recommends: &[&EcosystemRecommendDecl],
    chosen: &[String],
    user: Option<&SlotExtras>,
    policy: ExtrasPolicy,
) -> Vec<String> {
    let dropped = |id: &str| user.is_some_and(|extras| extras.drop.iter().any(|row| row == id));
    let mut items: Vec<String> = Vec::new();
    let mut push = |item: &String| {
        if !items.contains(item) {
            items.push(item.clone());
        }
    };
    if !policy.no_recommends {
        for row in recommends {
            // A drop is honoured in every case, this one included: it is a
            // withdrawal of consent, not a member of a skipped list.
            if chosen.iter().any(|id| id == &row.id) && !dropped(&row.id) {
                push(&row.item);
            }
        }
    }
    if !policy.no_extras {
        for item in user.map(|extras| extras.add.as_slice()).unwrap_or_default() {
            push(item);
        }
    }
    items
}

/// What one adapter would be handed, and the notices the computation
/// earned: the shape both `vibe app extras list` and the apply read.
pub(super) struct Computed {
    pub(super) items: Vec<String>,
    pub(super) add: Vec<String>,
    pub(super) drop: Vec<String>,
}

/// [`final_list`] for one declared adapter of one installation.
pub(super) fn computed(
    package: &ResolvedPackage,
    adapter: &EcosystemDecl,
    chosen: &[String],
    user: &UserExtras,
    policy: ExtrasPolicy,
) -> Computed {
    let recommends = package.application.ecosystem_recommends_of(&adapter.id);
    let own = user.by_ecosystem.get(&adapter.id);
    Computed {
        items: final_list(&recommends, chosen, own, policy),
        add: own.map(|extras| extras.add.clone()).unwrap_or_default(),
        drop: own.map(|extras| extras.drop.clone()).unwrap_or_default(),
    }
}

/// What `vibe app extras apply SLOT` — and an `install -g` that placed a
/// new version of a slot — runs.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
pub struct ExtrasApply<'a> {
    pub slot: &'a str,
    /// The version or the whole installation key; `None` is the version
    /// the slot's default points at. The apply after an install names the
    /// key it just placed, because the slot's default is the user's to
    /// move and an install need not have moved it.
    pub version: Option<&'a str>,
    /// One adapter by `id`; `None` is every adapter the package declares.
    pub ecosystem: Option<&'a str>,
    pub policy: ExtrasPolicy,
}

/// Compute each declared adapter's final list and hand it to that
/// adapter's `install` verb (PROP-075 `#SLOT-EXTRAS`).
///
/// Nothing of the runtime is ever rolled back: every adapter runs
/// independently, a failure is recorded and reported with its retry
/// command, and the answer's `ok` is the only thing a caller may turn into
/// a warning — never into an exit status of the install the user asked
/// for.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
pub fn extras_apply(
    ctx: &OperationContext<'_>,
    request: &ExtrasApply<'_>,
) -> Result<wire::ApplicationEcosystem, AppError> {
    super::run_all(
        ctx,
        &super::Target {
            slot: request.slot,
            version: request.version,
            ecosystem: request.ecosystem,
            verb: EcosystemVerb::Install,
        },
        super::Items::Computed(request.policy),
    )
}

/// Fold one run's adapter rows into the installation's extras records
/// (PROP-075 `#SLOT-EXTRAS`), newest wins, one record per adapter id.
///
/// A `skipped` adapter writes nothing: it did no work, and a record that
/// retains nothing would push the home onto the `/8` reader for no fact.
pub(super) fn remember(
    records: &mut Vec<ExtrasRecord>,
    document: &wire::ApplicationEcosystem,
) -> bool {
    let mut changed = false;
    for row in &document.adapters {
        if row.outcome == wire::Outcome::Skipped {
            continue;
        }
        let outcome = match row.outcome {
            wire::Outcome::Done => ExtrasResult::Done,
            wire::Outcome::Failed => ExtrasResult::Failed,
            wire::Outcome::Skipped => ExtrasResult::Skipped,
        };
        let at = row.finished.clone().or_else(|| row.started.clone());
        if !records.iter().any(|kept| kept.id == row.id) {
            records.push(ExtrasRecord::new(&row.id));
        }
        let Some(record) = records.iter_mut().find(|kept| kept.id == row.id) else {
            continue;
        };
        match document.verb {
            wire::Verb::List => {
                // A failed observation leaves the previous list standing
                // and names itself, because a lost observation costs one
                // re-run and never a wrong answer.
                if outcome == ExtrasResult::Done {
                    record.observed = row.observed.clone();
                }
                record.observed_at = at;
                record.observed_outcome = Some(outcome);
            }
            wire::Verb::Install => {
                record.applied = row.items.clone();
                record.applied_at = at;
                record.applied_outcome = Some(outcome);
            }
            // `remove` and `self_upgrade` change what the version carries
            // without being an apply, so the *applied* list is untouched
            // and only a fresh `list` can say what is there now.
            wire::Verb::Remove | wire::Verb::SelfUpgrade => continue,
        }
        changed = true;
    }
    changed
}
