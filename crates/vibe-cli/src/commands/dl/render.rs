//! The human form of `vibe dl`, in the display language every command
//! uses (PROP-067): the route's phases as one list of rows on stderr, the
//! response head on stdout (a rule and rows on a colour terminal, plain
//! `Name: value` lines otherwise), the closing trailer.

use std::io::IsTerminal;
use std::time::Duration;

use vibe_net::probe::{FailureClass, Hop, Probe};

use super::report::Downloaded;
use crate::output::Context;
use crate::output::language::{self, Block, Cell, Glyph, Kind, List, Meta, Row, Stream};

const LIST: &str = "dl-phases";

fn shown(phase: &str) -> &str {
    match phase {
        "TLS" => "tls",
        "TLS trust" => "tls trust",
        other => other,
    }
}

fn took(value: Option<u64>) -> Option<Meta> {
    value.map(|ms| Meta::Time(Duration::from_millis(ms)))
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
    hop_rows_with_origin(
        probe,
        index,
        hop,
        verbose,
        vibe_net::current().route().mode_origin().as_str(),
    )
}

// Keep synthetic row tests independent of the process's installed network route.
fn hop_rows_with_origin(
    probe: &Probe,
    index: usize,
    hop: &Hop,
    verbose: bool,
    mode_origin: &str,
) -> Vec<PhaseRow> {
    let mut rows = Vec::new();
    let row = |phase, text: String, meta: Vec<Option<Meta>>| PhaseRow {
        ok: true,
        phase,
        text,
        meta: meta.into_iter().flatten().collect(),
        children: Vec::new(),
    };
    let route = match &hop.via {
        Some(via) => format!("{} via {via}", hop.url),
        None => hop.url.clone(),
    };
    let mut first = row(if index == 0 { "route" } else { "redirect" }, route, vec![]);
    if index == 0 {
        first.meta.push(Meta::note(format!("mode {}", probe.mode)));
        first.meta.push(Meta::note(mode_origin));
    }
    if hop.bypassed {
        first.meta.push(Meta::State("no_proxy".into()));
    } else if index == 0 && hop.via.is_none() {
        first.meta.push(Meta::State("direct".into()));
    }
    rows.push(first);
    if !hop.addresses.is_empty() {
        rows.push(row(
            "dns",
            hop.addresses.join(", "),
            vec![took(hop.timings.dns_ms)],
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
            vec![took(Some(connect))],
        ));
    }
    if let Some(status) = &hop.connect_status {
        rows.push(row(
            "CONNECT",
            status.clone(),
            vec![took(hop.timings.proxy_ms)],
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
                took(hop.timings.tls_ms),
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
        let mut response = row("response", status.clone(), vec![took(hop.timings.request_ms)]);
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
    let width = rows.iter().map(|row| shown(row.phase).len()).max().unwrap_or(0);
    for row in rows {
        let glyph = if row.ok { Glyph::Done } else { Glyph::Failed };
        let mut block = Block::new(glyph, format!("{:<width$}  {}", shown(row.phase), row.text))
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
    let mut rows = probe_rows(probe, verbose);
    let tls_hops = probe.hops.iter().filter(|hop| hop.tls.is_some());
    for (row, hop) in rows
        .iter_mut()
        .filter(|row| row.phase == "TLS" && row.ok)
        .zip(tls_hops)
    {
        if let Some(tls) = &hop.tls {
            row.text = tls.version.clone();
            row.meta = tls
                .chain
                .first()
                .map(|leaf| Meta::note(leaf.subject.clone()))
                .into_iter()
                .chain(took(hop.timings.tls_ms))
                .chain(Some(Meta::State(tls.trusted_by.as_ref().map_or_else(
                    || "trust source unknown".to_owned(),
                    |source| source.describe(),
                ))))
                .collect();
        }
    }
    print_rows(ctx, &rows);
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
        headers_text(
            status_line,
            headers,
            decorated_stdout(),
            language::rule_width()
        )
    );
}

/// The response head as text: on a colour terminal a labelled rule and
/// measured rows drawn by `output::language`; otherwise the status line and `Name: value`
/// lines, then the blank line that ends an HTTP head.
pub(super) fn headers_text(
    status_line: &str,
    headers: &[(String, String)],
    decorated: bool,
    width: usize,
) -> String {
    if decorated {
        let rows = headers
            .iter()
            .map(|(name, value)| Row::new(vec![Cell::text(name), Cell::text(value)]))
            .collect();
        format!(
            "{}\n\n{}\n",
            language::rule_at(&format!("response head · {status_line}"), None, width),
            List::new(rows).render_at(width)
        )
    } else {
        let mut out = format!("{status_line}\n");
        for (name, value) in headers {
            out.push_str(&format!("{name}: {value}\n"));
        }
        out.push('\n');
        out
    }
}

/// The closing trailer of a file download: what was written, its size and
/// the time, then the digest on a dim line of its own; quiet keeps the
/// first line only.
fn done_text(download: &Downloaded, elapsed: Duration, quiet: bool) -> String {
    let what = match &download.destination {
        Some(path) => format!("downloaded {}", path.display()),
        None => "downloaded".to_owned(),
    };
    let line = language::trailer(
        true,
        &what,
        vec![
            Meta::Count(language::bytes(download.bytes)),
            Meta::Time(elapsed),
        ],
        None,
    );
    if quiet {
        return line;
    }
    format!(
        "{line}\n  {}",
        language::hint(&format!("sha256 {}", download.sha256))
    )
}

/// The closing trailer of a file download, on stderr.
pub(super) fn done(ctx: &Context, download: &Downloaded, elapsed: Duration) {
    if ctx.is_json() {
        return;
    }
    let line = done_text(download, elapsed, ctx.is_quiet());
    let gap = if ctx.is_quiet() {
        ""
    } else {
        language::before(Stream::Err, Kind::Block)
    };
    ctx.suspend_progress(|| eprintln!("{gap}{line}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dl_rows_route_names_mode_and_origin() {
        let probe = Probe {
            mode: "direct",
            hops: vec![Hop {
                url: "https://h/a".into(),
                ..Default::default()
            }],
            failure: None,
        };
        let rows = hop_rows_with_origin(&probe, 0, &probe.hops[0], false, "default");
        assert_eq!(rows[0].text, "https://h/a");
        assert_eq!(rows[0].meta.len(), 3);
        assert_eq!(rows[0].meta[0], Meta::note("mode direct"));
        assert_eq!(rows[0].meta[1], Meta::note("default"));
        assert_eq!(rows[0].meta.last(), Some(&Meta::State("direct".into())));
    }

    #[test]
    fn dl_rows_bypassed_route_carries_no_proxy_state() {
        let probe = Probe {
            mode: "explicit",
            hops: vec![Hop {
                url: "https://h/a".into(),
                bypassed: true,
                ..Default::default()
            }],
            failure: None,
        };
        let rows = hop_rows_with_origin(&probe, 0, &probe.hops[0], false, "user config");
        assert_eq!(rows[0].meta.last(), Some(&Meta::State("no_proxy".into())));
    }

    #[test]
    fn dl_rows_times_use_the_elapsed_form() {
        let probe = Probe {
            hops: vec![Hop {
                addresses: vec!["127.0.0.1".into()],
                timings: vibe_net::probe::Timings {
                    dns_ms: Some(1500),
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        };
        let rows = hop_rows_with_origin(&probe, 0, &probe.hops[0], false, "default");
        let meta = &rows.iter().find(|row| row.phase == "dns").unwrap().meta;
        assert_eq!(*meta, vec![Meta::Time(Duration::from_millis(1500))]);
        assert_eq!(language::join_meta(meta), " · 1s");
    }

    #[test]
    fn dl_rows_tls_label_is_lower_case() {
        assert_eq!(shown("TLS"), "tls");
        assert_eq!(shown("CONNECT"), "CONNECT");
        assert_eq!(shown("route"), "route");
    }

    #[test]
    fn dl_human_head_is_a_rule_and_rows() {
        let text = headers_text(
            "HTTP/1.1 200 OK",
            &[
                ("server".into(), "x".into()),
                ("content-length".into(), "6".into()),
            ],
            true,
            40,
        );
        assert_eq!(
            text,
            "── response head · HTTP/1.1 200 OK ─────\n\n  server          x\n  content-length  6\n"
        );
    }

    #[test]
    fn dl_human_digest_on_its_own_line() {
        let download = Downloaded {
            destination: Some("/o/f.txt".into()),
            bytes: 6,
            sha256: "ab".repeat(32),
            status: 200,
        };
        assert_eq!(
            done_text(&download, Duration::ZERO, false),
            format!("✻ downloaded /o/f.txt · 6 B · 0s\n  sha256 {}", "ab".repeat(32))
        );
    }

    #[test]
    fn dl_human_quiet_trailer_is_one_line() {
        let download = Downloaded {
            destination: Some("/o/f.txt".into()),
            bytes: 6,
            sha256: "ab".repeat(32),
            status: 200,
        };
        assert_eq!(
            done_text(&download, Duration::ZERO, true),
            "✻ downloaded /o/f.txt · 6 B · 0s"
        );
    }

    #[test]
    fn plain_headers_are_name_value_lines_ending_in_a_blank_line() {
        let text = headers_text(
            "HTTP/1.1 200 OK",
            &[("content-type".into(), "text/plain".into())],
            false,
            80,
        );
        assert_eq!(text, "HTTP/1.1 200 OK\ncontent-type: text/plain\n\n");
    }
}
