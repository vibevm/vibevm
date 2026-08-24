//! The daemon-management verb family (D13): `mc start|stop|status`.
//! Split from `main.rs` along the responsibility seam when the verb
//! surface outgrew the file budget (C2 Ф2).

use clap::Subcommand;
use fractality_mc_client::lock::Lockfile;
use fractality_mc_client::{McClient, connect_or_start};

use crate::{EXIT_INFRA, EXIT_NEGATIVE, EXIT_OK};

specmark::scope!("spec://fractality/PROP-001#architecture");

#[derive(Subcommand)]
pub(crate) enum McCmd {
    /// Start the daemon (idempotent; exit 0 when already running).
    Start,
    /// Stop the daemon gracefully (idempotent; exit 0 when not running).
    Stop,
    /// Report daemon state. Exit 0 running, 1 stopped, 2 error.
    Status {
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
}

pub(crate) async fn mc_start(home: &camino::Utf8Path) -> u8 {
    match connect_or_start(home).await {
        Ok(client) => match (client.health().await, Lockfile::read(home)) {
            (Ok(health), Ok(Some(lock))) => {
                println!(
                    "mc running pid={} port={} uptime={} runs_open={}",
                    health.pid,
                    lock.port,
                    fractality_core::time::format_duration_ms(
                        fractality_core::time::now_ms().saturating_sub(health.started_ts_ms)
                    ),
                    health.runs_open,
                );
                EXIT_OK
            }
            (Err(e), _) => {
                eprintln!("fractality: daemon started but health failed: {e}");
                EXIT_INFRA
            }
            (_, lock) => {
                eprintln!("fractality: daemon healthy but lockfile unreadable: {lock:?}");
                EXIT_INFRA
            }
        },
        Err(e) => {
            eprintln!("fractality: {e}");
            EXIT_INFRA
        }
    }
}

pub(crate) async fn mc_stop(home: &camino::Utf8Path) -> u8 {
    match McClient::connect(home).await {
        Ok(None) => {
            println!("mc is not running");
            EXIT_OK
        }
        Ok(Some(client)) => {
            if let Err(e) = client.shutdown().await {
                eprintln!("fractality: shutdown call failed: {e}");
                return EXIT_INFRA;
            }
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                match McClient::connect(home).await {
                    Ok(None) => {
                        println!("mc stopped");
                        return EXIT_OK;
                    }
                    Ok(Some(_)) if std::time::Instant::now() < deadline => continue,
                    Ok(Some(_)) => {
                        eprintln!("fractality: daemon still answering after 10s");
                        return EXIT_INFRA;
                    }
                    Err(e) => {
                        eprintln!("fractality: {e}");
                        return EXIT_INFRA;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("fractality: {e}");
            EXIT_INFRA
        }
    }
}

pub(crate) async fn mc_status(home: &camino::Utf8Path, json: bool) -> u8 {
    match McClient::connect(home).await {
        Ok(Some(client)) => match client.health().await {
            Ok(health) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&health).expect("health serializes")
                    );
                } else {
                    let port = Lockfile::read(home)
                        .ok()
                        .flatten()
                        .map(|l| l.port.to_string())
                        .unwrap_or_else(|| "?".to_owned());
                    println!(
                        "running pid={} port={} uptime={} runs={}/{}",
                        health.pid,
                        port,
                        fractality_core::time::format_duration_ms(
                            fractality_core::time::now_ms().saturating_sub(health.started_ts_ms)
                        ),
                        health.runs_open,
                        health.runs_total,
                    );
                }
                EXIT_OK
            }
            Err(e) => {
                eprintln!("fractality: {e}");
                EXIT_INFRA
            }
        },
        Ok(None) => {
            if json {
                println!("{{\"status\":\"stopped\"}}");
            } else {
                println!("stopped");
            }
            EXIT_NEGATIVE
        }
        Err(e) => {
            eprintln!("fractality: {e}");
            EXIT_INFRA
        }
    }
}
