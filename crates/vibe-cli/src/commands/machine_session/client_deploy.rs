//! The declared CLI mode and embedding path share one framed owner adapter.
use std::{
    io::{Read, Write},
    sync::Arc,
};
use vibe_orchestrator::machine_operation::client_deploy::ClientDeployRoute;
pub(crate) fn exchange<R: Read, W: Write>(
    args: &crate::cli::MachineSessionArgs,
    host: Option<Arc<ClientDeployRoute>>,
    reader: &mut R,
    writer: &mut W,
) -> anyhow::Result<()> {
    let host=host.ok_or_else(||anyhow::anyhow!("client deployment requires a trusted host with roots, namespace, prepared exact grants, runner and complete state observer"))?;
    let context =
        vibe_mcp::ServerContext::with_store_root(&args.root, args.root.join("unused-store"))
            .with_client_deploy_route(host)
            .map_err(anyhow::Error::msg)?;
    let mut runner =
        vibe_mcp::machine_session::client_deploy::ClientDeployMachineSessionRunner::new(context)
            .map_err(anyhow::Error::msg)?;
    runner
        .run(reader, writer)
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    Ok(())
}
#[cfg(test)]
mod m_08_d_2_cases;
