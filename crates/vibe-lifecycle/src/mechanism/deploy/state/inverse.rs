//! Durable causal marker for an in-progress opt-launcher saga inverse.

use serde::{Deserialize, Serialize};

use super::{DeployState, DeploymentHome};
use crate::mechanism::deploy::error::DeployError;

const FILE: &str = "inverse.json";
const EPOCH: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InverseRecord {
    pub(crate) schema: u32,
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
            generation,
            provider_pin: provider_pin.to_owned(),
            resource: resource.to_owned(),
            receipt_post_digest: receipt_post_digest.to_owned(),
            prior_state_handle: prior_state_handle.to_owned(),
        }
    }

    fn validate(&self) -> Result<(), DeployError> {
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
        Ok(Some(marker))
    }

    pub(crate) fn retire_inverse(&self, home: &DeploymentHome) -> Result<(), DeployError> {
        self.remove(&home.member(FILE))
    }
}
