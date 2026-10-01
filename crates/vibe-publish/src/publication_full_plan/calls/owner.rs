//! One private publication-owner step from an already sealed ReadyRun.
//! Selection and a single dispatch are bounded; there is no loop or public
//! route, and uncertainty never falls through to another effect.

use crate::github_release::GithubReleaseClient;
use crate::publication_full_plan::github_remote::GithubRemoteReadPort;
use crate::publication_full_plan::remote_observation::RemoteReadPort;
use crate::publication_full_plan::session::ReadyRun;

use super::super::{Error, Result};
use super::asset_driver::{self, AssetCallTransport};
use super::asset_history::{self, AssetInspection};
use super::driver::{self, CallTransport};
use super::noop_completion::{self, NoOpInspection};
use super::terminal::{self, TerminalInspection};
use super::{CallInspection, inspect_calls_proved};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    TagRelease,
    Asset,
    Checkpoint,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CallRef {
    pub phase: Phase,
    pub ordinal: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum OwnerStep {
    Ready {
        run_id: String,
        next: CallRef,
    },
    Advanced {
        run_id: String,
        attempted: CallRef,
        next: CallRef,
    },
    Uncertain {
        run_id: String,
        at: CallRef,
        attempted: Option<CallRef>,
    },
    Unavailable {
        run_id: String,
        phase: Phase,
        ordinal: Option<usize>,
        attempted: Option<CallRef>,
        reason: &'static str,
    },
    Completed {
        run_id: String,
        calls: usize,
        sealed_now: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Cursor {
    Next(CallRef),
    Uncertain(CallRef),
    Unavailable(Phase, &'static str),
    Completed(usize),
}

fn call(phase: Phase, ordinal: usize) -> CallRef {
    CallRef { phase, ordinal }
}

fn select(session: &ReadyRun<'_>) -> Result<Cursor> {
    session.recheck_small()?;
    match inspect_calls_proved(session)? {
        CallInspection::Next { ordinal } => {
            return Ok(Cursor::Next(call(Phase::TagRelease, ordinal)));
        }
        CallInspection::Uncertain { ordinal } => {
            return Ok(Cursor::Uncertain(call(Phase::TagRelease, ordinal)));
        }
        CallInspection::Unavailable { .. } => {}
    }
    match asset_history::inspect_assets_proved(session)? {
        AssetInspection::Next { call: selected, .. } => {
            return Ok(Cursor::Next(call(Phase::Asset, selected.ordinal)));
        }
        AssetInspection::Uncertain { ordinal } => {
            return Ok(Cursor::Uncertain(call(Phase::Asset, ordinal)));
        }
        AssetInspection::Unavailable { landed: 38 } => {}
        AssetInspection::Unavailable { .. } => {
            return Ok(Cursor::Unavailable(
                Phase::Asset,
                "asset call topology is unavailable",
            ));
        }
    }
    match noop_completion::inspect_proved(session)? {
        NoOpInspection::Next { step } => {
            let Some(calls) = noop_completion::schedule(session)? else {
                return Ok(Cursor::Unavailable(
                    Phase::Checkpoint,
                    "read-only checkpoint topology is unavailable",
                ));
            };
            return Ok(Cursor::Next(call(Phase::Checkpoint, calls[step].ordinal)));
        }
        NoOpInspection::Uncertain { step } => {
            let ordinal = noop_completion::schedule(session)?
                .map(|calls| calls[step].ordinal)
                .unwrap_or(step);
            return Ok(Cursor::Uncertain(call(Phase::Checkpoint, ordinal)));
        }
        NoOpInspection::Unavailable => {
            return Ok(Cursor::Unavailable(
                Phase::Checkpoint,
                "cleanup or finalize checkpoint is unavailable",
            ));
        }
        NoOpInspection::CheckpointsLanded => {}
    }
    let terminal_ordinal = noop_completion::schedule(session)?
        .map(|calls| calls[1].ordinal + 1)
        .unwrap_or(0);
    match terminal::inspect_proved(session)? {
        TerminalInspection::Ready { .. } => {
            Ok(Cursor::Next(call(Phase::Terminal, terminal_ordinal)))
        }
        TerminalInspection::Uncertain | TerminalInspection::NotReady => {
            Ok(Cursor::Uncertain(call(Phase::Terminal, terminal_ordinal)))
        }
        TerminalInspection::Completed { calls } => Ok(Cursor::Completed(calls)),
    }
}

fn status(session: &ReadyRun<'_>, cursor: Cursor, attempted: Option<CallRef>) -> OwnerStep {
    let run_id = session.run_id.clone();
    match cursor {
        Cursor::Next(next) => match attempted {
            Some(attempted) => OwnerStep::Advanced {
                run_id,
                attempted,
                next,
            },
            None => OwnerStep::Ready { run_id, next },
        },
        Cursor::Uncertain(at) => OwnerStep::Uncertain {
            run_id,
            at,
            attempted,
        },
        Cursor::Unavailable(phase, reason) => OwnerStep::Unavailable {
            run_id,
            phase,
            ordinal: None,
            attempted,
            reason,
        },
        Cursor::Completed(calls) => OwnerStep::Completed {
            run_id,
            calls,
            sealed_now: attempted.is_some(),
        },
    }
}

fn drive_inner_with_hook(
    session: &ReadyRun<'_>,
    reader: &mut dyn RemoteReadPort,
    calls: &dyn CallTransport,
    assets: &dyn AssetCallTransport,
    after_step: impl FnOnce(),
) -> Result<OwnerStep> {
    let selected = select(session)?;
    let Cursor::Next(attempted) = selected else {
        return Ok(status(session, selected, None));
    };
    let uncertain = || OwnerStep::Uncertain {
        run_id: session.run_id.clone(),
        at: attempted,
        attempted: Some(attempted),
    };
    match attempted.phase {
        Phase::TagRelease => match driver::drive_one(session, reader, calls) {
            Err(_) => return Ok(uncertain()),
            Ok(value) => match value {
                CallInspection::Uncertain { .. } => return Ok(uncertain()),
                CallInspection::Next { .. } | CallInspection::Unavailable { .. } => {}
            },
        },
        Phase::Asset => match asset_driver::drive_one(session, reader, assets) {
            Err(_) => return Ok(uncertain()),
            Ok(value) => match value {
                AssetInspection::Uncertain { .. } => return Ok(uncertain()),
                AssetInspection::Next { .. } | AssetInspection::Unavailable { landed: 38 } => {}
                AssetInspection::Unavailable { .. } => {
                    return Ok(OwnerStep::Unavailable {
                        run_id: session.run_id.clone(),
                        phase: Phase::Asset,
                        ordinal: Some(attempted.ordinal),
                        attempted: Some(attempted),
                        reason: "asset call topology is unavailable",
                    });
                }
            },
        },
        Phase::Checkpoint => match noop_completion::drive_one_read_only(session, reader) {
            Err(_) => return Ok(uncertain()),
            Ok(value) => match value {
                NoOpInspection::Uncertain { .. } => return Ok(uncertain()),
                NoOpInspection::Next { .. } | NoOpInspection::CheckpointsLanded => {}
                NoOpInspection::Unavailable => {
                    return Ok(OwnerStep::Unavailable {
                        run_id: session.run_id.clone(),
                        phase: Phase::Checkpoint,
                        ordinal: Some(attempted.ordinal),
                        attempted: Some(attempted),
                        reason: "read-only checkpoint topology is unavailable",
                    });
                }
            },
        },
        Phase::Terminal => match terminal::seal_inner(session, reader) {
            Err(_) => return Ok(uncertain()),
            Ok(value) => match value {
                TerminalInspection::Uncertain
                | TerminalInspection::Ready { .. }
                | TerminalInspection::NotReady => return Ok(uncertain()),
                TerminalInspection::Completed { .. } => {}
            },
        },
    }
    after_step();
    // This second inspection is read-only. It may classify drift, but cannot
    // dispatch a second call in the same invocation. A failed read after a
    // possible effect must stay typed Uncertain rather than escaping as Err.
    let next = match select(session) {
        Ok(value) => value,
        Err(_) => return Ok(uncertain()),
    };
    Ok(status(session, next, Some(attempted)))
}

fn drive_inner(
    session: &ReadyRun<'_>,
    reader: &mut dyn RemoteReadPort,
    calls: &dyn CallTransport,
    assets: &dyn AssetCallTransport,
) -> Result<OwnerStep> {
    drive_inner_with_hook(session, reader, calls, assets, || {})
}

/// The only production transport binding: canonical GitHub repository, one
/// bounded read/effect/checkpoint/terminal step, no public invocation route.
#[allow(dead_code)]
pub(super) fn drive_one_fixed(
    session: &ReadyRun<'_>,
    client: &GithubReleaseClient,
) -> Result<OwnerStep> {
    let mut reader = GithubRemoteReadPort::new_fixed(client).map_err(Error::io)?;
    drive_inner(session, &mut reader, client, client)
}

#[cfg(test)]
pub(super) fn drive_one_fake(
    session: &ReadyRun<'_>,
    reader: &mut dyn RemoteReadPort,
    calls: &dyn CallTransport,
    assets: &dyn AssetCallTransport,
) -> Result<OwnerStep> {
    drive_inner(session, reader, calls, assets)
}

#[cfg(test)]
pub(super) fn inspect_one(session: &ReadyRun<'_>) -> Result<OwnerStep> {
    Ok(status(session, select(session)?, None))
}

#[cfg(test)]
pub(super) fn drive_one_fake_with_hook(
    session: &ReadyRun<'_>,
    reader: &mut dyn RemoteReadPort,
    calls: &dyn CallTransport,
    assets: &dyn AssetCallTransport,
    after_step: impl FnOnce(),
) -> Result<OwnerStep> {
    drive_inner_with_hook(session, reader, calls, assets, after_step)
}
