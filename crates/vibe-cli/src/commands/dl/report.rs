//! The `--json` form of `vibe dl` (`vibe-dl/1`, `schemas/dl_report.jtd.json`).

use std::path::PathBuf;

use anyhow::Result;
use vibe_net::probe::Probe;
use vibe_wire::generated::dl_report as wire;

use super::DlError;
use super::test_proxy::CheckRow;
use crate::output::Context;

pub(super) const PROTOCOL: &str = "vibe-dl/1";

/// What a successful transfer fetched.
#[derive(Debug, Clone)]
pub(super) struct Downloaded {
    pub(super) destination: Option<PathBuf>,
    pub(super) bytes: u64,
    pub(super) sha256: String,
    pub(super) status: u16,
}

fn hops(probe: &Probe) -> Vec<wire::Hop> {
    probe
        .hops
        .iter()
        .map(|hop| wire::Hop {
            url: hop.url.clone(),
            via: hop.via.clone(),
            bypassed: hop.bypassed,
            addresses: hop.addresses.clone(),
            connect_status: hop.connect_status.clone(),
            tls: hop.tls.as_ref().map(|tls| wire::Tls {
                version: tls.version.clone(),
                cipher: tls.cipher.clone(),
                chain: tls
                    .chain
                    .iter()
                    .map(|cert| wire::Certificate {
                        subject: cert.subject.clone(),
                        issuer: cert.issuer.clone(),
                    })
                    .collect(),
                trusted_by: tls.trusted_by.map(|source| source.as_str().to_owned()),
            }),
            status_line: hop.status_line.clone(),
            headers: hop
                .headers
                .iter()
                .map(|(name, value)| wire::Header {
                    name: name.clone(),
                    value: value.clone(),
                })
                .collect(),
            timings: wire::Timings {
                dns_ms: hop.timings.dns_ms,
                connect_ms: hop.timings.connect_ms,
                proxy_ms: hop.timings.proxy_ms,
                tls_ms: hop.timings.tls_ms,
                request_ms: hop.timings.request_ms,
            },
        })
        .collect()
}

pub(super) fn build(
    url: &reqwest::Url,
    probe: &Probe,
    download: Option<&Downloaded>,
    failure: Option<&DlError>,
    checks: &[CheckRow],
) -> wire::DlReport {
    wire::DlReport {
        protocol: PROTOCOL.to_owned(),
        ok: failure.is_none(),
        url: vibe_net::probe::display_url(url),
        mode: vibe_net::current().route().mode_name().to_owned(),
        mode_origin: vibe_net::current()
            .route()
            .mode_origin()
            .as_str()
            .to_owned(),
        hops: hops(probe),
        checks: checks
            .iter()
            .map(|row| wire::Check {
                check: row.check.clone(),
                result: row.result.to_owned(),
                detail: row.detail.clone(),
                cause: row.cause.clone(),
                fix: row.fix.clone(),
            })
            .collect(),
        download: download.map(|download| wire::Download {
            destination: download
                .destination
                .as_ref()
                .map(|path| path.display().to_string()),
            bytes: download.bytes,
            sha256: download.sha256.clone(),
            status: download.status,
        }),
        failure: failure.map(|error| wire::Failure {
            class: error.code.to_owned(),
            exit_code: error.exit,
            detail: error.detail.clone(),
            fix: error.fix.clone(),
        }),
    }
}

/// Print the report on stdout under `--json`; nothing otherwise.
pub(super) fn emit(
    ctx: &Context,
    url: &reqwest::Url,
    probe: &Probe,
    download: Option<&Downloaded>,
    failure: Option<&DlError>,
) -> Result<()> {
    if !ctx.is_json() {
        return Ok(());
    }
    ctx.emit_json(&build(url, probe, download, failure, &[]))
}
