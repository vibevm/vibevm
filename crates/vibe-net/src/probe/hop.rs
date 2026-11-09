//! One request of the probe: the connection (direct or through the
//! proxy), the proxy's `CONNECT`, the TLS session and the response head.

use super::tls::handshake;
use super::*;
use base64::Engine;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

enum Stream {
    Plain(TcpStream),
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>),
}

impl Stream {
    fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.write_all(bytes),
            Self::Tls(s) => s.write_all(bytes),
        }
    }
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            Self::Tls(s) => s.read(buf),
        }
    }
}

pub(super) fn io_class(error: &std::io::Error) -> FailureClass {
    match error.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => {
            FailureClass::ConnectTimeout
        }
        _ => FailureClass::ConnectFailed,
    }
}

fn connect_any(addresses: &[SocketAddr], timeout: Duration) -> Result<TcpStream, std::io::Error> {
    let mut last = std::io::Error::other("no address");
    for address in addresses {
        match TcpStream::connect_timeout(address, timeout) {
            Ok(stream) => return Ok(stream),
            Err(error) => last = error,
        }
    }
    Err(last)
}

fn resolve(host: &str, port: u16) -> Result<Vec<SocketAddr>, std::io::Error> {
    let host = host.trim_start_matches('[').trim_end_matches(']');
    Ok((host, port).to_socket_addrs()?.collect())
}

pub(super) fn one_hop(
    session: &Arc<NetworkSession>,
    url: &url::Url,
    options: &ProbeOptions,
    hop: &mut Hop,
) -> Result<(), Failure> {
    let host = url
        .host_str()
        .ok_or_else(|| fail(FailureClass::Protocol, "the URL has no host"))?
        .to_owned();
    let port = url
        .port_or_known_default()
        .ok_or_else(|| fail(FailureClass::Protocol, "the URL has no port"))?;
    let https = match url.scheme() {
        "https" => true,
        "http" => false,
        other => {
            return Err(fail(
                FailureClass::Protocol,
                format!("`{other}` is not HTTP(S)"),
            ));
        }
    };
    let proxy = session.proxy_for(url).map_err(|error| {
        hop.via = Some("px (not running)".to_owned());
        fail(FailureClass::Px, error.to_string())
    })?;
    hop.bypassed = proxy.is_none() && session.route().is_proxied();
    let started = Instant::now();
    let tcp = match &proxy {
        None => {
            let addresses = resolve(&host, port).map_err(|error| {
                fail(
                    FailureClass::Dns,
                    format!("`{host}` does not resolve: {error}"),
                )
            })?;
            hop.timings.dns_ms = Some(ms(started));
            hop.addresses = addresses.iter().map(|a| a.ip().to_string()).collect();
            if addresses.is_empty() {
                return Err(fail(FailureClass::Dns, format!("`{host}` has no address")));
            }
            let connected = Instant::now();
            let tcp = connect_any(&addresses, options.timeout).map_err(|error| {
                fail(
                    io_class(&error),
                    format!("connecting to {host}:{port}: {error}"),
                )
            })?;
            hop.timings.connect_ms = Some(ms(connected));
            tcp
        }
        Some(proxy) => {
            let shown = if session.route().px().is_some() {
                format!("px {}", vibe_core::net::printable(proxy))
            } else {
                vibe_core::net::printable(proxy)
            };
            hop.via = Some(shown.clone());
            let proxy_host = proxy.host_str().unwrap_or_default().to_owned();
            let proxy_port = proxy.port_or_known_default().unwrap_or(3128);
            let addresses = resolve(&proxy_host, proxy_port).map_err(|error| {
                fail(
                    FailureClass::Dns,
                    format!("the proxy `{proxy_host}` does not resolve: {error}"),
                )
            })?;
            hop.timings.dns_ms = Some(ms(started));
            let connected = Instant::now();
            let tcp = connect_any(&addresses, options.timeout).map_err(|error| {
                fail(
                    io_class(&error),
                    format!("connecting to the proxy {shown}: {error}"),
                )
            })?;
            hop.timings.connect_ms = Some(ms(connected));
            tcp
        }
    };
    let _ = tcp.set_read_timeout(Some(options.timeout));
    let _ = tcp.set_write_timeout(Some(options.timeout));
    let mut tcp = tcp;
    let authority = format!("{host}:{port}");
    if let (Some(proxy), true) = (&proxy, https) {
        let asked = Instant::now();
        let mut request = format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n");
        if let Some(header) = proxy_authorization(proxy) {
            request.push_str(&header);
        }
        request.push_str("\r\n");
        tcp.write_all(request.as_bytes())
            .map_err(|error| fail(io_class(&error), format!("sending CONNECT: {error}")))?;
        let head = read_head(&mut |buf| tcp.read(buf)).map_err(|error| {
            fail(
                io_class(&error),
                format!("reading the CONNECT answer: {error}"),
            )
        })?;
        let line = head.lines().next().unwrap_or_default().trim().to_owned();
        hop.timings.proxy_ms = Some(ms(asked));
        hop.connect_status = Some(line.clone());
        match status_of(&line) {
            Some(200) => {}
            Some(407) => {
                return Err(fail(
                    FailureClass::ProxyAuth,
                    format!("the proxy answered `{line}`"),
                ));
            }
            Some(_) => {
                return Err(fail(
                    FailureClass::ConnectRejected,
                    format!("the proxy answered `{line}` to CONNECT {authority}"),
                ));
            }
            None => {
                return Err(fail(
                    FailureClass::Protocol,
                    "the proxy did not answer CONNECT with HTTP",
                ));
            }
        }
    }
    let mut stream = if https {
        let shaking = Instant::now();
        let (stream, tls) = handshake(session, &host, tcp)?;
        hop.timings.tls_ms = Some(ms(shaking));
        hop.tls = Some(tls);
        Stream::Tls(Box::new(stream))
    } else {
        Stream::Plain(tcp)
    };
    let target = if proxy.is_some() && !https {
        display_url(url)
    } else {
        let mut path = url.path().to_owned();
        if let Some(query) = url.query() {
            path.push('?');
            path.push_str(query);
        }
        path
    };
    let mut request = format!(
        "GET {target} HTTP/1.1\r\nHost: {}\r\nUser-Agent: {}\r\nAccept: */*\r\nConnection: close\r\n",
        host_header(url),
        options.user_agent
    );
    if let (Some(proxy), false) = (&proxy, https)
        && let Some(header) = proxy_authorization(proxy)
    {
        request.push_str(&header);
    }
    request.push_str("\r\n");
    let asked = Instant::now();
    stream.write_all(request.as_bytes()).map_err(|error| {
        fail(
            FailureClass::Protocol,
            format!("sending the request: {error}"),
        )
    })?;
    let head = read_head(&mut |buf| stream.read(buf)).map_err(|error| {
        let class = if proxy.is_some() && !https {
            io_class(&error)
        } else {
            FailureClass::Protocol
        };
        fail(class, format!("reading the response: {error}"))
    })?;
    hop.timings.request_ms = Some(ms(asked));
    // The head is all the probe needs: close at once rather than let the
    // body keep arriving.
    if let Stream::Tls(tls) = &mut stream {
        tls.conn.send_close_notify();
        let _ = tls.conn.write_tls(&mut tls.sock);
        let _ = tls.sock.shutdown(std::net::Shutdown::Both);
    } else if let Stream::Plain(tcp) = &stream {
        let _ = tcp.shutdown(std::net::Shutdown::Both);
    }
    let mut lines = head.lines();
    let line = lines.next().unwrap_or_default().trim().to_owned();
    hop.status = status_of(&line);
    hop.status_line = Some(line.clone());
    hop.headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| {
            let name = name.trim().to_owned();
            let value = if name.eq_ignore_ascii_case("set-cookie") {
                "<redacted>".to_owned()
            } else {
                value.trim().to_owned()
            };
            (name, value)
        })
        .collect();
    if hop.status.is_none() {
        return Err(fail(
            FailureClass::Protocol,
            format!("the server did not answer HTTP (`{line}`)"),
        ));
    }
    // Plain HTTP through a proxy: the proxy speaks for the server.
    if proxy.is_some() && !https && hop.status == Some(407) {
        return Err(fail(
            FailureClass::ProxyAuth,
            format!("the proxy answered `{line}`"),
        ));
    }
    Ok(())
}

fn host_header(url: &url::Url) -> String {
    let host = url.host_str().unwrap_or_default();
    match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    }
}

fn proxy_authorization(proxy: &url::Url) -> Option<String> {
    if proxy.username().is_empty() {
        return None;
    }
    let token = base64::engine::general_purpose::STANDARD.encode(format!(
        "{}:{}",
        percent_decode(proxy.username()),
        percent_decode(proxy.password().unwrap_or_default())
    ));
    Some(format!("Proxy-Authorization: Basic {token}\r\n"))
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = raw.get(i + 1..i + 3)
            && let Ok(value) = u8::from_str_radix(hex, 16)
        {
            out.push(value);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn status_of(line: &str) -> Option<u16> {
    let mut parts = line.split_whitespace();
    let version = parts.next()?;
    if !version.starts_with("HTTP/") {
        return None;
    }
    parts.next()?.parse().ok()
}

/// Read a response head (through `\r\n\r\n`, at most 64 KiB).
fn read_head(read: &mut dyn FnMut(&mut [u8]) -> std::io::Result<usize>) -> std::io::Result<String> {
    let mut data = Vec::new();
    let mut buf = [0u8; 1];
    while !data.ends_with(b"\r\n\r\n") {
        if data.len() > 64 * 1024 {
            return Err(std::io::Error::other("response head larger than 64 KiB"));
        }
        match read(&mut buf) {
            Ok(0) => {
                if data.is_empty() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "the connection closed without an answer",
                    ));
                }
                break;
            }
            Ok(_) => data.push(buf[0]),
            Err(error) => return Err(error),
        }
    }
    Ok(String::from_utf8_lossy(&data).into_owned())
}
