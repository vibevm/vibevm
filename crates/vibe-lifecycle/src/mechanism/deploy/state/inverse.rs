//! Durable causal marker for an in-progress owned inverse or bound cleanup.

use super::{DeployState, DeploymentHome};
use crate::mechanism::deploy::error::DeployError;
use vibe_wire::behaviour::deploy_records::validate_operation_binding;
use vibe_wire::generated::shared::EnvironmentOperationBinding;

const FILE: &str = "inverse.json";
const EPOCH: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InverseRecord {
    pub(crate) schema: u32,
    pub(crate) operation_binding: Option<EnvironmentOperationBinding>,
    pub(crate) generation: u32,
    pub(crate) provider_pin: String,
    pub(crate) resource: String,
    pub(crate) receipt_post_digest: String,
    pub(crate) prior_state_handle: String,
}

impl InverseRecord {
    pub(crate) fn new(
        generation: u32,
        provider_pin: &str,
        resource: &str,
        receipt_post_digest: &str,
        prior_state_handle: &str,
    ) -> Self {
        Self {
            schema: EPOCH,
            operation_binding: None,
            generation,
            provider_pin: provider_pin.to_owned(),
            resource: resource.to_owned(),
            receipt_post_digest: receipt_post_digest.to_owned(),
            prior_state_handle: prior_state_handle.to_owned(),
        }
    }

    /// A closed cleanup recipe, not fabricated prior bytes or forward authority.
    pub(crate) fn bound_cleanup(
        receipt: &vibe_wire::generated::deploy_receipt::DeployReceipt,
    ) -> Result<Self, DeployError> {
        use sha2::{Digest, Sha256};
        let owned = receipt
            .resources
            .first()
            .ok_or(DeployError::BindingMismatch {
                record: "empty cleanup ownership",
            })?;
        let hash = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(receipt).map_err(|_| {
                DeployError::BindingMismatch {
                    record: "cleanup receipt encoding",
                }
            })?)
        );
        let mut marker = Self::new(
            receipt.generation,
            &receipt.provider.key,
            &owned.resource,
            &owned.post_digest,
            &format!("bound-cleanup/1:{hash}"),
        );
        marker.operation_binding = receipt.operation_binding.clone();
        Ok(marker)
    }

    fn validate(&self) -> Result<(), DeployError> {
        if let Some(binding) = &self.operation_binding {
            validate_operation_binding(binding).map_err(|error| DeployError::RecordInvalid {
                record: FILE,
                reason: error.to_string(),
            })?;
        }
        if self.schema != EPOCH
            || self.provider_pin.is_empty()
            || self.resource.is_empty()
            || self.receipt_post_digest.len() != 64
            || self.prior_state_handle.is_empty()
        {
            return Err(DeployError::RecordInvalid {
                record: FILE,
                reason: "inverse marker has the wrong epoch or an empty prior-state handle"
                    .to_owned(),
            });
        }
        Ok(())
    }
}

impl DeployState {
    pub(crate) fn write_inverse(
        &self,
        home: &DeploymentHome,
        marker: &InverseRecord,
    ) -> Result<(), DeployError> {
        marker.validate()?;
        home.ensure_binding(FILE, marker.operation_binding.as_ref())?;
        self.publish(&home.member(FILE), marker)
    }

    pub(crate) fn read_inverse(
        &self,
        home: &DeploymentHome,
    ) -> Result<Option<InverseRecord>, DeployError> {
        let Some(marker) = self.read::<InverseRecord>(&home.member(FILE))? else {
            return Ok(None);
        };
        marker.validate()?;
        home.ensure_binding(FILE, marker.operation_binding.as_ref())?;
        Ok(Some(marker))
    }

    pub(crate) fn retire_inverse(&self, home: &DeploymentHome) -> Result<(), DeployError> {
        self.remove(&home.member(FILE))
    }
}
