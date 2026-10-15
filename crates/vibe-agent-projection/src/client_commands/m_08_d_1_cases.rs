use super::*;
#[test]
fn fixed_recipes_are_deterministic() {
    let cases = [
        (Client::Claude, Operation::Version, vec!["--version"]),
        (Client::Codex, Operation::Version, vec!["--version"]),
        (Client::OpenCode, Operation::Version, vec!["--version"]),
        (
            Client::Claude,
            Operation::List,
            vec!["plugin", "list", "--json"],
        ),
        (
            Client::Codex,
            Operation::List,
            vec!["plugin", "list", "--json"],
        ),
        (
            Client::Claude,
            Operation::MarketplaceAdd("/tmp/market".into()),
            vec![
                "plugin",
                "marketplace",
                "add",
                "--scope",
                "user",
                "/tmp/market",
            ],
        ),
        (
            Client::Codex,
            Operation::MarketplaceAdd("/tmp/market".into()),
            vec!["plugin", "marketplace", "add", "--json", "/tmp/market"],
        ),
        (
            Client::Claude,
            Operation::Install("p@m".into()),
            vec!["plugin", "install", "--scope", "user", "p@m"],
        ),
        (
            Client::Codex,
            Operation::Install("p@m".into()),
            vec!["plugin", "add", "--json", "p@m"],
        ),
        (
            Client::Claude,
            Operation::Remove("p@m".into()),
            vec!["plugin", "uninstall", "--scope", "user", "p@m"],
        ),
        (
            Client::Codex,
            Operation::Remove("p@m".into()),
            vec!["plugin", "remove", "--json", "p@m"],
        ),
    ];
    for (client, op, expected) in cases {
        let expected: Vec<OsString> = expected.into_iter().map(OsString::from).collect();
        assert_eq!(render(client, &op).unwrap(), expected);
    }
    for (client, version) in [
        (Client::Claude, "2.1.7"),
        (Client::Codex, "0.148.2"),
        (Client::OpenCode, "1.17.1"),
    ] {
        assert!(supported_version(client, version));
    }
}
#[test]
fn literal_values_and_option_refusal() {
    let value = "literal/\"quote\"/$(touch nope); data";
    let args = render(Client::Claude, &Operation::Install(value.into())).unwrap();
    assert_eq!(args.len(), 5);
    assert_eq!(args[4], value);
    let path = PathBuf::from("/tmp/\"quote\"/$(literal)");
    assert_eq!(
        render(Client::Codex, &Operation::MarketplaceAdd(path.clone()))
            .unwrap()
            .last()
            .unwrap(),
        path.as_os_str()
    );
    for value in ["-evil", "", "x\n--evil", "x\0y"] {
        assert!(render(Client::Claude, &Operation::Install(value.into())).is_err());
    }
    for path in ["--evil", "relative", "/tmp/../outside"] {
        assert!(render(Client::Claude, &Operation::MarketplaceAdd(path.into())).is_err());
    }
}
#[test]
fn unsupported_contracts_refuse() {
    assert!(!supported_version(Client::Claude, "2.2.0"));
    assert!(!supported_version(Client::Codex, "0.148.2;sh"));
    for op in [
        Operation::List,
        Operation::Install("p@m".into()),
        Operation::Remove("p@m".into()),
        Operation::MarketplaceAdd("/tmp/m".into()),
    ] {
        assert!(render(Client::OpenCode, &op).is_err());
    }
}
#[test]
fn native_paths_are_lossless() {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let value = OsString::from_vec(b"/tmp/\xff".to_vec());
        assert_eq!(
            render(
                Client::Codex,
                &Operation::MarketplaceAdd(value.clone().into())
            )
            .unwrap()
            .last(),
            Some(&value)
        );
    }
}
