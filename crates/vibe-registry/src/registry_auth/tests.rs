//! Auth declaration lowering is independent of credentials and catalog lookup.
use super::*;

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth"
)]
fn omitted_auth_follows_actual_source_explicit_modes_keep_their_ceiling() {
    for (source, expected) in [
        ("https://host/org", AuthKind::None),
        ("ssh://alias/org", AuthKind::Ssh),
        ("git@alias:org/repo", AuthKind::Ssh),
        ("file:///tmp/registry", AuthKind::None),
    ] {
        let resolve = |auth| {
            resolve_registry_auth(RegistryAuthInput::LockedSource {
                source_url: source,
                auth,
                token_env: None,
            })
            .unwrap()
        };
        assert_eq!(resolve(None).kind, expected);
        for explicit in [
            AuthKind::None,
            AuthKind::TokenEnv,
            AuthKind::Ssh,
            AuthKind::CredentialHelper,
        ] {
            assert_eq!(resolve(Some(explicit)).kind, explicit);
        }
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth"
)]
fn contradictory_token_declaration_fails_without_exposing_url_secret() {
    for auth in [None, Some(AuthKind::None), Some(AuthKind::Ssh)] {
        let error = resolve_registry_auth(RegistryAuthInput::LockedSource {
            source_url: "https://user:do-not-echo@host/org",
            auth,
            token_env: Some("TOKEN_NAME"),
        })
        .unwrap_err();
        assert!(matches!(error, RegistryError::AuthDeclaration { .. }));
        assert!(!error.to_string().contains("do-not-echo"));
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth"
)]
fn operator_source_defaults_and_explicit_ceilings_are_independent_inputs() {
    for (source, default) in [
        ("ssh://operator-alias/org", AuthKind::Ssh),
        ("https://host/org", AuthKind::None),
    ] {
        let resolve = |auth, token_env| {
            resolve_registry_auth(RegistryAuthInput::OperatorSource {
                source_url: source,
                auth,
                token_env,
            })
        };
        assert_eq!(resolve(None, None).unwrap().kind, default);
        for auth in [
            AuthKind::None,
            AuthKind::Ssh,
            AuthKind::TokenEnv,
            AuthKind::CredentialHelper,
        ] {
            assert_eq!(resolve(Some(auth), None).unwrap().kind, auth);
        }
        assert!(resolve(Some(AuthKind::None), Some("MUST_NOT_BE_READ")).is_err());
        assert_eq!(
            resolve(Some(AuthKind::TokenEnv), Some("OPERATOR_TOKEN"))
                .unwrap()
                .token_env_name
                .as_deref(),
            Some("OPERATOR_TOKEN")
        );
    }
}
