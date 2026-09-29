//! Decision-log client verbs (D-C3-8): the soft-label table's producer
//! (`record_decision`) and consumer (`decisions`). Split from `lib.rs`
//! along the cell budget; both hang off `McClient` like the other verbs,
//! reaching its private request helper as a child module.

use crate::{ClientError, McClient};

specmark::scope!("spec://fractality/PROP-001#architecture");

impl McClient {
    /// Appends a need-gate decision to the decisions journal (D-C3-8) —
    /// the `gate --record` producer of the soft-label table.
    pub async fn record_decision(
        &self,
        record: &fractality_core::DecisionRecord,
    ) -> Result<fractality_core::api::Ack, ClientError> {
        self.request(
            "POST /v0/decisions",
            self.http.post(self.url("/decisions")).json(record),
        )
        .await
    }

    /// The decision log, oldest first (the soft-label table's raw rows).
    pub async fn decisions(&self) -> Result<Vec<fractality_core::DecisionRecord>, ClientError> {
        let resp: fractality_core::api::DecisionListResponse = self
            .request("GET /v0/decisions", self.http.get(self.url("/decisions")))
            .await?;
        Ok(resp.decisions)
    }
}
