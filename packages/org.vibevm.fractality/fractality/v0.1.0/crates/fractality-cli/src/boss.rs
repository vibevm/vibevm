//! The boss-side triage and telemetry verbs (D16/D18): `questions`,
//! `answer`, `stats`.

use fractality_core::run::RunState;
use fractality_mc_client::{ClientError, connect_or_start};

use crate::{EXIT_NEGATIVE, EXIT_OK, err_code, fail_code, out, resolve_run};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// `fractality questions`: the boss's triage inbox (D18).
pub(crate) async fn questions(home: &camino::Utf8Path, json: bool) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    match client.runs(Some(RunState::WaitingOnBoss), None).await {
        Ok(runs) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&runs).expect("runs serialize")
                );
                return EXIT_OK;
            }
            for r in &runs {
                println!(
                    "{} {} {}",
                    r.run_id,
                    fractality_core::time::format_duration_ms(
                        fractality_core::time::now_ms().saturating_sub(r.updated_ts_ms)
                    ),
                    r.question.as_deref().unwrap_or("-"),
                );
            }
            EXIT_OK
        }
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}

/// `fractality answer <id> [<text>|--file <f>]` (D18).
pub(crate) async fn answer(
    home: &camino::Utf8Path,
    raw_id: &str,
    text: Option<&str>,
    file: Option<&camino::Utf8Path>,
) -> u8 {
    let body = match (text, file) {
        (Some(t), None) => t.to_owned(),
        (None, Some(path)) => match std::fs::read_to_string(path.as_std_path()) {
            Ok(t) => t,
            Err(e) => return fail_code(EXIT_NEGATIVE, &format!("reading `{path}`: {e}")),
        },
        _ => {
            return fail_code(
                EXIT_NEGATIVE,
                "give the answer as an argument or with --file",
            );
        }
    };
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let run = match resolve_run(&client, raw_id).await {
        Ok(r) => r,
        Err((code, message)) => return fail_code(code, &message),
    };
    match client.answer(run.run_id, &body).await {
        Ok(r) => {
            println!("{} {}", r.run_id, r.state);
            EXIT_OK
        }
        Err(ClientError::Api {
            status: 409,
            message,
            ..
        }) => fail_code(EXIT_NEGATIVE, &format!("run is not waiting: {message}")),
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}

/// `fractality stats`: a thin client over GET /v0/metrics (D16 — no
/// shadow accounting anywhere else).
pub(crate) async fn stats(home: &camino::Utf8Path, json: bool) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    match client.metrics().await {
        Ok(m) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&m).expect("metrics serialize")
                );
            } else {
                out::print_metrics(&m);
                // The D12 monthly quota rollup (IGNITION §15 leftover,
                // absorbed by C2 Ф2): one consumer sum, engine-rendered.
                let month = fractality_core::time::utc_date_string(fractality_core::time::now_ms())
                    [..7]
                    .to_owned();
                println!(
                    "month {}: {} web-tool calls",
                    month,
                    fractality_initiative::month_web_calls(&m, &month),
                );
            }
            EXIT_OK
        }
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}
