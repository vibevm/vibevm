use super::*;

fn network(raw: &str) -> NetworkAuthority {
    let EndpointObservation::Network(value) = observe_endpoint(raw) else {
        panic!("expected network authority for {raw:?}");
    };
    value
}

#[test]
fn exact_known_hosts_are_normalized_without_credentials_or_path() {
    for raw in [
        "https://user:SYN_SECRET@GitHub.COM/a?token=SYN_QUERY#SYN_FRAGMENT",
        "git+https://github.com/a",
        "https://GitHub.COM./a",
        "git@github.com.:a",
        "git://github.com/a",
        "ssh://git@GITHUB.COM/a",
        "git@github.com:a",
        "github.com:a",
    ] {
        let value = network(raw);
        assert_eq!(value.host, "github.com");
        assert_eq!(value.hosting, Some("github"));
        assert!(!format!("{value:?}").contains("SYN_"));
    }
    for raw in [
        "https://gitverse.ru/a",
        "https://gitverse.ru./a",
        "git@gitverse.ru:a",
        "git+ssh://git@gitverse.ru/a",
    ] {
        assert_eq!(network(raw).hosting, Some("gitverse"));
    }
    for host in [
        "github.com.attacker.test",
        "evilgithub.com",
        "github.company.test",
        "gitverse.ru.attacker.test",
    ] {
        assert_eq!(network(&format!("https://{host}/github.com")).hosting, None);
    }
    assert_eq!(network("https://github.com@attacker.test/a").hosting, None);
}

#[test]
fn ports_ipv6_and_idna_follow_url_host_normalization() {
    assert_eq!(network("https://github.com:443/a").port, None);
    assert_eq!(network("https://github.com:8443/a").port, Some(8443));
    assert_eq!(network("ssh://git@github.com:22/a").port, Some(22));
    assert_eq!(network("http://github.com:80/a").port, None);
    let ipv6 = network("ssh://git@[2001:db8::1]:2222/a");
    assert_eq!(ipv6.host, "[2001:db8::1]");
    assert_eq!(ipv6.port, Some(2222));
    assert_eq!(ipv6.hosting, None);
    assert_eq!(network("git@[2001:db8::1]:repo").host, ipv6.host);
    for raw in [
        "https://bücher.example/a",
        "ssh://git@BÜCHER.example/a",
        "git@bücher.example:a",
    ] {
        let value = network(raw);
        assert_eq!(value.host, "xn--bcher-kva.example");
        assert_eq!(value.hosting, None);
    }
    assert_eq!(network("git://192.0.2.1/a").host, "192.0.2.1");
}

#[test]
fn local_nonlocal_and_opaque_sources_do_not_claim_hosting() {
    for raw in [
        "/srv/repo",
        "./repo",
        "../repo",
        "repo",
        "~/repo",
        "C:/repo",
        "c:repo",
        r"C:\repo",
        "file:///srv/repo",
        "file://localhost/srv/repo",
        "git+file:///srv/repo",
    ] {
        assert_eq!(observe_endpoint(raw), EndpointObservation::Local, "{raw}");
    }
    for raw in [
        r"\\server\share\repo",
        "//server/share/repo",
        "file://server/share/repo",
        "file://github.com/repo",
    ] {
        assert_eq!(
            observe_endpoint(raw),
            EndpointObservation::NonLocalFile,
            "{raw}"
        );
    }
    for raw in [
        "",
        "git@work:repo",
        "work:repo",
        "ssh://git@work/repo",
        "ext::command",
        "github.com::[payload]",
        "git@github.com::[payload]",
        "helper::https://github.com/a",
        "git+git+https://github.com/a",
        "ftp://github.com/a",
        "https://[bad/a",
        "https://github.com:70000/a",
        "https://github.com\n/a",
        "git@github.com:",
    ] {
        assert_eq!(observe_endpoint(raw), EndpointObservation::Opaque, "{raw}");
    }
}

#[test]
fn normalized_host_keys_remove_one_dns_root_dot_and_preserve_ip_semantics() {
    for (host, key) in [
        ("EXAMPLE.invalid.", "example.invalid"),
        ("bücher.example.", "xn--bcher-kva.example"),
        ("127.1", "127.0.0.1"),
        ("[2001:0db8::1]", "[2001:db8::1]"),
    ] {
        assert_eq!(normalized_host_key(host).as_deref(), Some(key));
    }
    for malformed in [
        ".",
        "example.invalid..",
        ".example.invalid",
        "example..invalid",
        "user@example.invalid",
        "example.invalid/path",
    ] {
        assert_eq!(normalized_host_key(malformed), None);
    }
    assert_eq!(
        network("https://github.com.attacker.invalid./a").hosting,
        None
    );
    assert_eq!(
        observe_endpoint("https://github.com../a"),
        EndpointObservation::Opaque
    );
    assert_eq!(
        observe_endpoint("git@alias.:repo"),
        EndpointObservation::Opaque
    );
}
