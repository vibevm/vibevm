use super::*;
use vibe_core::manifest::ExtensionConfig;

fn config(text: &str) -> ExtensionConfig {
    ExtensionConfig::from_table(text.parse::<toml::Table>().expect("fixture config parses"))
}

#[test]
fn all_three_rows_answer_under_their_own_provider_identity() {
    for client in PluginClient::ALL {
        let provider = ClientPluginProvider::new(client);
        let descriptor = provider.descriptor();
        assert_eq!(descriptor.provider.key, client.pin());
        assert!(descriptor.reference_ownership);
        assert!(descriptor.atomic_replacement);
        assert_eq!(descriptor.provider.operations.len(), 6);
    }
}

#[test]
fn config_is_exact_and_uses_the_shared_portable_name_grammar() {
    assert_eq!(
        PluginDeployConfig::parse("demo", Some(&config("name = 'portable-name'")))
            .unwrap()
            .name,
        "portable-name"
    );
    for invalid in [
        "name = 'WithCaps'",
        "name = 'has_under'",
        "name = '../escape'",
        "name = 'ok'\nclient = 'claude'",
    ] {
        assert!(PluginDeployConfig::parse("demo", Some(&config(invalid))).is_err());
    }
    assert!(PluginDeployConfig::parse("demo", None).is_err());
}

#[test]
fn desired_cli_resource_digest_separates_every_client_field() {
    let base = framed_hash(
        RESOURCE_DOMAIN,
        &[
            ("client", "claude"),
            ("name", "demo"),
            ("marketplace", "vibevm-a"),
            ("artifact", &"0".repeat(64)),
            ("version", "1.2.3"),
        ],
    );
    let other = framed_hash(
        RESOURCE_DOMAIN,
        &[
            ("client", "codex"),
            ("name", "demo"),
            ("marketplace", "vibevm-a"),
            ("artifact", &"0".repeat(64)),
            ("version", "1.2.3"),
        ],
    );
    assert_ne!(base, other);
    assert_eq!(base.len(), 64);
}
