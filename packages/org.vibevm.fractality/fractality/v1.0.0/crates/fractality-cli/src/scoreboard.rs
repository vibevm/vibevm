//! `fractality scoreboard` (Campaign 2 D5/D7): the initiative
//! scoreboard, rendered by the pure engine from bus facts. A thin
//! shell: resolve which session to show (explicit flag → the
//! `FRACTALITY_BOSS_SESSION` env this session's adapter exported →
//! global-only), fetch, render.

use fractality_mc_client::connect_or_start;

use crate::{EXIT_OK, err_code, fail_code, session, swarm};

specmark::scope!("spec://fractality/PROP-001#sessions");

pub(crate) async fn scoreboard(
    home: &camino::Utf8Path,
    session_arg: Option<&str>,
    line: bool,
    json: bool,
) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let global = match client.metrics().await {
        Ok(m) => m,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    // Session pick: the explicit flag must resolve (a typo is an
    // error); the ambient env is best-effort (a stale id from another
    // home degrades to the global board, quietly — availability law).
    let session_metrics = match session_arg {
        Some(raw) => match session::resolve_session(&client, raw).await {
            Ok(s) => client.session_metrics(s.session_id).await.ok(),
            Err((code, message)) => return fail_code(code, &message),
        },
        None => match swarm::origin_session_from_env() {
            Some(id) => client.session_metrics(id).await.ok(),
            None => None,
        },
    };

    let now = fractality_core::time::now_ms();
    let today = fractality_core::time::utc_date_string(now);
    let month = today[..7].to_owned();

    if json {
        let doc = serde_json::json!({
            "session": session_metrics,
            "global": global,
            "today": today,
            "month": month,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&doc).expect("scoreboard serializes")
        );
        return EXIT_OK;
    }
    if line {
        match &session_metrics {
            Some(s) => println!("{}", fractality_initiative::render_line(s)),
            None => {
                let t = global.by_day.get(&today);
                println!(
                    "frl: today {} runs · {} completed",
                    t.map_or(0, |b| b.runs),
                    t.map_or(0, |b| b.completed),
                );
            }
        }
        return EXIT_OK;
    }
    // PP-002: the worker-credibility fact, computed from the run snapshot
    // (best-effort — a fetch failure just omits the line, never fails the
    // board). `worker_credibility` returns None unless a completed-green
    // acceptance backs it (D7).
    let credibility = client
        .runs(None, None)
        .await
        .ok()
        .and_then(|runs| fractality_core::worker_credibility(&runs));
    print!(
        "{}",
        fractality_initiative::render_board(
            &global,
            session_metrics.as_ref(),
            credibility.as_ref(),
            now,
            &today,
            &month,
        )
    );
    EXIT_OK
}
