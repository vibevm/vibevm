use vibe_spec::CompilerInvocationReceipts;

use crate::{WorkspaceError, boot_artifacts};

use super::{DirectPending, ReceiptIdentity, ReplayLane, replay_error};

pub(super) fn validate_ready(
    lane: &mut ReplayLane,
    continuation: Option<&boot_artifacts::OwnerNativeCompileContinuation>,
) -> Result<(), WorkspaceError> {
    if !lane.candidate.native() {
        return if continuation.is_none() {
            Ok(())
        } else {
            Err(replay_error(
                &lane.candidate.owner().to_string(),
                "builtin replay produced a native continuation",
            ))
        };
    }
    let Some(boot_artifacts::OwnerNativeCompileContinuation::Ready { receipts }) = continuation
    else {
        return Err(replay_error(
            &lane.candidate.owner().to_string(),
            "native replay did not produce Ready",
        ));
    };
    match &lane.direct {
        Some(direct) => validate_receipts(&direct.expected, receipts)
            .map_err(|reason| replay_error(&lane.candidate.owner().to_string(), reason)),
        None if receipts.is_empty() => Ok(()),
        None => Err(replay_error(
            &lane.candidate.owner().to_string(),
            "Fail replay produced invocation receipts",
        )),
    }?;
    if let Some(direct) = lane.direct.take() {
        consume_validated_direct(lane, direct)?;
    }
    Ok(())
}

fn consume_validated_direct(
    lane: &ReplayLane,
    direct: DirectPending,
) -> Result<(), WorkspaceError> {
    if direct.pending.is_some() {
        return Err(replay_error(
            &lane.candidate.owner().to_string(),
            "resolved pending set survived policy construction",
        ));
    }
    let evidence = direct.evidence.ok_or_else(|| {
        replay_error(
            &lane.candidate.owner().to_string(),
            "pending evidence visited twice",
        )
    })?;
    drop(evidence);
    Ok(())
}

fn validate_receipts(
    expected: &[ReceiptIdentity],
    receipts: &CompilerInvocationReceipts,
) -> Result<(), &'static str> {
    if receipts.len() != expected.len() {
        return Err("Resolve receipt length differs from the pending snapshot");
    }
    for (expected, (actual, count)) in expected.iter().zip(receipts.iter()) {
        if count == 0
            || expected.plan_digest != *actual.plan_digest_bytes()
            || expected.order != actual.order()
            || expected.key != *actual.key()
        {
            return Err("Resolve receipt identity/count differs from the pending snapshot");
        }
    }
    Ok(())
}
