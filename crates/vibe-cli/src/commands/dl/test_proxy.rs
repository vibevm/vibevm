//! `vibe dl --test-proxy [<url>]` (PROP-068 `#dl`): the configured proxy
//! checked against one target, step by step — the effective
//! configuration and where it came from, the bypass list, px (present,
//! started, `px --test`), the route, the proxy's `CONNECT` answer, TLS and
//! the HTTP answer. It continues past a failure where the next step can
//! still be judged, so the table shows every problem at once.

use std::time::{Duration, Instant};

use anyhow::Result;
use vibe_core::cancellation::CancellationToken;
use vibe_net::probe::{FailureClass, ProbeOptions};

use super::render::{self, PhaseRow};
use super::{DlError, chosen_route, parse_http};
use crate::cli::{DEFAULT_TEST_TARGET, DlArgs};
use crate::output::Context;
use crate::output::language::{self, Kind, Meta, Stream};

/// One step of the test.
#[derive(Debug, Clone)]
pub(super) struct CheckRow {
    pub(super) check: String,
    /// `ok`, `fail`, `warn` or `skip`.
    pub(super) result: &'static str,
    pub(super) detail: String,
    pub(super) cause: Option<String>,
    pub(super) fix: Option<String>,
    /// A failure of the proxy path (exit 21), not of the target (exit 22).
    proxy_side: bool,
}

impl CheckRow {
    fn ok(check: &str, detail: impl Into<String>) -> Self {
        Self {
            check: check.to_owned(),
            result: "ok",
            detail: detail.into(),
            cause: None,
            fix: None,
            proxy_side: false,
        }
    }
    fn failed(check: &str, detail: impl Into<String>, class: FailureClass, proxy: bool) -> Self {
        Self {
            check: check.to_owned(),
            result: "fail",
            detail: detail.into(),
            cause: Some(class.code().to_owned()),
            fix: Some(class.fix().to_owned()),
            proxy_side: proxy,
        }
    }
    fn warn(check: &str, detail: impl Into<String>, fix: &str) -> Self {
        Self {
            check: check.to_owned(),
            result: "warn",
            detail: detail.into(),
            cause: None,
            fix: Some(fix.to_owned()),
            proxy_side: false,
        }
    }
}

const PROXY_PROBLEM: u8 = 21;
const TARGET_PROBLEM: u8 = 22;

/// Where the `[proxy]` table came from: the user's file, the site
/// profile, both, or neither.
fn table_origin() -> String {
    let own = vibe_core::user_config::UserConfig::load_own()
        .map(|cfg| !cfg.proxy.is_default())
        .unwrap_or(false);
    let site = vibe_core::site_profile::installed()
        .ok()
        .flatten()
        .is_some_and(|site| !site.profile.proxy.is_default());
    match (own, site) {
        (true, true) => "user config over site profile".into(),
        (true, false) => "user config".into(),
        (false, true) => "site profile".into(),
        (false, false) => "no [proxy] table".into(),
    }
}

pub(super) fn run(ctx: &Context, args: &DlArgs, offline: bool) -> Result<()> {
    let raw = args
        .url
        .clone()
        .unwrap_or_else(|| DEFAULT_TEST_TARGET.to_owned());
    let url = parse_http(&raw)?;
    if offline {
        return Err(DlError::local(
            "dl_offline",
            3,
            "`vibe dl --test-proxy` needs the network and `--offline` is set",
            "drop `--offline` for this command",
        )
        .into());
    }
    if args.dest.is_some() || args.show || args.headers || args.sha256.is_some() {
        return Err(DlError::usage(
            "`--test-proxy` takes only a target URL",
            "drop the destination, `--show`, `--headers` and `--sha256`",
        )
        .into());
    }
    let started = Instant::now();
    let route = chosen_route(args.proxy.as_deref(), true, args.no_px)?;
    let _guard = route.map(|route| vibe_net::enter(&CancellationToken::new(), route));
    let session = vibe_net::current();
    let mut rows = vec![CheckRow::ok(
        "config",
        format!(
            "mode {} ({}) · {}{}{}",
            session.route().mode_name(),
            session.route().mode_origin().as_str(),
            table_origin(),
            match session.route().extra_ca() {
                [] => String::new(),
                paths => format!(
                    " · extra_ca {}",
                    paths
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            },
            // Said only when a layer asked for it: the default row stays
            // exactly as it read before the key existed.
            match session.route().revocation() {
                vibe_core::user_config::RevocationCheck::Check => String::new(),
                other => format!(
                    " · revocation {} ({})",
                    other.as_str(),
                    session
                        .route()
                        .revocation_origin()
                        .label(vibe_core::net::REVOCATION_ENV)
                ),
            }
        ),
    )];
    let host = url.host_str().unwrap_or_default();
    if session.route().bypass().matches(host) {
        rows.push(CheckRow::warn(
            "no_proxy",
            format!("`{host}` bypasses the proxy (no_proxy or loopback)"),
            "test a host outside `no_proxy`",
        ));
    } else {
        rows.push(CheckRow::ok(
            "no_proxy",
            format!("`{host}` is not bypassed"),
        ));
    }
    if let Some(px) = session.route().px() {
        let source = match &px.pac {
            Some(pac) => format!(
                "PAC {} ({})",
                match pac {
                    vibe_core::user_config::PacSource::Url(url) => format!("url {url}"),
                    vibe_core::user_config::PacSource::File(file) => format!("file {file}"),
                    // `system` is resolved by vibe; what it resolved to is
                    // the check's own detail, so the name is enough here.
                    vibe_core::user_config::PacSource::System => "system".to_owned(),
                },
                px.pac_origin.label(vibe_core::net::PAC_ENV)
            ),
            None => format!(
                "no PAC; upstream {}",
                px.upstream.as_deref().unwrap_or("none")
            ),
        };
        match vibe_net::doctor::pac_check(&session, Duration::from_secs(30)) {
            Some(check) if check.status == vibe_net::doctor::Status::Fail => rows.push(CheckRow {
                fix: check.fix.clone(),
                ..CheckRow::failed(
                    "pac",
                    format!("{source} · {}", check.detail),
                    FailureClass::Px,
                    true,
                )
            }),
            Some(check) => rows.push(CheckRow::ok(
                "pac",
                format!(
                    "{source} · {} · answer per target not exposed by px",
                    check.detail
                ),
            )),
            None => rows.push(CheckRow::ok("pac", source)),
        }
    }
    let mut px_usable = true;
    if let Some(px) = vibe_net::probe::px_checks(&session, &url, Duration::from_secs(90)) {
        match &px.program {
            Ok(path) => rows.push(CheckRow::ok("px present", path.display().to_string())),
            Err(reason) => {
                rows.push(CheckRow::failed(
                    "px present",
                    reason,
                    FailureClass::Px,
                    true,
                ));
                px_usable = false;
            }
        }
        match &px.started {
            Some(Ok(address)) => {
                rows.push(CheckRow::ok("px start", format!("listening on {address}")))
            }
            Some(Err(reason)) => {
                rows.push(CheckRow::failed("px start", reason, FailureClass::Px, true));
                px_usable = false;
            }
            None => {}
        }
        match &px.self_test {
            Some(Ok(())) => rows.push(CheckRow::ok("px --test", format!("px reached {url}"))),
            Some(Err(reason)) => rows.push(CheckRow {
                fix: Some(
                    "single sign-on or the PAC route failed: check that you are logged on to the domain, that the PAC URL answers, and `px_args` (`--auth=…`)"
                        .to_owned(),
                ),
                ..CheckRow::failed("px --test", reason, FailureClass::Px, true)
            }),
            None => {}
        }
    }
    let probe = if px_usable {
        let operation = crate::commands::app::global::source_operation(Default::default())?;
        let admission = operation.http_admission()?;
        let admit = |candidate: &reqwest::Url| match &admission {
            Some(admission) => admission
                .check_address(candidate.as_str())
                .map_err(|error| error.to_string()),
            None => Ok(()),
        };
        let probe = vibe_net::probe::probe(&session, &url, &ProbeOptions::default(), &admit);
        rows.extend(probe_checks(&probe));
        Some(probe)
    } else {
        None
    };
    let failed: Vec<CheckRow> = rows
        .iter()
        .filter(|row| row.result == "fail" || row.result == "warn")
        .cloned()
        .collect();
    let hard: Vec<&CheckRow> = failed.iter().filter(|row| row.result == "fail").collect();
    let error = if hard.is_empty() {
        None
    } else {
        let proxy = hard.iter().any(|row| row.proxy_side);
        let first = hard[0];
        Some(DlError {
            code: if proxy {
                "dl_proxy_problem"
            } else {
                "dl_target_problem"
            },
            exit: if proxy { PROXY_PROBLEM } else { TARGET_PROBLEM },
            detail: format!(
                "{} of {} checks failed; first: {} — {}",
                hard.len(),
                rows.len(),
                first.check,
                first.detail
            ),
            fix: first.fix.clone().unwrap_or_default(),
        })
    };
    if ctx.is_json() {
        let empty = vibe_net::probe::Probe::default();
        ctx.emit_json(&super::report::build(
            &url,
            probe.as_ref().unwrap_or(&empty),
            None,
            error.as_ref(),
            &rows,
        ))?;
    } else if !ctx.is_quiet() {
        render::print_rows(ctx, &rows_as_phases(&rows));
        render::check_table(ctx, &failed);
        if error.is_none() {
            let mut meta = vec![Meta::note(vibe_net::probe::display_url(&url))];
            if let Some(probe) = &probe {
                if let Some(via) = probe.hops.first().and_then(|hop| hop.via.clone()) {
                    meta.push(Meta::note(format!("via {via}")));
                }
                if let Some(tls) = probe.hops.last().and_then(|hop| hop.tls.as_ref()) {
                    meta.push(Meta::note(tls.version.clone()));
                }
                if let Some(status) = probe.final_status() {
                    meta.push(Meta::note(status.to_string()));
                }
            }
            meta.push(Meta::Time(started.elapsed()));
            let line = language::trailer(true, "proxy test passed", meta, None);
            let gap = language::before(Stream::Err, Kind::Block);
            ctx.suspend_progress(|| eprintln!("{gap}{line}"));
        }
    }
    match error {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}

/// The probe's phases as check rows, the failed phase classified.
fn probe_checks(probe: &vibe_net::probe::Probe) -> Vec<CheckRow> {
    let mut rows: Vec<CheckRow> = render::probe_rows(probe, false)
        .into_iter()
        .filter(|row| row.ok)
        .map(|row| CheckRow::ok(row.phase, row.text))
        .collect();
    if let Some(failure) = &probe.failure {
        let via_proxy = probe.hops.last().is_some_and(|hop| hop.via.is_some());
        let proxy_side = match failure.class {
            FailureClass::Px | FailureClass::ProxyAuth | FailureClass::ConnectRejected => true,
            FailureClass::Dns | FailureClass::ConnectTimeout | FailureClass::ConnectFailed => {
                via_proxy
            }
            _ => false,
        };
        let check = match (failure.class, via_proxy) {
            (FailureClass::ConnectTimeout | FailureClass::ConnectFailed, true) => "proxy reachable",
            (FailureClass::ConnectTimeout, false) => "direct route",
            (FailureClass::ProxyAuth, _) => "proxy auth",
            (FailureClass::UntrustedCertificate | FailureClass::NameMismatch, _) => "TLS trust",
            (class, _) => render::phase_of(class),
        };
        rows.push(CheckRow::failed(
            check,
            failure.detail.clone(),
            failure.class,
            proxy_side,
        ));
    }
    rows
}

fn rows_as_phases(rows: &[CheckRow]) -> Vec<PhaseRow> {
    rows.iter()
        .map(|row| PhaseRow {
            ok: row.result != "fail",
            phase: leak_free(&row.check),
            text: if row.result == "warn" {
                format!("{} (warning)", row.detail)
            } else {
                row.detail.clone()
            },
            meta: Vec::new(),
            children: Vec::new(),
        })
        .collect()
}

/// The fixed check names as `'static` labels.
fn leak_free(check: &str) -> &'static str {
    match check {
        "config" => "config",
        "no_proxy" => "no_proxy",
        "pac" => "pac",
        "px present" => "px present",
        "px start" => "px start",
        "px --test" => "px --test",
        "route" => "route",
        "redirect" => "redirect",
        "dns" => "dns",
        "connect" => "connect",
        "CONNECT" => "CONNECT",
        "TLS" => "TLS",
        "TLS trust" => "TLS trust",
        "response" => "response",
        "proxy reachable" => "proxy reachable",
        "proxy auth" => "proxy auth",
        "direct route" => "direct route",
        _ => "check",
    }
}
