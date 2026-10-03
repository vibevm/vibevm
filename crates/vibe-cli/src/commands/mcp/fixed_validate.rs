//! Explicit MCP startup binding to the incumbent fixed Validate host.
use std::{path::Path, sync::Arc};

use anyhow::Result;
use vibe_core::manifest::TargetOs;
use vibe_mcp::ServerContext;
use vibe_orchestrator::machine_submission::{
    HostPolicyFacts, TrustedPolicySource, cli_grant::CliGrantService,
};

use crate::cli::McpFixedValidateMode;

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-HOST-REATTACH-LAW"
)]
pub(super) fn server_context(
    context: ServerContext,
    root: &Path,
    mode: McpFixedValidateMode,
    ttl_ms: Option<u64>,
    facts: HostPolicyFacts,
) -> Result<ServerContext> {
    let target_os = TargetOs::current().ok_or_else(|| {
        anyhow::anyhow!("violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-LOCAL-TRUST-DECISION: fixed Validate MCP host requires a supported target platform; fix surface: run on a supported platform")
    })?;
    let project = Arc::new(vibe_safefs::Project::open(root)?);
    let target = format!("physical-root:{}", project.root_correlation_token()?);
    let service = CliGrantService::from_trusted_host(
        project,
        target,
        "vibe-cli-project-local/e1".into(),
        target_os,
        TrustedPolicySource::from_host_identifier("vibe-cli/fixed-validate/e1".into())?,
        facts,
    )?;
    let host = match mode {
        McpFixedValidateMode::Issue => service.issue(ttl_ms.ok_or_else(|| {
            anyhow::anyhow!("fixed Validate issue requires --ttl-ms <1..=86400000>")
        })?)?,
        McpFixedValidateMode::Reattach => service.reattach()?,
    };
    Ok(context.with_fixed_lifecycle_host(Arc::new(host))?)
}
