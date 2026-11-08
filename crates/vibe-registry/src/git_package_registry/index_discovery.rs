//! Demand-driven index handshake; retained binding precedes every request.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::*;
use crate::index_client::{IndexAuth, IndexClient, ProbeOutcome};

pub(super) struct IndexDiscovery {
    url: String,
    auth: vibe_core::manifest::RegistrySection,
    admission: Option<Arc<crate::HttpAdmission>>,
    native: bool,
}
impl GitPerPackageRegistry {
    pub(crate) fn with_index_discovery(
        mut self,
        url: String,
        auth: vibe_core::manifest::RegistrySection,
        admission: Option<Arc<crate::HttpAdmission>>,
        native: bool,
    ) -> Self {
        self.index_discovery = Some(IndexDiscovery {
            url,
            auth,
            admission,
            native,
        });
        self
    }
    pub(crate) fn attach_index_transfers(
        &self,
        transfers: vibe_core::progress::transfer::Transfers,
    ) {
        if let Some(client) = self.index_client() {
            client.attach_transfers(&transfers);
        }
        let _ = self.index_transfers.set(transfers);
    }
    pub(crate) fn discover_index(&self) -> Result<Option<&IndexClient>, RegistryError> {
        if self.index_client.get().is_none() {
            let outcome = match &self.index_discovery {
                None => ProbeOutcome::Absent,
                Some(input) => probe(input)?,
            };
            let client = match outcome {
                ProbeOutcome::Found(client) => {
                    if let Some(transfers) = self.index_transfers.get() {
                        client.attach_transfers(transfers);
                    }
                    Some(*client)
                }
                ProbeOutcome::Refused { reason } => {
                    tracing::warn!(registry = %self.name, "registry index refused: {reason}");
                    None
                }
                ProbeOutcome::Absent => None,
            };
            let _ = self.index_client.set(client);
        }
        Ok(self.index_client())
    }
}

fn probe(input: &IndexDiscovery) -> Result<ProbeOutcome, RegistryError> {
    if let Some(binding) = &input.admission {
        match binding.check_address(&input.url) {
            Ok(()) => (),
            Err(crate::ContactRefusal::ExcludedHost { .. }) if input.native => {
                return Ok(ProbeOutcome::Absent);
            }
            Err(error) => return Err(error.into()),
        }
    }
    let auth = IndexAuth::for_registry(&input.auth, &input.url)?;
    match &input.admission {
        Some(binding) => {
            IndexClient::probe_with_admission(&input.url, auth, binding.clone()).map_err(Into::into)
        }
        None => Ok(IndexClient::probe(&input.url, auth)),
    }
}
