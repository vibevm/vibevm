//! The pod leg of the mc-client (D3/D10): a per-run pod supervisor
//! registers, heartbeats, and reports worker lifecycle events. Split from
//! the client-leg verbs in `lib.rs` along the seam `api.rs` already draws;
//! these inherent `McClient` methods reach the client's private
//! `request`/`url`/`http` from this child module.

use fractality_core::api::{
    Ack, PodEventRequest, PodHeartbeat, PodHeartbeatResponse, PodRegisterRequest,
    PodRegisterResponse,
};
use fractality_core::ids::PodId;

use crate::{ClientError, McClient};

specmark::scope!("spec://fractality/PROP-001#architecture");

impl McClient {
    pub async fn pod_register(
        &self,
        req: &PodRegisterRequest,
    ) -> Result<PodRegisterResponse, ClientError> {
        self.request(
            "POST /v0/pods/register",
            self.http.post(self.url("/pods/register")).json(req),
        )
        .await
    }

    pub async fn pod_heartbeat(
        &self,
        pod_id: PodId,
        hb: &PodHeartbeat,
    ) -> Result<PodHeartbeatResponse, ClientError> {
        self.request(
            "POST /v0/pods/:id/heartbeat",
            self.http
                .post(self.url(&format!("/pods/{pod_id}/heartbeat")))
                .json(hb),
        )
        .await
    }

    pub async fn pod_event(&self, pod_id: PodId, ev: &PodEventRequest) -> Result<Ack, ClientError> {
        self.request(
            "POST /v0/pods/:id/event",
            self.http
                .post(self.url(&format!("/pods/{pod_id}/event")))
                .json(ev),
        )
        .await
    }
}
