//! One lease-bound Tag/Release API call. No CLI or public caller reaches this
//! module; even an approved plan is inert without a sealed ReadyRun.

use crate::github_release::{
    DeleteReleaseResponseEvidence, DraftCallFields, DraftResponseEvidence, GithubReleaseClient,
    TagResponseEvidence,
};
use crate::publication_full_plan::remote_observation::{RemoteReadPort, observe_twice};
use crate::publication_full_plan::session::ReadyRun;

use super::*;

pub(super) trait CallTransport {
    fn create_tag(&self, commit: &str) -> std::result::Result<TagResponseEvidence, String>;
    fn delete_release(
        &self,
        release_id: u64,
    ) -> std::result::Result<DeleteReleaseResponseEvidence, String>;
    fn create_draft(
        &self,
        selected: &DraftCallFields,
    ) -> std::result::Result<DraftResponseEvidence, String>;
}

impl CallTransport for GithubReleaseClient {
    fn create_tag(&self, commit: &str) -> std::result::Result<TagResponseEvidence, String> {
        self.create_fixed_tag_ref_explicit(commit)
            .map_err(|error| error.to_string())
    }

    fn delete_release(
        &self,
        release_id: u64,
    ) -> std::result::Result<DeleteReleaseResponseEvidence, String> {
        self.delete_prior_release_explicit(release_id)
            .map_err(|error| error.to_string())
    }

    fn create_draft(
        &self,
        selected: &DraftCallFields,
    ) -> std::result::Result<DraftResponseEvidence, String> {
        self.create_fixed_draft_explicit(selected)
            .map_err(|error| error.to_string())
    }
}

/// The only production binding: fixed-target bounded observer plus the
/// no-redirect single-call transport. Nothing invokes this from a live route.
pub(super) fn drive_one_fixed(
    session: &ReadyRun<'_>,
    client: &GithubReleaseClient,
) -> Result<CallInspection> {
    let mut reader =
        super::super::github_remote::GithubRemoteReadPort::new_fixed(client).map_err(Error::io)?;
    drive_one(session, &mut reader, client)
}

fn callback_identity(call: &PlannedCall, transport: &dyn CallTransport) -> Option<String> {
    match &call.action {
        CallAction::RetainTag { .. } => Some("retained-read-only".into()),
        CallAction::CreateTag { tag, commit } => {
            let response = transport.create_tag(commit).ok()?;
            (response.reference == format!("refs/tags/{tag}") && response.object_sha == *commit)
                .then(|| format!("tag:{tag}:{commit}"))
        }
        CallAction::DeletePriorRelease { release_id, .. } => {
            let id = release_id.parse::<u64>().ok().filter(|id| *id != 0)?;
            let response = transport.delete_release(id).ok()?;
            (response.release_id == id).then(|| format!("deleted:{id}"))
        }
        CallAction::CreateDraft {
            tag,
            target_commit,
            name,
            body,
            prerelease,
        } => {
            let selected = DraftCallFields {
                tag: tag.clone(),
                target_commit: target_commit.clone(),
                name: name.clone(),
                body: body.clone(),
                draft: true,
                prerelease: *prerelease,
            };
            let response = transport.create_draft(&selected).ok()?;
            (response.release_id != 0
                && response.tag == *tag
                && response.target_commitish == *target_commit
                && response.draft
                && response.name == *name
                && response.body == *body
                && response.prerelease == *prerelease)
                .then(|| response.release_id.to_string())
        }
    }
}

/// Exactly one call. A missing or damaged callback after Pending is uncertain,
/// even if a later observer sees the desired bytes; it never retries or
/// advances to a subsequent call on that evidence alone.
pub(super) fn drive_one(
    session: &ReadyRun<'_>,
    reader: &mut dyn RemoteReadPort,
    transport: &dyn CallTransport,
) -> Result<CallInspection> {
    let ordinal = match inspect_calls_proved(session)? {
        CallInspection::Next { ordinal } => ordinal,
        other => return Ok(other),
    };
    let schedule = decompose(session.subject, session.plan, session.expected)?;
    let Some(call) = schedule.calls.get(ordinal) else {
        return Ok(CallInspection::Unavailable {
            landed: ordinal,
            reason: schedule.unavailable_after_prefix,
        });
    };
    let observed = observe_twice(reader, &session.subject.source.commit).map_err(Error::io)?;
    let before = CallObservation {
        generation_ref: observed.generation_ref,
        tag: observed.tag,
        release: observed.release,
        latest_release: observed.observed_latest_release,
    };
    let prior_after = if ordinal == 0 {
        None
    } else {
        let prior = journal::read::<Landed>(
            session.lease,
            &session.run_id,
            &suffix(ordinal - 1, "landed"),
        )?
        .ok_or_else(|| Error::Refused("prior call Landed record vanished".into()))?;
        Some(prior.after)
    };
    if before.latest_release != call.expected_latest_release
        || before.tag != call.expected_tag
        || before.release != call.expected_release
        || ordinal == 0
            && (before.generation_ref != session.plan.remote_generation_ref
                || before.tag != session.plan.observed_tag
                || before.release != session.plan.observed_release
                || before.latest_release != session.plan.observed_latest_release)
        || prior_after.as_ref().is_some_and(|prior| prior != &before)
    {
        return Err(Error::Refused(format!(
            "call {ordinal} remote precondition changed before Pending"
        )));
    }
    session.recheck_small()?;
    let pending = Pending {
        format: PENDING_FORMAT.into(),
        run_id: session.run_id.clone(),
        call: call.clone(),
        before: before.clone(),
        expected_observation_sha256: sha(&serde_json::to_vec(&before)?),
    };
    if journal::publish(
        session.lease,
        &session.run_id,
        &suffix(ordinal, "pending"),
        &pending,
    )
    .is_err()
    {
        return Ok(CallInspection::Uncertain { ordinal });
    }
    // Pending is durable. Every later refusal, including a lost response or
    // local proof drift, now preserves an inspectable uncertain run.
    if session.recheck_small().is_err() {
        return Ok(CallInspection::Uncertain { ordinal });
    }
    let Some(receipt_id) = callback_identity(call, transport) else {
        return Ok(CallInspection::Uncertain { ordinal });
    };
    let observed = match observe_twice(reader, &session.subject.source.commit) {
        Ok(observed) => observed,
        Err(_) => return Ok(CallInspection::Uncertain { ordinal }),
    };
    let after = CallObservation {
        generation_ref: observed.generation_ref,
        tag: observed.tag,
        release: observed.release,
        latest_release: observed.observed_latest_release,
    };
    if !verified_transition(call, &before, &after, &receipt_id) {
        return Ok(CallInspection::Uncertain { ordinal });
    }
    if session.recheck_small().is_err() {
        return Ok(CallInspection::Uncertain { ordinal });
    }
    let landed = Landed {
        format: LANDED_FORMAT.into(),
        run_id: session.run_id.clone(),
        pending_sha256: sha(&serde_json::to_vec(&pending)?),
        receipt_id,
        after_observation_sha256: sha(&serde_json::to_vec(&after)?),
        after,
    };
    if journal::publish(
        session.lease,
        &session.run_id,
        &suffix(ordinal, "landed"),
        &landed,
    )
    .is_err()
    {
        return Ok(CallInspection::Uncertain { ordinal });
    }
    inspect_calls_proved(session)
}

#[cfg(test)]
#[path = "driver_tests.rs"]
mod tests;
