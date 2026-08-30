//! The standalone skill's ENTRY DESTINATION — identity, observation,
//! publication and pruning: the filesystem half of the provider in the
//! parent cell.
//!
//! Its own file for the reason the packet names — "split provider,
//! filesystem/config and tests by responsibility" — and the split is a
//! real seam, not a shard: every law here is about ONE entry file under
//! the injected home (what it is called, how it is observed, how exact
//! bytes reach it, what removal may prune), while the parent cell owns
//! admission, identity and the six protocol verbs.
//!
//! Two laws live here and nowhere else:
//!
//! 1. **the resource identity is one forward-slashed home-relative
//!    member** (`home:.claude/skills/<name>/SKILL.md`, §6.3.0.9's owned
//!    and locked vocabulary for a user-scope destination), and the
//!    ABSOLUTE destination is named only by the PURE injected-home helper
//!    — never a join this cell invented;
//! 2. **the occupant judgement** (§6.3.1.1): a present entry is
//!    updateable only when the injected receipt owns the exact resource
//!    at the digest it recorded; no receipt, a receipt that owns another
//!    resource, a rolled-back/empty receipt or any digest mismatch never
//!    authorizes an identical-looking occupant. `plan` and `apply` run the
//!    SAME function, so a plan can only ever report a refusal apply
//!    would raise — with ONE precisely-scoped addition at plan time: the
//!    injected durable intent of §7.2's crash window may prove a present
//!    entry is this deployment's own INTERRUPTED write (recovery
//!    occupancy, never ownership), so the next ordinary run can reach the
//!    settlement that completes it. The general recovery law covers a
//!    crash after publishing an UPDATE exactly as it covers one after a
//!    first deployment: in both windows the receipt does not describe the
//!    observed bytes, and only the intent's exact evidence — resource,
//!    desired digest, and a `prior_generation` that agrees with the
//!    injected receipt state — tells the interrupted write from drift.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::path::{Path, PathBuf};

use vibe_wire::generated::deploy_intent::DeployIntent;
use vibe_wire::generated::deploy_receipt::DeployReceipt;

use crate::mechanism::contain::{checked_relative, digest_file, relative_to};
use crate::mechanism::deploy::skill::SkillDeployProvider;
use crate::mechanism::deploy::state::CheckpointLedger;
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::vibebin::store;
use crate::mechanism::{DeployTargetRequest, MechanismError};

use super::client::{ENTRY_DOCUMENT, SkillClient};
use super::config::SkillDeployConfig;
use super::{Destination, HOME_PREFIX, Occupancy};

impl SkillDeployProvider {
    /// §6.3.1's occupant judgement — the one law both `plan` and `apply`
    /// run, unchanged, so a plan can only ever report a refusal apply
    /// would raise.
    ///
    /// It consults the INJECTED receipt, never engine state: the receipt
    /// either owns the exact resource at the digest it recorded (an
    /// update may run), or nothing authorizes the occupant — including an
    /// occupant holding byte-identical content, which is the exact
    /// confusion a receipt exists to end.
    ///
    /// RECOVERY INTENTS ARE DELIBERATELY INVISIBLE HERE: this is the
    /// apply-time judgement, and its recheck stays receipt-only. A crash
    /// after atomic publication is settled by the engine's transaction —
    /// which decides reachability by PLAN HASH, not by this gate — so an
    /// intent this function consulted would be plan evidence turned into
    /// write authority, exactly what the frozen law forbids.
    pub(super) fn admit_occupant(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
    ) -> Result<Occupancy, DeployProviderError> {
        self.occupancy_under(request, destination, None)
    }

    /// The plan-time occupant judgement: the same law as
    /// [`Self::admit_occupant`], plus §7.2's interrupted windows.
    ///
    /// A crash after atomic publication leaves this deployment's desired
    /// bytes at the destination and an unretired intent beside them —
    /// and the receipt, wherever one exists, does not describe those
    /// bytes: a stranded FIRST deployment has no receipt anywhere, and a
    /// stranded UPDATE still holds the PRIOR generation's receipt at the
    /// prior generation's digest. The strict law would refuse both at
    /// plan time — and the next ordinary run could then never reach the
    /// `recover` that exists to complete them. So the PLAN judgement
    /// alone admits one extra case, in either shape: the occupant passes
    /// as [`Occupancy::Interrupted`] or [`Occupancy::InterruptedUpdate`]
    /// solely when the injected intent carries the exact evidence of
    /// [`interrupted_under`].
    ///
    /// This is settlement-reachability evidence, never ownership:
    ///
    /// - a receipt that owns the entry at its recorded digest answers
    ///   first, unchanged — an ordinary update never consults the intent;
    /// - a stale intent (another plan's, a digest that no longer matches,
    ///   or a `prior_generation` that disagrees with the injected
    ///   receipt state) grants nothing and the strict refusal stands;
    /// - `apply` runs [`Self::admit_occupant`] under the locks, which
    ///   never sees an intent, so plan evidence cannot become write
    ///   authority even if a caller injected it wrongly.
    pub(super) fn plan_occupancy(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
    ) -> Result<Occupancy, DeployProviderError> {
        self.occupancy_under(request, destination, request.recovery_intent)
    }

    /// The one occupant law, parameterised only by whether §7.2's
    /// interrupted window is in evidence.
    fn occupancy_under(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
        recovery_intent: Option<&DeployIntent>,
    ) -> Result<Occupancy, DeployProviderError> {
        let target = &request.target.id;
        let Some(observed) = self.observe(request, &destination.resource)? else {
            return Ok(Occupancy::Absent);
        };
        let owned = request.prior_receipt.and_then(|receipt| {
            receipt
                .resources
                .iter()
                .find(|owned| owned.resource == destination.resource)
                .map(|owned| (receipt, owned))
        });
        // Exact receipt ownership answers FIRST, unchanged: a receipt
        // that owns the entry at the digest it recorded is an ordinary
        // update, whatever any journal beside it claims.
        if let Some((_, owned)) = owned.as_ref()
            && owned.post_digest == observed
        {
            return Ok(Occupancy::Owned);
        }
        // The interrupted windows, and ONLY at plan time: exact intent
        // evidence may recognise this deployment's own stranded write
        // BEFORE the ordinary refusal — whether the receipt below it
        // drifted (an interrupted UPDATE; the receipt still records the
        // prior generation's digest) or no receipt owns the entry at all
        // (an interrupted FIRST deployment). Anything less than that
        // exact evidence falls through to the unchanged refusals below.
        if let Some(occupancy) = interrupted_under(
            recovery_intent,
            request.prior_receipt,
            destination,
            &observed,
        ) {
            return Ok(occupancy);
        }
        if let Some((_, owned)) = owned {
            return Err(DeployProviderError::OccupantDrifted {
                target: target.clone(),
                resource: destination.resource.clone(),
                recorded: owned.post_digest.clone(),
                observed,
            });
        }
        Err(DeployProviderError::OccupantUnowned {
            target: target.clone(),
            resource: destination.resource.clone(),
            observed,
        })
    }

    /// Observe one owned resource's digest, or `None` when nothing is
    /// there — absence is a value, exactly as the protocol states.
    pub(super) fn observe(
        &self,
        request: &DeployTargetRequest<'_>,
        resource: &str,
    ) -> Result<Option<String>, DeployProviderError> {
        let path = self.path_of(request, resource)?;
        match digest_file(&path) {
            Ok((digest, _)) => Ok(Some(digest)),
            Err(crate::mechanism::contain::FileFault::Missing(_)) => Ok(None),
            Err(fault) => Err(DeployProviderError::Observe {
                target: request.target.id.clone(),
                resource: resource.to_owned(),
                reason: fault.reason(),
            }),
        }
    }

    /// One recorded resource identity, proven to name a place inside the
    /// INJECTED home before it is joined to it.
    pub(super) fn path_of(
        &self,
        request: &DeployTargetRequest<'_>,
        resource: &str,
    ) -> Result<PathBuf, DeployProviderError> {
        let refuse = |reason: String| DeployProviderError::Observe {
            target: request.target.id.clone(),
            resource: resource.to_owned(),
            reason,
        };
        let tail = resource
            .strip_prefix(HOME_PREFIX)
            .ok_or_else(|| refuse(format!("it does not carry the `{HOME_PREFIX}` root")))?
            .to_owned();
        let relative = checked_relative(&tail)
            .map_err(|fault| refuse(format!("`{tail}` is unusable: {}", fault.reason())))?;
        Ok(store::join(request.user_home, &relative))
    }

    /// Prove that the pure Agent helper and the provider's owned/locked
    /// resource identity name the same entry below the injected home.
    ///
    /// The two spellings serve different consumers — an absolute path for
    /// filesystem publication and a forward-slashed string for locks and
    /// receipts — so agreement is checked rather than assumed. A future
    /// client-map drift therefore refuses during `plan`; it can never publish
    /// one path while recording ownership of another.
    pub(super) fn exact_entry_relative(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
    ) -> Result<String, DeployProviderError> {
        let entry = entry_of(self.client, destination, request.user_home);
        let relative =
            relative_to(&entry, request.user_home).ok_or_else(|| DeployProviderError::Observe {
                target: request.target.id.clone(),
                resource: destination.resource.clone(),
                reason: format!(
                    "the pure {} skill helper resolved `{}` outside the injected user home `{}`",
                    self.client.as_str(),
                    entry.display(),
                    request.user_home.display(),
                ),
            })?;
        if relative != destination.relative {
            return Err(DeployProviderError::Observe {
                target: request.target.id.clone(),
                resource: destination.resource.clone(),
                reason: format!(
                    "the pure {} skill helper names `{relative}`, but the owned/locked resource identity names `{}`",
                    self.client.as_str(),
                    destination.relative,
                ),
            });
        }
        Ok(relative)
    }

    /// Publish the exact bytes atomically and checkpoint the entry.
    ///
    /// The ABSOLUTE destination is named by the PURE helper (§6.3.1.7),
    /// then proven to sit under the injected home before the audited
    /// staged-rename primitive takes it — so the client root a
    /// destination really lands under is the helper's answer, never a
    /// join this cell invented.
    pub(super) fn publish(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
        bytes: &[u8],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<(), MechanismError> {
        let relative = self.exact_entry_relative(request, destination)?;
        store::place_resource(
            &request.target.id,
            request.user_home,
            request.staging,
            &relative,
            bytes,
            false,
        )?;
        checkpoint.completed(&destination.resource)
    }

    /// Prune only directories proven empty, stopping no later than the
    /// named skill directory — the skills root and every ancestor above
    /// it are unrecorded by this deployment and survive byte-exact.
    ///
    /// The skills root itself is deliberately never pruned (§6.3.1.6's
    /// boundary, read conservatively): another skill may claim it the
    /// moment this one leaves, and an empty-but-present root is a cheaper
    /// state to reverse than a deleted one.
    pub(super) fn prune(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
    ) -> Result<(), MechanismError> {
        let Some(named) = self
            .client
            .agent()
            .user_skills_root_from_home(request.user_home)
            .map(|root| root.join(&destination.name))
        else {
            return Ok(());
        };
        if std::fs::read_dir(&named).is_ok_and(|entries| entries.flatten().count() == 0) {
            // `remove_dir` (never `remove_dir_all`): it refuses a
            // non-empty directory by itself, which is the proven-empty
            // law as a primitive rather than as a check-then-act.
            std::fs::remove_dir(&named).map_err(|error| DeployProviderError::Write {
                target: request.target.id.clone(),
                path: destination.relative.clone(),
                reason: error.to_string(),
            })?;
        }
        Ok(())
    }
}

/// The narrow plan-time predicate for §7.2's interrupted occupant —
/// reachability evidence, never ownership.
///
/// Every clause is exact, and all three must hold:
///
/// - the intent names the EXACT resource;
/// - its recorded desired digest equals the digest INDEPENDENTLY observed
///   there now — the stranded write this run would settle, not an entry
///   that merely resembles one;
/// - its `prior_generation` AGREES with the injected receipt state: a
///   journal opened against no receipt claims `None`, and one opened over
///   a receipt claims that receipt's generation. A journal that disagrees
///   was opened against a different ownership history than the one now
///   injected, so it describes somebody else's crash and masks nothing.
///
/// The answer names WHICH window the evidence proves, because the two
/// settle with different honest reversibility answers: a stranded first
/// deployment is undone by removal, while a stranded update superseded
/// prior bytes no record can restore.
fn interrupted_under(
    intent: Option<&DeployIntent>,
    prior_receipt: Option<&DeployReceipt>,
    destination: &Destination,
    observed: &str,
) -> Option<Occupancy> {
    let intent = intent?;
    let planned = intent
        .resources
        .iter()
        .find(|planned| planned.resource == destination.resource)?;
    if planned.desired_digest != observed {
        return None;
    }
    if intent.prior_generation != prior_receipt.map(|receipt| receipt.generation) {
        return None;
    }
    Some(if prior_receipt.is_some() {
        Occupancy::InterruptedUpdate
    } else {
        Occupancy::Interrupted
    })
}

/// The ONE Agent Skills frontmatter reader, reached through the producer's
/// own module — returning the validated identity this provider's config
/// must agree with.
pub(super) fn frontmatter_of(
    target: &str,
    artifact: &str,
    document: &str,
) -> Result<String, DeployProviderError> {
    crate::mechanism::skill::frontmatter::parse(target, document)
        .map(|parsed| parsed.name)
        .map_err(|error| DeployProviderError::SkillUnreadable {
            target: target.to_owned(),
            artifact: artifact.to_owned(),
            reason: error.to_string(),
        })
}

/// Resolve one target's destination from its config alone.
///
/// Pure: no filesystem, no ambient state — the identity is a function of
/// the closed client vocabulary and the validated name.
pub(super) fn destination_of(client: SkillClient, config: &SkillDeployConfig) -> Destination {
    let under = format!(
        "{}/{}/{}",
        client.skills_relative(),
        config.name,
        ENTRY_DOCUMENT
    );
    Destination {
        name: config.name.clone(),
        resource: format!("{HOME_PREFIX}{under}"),
        relative: under,
    }
}

/// The absolute entry path, from the pure helper only.
pub(super) fn entry_of(
    client: SkillClient,
    destination: &Destination,
    user_home: &Path,
) -> PathBuf {
    client
        .agent()
        .user_skill_entry_from_home(user_home, &destination.name)
        .unwrap_or_else(|| {
            // Unreachable: every SkillClient maps to a skill-loading
            // agent, and the client vocabulary is closed. A panic here
            // would be a vocabulary defect, not a destination fault.
            panic!(
                "the {} skill client resolves a user skills root",
                client.as_str(),
            );
        })
}
