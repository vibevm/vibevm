//! The daemon binary: resolve home from `FRACTALITY_HOME`, start, run
//! until Ctrl-C or `POST /v0/shutdown`, stop clean.

use tracing_subscriber::EnvFilter;

specmark::scope!("spec://fractality/PROP-001#architecture");

#[tokio::main]
async fn main() -> std::process::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("FRACTALITY_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let home = match fractality_mission_control::resolve_home(None) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(error = %e, "cannot resolve the fractality home");
            return std::process::ExitCode::from(2);
        }
    };

    let server = match fractality_mission_control::start(fractality_mission_control::Config::new(
        home,
    ))
    .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "mission-control failed to start");
            return std::process::ExitCode::from(2);
        }
    };

    let mut shutdown_rx = server.state.shutdown.subscribe();
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("ctrl-c received; shutting down");
        }
        _ = shutdown_rx.wait_for(|v| *v) => {}
    }
    server.stop().await;
    std::process::ExitCode::SUCCESS
}
