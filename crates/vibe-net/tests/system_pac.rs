//! `[proxy] pac = "system"` end to end (`PROP-068#proxy-system`): the
//! Windows settings are read by vibe, px is started with the explicit
//! `--pac=<url>` that came out, px fetches that PAC, and a request
//! through the session arrives at the proxy the PAC named.
//!
//! Everything is loopback and nothing touches this machine's own
//! settings: the Windows reader is replaced through the
//! [`vibe_core::net::system_proxy`] seam, the PAC is served by a thread
//! in this process, and the proxy the PAC names is another thread that
//! answers the one request. Windows only, because the reader this case
//! replaces is the Windows one and the run needs a real px; the three
//! resolution orders themselves — Windows, macOS and the Linux desktops
//! — are tested on every host in `vibe-core`, the px arguments in
//! `vibe-net`'s own unit tests, and the whole `vibe` over a stated
//! desktop configuration in `vibe-cli`'s `cli_self_apply`.
#![cfg(windows)]

use std::{
    io::{BufRead, BufReader, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpListener},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use vibe_core::{
    net::{IeProxyConfig, ProxyRoute, WindowsProxySource, system_proxy},
    user_config::ProxyConfig,
};
use vibe_net::NetworkSession;

/// The settings this fake Windows machine carries: an auto-config URL and
/// nothing else, the shape a managed desktop has.
#[derive(Debug)]
struct Settings(String);

impl WindowsProxySource for Settings {
    fn policy_auto_config_url(&self, _machine: bool) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn ie_proxy_config(&self) -> Result<Option<IeProxyConfig>, String> {
        Ok(Some(IeProxyConfig {
            auto_config_url: Some(self.0.clone()),
            ..Default::default()
        }))
    }
    fn auto_config_url_value(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn default_connection_settings(&self) -> Result<Option<Vec<u8>>, String> {
        Ok(None)
    }
    fn detect_auto_proxy_url(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
}

/// The px to run: `VIBEVM_TEST_PX` when it names one, else the one this
/// installation carries or has on PATH. `None` skips the test — the
/// companion tool is not part of a source checkout
/// (`vibe self px install`, `docs/admin/closed-network.md` §5.2).
fn px() -> Option<PathBuf> {
    if let Some(named) = std::env::var_os("VIBEVM_TEST_PX") {
        let path = PathBuf::from(named);
        return path.is_file().then_some(path);
    }
    vibe_net::bundled_px_candidates()
        .into_iter()
        .chain(vibe_net::px_store::stored_executable())
        .find(|candidate| candidate.is_file())
        .or_else(|| {
            std::env::split_paths(&std::env::var_os("PATH")?)
                .map(|dir| dir.join("px.exe"))
                .find(|candidate| candidate.is_file())
        })
}

/// A loopback server that answers every connection with `response` and
/// counts what it was asked for. Returns its port and the counter.
fn serve(name: &'static str, response: &'static str) -> (u16, Arc<AtomicUsize>) {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let asked = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&asked);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(&stream);
            let mut request = String::new();
            // The request line, then the headers up to the blank one.
            let _ = reader.read_line(&mut request);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) if line.trim().is_empty() => break,
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            eprintln!("{name} asked: {}", request.trim());
            counter.fetch_add(1, Ordering::SeqCst);
            let mut stream = stream;
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (port, asked)
}

#[test]
fn px_is_started_with_the_resolved_pac_and_the_request_follows_it() {
    let Some(px) = px() else {
        eprintln!("skipped: no px on this machine (`vibe self px install`)");
        return;
    };

    // The proxy the PAC will name: it answers the one absolute-form
    // request px forwards to it.
    let (proxy_port, proxied) = serve(
        "proxy",
        "HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\nhello-proxy",
    );
    // The PAC file itself, served over loopback. A leaked `{` would end
    // the format string, so the body is written without one.
    let pac_body = format!(
        "function FindProxyForURL(url, host) {{ return \"PROXY 127.0.0.1:{proxy_port}\"; }}"
    );
    let pac_body: &'static str = Box::leak(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-ns-proxy-autoconfig\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{pac_body}",
            pac_body.len()
        )
        .into_boxed_str(),
    );
    let (pac_port, pac_asked) = serve("pac", pac_body);
    let pac_url = format!("http://127.0.0.1:{pac_port}/proxy.pac");

    // The Windows settings name that PAC, and nothing else.
    system_proxy::install_source(Some(Arc::new(Settings(pac_url.clone()))));
    let resolved = vibe_net::system_proxy::resolve().unwrap();
    assert_eq!(resolved.pac_url.as_deref(), Some(pac_url.as_str()));

    let cfg: ProxyConfig = toml::from_str(&format!(
        "mode = 'px'\npac = 'system'\npx_path = '{}'\n",
        px.display().to_string().replace('\\', "/")
    ))
    .unwrap();
    let session = NetworkSession::new(ProxyRoute::from_config(&cfg).unwrap());

    // A name outside loopback, so the route is not bypassed; it never has
    // to resolve, because px hands the request to the PAC's proxy by name.
    let target = url::Url::parse("http://vibe-fixture.test/hello").unwrap();
    let address = session
        .proxy_for(&target)
        .expect("px starts with the resolved --pac")
        .expect("a non-bypassed host goes through px");
    assert_eq!(address.host_str(), Some("127.0.0.1"));

    let body = reqwest::blocking::Client::builder()
        .proxy(reqwest::Proxy::all(address.as_str()).unwrap())
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap()
        .get(target)
        .send()
        .unwrap()
        .text()
        .unwrap();

    system_proxy::install_source(None);
    drop(session);

    assert_eq!(body, "hello-proxy");
    assert!(pac_asked.load(Ordering::SeqCst) > 0, "px fetched the PAC");
    assert!(
        proxied.load(Ordering::SeqCst) > 0,
        "the request went to the proxy the PAC named"
    );
}
