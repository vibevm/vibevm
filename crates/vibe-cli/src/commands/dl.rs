//! `vibe dl` (PROP-068 `#dl`): one file over HTTP(S) through exactly the
//! stack every vibe download uses — the operation's source policy and
//! controlled transport, `[proxy]` / `no_proxy`, the vibe-managed px, the
//! operating system's trust store plus `[proxy] ca_file` — with each phase
//! of the route described and each failure classified with its cause, its
//! fix and a distinct exit code.
//!
//! A diagnostic probe (`vibe_net::probe`) walks the route first: the route
//! decision, DNS on a direct route, the proxy's `CONNECT` answer, the TLS
//! session with the server's chain and the trust source that accepted it,
//! the response head and every redirect. The bytes are then fetched by the
//! operation's controlled transport, not by the probe. Remap rows are not
//! applied: `vibe dl` fetches the URL it is given, so the route it reports
//! is the route of that URL.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#dl");

use specmark::spec;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use vibe_core::cancellation::CancellationToken;
use vibe_core::net::{ModeOrigin, ProxyRoute};
use vibe_core::progress::transfer::{self, Destination, Purpose, TransferRecord};
use vibe_core::user_config::{ProxyMode, UserConfig};
use vibe_net::probe::{FailureClass, ProbeOptions};

use crate::cli::DlArgs;
use crate::output::Context;

mod error;
pub use error::DlError;
mod render;
mod report;
mod test_proxy;
mod write;
use write::{write_file, write_stdout};

/// The whole-request budget of the transfer.
const TIMEOUT: Duration = Duration::from_secs(600);

/// The fix of a `vibe dl` that has nowhere to put the body: the two
/// commands that do (`refusal_of` draws each on its own line).
const SHOW_OR_DEST_FIX: &str = "`vibe dl <url> <dest>`, or `vibe dl <url> --show`";

/// `vibe dl`.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-068#dl")]
pub fn run(ctx: &Context, args: DlArgs, offline: bool, verbose: bool) -> Result<()> {
    if args.test_proxy {
        return test_proxy::run(ctx, &args, offline);
    }
    let raw = args
        .url
        .clone()
        .ok_or_else(|| DlError::usage("a URL is required", SHOW_OR_DEST_FIX))?;
    let url = parse_http(&raw)?;
    if offline {
        return Err(DlError::local(
            "dl_offline",
            3,
            "`vibe dl` needs the network and `--offline` is set",
            "drop `--offline` (or `[net] offline`) for this command",
        )
        .into());
    }
    let to_stdout = args.show || args.headers;
    if to_stdout && args.dest.is_some() {
        return Err(DlError::usage(
            "a destination is not allowed with `--show` / `--headers`",
            "drop the destination, or drop `--show` / `--headers`",
        )
        .into());
    }
    if to_stdout && ctx.is_json() {
        return Err(DlError::usage(
            "`--json` writes the report to stdout; `--show` / `--headers` would mix into it",
            "use `--json` alone, or `--show` / `--headers` alone",
        )
        .into());
    }
    let destination = match (&args.dest, to_stdout) {
        (Some(dest), false) => Some(destination(dest, &url, args.force)?),
        (None, false) => {
            return Err(DlError::usage("a destination is required", SHOW_OR_DEST_FIX).into());
        }
        (_, true) => None,
    };
    let expected = args.sha256.as_deref().map(parse_sha256).transpose()?;
    // A one-shot route for this command, closed (px reaped) when it returns.
    let _route = chosen_route(args.proxy.as_deref(), false, false)?
        .map(|route| vibe_net::enter(&CancellationToken::new(), route));
    let session = vibe_net::current();
    let operation = crate::commands::app::global::source_operation(Default::default())?;
    let admission = operation
        .http_admission()
        .context("establishing the source policy for `vibe dl`")?;
    let admit = |candidate: &reqwest::Url| match &admission {
        Some(admission) => admission
            .check_address(candidate.as_str())
            .map_err(|error| error.to_string()),
        None => Ok(()),
    };
    // A certificate file that cannot be read is a mistake in the
    // configuration, not a failure of this server's TLS: reported here,
    // before a connection is made, so the fix names the key and the file
    // rather than the server's clock (PROP-068 `#proxy-trust`).
    if let Err(error) = session.trust() {
        return Err(error::trust_refusal(&error).into());
    }
    let started = Instant::now();
    let probe = vibe_net::probe::probe(&session, &url, &ProbeOptions::default(), &admit);
    render::phases(ctx, &probe, verbose);
    let token = ctx.progress().cancellation().clone();
    token.checkpoint()?;
    if let Some(failure) = &probe.failure {
        if args.headers
            && let Some(hop) = probe.hops.last()
            && let Some(line) = &hop.status_line
        {
            render::headers(line, &hop.headers);
        }
        let error = DlError::class(failure.class, failure.detail.clone());
        report::emit(ctx, &url, &probe, None, Some(&error))?;
        return Err(error.into());
    }

    let http = operation.http_client(TIMEOUT);
    let mut response = http
        .get_anonymous(url.as_str(), &token)
        .map_err(|error| transfer_error(&error))?;
    token.checkpoint()?;
    let status = response.status();
    if !status.is_success() {
        let class = if status.is_server_error() {
            FailureClass::HttpServer
        } else {
            FailureClass::HttpClient
        };
        let error = DlError::class(class, format!("the server answered {status}"));
        report::emit(ctx, &url, &probe, None, Some(&error))?;
        return Err(error.into());
    }
    if args.headers {
        let line = match probe.hops.last() {
            Some(hop) if hop.status == Some(status.as_u16()) => {
                hop.status_line.clone().unwrap_or_default()
            }
            _ => format!(
                "HTTP/1.1 {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or_default()
            ),
        };
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(name, value)| {
                let value = if name.as_str().eq_ignore_ascii_case("set-cookie") {
                    "<redacted>".to_owned()
                } else {
                    String::from_utf8_lossy(value.as_bytes()).into_owned()
                };
                (name.as_str().to_owned(), value)
            })
            .collect();
        render::headers(&line, &headers);
        if !args.show {
            return Ok(());
        }
    }
    let total = response.content_length();
    let record = TransferRecord::http(Purpose::Other, url.as_str(), Destination::Work).total(total);
    // A body printed to a terminal is its own progress: a live row there
    // would be drawn over the bytes (PROP-067 `#block`).
    let body_on_terminal = destination.is_none() && std::io::stdout().is_terminal();
    let progress = ctx.progress();
    let task = (!body_on_terminal).then(|| {
        progress.task_transferring(format!("transfer  {}", label_of(&url)), record.clone())
    });
    let reader = match &task {
        Some(task) => transfer::begin(record, Some(task)).reader(&mut response),
        None => {
            let gap = crate::output::language::before(
                crate::output::language::Stream::Out,
                crate::output::language::Kind::Block,
            );
            print!("{gap}");
            transfer::begin_beneath(record, None, progress.transfers(), Instant::now())
                .reader(&mut response)
        }
    };
    let outcome = match &destination {
        Some(path) => write_file(reader, path, args.force, expected.as_deref()),
        None => write_stdout(reader, args.force, expected.as_deref()),
    };
    let (bytes, digest) = match outcome {
        Ok(done) => {
            if let Some(task) = &task {
                task.finish();
            }
            done
        }
        Err(error) => {
            if let Some(task) = &task {
                task.fail(error.to_string());
            }
            token.checkpoint()?;
            return Err(error);
        }
    };
    let download = report::Downloaded {
        destination: destination.clone(),
        bytes,
        sha256: digest.clone(),
        status: status.as_u16(),
    };
    report::emit(ctx, &url, &probe, Some(&download), None)?;
    if args.show {
        return Ok(());
    }
    render::done(ctx, &download, started.elapsed());
    Ok(())
}

fn parse_http(raw: &str) -> Result<reqwest::Url, DlError> {
    let url = reqwest::Url::parse(raw).map_err(|error| {
        DlError::local(
            "dl_failed",
            1,
            format!("`{raw}` is not a URL: {error}"),
            "give an `http://` or `https://` URL",
        )
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(DlError::local(
            "dl_failed",
            1,
            format!(
                "`{}` is not an HTTP(S) URL",
                vibe_net::probe::display_url(&url)
            ),
            "give an `http://` or `https://` URL",
        ));
    }
    Ok(url)
}

fn parse_sha256(raw: &str) -> Result<String, DlError> {
    let hex = raw
        .trim()
        .trim_start_matches("sha256:")
        .to_ascii_lowercase();
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(DlError::usage(
            format!("`{raw}` is not a SHA-256 digest"),
            "give 64 hex digits",
        ));
    }
    Ok(hex)
}

/// The URL's last segment, or `index.html` for a bare host.
fn label_of(url: &reqwest::Url) -> String {
    url.path_segments()
        .and_then(|mut segments| segments.next_back().map(str::to_owned))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "index.html".to_owned())
}

/// `dest` itself, or `dest/<last segment>` for an existing directory;
/// refused when it exists and `--force` is absent.
fn destination(dest: &Path, url: &reqwest::Url, force: bool) -> Result<PathBuf, DlError> {
    let path = if dest.is_dir() {
        let name = label_of(url);
        if name.contains(['/', '\\']) || name == ".." {
            return Err(DlError::usage(
                format!("the URL names no usable file name (`{name}`)"),
                "give a file path as the destination",
            ));
        }
        dest.join(name)
    } else {
        dest.to_path_buf()
    };
    if path.exists() && !force {
        return Err(DlError::local(
            "dl_destination_exists",
            17,
            format!("`{}` already exists", path.display()),
            "pass `--force` to replace it, or choose another destination",
        ));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !parent.is_dir() {
        return Err(DlError::local(
            "dl_local_io",
            18,
            format!("the directory `{}` does not exist", parent.display()),
            "create it first, or choose another destination",
        ));
    }
    Ok(path)
}

/// The route this one command uses instead of the configured one; `None`
/// keeps the configuration (with nothing configured: no proxy).
pub(super) fn chosen_route(
    proxy: Option<&str>,
    test: bool,
    no_px: bool,
) -> Result<Option<ProxyRoute>> {
    let mut table = UserConfig::load()
        .context("reading `[proxy]` from the user configuration")?
        .proxy;
    let none = || {
        DlError::usage(
            "no proxy is configured",
            "set `[proxy] pac` / `upstream` (px) or `https` in ~/.vibe/config.toml or the site profile, or pass `--proxy <url>`",
        )
    };
    if no_px {
        let explicit = table
            .https
            .clone()
            .or_else(|| table.http.clone())
            .or_else(|| table.upstream.as_ref().map(|up| format!("http://{up}")));
        let Some(explicit) = explicit else {
            return Err(DlError::usage(
                "`--no-px` needs an explicit or upstream proxy and none is configured",
                "set `[proxy] https` or `upstream`, or drop `--no-px`",
            )
            .into());
        };
        table.mode = Some(ProxyMode::Explicit);
        table.https.get_or_insert(explicit.clone());
        table.http.get_or_insert(explicit);
    } else {
        match proxy {
            None if !test => return Ok(None),
            None if vibe_core::net::route().is_proxied() => return Ok(None),
            None | Some("") | Some("px") => {
                if table.pac.is_some() || table.upstream.is_some() {
                    table.mode = Some(ProxyMode::Px);
                } else if test && (table.https.is_some() || table.http.is_some()) {
                    table.mode = Some(ProxyMode::Explicit);
                } else {
                    return Err(none().into());
                }
            }
            Some(value) if value.starts_with("pac=") => {
                table.mode = Some(ProxyMode::Px);
                table.pac = Some(value["pac=".len()..].to_owned());
                table.upstream = None;
            }
            Some(value) if value.starts_with("http://") || value.starts_with("https://") => {
                table.mode = Some(ProxyMode::Explicit);
                table.https = Some(value.to_owned());
                table.http = Some(value.to_owned());
            }
            Some(value) => {
                return Err(DlError::usage(
                    format!("`--proxy {value}` is not a proxy"),
                    "use `--proxy` (px), `--proxy px`, `--proxy pac=<url>` or `--proxy http://host:port`",
                )
                .into());
            }
        }
    }
    let pac_flag = matches!(proxy, Some(value) if value.starts_with("pac="));
    let route = ProxyRoute::from_config(&table)
        .map_err(|error| DlError::usage(error.to_string(), "correct the `[proxy]` table"))?
        .with_origin(ModeOrigin::Flag);
    Ok(Some(if pac_flag {
        route.with_pac_origin(ModeOrigin::Flag)
    } else {
        route
    }))
}

/// The transfer failed after the probe passed: classify what reqwest saw.
fn transfer_error(error: &vibe_package_source::SourceHttpError) -> anyhow::Error {
    use vibe_registry::http_admission::{ContactRefusal, HttpGetError};
    if let vibe_package_source::SourceHttpError::Transport(HttpGetError::Refused(refusal)) = error {
        if matches!(refusal, ContactRefusal::OperationCancelled) {
            return anyhow::Error::new(vibe_core::cancellation::Cancelled);
        }
        return DlError::class(FailureClass::Policy, refusal.to_string()).into();
    }
    let mut chain = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(next) = source {
        chain.push_str(": ");
        chain.push_str(&next.to_string());
        source = next.source();
    }
    let class = match vibe_net::doctor::cause_of(&chain, chain.contains("timed out")).0 {
        "untrusted-ca" => FailureClass::UntrustedCertificate,
        "proxy-auth" => FailureClass::ProxyAuth,
        "timeout" => FailureClass::ConnectTimeout,
        _ => FailureClass::Protocol,
    };
    DlError::class(class, format!("the transfer failed: {chain}")).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dl_human_usage_fix_names_two_commands() {
        let refusal = crate::output::refusal_of(&anyhow::Error::new(DlError::usage(
            "a destination is required",
            SHOW_OR_DEST_FIX,
        )));
        assert_eq!(
            refusal.fix,
            vec!["vibe dl <url> <dest>", "vibe dl <url> --show"]
        );
        assert_eq!(refusal.code, Some("dl_usage".into()));
    }

    #[test]
    fn text_detection_allows_a_cut_character_and_refuses_nul() {
        assert!(write::is_text(b"hello"));
        assert!(write::is_text(&"h\u{e9}".as_bytes()[..2]));
        assert!(!write::is_text(b"a\0b"));
        assert!(!write::is_text(&[0xff, 0xfe, 0x41]));
    }

    #[test]
    fn digests_are_64_hex_digits() {
        assert!(parse_sha256(&"A".repeat(64)).is_ok());
        assert!(parse_sha256("abc").is_err());
    }

    #[test]
    fn the_error_text_is_a_refusal_with_code_and_fix() {
        let error = DlError::class(FailureClass::ProxyAuth, "the proxy answered 407");
        let text = error.to_string();
        assert!(text.starts_with("dl_proxy_auth: "));
        assert!(text.contains("PROP-068#dl") && text.contains("fix: "));
        assert_eq!(error.exit_code(), 9);
    }
}
