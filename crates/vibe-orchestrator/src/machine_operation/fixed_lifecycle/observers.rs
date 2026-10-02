//! Connection-local opaque subscriptions. Table -> clone -> unlock -> entry;
//! never hold the table while calling filesystem, owner, replay or control.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-STREAM");
use super::{FixedLifecycleRoute, contract::*};
use crate::machine_submission::{ObservationHandle, ScopedFixedJob};
use LifecycleActionErrorCode as Code;
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};
use vibe_wire::generated::machine::e1::operation_stream_message::OperationStreamMessage;
type Failure = (Code, String, bool);
struct Entry {
    job: ScopedFixedJob,
    // None is permanently closed, including for clones held before detach.
    handle: Mutex<Option<ObservationHandle>>,
}
#[derive(Default)]
struct Table {
    entries: BTreeMap<String, Arc<Entry>>,
    claims: usize,
}
#[derive(Default)]
pub(crate) struct Observers {
    table: Mutex<Table>,
}
fn capacity() -> Failure {
    (
        Code::Capacity,
        "observer capacity unavailable".into(),
        false,
    )
}
fn unknown() -> Failure {
    (
        Code::UnknownReference,
        "observer reference is not active in this bridge".into(),
        false,
    )
}
fn owner(e: impl std::fmt::Display) -> Failure {
    (Code::InspectionUnavailable, e.to_string(), true)
}
impl FixedLifecycleRoute {
    pub(crate) fn attach_observer(&self, key: &str, root: &Path) -> Result<Reply, Failure> {
        {
            let mut t = self.observers.table.lock().map_err(|_| capacity())?;
            if t.entries.len() + t.claims >= 8 {
                return Err(capacity());
            }
            t.claims += 1;
        }
        // Capacity is claimed before subscription; all ordinary errors roll it back.
        let result = (|| {
            let job = self.retained_job(key).map_err(owner)?.ok_or_else(unknown)?;
            let id = vibe_lifecycle::process::propose_run_id(root).map_err(owner)?;
            let handle = job.attach().map_err(owner)?;
            let s = handle.summary();
            let reply = Reply::Attached(Box::new(ReplyAttached {
                observer_ref: id.clone(),
                context_id: s.context_id().into(),
                target_id: s.target_id().map(str::to_owned),
                operation_id: s.operation_id().into(),
                run_id: s.run_id().into(),
                stream_id: s.stream_id().into(),
            }));
            // Ensure publication is bounded before installing a subscription.
            size(&reply, PUBLICATION_CAP).map_err(|e| (Code::PublicationUnavailable, e, true))?;
            Ok((
                id,
                Arc::new(Entry {
                    job,
                    handle: Mutex::new(Some(handle)),
                }),
                reply,
            ))
        })();
        {
            let mut t = self.observers.table.lock().map_err(|_| capacity())?;
            t.claims -= 1;
            // Keep capacity reserved through insertion on the success path.
            if let Ok((id, entry, _)) = &result {
                if t.entries.contains_key(id) {
                    drop(t);
                    return Err(capacity());
                }
                t.entries.insert(id.clone(), entry.clone());
            }
        }
        let (_, _, reply) = result?;
        Ok(reply)
    }
    pub(crate) fn stream_observer(
        &self,
        id: &str,
        json: &str,
        control: bool,
    ) -> Result<Reply, Failure> {
        if json.len() > crate::machine_operation::stream_message::MAX_FRAME_BYTES {
            return Err((
                Code::InvalidRequest,
                "stream frame exceeds 64KiB".into(),
                false,
            ));
        }
        let message: OperationStreamMessage = serde_json::from_str(json).map_err(|_| {
            (
                Code::InvalidRequest,
                "generated stream shape refused".into(),
                false,
            )
        })?;
        let entry = self
            .observers
            .table
            .lock()
            .map_err(|_| capacity())?
            .entries
            .get(id)
            .cloned()
            .ok_or_else(unknown)?;
        let mut guard = entry.handle.lock().map_err(|_| capacity())?;
        let handle = guard.as_mut().ok_or_else(unknown)?;
        // The incumbent validates the exact semantic binding before its control gate.
        let response = if control {
            entry.job.control(handle, &message)
        } else {
            handle.replay(&message)
        }
        .map_err(owner)?;
        use vibe_wire::generated::machine::e1::operation_stream_message::{
            Body, BodyControlReplyStatus,
        };
        let detached = matches!(&response.body, Body::ControlReply(r)
            if r.status == BodyControlReplyStatus::Detached);
        if detached {
            // Explicit owner-confirmed detach closes clones before releasing capacity.
            drop(guard.take());
        }
        drop(guard);
        if detached {
            self.observers
                .table
                .lock()
                .map_err(|_| capacity())?
                .entries
                .remove(id);
        }
        let payload_json = encode(
            &response,
            crate::machine_operation::stream_message::MAX_FRAME_BYTES,
        )
        .map_err(|e| (Code::PublicationUnavailable, e, true))?;
        Ok(Reply::StreamResponse(Box::new(ReplyStreamResponse {
            observer_ref: id.into(),
            format_id: ReplyStreamResponseFormatId::MachineOperationStreamMessage,
            payload_json,
        })))
    }
    pub(crate) fn detach_observer(&self, id: &str) -> Result<Reply, Failure> {
        let entry = self
            .observers
            .table
            .lock()
            .map_err(|_| capacity())?
            .entries
            .get(id)
            .cloned()
            .ok_or_else(unknown)?;
        let mut guard = entry.handle.lock().map_err(|_| capacity())?;
        let mut handle = guard.take().ok_or_else(unknown)?;
        handle.detach();
        drop(handle);
        drop(guard);
        self.observers
            .table
            .lock()
            .map_err(|_| capacity())?
            .entries
            .remove(id);
        Ok(Reply::Detached(Box::new(ReplyDetached {
            observer_ref: id.into(),
        })))
    }
}
