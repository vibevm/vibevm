//! Persistence codecs for the generated deploy record roots.

use vibe_wire::generated::deploy_checkpoints::DeployCheckpoints;
use vibe_wire::generated::deploy_intent::DeployIntent;
use vibe_wire::generated::deploy_inverse::DeployInverse;
use vibe_wire::generated::deploy_receipt::DeployReceipt;

use super::{CHECKPOINT_EPOCH, CHECKPOINT_FILE, CheckpointRecord, InverseRecord};
use crate::mechanism::deploy::error::DeployError;
use crate::mechanism::deploy::sidecar::{self, LockResources};

pub(in crate::mechanism::deploy) trait StateRecord: Sized {
    fn encode(&self) -> Result<Vec<u8>, String>;
    fn decode(bytes: &[u8]) -> Result<Self, String>;
}

pub(super) fn validate_checkpoint(value: &CheckpointRecord) -> Result<(), DeployError> {
    if value.schema != CHECKPOINT_EPOCH {
        return Err(DeployError::RecordInvalid {
            record: CHECKPOINT_FILE,
            reason: format!(
                "schema epoch {} is not the {CHECKPOINT_EPOCH} this engine writes",
                value.schema
            ),
        });
    }
    Ok(())
}

impl StateRecord for DeployIntent {
    fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(self).map_err(|error| error.to_string())
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|error| error.to_string())
    }
}

impl StateRecord for DeployReceipt {
    fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(self).map_err(|error| error.to_string())
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|error| error.to_string())
    }
}

impl StateRecord for LockResources {
    fn encode(&self) -> Result<Vec<u8>, String> {
        sidecar::wire::encode(self)
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        sidecar::wire::decode(bytes)
    }
}

impl StateRecord for CheckpointRecord {
    fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(&DeployCheckpoints {
            schema: self.schema,
            plan_hash: self.plan_hash.clone(),
            completed: self.completed.clone(),
        })
        .map_err(|error| error.to_string())
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice::<DeployCheckpoints>(bytes)
            .map(|value| Self {
                schema: value.schema,
                plan_hash: value.plan_hash,
                completed: value.completed,
            })
            .map_err(|error| error.to_string())
    }
}

impl StateRecord for InverseRecord {
    fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(&DeployInverse {
            schema: self.schema,
            generation: self.generation,
            provider_pin: self.provider_pin.clone(),
            resource: self.resource.clone(),
            receipt_post_digest: self.receipt_post_digest.clone(),
            prior_state_handle: self.prior_state_handle.clone(),
        })
        .map_err(|error| error.to_string())
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice::<DeployInverse>(bytes)
            .map(|value| Self {
                schema: value.schema,
                generation: value.generation,
                provider_pin: value.provider_pin,
                resource: value.resource,
                receipt_post_digest: value.receipt_post_digest,
                prior_state_handle: value.prior_state_handle,
            })
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_domain_conversion_keeps_exact_bytes_and_strict_reading() {
        let value = CheckpointRecord {
            schema: 1,
            plan_hash: "a".repeat(64),
            completed: vec!["bin/one".to_owned(), "bin/two".to_owned()],
        };
        validate_checkpoint(&value).unwrap();
        let bytes = value.encode().unwrap();
        assert_eq!(
            bytes,
            b"{\n  \"schema\": 1,\n  \"plan_hash\": \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\n  \"completed\": [\n    \"bin/one\",\n    \"bin/two\"\n  ]\n}"
        );
        assert_eq!(CheckpointRecord::decode(&bytes).unwrap(), value);
        let unknown = br#"{"schema":1,"plan_hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","completed":[],"retry":true}"#;
        assert!(
            CheckpointRecord::decode(unknown)
                .unwrap_err()
                .contains("retry")
        );
        let mut wrong_epoch = value;
        wrong_epoch.schema = 2;
        assert!(matches!(
            validate_checkpoint(&wrong_epoch),
            Err(DeployError::RecordInvalid { .. })
        ));
    }

    #[test]
    fn inverse_domain_conversion_keeps_exact_bytes_and_strict_reading() {
        let value = InverseRecord::new(
            4,
            "org.example/providers#native",
            "bin/tool",
            &"b".repeat(64),
            "rollback/generation-3",
        );
        let bytes = value.encode().unwrap();
        assert_eq!(
            bytes,
            b"{\n  \"schema\": 1,\n  \"generation\": 4,\n  \"provider_pin\": \"org.example/providers#native\",\n  \"resource\": \"bin/tool\",\n  \"receipt_post_digest\": \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\n  \"prior_state_handle\": \"rollback/generation-3\"\n}"
        );
        assert_eq!(InverseRecord::decode(&bytes).unwrap(), value);
        let unknown = br#"{"schema":1,"generation":4,"provider_pin":"org.example/providers#native","resource":"bin/tool","receipt_post_digest":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","prior_state_handle":"rollback/generation-3","force":true}"#;
        assert!(
            InverseRecord::decode(unknown)
                .unwrap_err()
                .contains("force")
        );
    }
}
