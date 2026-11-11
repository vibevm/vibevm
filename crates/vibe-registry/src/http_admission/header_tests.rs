//! Caller-supplied request headers (`PROP-075#DOWNLOAD-HEADERS`) on the
//! managed client: the admission is the same check in the same place, and
//! the headers never leave the origin the address named.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use vibe_core::download_headers::DownloadHeaders;

const SECRET: &str = "Bearer s3cr3t-gateway-token";

fn headers() -> DownloadHeaders {
    DownloadHeaders::parse(&format!("Authorization: {SECRET}\nX-Mirror: eu\n"), "h").unwrap()
}

/// Answer each connection with `reply(n)`, recording every request head.
fn serve(
    listener: TcpListener,
    reply: impl Fn(usize) -> String + Send + 'static,
) -> Arc<Mutex<Vec<String>>> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    std::thread::spawn(move || {
        for (index, stream) in listener.incoming().flatten().enumerate() {
            let mut stream = stream;
            let mut request = Vec::new();
            let mut buffer = [0u8; 2048];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => request.extend_from_slice(&buffer[..read]),
                }
            }
            log.lock()
                .unwrap()
                .push(String::from_utf8_lossy(&request).into_owned());
            let _ = stream.write_all(reply(index).as_bytes());
        }
    });
    seen
}

fn ok(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn moved(location: &str) -> String {
    format!(
        "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

fn port(address: SocketAddr) -> String {
    format!("http://{address}")
}

/// The refusal of a result whose success value does not print.
fn refusal(result: Result<ManagedResponse, HttpGetError>) -> HttpGetError {
    match result {
        Ok(_) => panic!("expected a refusal"),
        Err(error) => error,
    }
}

fn carried(request: &str) -> bool {
    request.to_ascii_lowercase().contains("authorization:")
}

/// A header decides nothing about *whether* an endpoint may be contacted.
/// An excluded host refuses with the same code with and without one, and
/// the socket is never accepted either way.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn an_excluded_host_refuses_the_same_way_with_headers_as_without() {
    let admission = Arc::new(HttpAdmission::new(["example.invalid"]).unwrap());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let socket = listener.local_addr().unwrap();
    let address = format!("http://example.invalid:{}/payload.bin", socket.port());
    let token = vibe_core::cancellation::CancellationToken::default();

    let bound = ManagedHttpClient::bound(Duration::from_millis(100), admission.clone()).unwrap();
    let without = refusal(bound.get_anonymous(&address, &token));
    let direct =
        ManagedHttpClient::direct(Duration::from_millis(100), Some(admission.clone())).unwrap();
    let with = refusal(direct.get_with_headers(&address, &token, &headers()));

    for refusal in [&without, &with] {
        assert!(
            matches!(
                refusal,
                HttpGetError::Refused(ContactRefusal::ExcludedHost { .. })
            ),
            "{refusal:?}"
        );
    }
    assert_eq!(without.to_string(), with.to_string());
    assert!(!format!("{with:?} {with}").contains("s3cr3t"), "{with}");
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock,
        "a refused endpoint is never contacted"
    );
}

/// A redirect that stays on the origin the address named keeps carrying
/// the headers; one that leaves it does not, and leaving is permanent.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-075#DOWNLOAD-HEADERS")]
fn the_headers_travel_within_the_origin_and_never_leave_it() {
    let first = TcpListener::bind("127.0.0.1:0").unwrap();
    let second = TcpListener::bind("127.0.0.1:0").unwrap();
    let home = first.local_addr().unwrap();
    let elsewhere = second.local_addr().unwrap();

    // `/a` redirects within the origin, `/b` out of it, and the last hop
    // comes back home — where the headers are still not re-attached.
    let back = format!("{}/c", port(home));
    let away = format!("{}/b", port(elsewhere));
    let first_seen = serve(first, move |index| match index {
        0 => moved("/a"),
        1 => moved(&away),
        _ => ok("payload"),
    });
    let home_again = back.clone();
    let second_seen = serve(second, move |_| moved(&home_again));

    let client = ManagedHttpClient::direct(Duration::from_secs(5), None).unwrap();
    let response = client
        .get_with_headers(
            &format!("{}/start", port(home)),
            &vibe_core::cancellation::CancellationToken::default(),
            &headers(),
        )
        .unwrap();
    assert!(response.status().is_success());

    let at_home = first_seen.lock().unwrap().clone();
    let away = second_seen.lock().unwrap().clone();
    assert_eq!(at_home.len(), 3, "{at_home:?}");
    assert!(carried(&at_home[0]), "the declared URL gets the headers");
    assert!(carried(&at_home[1]), "a same-origin hop keeps them");
    assert_eq!(away.len(), 1);
    assert!(
        !away[0].contains("s3cr3t") && !carried(&away[0]),
        "another origin gets nothing: {:?}",
        away[0]
    );
    assert!(
        !at_home[2].contains("s3cr3t") && !carried(&at_home[2]),
        "coming back does not re-attach them: {:?}",
        at_home[2]
    );
    // The standing credential headers are stripped with them, and the
    // non-credential one travels no further either.
    assert!(!away[0].to_ascii_lowercase().contains("x-mirror:"));
}

/// A pool that follows its own redirects cannot carry these headers,
/// because nothing would then decide where they may go.
#[test]
fn a_redirect_following_pool_refuses_caller_headers() {
    let client = ManagedHttpClient::legacy(Duration::from_millis(100)).unwrap();
    let error = refusal(client.get_with_headers(
        "http://127.0.0.1:9/payload.bin",
        &vibe_core::cancellation::CancellationToken::default(),
        &headers(),
    ));
    assert!(
        matches!(
            error,
            HttpGetError::Refused(ContactRefusal::UnsupportedControlledRequest)
        ),
        "{error:?}"
    );
}
