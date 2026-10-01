//! Strict private e2 Ready seal. A typed provenance arm prevents a populated
//! aggregate from being represented as a no-Gitlinks receipt.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum ReadyProvenance {
    NoGitlinks {
        pending_sha256: String,
        landed_sha256: String,
    },
    Populated {
        pending_sha256: String,
        landed_sha256: String,
        flattened_submodules_sha256: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadySeal {
    pub format: String,
    pub run_id: String,
    pub namespace_id: String,
    pub scope_context: String,
    pub scope_target: String,
    pub scope_principal: String,
    pub key_sha256: String,
    pub request_sha256: String,
    pub c1_plan_sha256: String,
    pub semantic_plan_sha256: String,
    pub lease_root_token: String,
    pub source_root_token: String,
    pub vault_root_token: String,
    pub object_root_token: String,
    pub intent_sha256: String,
    pub payload_manifest_sha256: String,
    pub stage_receipt_sha256: String,
    pub tree_receipt_sha256: String,
    pub provenance: ReadyProvenance,
    pub parent_pending_sha256: Option<String>,
    pub parent_landed_sha256: Option<String>,
    pub object_receipt_sha256: String,
    pub selected_tree_oid: String,
    pub selected_commit_oid: String,
    pub selected_tag_oid: String,
}
