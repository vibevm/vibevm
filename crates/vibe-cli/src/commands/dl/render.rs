//! The human form of `vibe dl`, in the display language every command
//! uses (PROP-067): the route's phases as one list of rows on stderr, the
//! response head on stdout (a box-drawn table on a colour terminal, plain
//! `Name: value` lines otherwise), the closing trailer.

use std::io::IsTerminal;
use std::time::Duration;

use vibe_net::probe::{FailureClass, Hop, Probe};

use super::report::Downloaded;
use crate::output::Context;
use crate::output::language::{self, Block, Glyph, Kind, Meta, Stream, Table};

const LIST: &str = "dl-phases";

fn ms(value: Option<u64>) -> Option<Meta> {
    value.map(|ms| Meta::note(format!("{ms} ms")))
}

/// Which phase a failure class stops at.
pub(super) fn phase_of(class: FailureClass) -> &'static str {
    match class {
        FailureClass::Policy | FailureClass::Px => "route",
        FailureClass::Dns => "dns",
        FailureClass::ConnectTimeout | FailureClass::ConnectFailed => "connect",
        FailureClass::ProxyAuth | FailureClass::ConnectRejected => "CONNECT",
        FailureClass::UntrustedCertificate | FailureClass::NameMismatch | FailureClass::Tls => {
            "TLS"
        }
        FailureClass::HttpClient
        | FailureClass::HttpServer
        | FailureClass::Redirect
        | FailureClass::Protocol => "response",
    }
}

/// One row of the phase list: `(glyph, phase, text, meta, children)`.
pub(super) struct PhaseRow {
    pub(super) ok: bool,
    pub(super) phase: &'static str,
    pub(super) text: String,
    pub(super) meta: Vec<Meta>,
    pub(super) children: Vec<String>,
}

/// The rows of one hop.
pub(super) fn hop_rows(probe: &Probe, index: usize, hop: &Hop, verbose: bool) -> Vec<PhaseRow> {
    let mut rows = Vec::new();
    let row = |phase, text: String, meta: Vec<Option<Meta>>| PhaseRow {
        ok: true,
        phase,
        text,
        meta: meta.into_iter().flatten().collect(),
        children: Vec::new(),
    };
    let route = match (&hop.via, hop.bypassed) {
        (Some(via), _) => format!("{} via {via}", hop.url),
        (None, true) => format!("{} direct (no_proxy)", hop.url),
        (None, false) => format!("{} direct", hop.url),
    };
    let mut first = row(if index == 0 { "route" } else { "redirect" }, route, vec![]);
    if index == 0 {
        first.meta.push(Meta::note(format!(
            "mode {} ({})",
            probe.mode,
            vibe_net::current().route().mode_origin().as_str()
        )));
    }
    rows.push(first);
    if !hop.addresses.is_empty() {
        rows.push(row(
            "dns",
            hop.addresses.join(", "),
            vec![ms(hop.timings.dns_ms)],
        ));
    }
    if let Some(connect) = hop.timings.connect_ms {
        rows.push(row(
            "connect",
            if hop.via.is_some() {
                "proxy reachable".to_owned()
            } else {
                "connected".to_owned()
            },
            vec![ms(Some(connect))],
        ));
    }
    if let Some(status) = &hop.connect_status {
        rows.push(row(
            "CONNECT",
            status.clone(),
            vec![ms(hop.timings.proxy_ms)],
        ));
    }
    if let Some(tls) = &hop.tls {
        let mut tls_row = row(
            "TLS",
            format!(
                "{} · {}",
                tls.version,
                tls.trusted_by.as_ref().map_or_else(
                    || "trust source unknown".to_owned(),
                    |source| source.describe()
                )
            ),
            vec![
                tls.chain
                    .first()
                    .map(|leaf| Meta::note(leaf.subject.clone())),
                ms(hop.timings.tls_ms),
            ],
        );
        if verbose {
            tls_row.children = tls
                .chain
                .iter()
                .map(|cert| format!("{}  ← issued by {}", cert.subject, cert.issuer))
                .collect();
            tls_row.children.push(format!("cipher {}", tls.cipher));
        }
        rows.push(tls_row);
    }
    if let Some(status) = &hop.status_line {
        let mut response = row("response", status.clone(), vec![ms(hop.timings.request_ms)]);
        if verbose && index + 1 < probe.hops.len() {
            response.children = hop
                .headers
                .iter()
                .map(|(name, value)| format!("{name}: {value}"))
                .collect();
        }
        rows.push(response);
    }
    rows
}

/// Every row of the probe, the failed phase last.
pub(super) fn probe_rows(probe: &Probe, verbose: bool) -> Vec<PhaseRow> {
    let mut rows: Vec<PhaseRow> = probe
        .hops
        .iter()
        .enumerate()
        .flat_map(|(index, hop)| hop_rows(probe, index, hop, verbose))
        .collect();
    if let Some(failure) = &probe.failure {
        let phase = phase_of(failure.class);
        // A failed response is the last row already; mark it.
        match rows.last_mut() {
            Some(last) if last.phase == phase => last.ok = false,
            _ => rows.push(PhaseRow {
                ok: false,
                phase,
                text: failure.class.label().to_owned(),
                meta: Vec::new(),
                children: Vec::new(),
            }),
        }
    }
    rows
}

pub(super) fn print_rows(ctx: &Context, rows: &[PhaseRow]) {
    let width = rows.iter().map(|row| row.phase.len()).max().unwrap_or(0);
    for row in rows {
        let glyph = if row.ok { Glyph::Done } else { Glyph::Failed };
        let mut block = Block::new(glyph, format!("{:<width$}  {}", row.phase, row.text))
            .with_meta(row.meta.clone());
        if !row.children.is_empty() {
            block = block.with_body(
                row.children
                    .iter()
                    .cloned()
                    .map(language::Line::Child)
                    .collect(),
            );
        }
        let text = block.render();
        let gap = language::before(Stream::Err, Kind::Row(LIST));
        ctx.suspend_progress(|| eprintln!("{gap}{text}"));
    }
}

/// The probe's phases on stderr (human modes only).
pub(super) fn phases(ctx: &Context, probe: &Probe, verbose: bool) {
    if ctx.is_json() || ctx.is_quiet() {
        return;
    }
    print_rows(ctx, &probe_rows(probe, verbose));
}

/// Whether stdout gets the decorated form: a colour terminal.
pub(super) fn decorated_stdout() -> bool {
    std::io::stdout().is_terminal() && language::colour_on()
}

/// The response head on stdout.
pub(super) fn headers(status_line: &str, headers: &[(String, String)]) {
    // On one screen the head is a block after the phase rows; a redirected
    // stdout starts with it, no blank line.
    let gap = language::before(Stream::Out, Kind::Block);
    print!(
        "{gap}{}",
        headers_text(status_line, headers, decorated_stdout())
    );
}

/// The response head as text: on a colour terminal the status line and a
/// two-column table (`Header`, `Value`) drawn by `output::language`; otherwise the status line and `Name: value`
/// lines, then the blank line that ends an HTTP head.
pub(super) fn headers_text(
    status_line: &str,
    headers: &[(String, String)],
    decorated: bool,
) -> String {
    if decorated {
        let mut table = Table::new(vec!["Header".into(), "Value".into()]);
        for (name, value) in headers {
            table = table.row(vec![name.clone(), value.clone()]);
        }
        format!("{status_line}\n{}\n\n", table.render())
    } else {
        let mut out = format!("{status_line}\n");
        for (name, value) in headers {
            out.push_str(&format!("{name}: {value}\n"));
        }
        out.push('\n');
        out
    }
}

/// The closing trailer of a file download, on stderr.
pub(super) fn done(ctx: &Context, download: &Downloaded, elapsed: Duration) {
    if ctx.is_json() || ctx.is_quiet() {
        return;
    }
    let what = match &download.destination {
        Some(path) => format!("downloaded {}", path.display()),
        None => "downloaded".to_owned(),
    };
    let line = language::trailer(
        true,
        &what,
        vec![
            Meta::Count(language::bytes(download.bytes)),
            Meta::note(format!("sha256 {}", download.sha256)),
            Meta::Time(elapsed),
        ],
        None,
    );
    let gap = language::before(Stream::Err, Kind::Block);
    ctx.suspend_progress(|| eprintln!("{gap}{line}"));
}

/// Plain `check: result — detail; cause; fix` lines (no terminal, NO_COLOR,
/// `--plain`).
pub(super) fn check_lines(rows: &[super::test_proxy::CheckRow]) -> String {
    rows.iter()
        .map(|row| {
            let mut line = format!("{}: {} — {}", row.check, row.result, row.detail);
            if let Some(cause) = &row.cause {
                line.push_str(&format!("; cause {cause}"));
            }
            if let Some(fix) = &row.fix {
                line.push_str(&format!("; fix: {fix}"));
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The failed and suspicious checks as a table with the columns `Check`,
/// `Result`, `Cause` and `Fix` (drawn by `output::language`), on stderr.
pub(super) fn check_table(ctx: &Context, rows: &[super::test_proxy::CheckRow]) {
    if ctx.is_json() || ctx.is_quiet() || rows.is_empty() {
        return;
    }
    let decorated = std::io::stderr().is_terminal() && language::colour_on();
    let text = if decorated {
        let mut table = Table::new(vec![
            "Check".into(),
            "Result".into(),
            "Cause".into(),
            "Fix".into(),
        ]);
        for row in rows {
            table = table.row(vec![
                row.check.clone(),
                format!("{} — {}", row.result, row.detail),
                row.cause.clone().unwrap_or_default(),
                row.fix.clone().unwrap_or_default(),
            ]);
        }
        table.render()
    } else {
        check_lines(rows)
    };
    let gap = language::before(Stream::Err, Kind::Block);
    ctx.suspend_progress(|| eprintln!("{gap}{text}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_headers_are_name_value_lines_ending_in_a_blank_line() {
        let text = headers_text(
            "HTTP/1.1 200 OK",
            &[("content-type".into(), "text/plain".into())],
            false,
        );
        assert_eq!(text, "HTTP/1.1 200 OK\ncontent-type: text/plain\n\n");
    }
}
