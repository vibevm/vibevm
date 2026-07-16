//! `vibe aiui inspect` — attach to the running vibeterm Electron page over the
//! Chrome DevTools Protocol (PROP-042 §4) and evaluate a JavaScript expression
//! in the live renderer, reading its REAL state — the xterm grid's `cols`/cell
//! metrics, the `.xterm-viewport` scrollbar box — straight from the runtime,
//! with no screenshot OCR. The CDP port comes from the session's discovery file
//! (`cdpPort`, written by vibeterm's `--cdp-port` switch).
//!
//! The CLI is otherwise blocking (`reqwest::blocking` for the control verbs);
//! chromiumoxide is async over tokio, so each `inspect` spins a current-thread
//! tokio runtime and `block_on`s the connect + evaluate.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-042#aiui-cli");

use std::time::Duration;

use anyhow::{Result, anyhow};
use futures::StreamExt;

use crate::cli::AiuiInspectArgs;

/// `vibe aiui inspect "<expr>"`: connect to the session's CDP endpoint and
/// evaluate the expression in the live page, printing the result value as JSON.
pub(super) fn inspect(a: AiuiInspectArgs) -> Result<()> {
    // A dedicated runtime per call keeps the async surface local to this verb;
    // the rest of the CLI never sees tokio. `current_thread` is enough — there
    // is one page to talk to and no worker fan-out.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow!("building the CDP tokio runtime: {e}"))?;
    rt.block_on(inspect_async(a))
}

async fn inspect_async(a: AiuiInspectArgs) -> Result<()> {
    let disc = super::control::read_discovery(a.session)?;
    let cdp_port = disc.cdp_port.ok_or_else(|| {
        anyhow!(
            "the session has no CDP endpoint (its vibeterm predates --cdp-port; \
             re-run `vibe aiui open`)"
        )
    })?;

    // `connect` accepts an http URL and resolves the websocket itself via the
    // browser's `/json/version` endpoint, so we never touch raw HTTP here.
    let (mut browser, mut handler) =
        chromiumoxide::Browser::connect(format!("http://127.0.0.1:{cdp_port}"))
            .await
            .map_err(|e| anyhow!("connecting to vibeterm's CDP endpoint (port {cdp_port}): {e}"))?;

    // Drive the websocket handler on a background task — chromiumoxide delivers
    // CDP responses through it, so it must be polled or every command hangs.
    tokio::spawn(async move { while handler.next().await.is_some() {} });

    // chromiumoxide only auto-tracks targets created AFTER a connection
    // (mattsse/chromiumoxide#49). The vibeterm page already exists, so fetch the
    // existing targets, then wait briefly for the page to register before we
    // enumerate it.
    browser
        .fetch_targets()
        .await
        .map_err(|e| anyhow!("fetching the CDP targets: {e}"))?;
    tokio::time::sleep(Duration::from_millis(150)).await;

    let mut pages = browser
        .pages()
        .await
        .map_err(|e| anyhow!("enumerating the CDP pages: {e}"))?;
    let page = pages.pop().ok_or_else(|| {
        anyhow!("no renderer page found on the CDP endpoint — vibeterm still starting?")
    })?;

    let result = page
        .evaluate(a.expr.as_str())
        .await
        .map_err(|e| anyhow!("evaluating the expression: {e}"))?;

    // `Runtime.evaluate` only materialises a value for JSON-serialisable results
    // (an expression that returns a DOM node, for instance, yields no value).
    match result.value() {
        Some(v) => println!("{v}"),
        None => println!(
            "(no serialisable value — the expression returned an object chrome did not mirror)"
        ),
    }
    Ok(())
}
