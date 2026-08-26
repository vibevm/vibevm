use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use vibe_llm::{
    ApiKey, ChatTransport, Endpoint, EndpointError, OpenAiCompatibleProvider, ProviderError,
    ReqwestChatTransport, TransportError,
};

const HTTPS_PROXY_CHILD: &str = "VIBE_LLM_HTTPS_PROXY_ROUTE_CHILD";

fn spawn_connection_observer(
    listener: TcpListener,
    reply: Option<&'static [u8]>,
) -> (
    mpsc::Sender<()>,
    mpsc::Receiver<Vec<u8>>,
    thread::JoinHandle<()>,
) {
    listener.set_nonblocking(true).unwrap();
    let (stop_tx, stop_rx) = mpsc::channel();
    let (hit_tx, hit_rx) = mpsc::channel();
    let observer = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if stop_rx.try_recv().is_ok() || Instant::now() >= deadline {
                return;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut buffer = [0_u8; 4096];
                    let read = stream.read(&mut buffer).unwrap_or(0);
                    if let Some(reply) = reply {
                        let _ = stream.write_all(reply);
                    }
                    hit_tx.send(buffer[..read].to_vec()).unwrap();
                    return;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("connection observer failed: {error}"),
            }
        }
    });
    (stop_tx, hit_rx, observer)
}

#[test]
fn endpoint_policy_accepts_only_the_credential_safe_matrix() {
    for endpoint in [
        "https://api.example.invalid/v1/chat/completions",
        "https://127.0.0.1/v1/chat/completions",
    ] {
        assert!(Endpoint::parse(endpoint, true).is_ok(), "{endpoint}");
        assert!(Endpoint::parse(endpoint, false).is_ok(), "{endpoint}");
    }

    for endpoint in [
        "http://localhost:11434/v1/chat/completions",
        "http://127.0.0.1:11434/v1/chat/completions",
        "http://127.42.1.9:11434/v1/chat/completions",
        "http://[::1]:11434/v1/chat/completions",
    ] {
        assert!(Endpoint::parse(endpoint, false).is_ok(), "{endpoint}");
        assert_eq!(
            Endpoint::parse(endpoint, true).unwrap_err(),
            EndpointError::KeyRequiresHttps,
            "{endpoint}"
        );
    }
}

#[test]
fn loopback_identity_is_retained_for_direct_http_and_https_routing() {
    for endpoint in [
        "http://localhost:11434/v1/chat/completions",
        "https://localhost/v1/chat/completions",
        "https://127.0.0.1/v1/chat/completions",
        "https://[::1]/v1/chat/completions",
    ] {
        assert!(
            Endpoint::parse(endpoint, false)
                .unwrap()
                .is_literal_loopback(),
            "{endpoint}"
        );
    }
    assert!(
        !Endpoint::parse("https://api.example.invalid/v1/chat/completions", false)
            .unwrap()
            .is_literal_loopback()
    );
}

#[test]
fn https_loopback_bypasses_ambient_proxy_in_production() {
    if let Ok(target_address) = std::env::var(HTTPS_PROXY_CHILD) {
        let endpoint = Endpoint::parse(
            &format!("https://{target_address}/v1/chat/completions"),
            false,
        )
        .unwrap();
        let transport = ReqwestChatTransport::new().unwrap();
        assert_eq!(
            transport.post_json(&endpoint, None, br#"{}"#).unwrap_err(),
            TransportError::RequestFailed
        );
        return;
    }

    let direct_target = TcpListener::bind("127.0.0.1:0").unwrap();
    let target_address = direct_target.local_addr().unwrap();
    let fake_proxy = TcpListener::bind("127.0.0.1:0").unwrap();
    let proxy_address = fake_proxy.local_addr().unwrap();

    let (target_stop, target_hit, target_observer) = spawn_connection_observer(direct_target, None);
    let (proxy_stop, proxy_hit, proxy_observer) = spawn_connection_observer(
        fake_proxy,
        Some(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
    );

    let proxy_url = format!("http://{proxy_address}");
    let output = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("https_loopback_bypasses_ambient_proxy_in_production")
        .arg("--nocapture")
        .env(HTTPS_PROXY_CHILD, target_address.to_string())
        .env("HTTPS_PROXY", &proxy_url)
        .env("https_proxy", &proxy_url)
        .env("ALL_PROXY", &proxy_url)
        .env("all_proxy", &proxy_url)
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .output()
        .unwrap();

    let _ = target_stop.send(());
    let _ = proxy_stop.send(());
    target_observer.join().unwrap();
    proxy_observer.join().unwrap();

    assert!(
        output.status.success(),
        "child failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let direct_bytes = target_hit
        .try_recv()
        .expect("direct HTTPS loopback target observed no connection");
    assert!(
        direct_bytes.len() >= 3 && direct_bytes[0] == 0x16 && direct_bytes[1] == 0x03,
        "direct target did not receive a TLS ClientHello prefix: {direct_bytes:02x?}"
    );
    assert!(
        proxy_hit.try_recv().is_err(),
        "ambient fake proxy observed HTTPS loopback traffic"
    );
}

#[test]
fn endpoint_policy_rejects_loopback_lookalikes_without_dns_resolution() {
    for endpoint in [
        "http://example.invalid/v1/chat/completions",
        "http://localhost.example/v1/chat/completions",
        "http://127.0.0.1.example/v1/chat/completions",
        "http://[::2]/v1/chat/completions",
    ] {
        assert_eq!(
            Endpoint::parse(endpoint, false).unwrap_err(),
            EndpointError::HttpRequiresLiteralLoopback,
            "{endpoint}"
        );
    }
}

#[test]
fn endpoint_policy_rejects_userinfo_fragments_and_other_schemes() {
    for endpoint in [
        "https://user@example.invalid/v1/chat/completions",
        "https://:password@example.invalid/v1/chat/completions",
        "https://@example.invalid/v1/chat/completions",
    ] {
        assert_eq!(
            Endpoint::parse(endpoint, false).unwrap_err(),
            EndpointError::UserInfo,
            "{endpoint}"
        );
    }
    assert_eq!(
        Endpoint::parse(
            "https://example.invalid/v1/chat/completions#credential",
            false
        )
        .unwrap_err(),
        EndpointError::Fragment
    );
    assert_eq!(
        Endpoint::parse("ftp://localhost/v1/chat/completions", false).unwrap_err(),
        EndpointError::UnsupportedScheme
    );
    assert_eq!(
        Endpoint::parse("not a URL", false).unwrap_err(),
        EndpointError::Malformed
    );

    let canary = "url-credential-canary";
    for endpoint in [
        format!("https://{canary}@example.invalid/v1/chat/completions"),
        format!("https://example.invalid/v1/chat/completions#{canary}"),
    ] {
        let error = Endpoint::parse(&endpoint, true).unwrap_err();
        assert!(!format!("{error:?}\n{error}").contains(canary));
    }
}

#[test]
fn public_constructor_and_final_transport_chokepoint_both_block_keyed_http() {
    let endpoint = Endpoint::parse("http://127.0.0.1:9/v1/chat/completions", false).unwrap();
    let key = ApiKey::new("key-never-sent").unwrap();
    let transport = Arc::new(ReqwestChatTransport::new().unwrap());
    assert!(matches!(
        OpenAiCompatibleProvider::new(
            "model",
            endpoint.clone(),
            Some(key.clone()),
            transport.clone()
        ),
        Err(ProviderError::CredentialRequiresHttps)
    ));
    assert_eq!(
        transport
            .post_json(&endpoint, Some(&key), br#"{}"#)
            .unwrap_err(),
        TransportError::CredentialRequiresHttps
    );
}
