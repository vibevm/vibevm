//! Completion of a causally marked opt-launcher saga inverse.

use vibe_wire::generated::deploy_receipt::{DeployReceipt, OwnedResource, ReceiptStatus};

use super::Transaction;
use crate::mechanism::deploy::error::DeployError;
use crate::mechanism::deploy::state::InverseRecord;
use crate::mechanism::record::sanitize;
use crate::mechanism::{DeployProvider, DeployTargetRequest};

impl Transaction<'_> {
    /// Finish an inverse whose marker was durable before restore began.
    pub(crate) fn resume_inverse(
        &self,
        provider: &dyn DeployProvider,
        request: &DeployTargetRequest<'_>,
        receipt: &DeployReceipt,
        marker: &InverseRecord,
    ) -> Result<Vec<String>, DeployError> {
        if marker.generation != receipt.generation
            || marker.provider_pin != self.provider_pin
            || receipt.provider.key != marker.provider_pin
        {
            return Err(DeployError::RecordInvalid {
                record: "inverse.json",
                reason: "inverse marker does not bind the current receipt generation and provider"
                    .to_owned(),
            });
        }
        if receipt.status == ReceiptStatus::RolledBack
            && receipt.prior_state_handle.is_none()
            && receipt
                .resources
                .iter()
                .any(|owned| owned.resource == marker.resource)
        {
            self.state.retire_inverse(self.home)?;
            return Ok(Vec::new());
        }
        if receipt.prior_state_handle.as_deref() != Some(&marker.prior_state_handle)
            || !receipt.resources.iter().any(|owned| {
                owned.resource == marker.resource && owned.post_digest == marker.receipt_post_digest
            })
        {
            return Err(DeployError::RecordInvalid {
                record: "inverse.json",
                reason: "inverse marker does not bind the receipt resource, digest, and prior-state handle"
                    .to_owned(),
            });
        }
        let resources: Vec<String> = receipt
            .resources
            .iter()
            .map(|owned| owned.resource.clone())
            .collect();
        let report = provider.remove(request, &resources, Some(&marker.prior_state_handle))?;
        let independently_observed =
            provider.verify(request, std::slice::from_ref(&marker.resource))?;
        if independently_observed != report.expected_remaining {
            return Err(DeployError::VerifyMismatch {
                target: request.target.id.clone(),
                resources: marker.resource.clone(),
            });
        }
        let restored: Vec<OwnedResource> = independently_observed
            .into_iter()
            .filter_map(|resource| {
                resource.digest.map(|digest| OwnedResource {
                    resource: resource.resource,
                    post_digest: digest,
                })
            })
            .collect();
        if restored.len() != 1 {
            return Err(DeployError::VerifyMismatch {
                target: request.target.id.clone(),
                resources: marker.resource.clone(),
            });
        }
        let mut reversed = receipt.clone();
        reversed.status = ReceiptStatus::RolledBack;
        reversed.finalized_at = Some(self.timestamp(request)?);
        reversed.resources = restored;
        reversed.prior_state_handle = None;
        reversed.evidence = Some(sanitize(&format!(
            "{}; resumed causally marked inverse; removed {} resource(s): {}",
            receipt.evidence.as_deref().unwrap_or("no prior evidence"),
            report.removed.len(),
            report.evidence,
        )));
        self.state.write_receipt(self.home, &reversed)?;
        self.state.retire_inverse(self.home)?;
        Ok(report.removed)
    }
}
