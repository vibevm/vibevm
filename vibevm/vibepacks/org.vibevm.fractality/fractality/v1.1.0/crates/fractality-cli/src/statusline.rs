//! The Claude Code statusline command (Campaign 2 D5/BD2): one ambient
//! line of measured wins. Reads the statusline stdin JSON (Ф0.s2
//! contract), resolves the session like the hook adapter does, prints
//! one line, always exits 0 — an initiative outage must never blank a
//! status bar into an error.

use fractality_core::api::SessionBeginRequest;
use fractality_mc_client::McClient;
use serde::Deserialize;

use crate::EXIT_OK;

specmark::scope!("spec://fractality/PROP-001#sessions");

/// The statusline stdin fields this cell reads (`/en/statusline`
/// capture, Ф0.s2); everything else is ignored by design.
#[derive(Debug, Deserialize)]
struct StatuslineInput {
    session_id: String,
    #[serde(default)]
    workspace: Option<Workspace>,
}

#[derive(Debug, Deserialize)]
struct Workspace {
    #[serde(default)]
    current_dir: Option<String>,
}

pub(crate) async fn statusline(home: &camino::Utf8Path) -> u8 {
    let mut raw = String::new();
    if std::io::Read::read_to_string(&mut std::io::stdin(), &mut raw).is_err() {
        return EXIT_OK;
    }
    let Ok(input) = serde_json::from_str::<StatuslineInput>(&raw) else {
        return EXIT_OK;
    };
    let Ok(Some(client)) = McClient::connect(home).await else {
        println!("frl: mc down");
        return EXIT_OK;
    };
    let cwd = input
        .workspace
        .and_then(|w| w.current_dir)
        .unwrap_or_default();
    let Ok(begun) = client
        .session_begin(&SessionBeginRequest {
            harness: "claude-code".to_owned(),
            external_id: input.session_id,
            cwd: cwd.into(),
        })
        .await
    else {
        println!("frl: mc down");
        return EXIT_OK;
    };
    match client.session_metrics(begun.session.session_id).await {
        Ok(m) => println!("{}", fractality_initiative::render_line(&m)),
        Err(_) => println!("frl: mc down"),
    }
    EXIT_OK
}
