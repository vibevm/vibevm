//! Typed portable admission rejects contradictory public/secret ownership.
#![cfg(test)]
use super::*;
use std::collections::BTreeMap;

const ROOT: &str = "org.example/root@1.0.0";
const CHILD: &str = "org.example/child@1.0.0";
const CANARY: &str = "SYN_PORTABLE_OVERLAP_VALUE_5107";

fn row(key: &str, mark: w::PortableApplicationMark) -> w::PortableApplication {
    w::PortableApplication {
        key: key.into(),
        installer: "main".into(),
        platform: "linux".into(),
        arch: "x86_64".into(),
        mark,
        held: false,
        hidden: false,
        license_accepted: false,
        configuration: w::PortableConfigurationState::LegacyEffective(Box::new(
            w::PortableConfigurationStateLegacyEffective {
                features: vec![],
                parameters: BTreeMap::from([("public".into(), "visible".into())]),
                secret_origins: BTreeMap::from([(
                    "token".into(),
                    w::ConfigurationParameterOrigin::Explicit(Box::new(
                        w::ConfigurationParameterOriginExplicit {},
                    )),
                )]),
            },
        )),
        requires: vec![],
    }
}

fn document() -> full::AppsManifest {
    let mut root = row(ROOT, w::PortableApplicationMark::Manual);
    root.requires.push(w::PortableRequirement {
        package: "org.example/child@1".into(),
        key: CHILD.into(),
        scope: w::PortableRequirementScope::Runtime,
        step: None,
    });
    full::AppsManifest {
        protocol: FULL_APPS_PROTOCOL.into(),
        roots: vec![ROOT.into()],
        exported: "2026-11-05T00:00:00Z".into(),
        home: full::AppsHome::User,
        platform: "linux".into(),
        arch: "x86_64".into(),
        application: vec![root, row(CHILD, w::PortableApplicationMark::Auto)],
        collection: vec![],
        pin: vec![],
        launcher: vec![],
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
fn legacy_public_secret_overlap_refuses_json_and_toml_without_echoing_the_value() {
    let valid = document();
    for text in [
        serde_json::to_string(&valid).unwrap(),
        toml::to_string(&valid).unwrap(),
    ] {
        assert_eq!(
            parse_portable_document("fixture", &text).unwrap(),
            PortableDocument::Full(valid.clone())
        );
    }
    let mut invalid = valid;
    let w::PortableConfigurationState::LegacyEffective(state) =
        &mut invalid.application[0].configuration
    else {
        panic!("legacy fixture");
    };
    state.parameters.insert("token".into(), CANARY.into());
    for text in [
        serde_json::to_string(&invalid).unwrap(),
        toml::to_string(&invalid).unwrap(),
    ] {
        let error = parse_portable_document("fixture", &text).unwrap_err();
        assert!(error.to_string().contains("secret parameter `token`"));
        assert!(!error.to_string().contains(CANARY));
        let mut source = std::error::Error::source(&error);
        while let Some(error) = source {
            assert!(!error.to_string().contains(CANARY));
            source = error.source();
        }
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
fn shared_row_admission_keeps_root_identity_and_required_edge_checks() {
    let valid = document();
    let bindings: BTreeMap<_, _> = valid.application.iter().map(|r| (&r.key, r)).collect();
    validate_rows(&valid.roots, bindings.values().copied()).unwrap();
    let mut variants = Vec::new();
    let mut invalid = valid.clone();
    invalid.roots.push(ROOT.into());
    variants.push(invalid);
    let mut invalid = valid.clone();
    invalid.roots = vec![CHILD.into()];
    variants.push(invalid);
    let mut invalid = valid.clone();
    invalid.application.push(invalid.application[0].clone());
    variants.push(invalid);
    let mut invalid = valid.clone();
    invalid.application[0].key = "not-an-exact-key".into();
    variants.push(invalid);
    let mut invalid = valid.clone();
    invalid.application[0].requires[0].package = "org.example/child@2".into();
    variants.push(invalid);
    let mut invalid = valid.clone();
    invalid.application[0].requires[0].key = "not-an-exact-key".into();
    variants.push(invalid);
    let mut invalid = valid.clone();
    invalid.application.pop();
    variants.push(invalid);
    for invalid in variants {
        assert!(validate_rows(&invalid.roots, &invalid.application).is_err());
        assert_eq!(
            validate_full(&invalid),
            validate_rows(&invalid.roots, &invalid.application)
        );
    }
    let mut wrong_protocol = valid;
    wrong_protocol.protocol = "vibe-apps/999".into();
    assert!(validate_full(&wrong_protocol).is_err());
    validate_rows(&wrong_protocol.roots, &wrong_protocol.application).unwrap();
}
