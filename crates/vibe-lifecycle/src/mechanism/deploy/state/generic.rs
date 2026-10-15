//! Generic state and private payloads belong to the incumbent state owner.
use super::*;
use vibe_wire::behaviour::deploy_generic_state::{BLOB_LIMIT, validate};
use vibe_wire::generated::deploy_generic_state::{Blob, DeployGenericState};

const FILE: &str = "generic-state.json";
fn invalid(reason: impl ToString) -> DeployError {
    DeployError::RecordInvalid {
        record: FILE,
        reason: reason.to_string(),
    }
}
impl DeployState {
    pub(crate) fn read_generic(
        &self,
        home: &DeploymentHome,
    ) -> Result<Option<DeployGenericState>, DeployError> {
        let bytes = self
            .project
            .read_file_bounded(&home.member(FILE), 4 * 1024 * 1024)
            .map_err(invalid)?;
        let Some(bytes) = bytes else { return Ok(None) };
        let value: DeployGenericState = serde_json::from_slice(&bytes).map_err(invalid)?;
        validate(&value).map_err(invalid)?;
        for b in [value.committed.as_ref(), value.pending.as_ref()]
            .into_iter()
            .flatten()
        {
            if b.deployment != home.id() {
                return Err(invalid("generic deployment join"));
            }
        }
        Ok(Some(value))
    }
    pub(crate) fn write_generic(
        &self,
        home: &DeploymentHome,
        value: &DeployGenericState,
    ) -> Result<(), DeployError> {
        validate(value).map_err(invalid)?;
        for b in [value.committed.as_ref(), value.pending.as_ref()]
            .into_iter()
            .flatten()
        {
            if b.deployment != home.id() {
                return Err(invalid("generic deployment join"));
            }
        }
        let bytes = serde_json::to_vec(value).map_err(invalid)?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err(invalid("generic state budget"));
        }
        self.project
            .write_atomic_with_mode(&home.member(FILE), &bytes, Some(0o600))
            .map_err(|e| invalid(e.into_report()))?;
        if self.read_generic(home)?.as_ref() != Some(value) {
            return Err(invalid("generic readback"));
        }
        #[cfg(test)]
        FAIL_PHASE.with(|phase| {
            let actual = value
                .pending
                .as_ref()
                .or(value.committed.as_ref())
                .map(|b| b.phase.clone());
            if phase.borrow().as_ref() == actual.as_ref() && actual.is_some() {
                phase.borrow_mut().take();
                return Err(invalid("injected generic publication interruption"));
            }
            Ok(())
        })?;
        Ok(())
    }
    pub(crate) fn retain_generic_blob(
        &self,
        home: &DeploymentHome,
        bytes: &[u8],
        mode: u32,
    ) -> Result<Blob, DeployError> {
        if bytes.len() > BLOB_LIMIT as usize || mode > 0o777 {
            return Err(invalid("blob budget or unsupported mode"));
        }
        let blob = Blob {
            digest: format!("{:x}", Sha256::digest(bytes)),
            bytes: bytes.len() as u32,
            mode,
        };
        let relative = home.member(&format!("private-blobs/{}", blob.digest));
        if let Some(old) = self
            .project
            .read_file_bounded(&relative, BLOB_LIMIT as usize)
            .map_err(invalid)?
        {
            if old != bytes {
                return Err(invalid("private blob digest collision or corruption"));
            }
        } else {
            // The bound applies to the owner-controlled store across all
            // generations, including blobs retained by interrupted attempts.
            // No garbage collection may discard a still-referenced image.
            let mut total = bytes.len() as u64;
            if let Some(directory) = self
                .project
                .dir_if_present(&[home.id(), "private-blobs"])
                .map_err(invalid)?
            {
                let names = self.project.child_names(&directory).map_err(invalid)?;
                if names.len() >= vibe_wire::behaviour::deploy_generic_state::RESOURCE_LIMIT * 2 {
                    return Err(invalid("private blob store entry budget"));
                }
                for name in names {
                    let old = self
                        .project
                        .read_file_bounded(
                            &home.member(&format!("private-blobs/{name}")),
                            BLOB_LIMIT as usize,
                        )
                        .map_err(invalid)?
                        .ok_or_else(|| invalid("private blob disappeared during budget check"))?;
                    total = total
                        .checked_add(old.len() as u64)
                        .ok_or_else(|| invalid("private blob store budget overflow"))?;
                    if total > vibe_wire::behaviour::deploy_generic_state::RETENTION_LIMIT {
                        return Err(invalid("private blob store aggregate budget"));
                    }
                }
            }
            self.project
                .write_atomic_with_mode(&relative, bytes, Some(0o600))
                .map_err(|e| invalid(e.into_report()))?;
        }
        self.read_generic_blob(home, &blob)?;
        Ok(blob)
    }
    pub(crate) fn read_generic_blob(
        &self,
        home: &DeploymentHome,
        blob: &Blob,
    ) -> Result<Vec<u8>, DeployError> {
        if blob.bytes > BLOB_LIMIT
            || blob.digest.len() != 64
            || !blob
                .digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("invalid private blob reference"));
        }
        let bytes = self
            .project
            .read_file_bounded(
                &home.member(&format!("private-blobs/{}", blob.digest)),
                BLOB_LIMIT as usize,
            )
            .map_err(invalid)?
            .ok_or_else(|| invalid("missing retained private blob"))?;
        if bytes.len() != blob.bytes as usize
            || format!("{:x}", Sha256::digest(&bytes)) != blob.digest
        {
            return Err(invalid("corrupt retained private blob"));
        }
        Ok(bytes)
    }
    /// Legacy paths must not silently operate on a typed generic generation.
    pub(crate) fn require_legacy_state(&self, home: &DeploymentHome) -> Result<(), DeployError> {
        if self.read_transfer_hold(home)?.is_some_and(|h| {
            !h.pending
                .as_ref()
                .and_then(|b| b.transfer.as_ref())
                .is_some_and(|t| {
                    t.phase == vibe_wire::generated::deploy_generic_state::TransferPhase::Restored
                })
        }) {
            return Err(invalid("deployment held by generic transfer"));
        }
        if self.read_generic(home)?.is_some() {
            return Err(invalid(
                "generic state requires current generic host admission",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
thread_local! { pub(crate) static FAIL_PHASE: std::cell::RefCell<Option<vibe_wire::generated::deploy_generic_state::BindingPhase>> = const { std::cell::RefCell::new(None) }; }

impl DeployState {
    pub(crate) fn read_transfer_hold(
        &self,
        home: &DeploymentHome,
    ) -> Result<Option<DeployGenericState>, DeployError> {
        self.project
            .read_file_bounded(&home.member("generic-transfer-hold.json"), 4 * 1024 * 1024)
            .map_err(invalid)?
            .map(|b| {
                let value = serde_json::from_slice(&b).map_err(invalid)?;
                validate(&value).map_err(invalid)?;
                Ok(value)
            })
            .transpose()
    }
    pub(crate) fn write_transfer_hold(
        &self,
        home: &DeploymentHome,
        value: &DeployGenericState,
    ) -> Result<(), DeployError> {
        validate(value).map_err(invalid)?;
        self.project
            .write_atomic_with_mode(
                &home.member("generic-transfer-hold.json"),
                &serde_json::to_vec(value).map_err(invalid)?,
                Some(0o600),
            )
            .map_err(|e| invalid(e.into_report()))?;
        Ok(())
    }
}
