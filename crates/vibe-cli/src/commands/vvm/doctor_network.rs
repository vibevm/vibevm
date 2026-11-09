//! `vibe self doctor`'s network diagnosis (`PROP-068#proxy`): px present
//! and started → `px --test` → `CONNECT` through the proxy → TLS with the
//! trust set. Runs when `[proxy]` configures a route or a CA file, or when
//! `--network <url>` asks for it; each failed step names its cause and fix.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#proxy");

use std::time::Duration;
use vibe_net::doctor::{Check, Status};

/// Where the diagnosis connects when `--network` names no URL: the host the
/// public release source and registries live on.
pub(super) const DEFAULT_TARGET: &str = "https://github.com/";

/// The diagnosed steps, or `None` when there is nothing to diagnose (no
/// route, no CA file, no `--network`).
pub(super) fn diagnose(requested: Option<&str>) -> anyhow::Result<Option<Vec<Check>>> {
    let session = vibe_net::current();
    let route = session.route();
    if requested.is_none() && !route.is_proxied() && route.ca_file().is_none() {
        return Ok(None);
    }
    let raw = requested
        .filter(|url| !url.is_empty())
        .unwrap_or(DEFAULT_TARGET);
    let target = reqwest::Url::parse(raw)
        .map_err(|error| anyhow::anyhow!("`--network {raw}` is not a URL: {error}"))?;
    Ok(Some(vibe_net::doctor::diagnose(
        &session,
        &target,
        Duration::from_secs(20),
    )))
}

/// The "Checking the network route" task under `parent`: the diagnosed
/// steps, or `None` when nothing asks for a diagnosis.
pub(super) fn run(
    parent: &vibe_core::progress::Progress,
    requested: Option<&str>,
) -> anyhow::Result<Option<Vec<Check>>> {
    let task = parent.task("Checking the network route");
    match diagnose(requested) {
        Ok(Some(found)) => {
            match failures(&found) {
                0 => task.finish(),
                n => task.fail(format!("{n} network problem(s)")),
            }
            Ok(Some(found))
        }
        Ok(None) => {
            task.skip("no `[proxy]` route (pass `--network` to check anyway)");
            Ok(None)
        }
        Err(error) => {
            task.fail(error.to_string());
            Err(error)
        }
    }
}

/// One human line per step.
pub(super) fn line(check: &Check) -> String {
    let mark = match check.status {
        Status::Ok => "ok  ",
        Status::Fail => "MISS",
        Status::Skip => "-   ",
    };
    match &check.fix {
        Some(fix) => format!(
            "{mark} network {}: {} — fix: {fix}",
            check.step, check.detail
        ),
        None => format!("{mark} network {}: {}", check.step, check.detail),
    }
}

pub(super) fn json(checks: &[Check]) -> serde_json::Value {
    serde_json::Value::Array(
        checks
            .iter()
            .map(|check| {
                serde_json::json!({
                    "step": check.step,
                    "status": check.status.as_str(),
                    "detail": check.detail,
                    "fix": check.fix,
                })
            })
            .collect(),
    )
}

pub(super) fn failures(checks: &[Check]) -> usize {
    checks
        .iter()
        .filter(|check| check.status == Status::Fail)
        .count()
}
