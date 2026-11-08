//! Whole-chain observations using the existing numeric-loopback fixture.
use super::*;

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn observed_return_to_initial_origin_keeps_cross_origin_strip_sticky() {
    use reqwest::header::{AUTHORIZATION, COOKIE, PROXY_AUTHORIZATION};
    let origin = Arc::new(std::sync::OnceLock::<String>::new());
    let copy = origin.clone();
    let b = Server::new("127.0.0.2", move |request, _| {
        assert!(!request.to_ascii_lowercase().contains("authorization:"));
        assert!(!request.to_ascii_lowercase().contains("cookie:"));
        response(
            302,
            "",
            &format!("Location: {}/return\r\n", copy.get().unwrap()),
        )
    });
    let other = b.url.clone();
    let a = Server::new("127.0.0.1", move |request, n| {
        let request = request.to_ascii_lowercase();
        assert!(!request.contains("referer:"));
        if n < 2 {
            assert!(request.contains("authorization: fixture"));
            assert!(request.contains("cookie: fixture"));
        } else {
            assert!(!request.contains("authorization:"));
            assert!(!request.contains("cookie:"));
        }
        match n {
            0 => response(302, "", "Location: /same-origin\r\n"),
            1 => response(302, "", &format!("Location: {other}/cross-origin\r\n")),
            _ => response(200, "ok", ""),
        }
    });
    origin.set(a.url.clone()).unwrap();
    let client = crate::http_admission::bound_builder(reqwest::blocking::Client::builder())
        .build()
        .unwrap();
    let mut request = client.get(&a.url);
    for key in [AUTHORIZATION, COOKIE, PROXY_AUTHORIZATION] {
        request = request.header(key, "fixture");
    }
    let observed =
        crate::http_admission::get_observed(&client, request, &a.url, &binding(&[])).unwrap();
    assert!(observed.crossed_origin);
    assert_eq!(observed.redirects_followed, 3);
    assert_eq!(
        observed.response.url().origin(),
        reqwest::Url::parse(&a.url).unwrap().origin()
    );
    assert_eq!(observed.response.text().unwrap(), "ok");
    assert_eq!(a.count(), 3);
    assert_eq!(b.count(), 1);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn observed_same_origin_redirect_retains_credentials_without_crossing() {
    let a = Server::new("127.0.0.1", |request, n| {
        let request = request.to_ascii_lowercase();
        for header in [
            "authorization: fixture",
            "cookie: fixture",
            "proxy-authorization: fixture",
        ] {
            assert!(request.contains(header));
        }
        if n == 0 {
            response(302, "", "Location: /next\r\n")
        } else {
            response(200, "same", "")
        }
    });
    let client = crate::http_admission::bound_builder(reqwest::blocking::Client::builder())
        .build()
        .unwrap();
    let request = client
        .get(&a.url)
        .header("authorization", "fixture")
        .header("cookie", "fixture")
        .header("proxy-authorization", "fixture");
    let observed =
        crate::http_admission::get_observed(&client, request, &a.url, &binding(&[])).unwrap();
    assert!(!observed.crossed_origin);
    assert_eq!(observed.redirects_followed, 1);
    assert_eq!(observed.response.text().unwrap(), "same");
    assert_eq!(a.count(), 2);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn observed_denied_redirect_never_contacts_destination() {
    let b = Server::new("127.0.0.2", |_, _| response(200, "forbidden", ""));
    let target = b.url.clone();
    let a = Server::new("127.0.0.1", move |_, _| {
        response(302, "", &format!("Location: {target}/next\r\n"))
    });
    let client = crate::http_admission::bound_builder(reqwest::blocking::Client::builder())
        .build()
        .unwrap();
    let result = crate::http_admission::get_observed(
        &client,
        client.get(&a.url),
        &a.url,
        &binding(&["127.0.0.2"]),
    );
    assert!(matches!(
        result,
        Err(crate::http_admission::HttpGetError::Refused(
            ContactRefusal::ExcludedHost { .. }
        ))
    ));
    assert_eq!(a.count(), 1);
    assert_eq!(b.count(), 0);
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn controlled_return_to_initial_origin_keeps_cross_origin_strip_sticky() {
    use reqwest::header::{AUTHORIZATION, COOKIE, PROXY_AUTHORIZATION};
    let origin = Arc::new(std::sync::OnceLock::<String>::new());
    let copy = origin.clone();
    let b = Server::new("127.0.0.2", move |request, _| {
        assert!(!request.to_ascii_lowercase().contains("authorization:"));
        assert!(!request.to_ascii_lowercase().contains("cookie:"));
        response(
            302,
            "",
            &format!("Location: {}/return\r\n", copy.get().unwrap()),
        )
    });
    let other = b.url.clone();
    let a = Server::new("127.0.0.1", move |request, n| {
        let request = request.to_ascii_lowercase();
        assert!(!request.contains("referer:"));
        if n < 2 {
            assert!(request.contains("authorization: fixture"));
            assert!(request.contains("cookie: fixture"));
        } else {
            assert!(!request.contains("authorization:"));
            assert!(!request.contains("cookie:"));
        }
        match n {
            0 => response(302, "", "Location: /same-origin\r\n"),
            1 => response(302, "", &format!("Location: {other}/cross-origin\r\n")),
            _ => response(200, "ok", ""),
        }
    });
    origin.set(a.url.clone()).unwrap();
    let client = crate::http_admission::ManagedHttpClient::bound(
        std::time::Duration::from_secs(5),
        binding(&[]),
    )
    .unwrap();
    let mut request = client.get(&a.url);
    for key in [AUTHORIZATION, COOKIE, PROXY_AUTHORIZATION] {
        request = request.header(key, "fixture");
    }
    let progress = vibe_core::progress::Progress::default()
        .with_cancellation(vibe_core::cancellation::CancellationToken::new());
    let observed = crate::git_backend::transfers::observed(Some(&progress), None, None, || {
        crate::http_admission::get_observed(&client, request, &a.url, &binding(&[]))
    })
    .unwrap();
    assert!(observed.crossed_origin);
    assert_eq!(observed.redirects_followed, 3);
    assert_eq!(
        observed.response.url().origin(),
        reqwest::Url::parse(&a.url).unwrap().origin()
    );
    assert_eq!(observed.response.text().unwrap(), "ok");
    assert_eq!(a.count(), 3);
    assert_eq!(b.count(), 1);
}
