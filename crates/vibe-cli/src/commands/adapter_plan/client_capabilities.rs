//! New-family transport, selected before the legacy route.
use super::*;
use vibe_orchestrator::machine_operation::adapter_plan::client_capabilities::ClientCapabilitiesHost;
pub(super) fn run(args: AdapterPlanArgs) -> Result<()> {
    anyhow::ensure!(
        args.scope.is_none() && args.profile.is_none(),
        "clients has no legacy scope/profile selector"
    );
    let root = super::super::resolve_project_root(&args.root)?;
    let host = ClientCapabilitiesHost::new(&root, args.prepare)?;
    serve_client_capabilities(
        &host,
        std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
    )
}
pub(super) fn serve_client_capabilities(
    host: &ClientCapabilitiesHost,
    mut input: impl BufRead,
    output: &mut impl Write,
) -> Result<()> {
    serde_json::to_writer(&mut *output, &host.describe())?;
    output.write_all(b"\n")?;
    output.flush()?;
    loop {
        let mut line = Vec::new();
        if input
            .by_ref()
            .take((adapter_plan::MAX_INPUT + 1) as u64)
            .read_until(b'\n', &mut line)?
            == 0
        {
            break;
        }
        anyhow::ensure!(
            line.len() <= adapter_plan::MAX_INPUT,
            "client capability input exceeds bound"
        );
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        serde_json::to_writer(&mut *output, &host.exchange(&line))?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}
