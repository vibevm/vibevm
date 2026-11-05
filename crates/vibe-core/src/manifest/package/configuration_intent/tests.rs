//! Real edge metadata survives every source bucket and refuses lossy serialization.
use crate::PackageRef;
use crate::manifest::{ConfigurationIntentDecl, Manifest, Requires};

fn input() -> ConfigurationIntentDecl {
    toml::from_str("policy='require-selection'\ndefault_features=false\nprofile='named'\nfeatures=['public']\nselections.mode={intent='exact'}\nselections.empty={intent='additive',members=[]}\nparameters.port=42").unwrap()
}
#[test]
fn ordinary_manifest_roundtrip_preserves_intent_and_source_metadata() {
    for source in [
        "version='^1'",
        "path='../dep',version='^1'",
        "git='https://example.com/dep',rev='abc',auth='token-env',token_env='TOKEN_NAME'",
        "version.var='family'",
    ] {
        let source = source.replace(',', "\n");
        let text = format!(
            "[project]\nname='test'\nversion='0.1.0'\n[requires.packages.'flow:org.example/dep']\n{source}\nlink='dynamic'\naccess='private'\nfriend=false\nexclude=['private/path']\n[requires.packages.'flow:org.example/dep'.configuration]\npolicy='require-selection'\ndefault_features=false\nprofile='named'\nfeatures=['public']\nselections.mode={{intent='exact'}}\nselections.empty={{intent='additive',members=[]}}\nparameters.port=42\n"
        );
        let m = Manifest::parse_str(&text).unwrap();
        assert_eq!(m.requires.configurations["org.example/dep"], input());
        let out = toml::to_string(&m).unwrap();
        let back = Manifest::parse_str(&out).unwrap();
        assert_eq!(m, back);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vibe.toml");
        m.write(&path).unwrap();
        assert_eq!(Manifest::read(path).unwrap(), m);
    }
}
#[test]
fn old_string_form_remains_and_normalized_duplicate_keys_refuse() {
    let r: Requires = toml::from_str("[packages]\n'org.example/dep'='^1'").unwrap();
    assert!(r.configurations.is_empty());
    assert!(toml::to_string(&r).unwrap().contains("= \"^1\""));
    let text = "[packages]\n'org.example/dep'='^1'\n'flow:org.example/dep'={path='../x',configuration={policy='require-selection'}}";
    assert!(
        toml::from_str::<Requires>(text)
            .unwrap_err()
            .to_string()
            .contains("declared more than once")
    );
}
#[test]
fn dangling_or_ambiguous_manual_requires_cannot_lose_input_on_serialization() {
    let mut r = Requires::default();
    r.configurations.insert("org.example/dep".into(), input());
    assert!(!r.is_empty());
    assert!(
        toml::to_string(&r)
            .unwrap_err()
            .to_string()
            .contains("no declared requires edge")
    );
    r.packages
        .push(PackageRef::parse("org.example/dep@^1").unwrap());
    assert!(toml::to_string(&r).is_ok());
    r.packages
        .push(PackageRef::parse("org.example/dep@^2").unwrap());
    assert!(
        toml::to_string(&r)
            .unwrap_err()
            .to_string()
            .contains("declared more than once")
    );
}
#[test]
fn source_shape_is_checked_without_a_target_and_values_are_typed() {
    for body in [
        "policy='require-selection',features=['x','x']",
        "policy='require-selection',profile=' '",
        "policy='require-selection',selections={g={intent='exact',members=['x','x']}}",
        "policy='require-selection',parameters={value=1.5}",
        "features=[]",
    ] {
        let text =
            format!("[packages]\n'org.example/dep'={{version='*',configuration={{{body}}}}}");
        assert!(toml::from_str::<Requires>(&text).is_err(), "{text}");
    }
}

#[test]
fn configuration_input_read_error_names_the_actual_edge_and_field_without_values() {
    let source = "[project]\nname='test'\nversion='0.1.0'\n[requires.packages]\n'org.example/first'={version='*',configuration={policy='require-selection'}}\n[requires.packages.'flow:org.example/second'.configuration]\npolicy='require-selection'\nselections.mode={intent='exact',members=['x','x']}\nparameters.token='SYN_VALUE_MUST_NOT_APPEAR'\n";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vibe.toml");
    std::fs::write(&path, source).unwrap();
    for error in [
        Manifest::parse_str(source).unwrap_err(),
        Manifest::read(&path).unwrap_err(),
    ] {
        let displayed = format!("{error}\n{error:?}");
        assert!(
            !displayed.contains("SYN_VALUE_MUST_NOT_APPEAR"),
            "{displayed}"
        );
        assert!(displayed.contains("requires.packages.\"flow:org.example/second\".configuration.selections.mode.members"),"{displayed}");
    }
}

#[test]
fn configuration_input_serialize_error_names_the_actual_edge_and_field_without_values() {
    let mut requires: Requires =
        toml::from_str("[packages]\n'org.example/first'='*'\n'org.example/second'='*'").unwrap();
    requires
        .configurations
        .insert("org.example/first".into(), input());
    let mut bad = input();
    bad.features = vec!["x".into(), "x".into()];
    bad.parameters.insert(
        "token".into(),
        crate::manifest::ParameterValue::Text("SYN_VALUE_MUST_NOT_APPEAR".into()),
    );
    requires
        .configurations
        .insert("org.example/second".into(), bad);
    let error = toml::to_string(&requires).unwrap_err();
    let displayed = format!("{error}\n{error:?}");
    assert!(
        !displayed.contains("SYN_VALUE_MUST_NOT_APPEAR"),
        "{displayed}"
    );
    assert!(
        displayed.contains("requires.packages.\"org.example/second\".configuration.features"),
        "{displayed}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#OPTIONAL-DECLARATIONS")]
fn optional_declarations_roundtrip_every_source_and_explicit_false() {
    for source in [
        "version='^1'",
        "path='../dep',version='^1'",
        "git='https://example.com/dep',rev='abc'",
        "version.var='family'",
    ] {
        for optional in [true, false] {
            let text = format!(
                "[packages]\n'flow:org.example/dep'={{{source},optional={optional},link='dynamic',configuration={{policy='require-selection',default_features=false}}}}"
            );
            let parsed: Requires = toml::from_str(&text).unwrap();
            assert_eq!(parsed.optional.get("org.example/dep"), Some(&optional));
            let encoded = toml::to_string(&parsed).unwrap();
            assert!(
                encoded.contains(&format!("optional = {optional}")),
                "{encoded}"
            );
            assert_eq!(toml::from_str::<Requires>(&encoded).unwrap(), parsed);
        }
    }
    let legacy: Requires = toml::from_str("[packages]\n'org.example/dep'='^1'").unwrap();
    assert!(legacy.optional.is_empty());
    assert!(!toml::to_string(&legacy).unwrap().contains("optional"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#OPTIONAL-DECLARATIONS")]
fn optional_declaration_metadata_refuses_wrong_type_and_dangling_key() {
    for value in ["'true'", "1", "[]", "{}"] {
        let text = format!("[packages]\n'org.example/dep'={{version='*',optional={value}}}");
        assert!(toml::from_str::<Requires>(&text).is_err(), "{text}");
    }
    let mut value = Requires::default();
    value.optional.insert("org.example/missing".into(), false);
    assert!(!value.is_empty());
    let error = toml::to_string(&value).unwrap_err().to_string();
    assert!(error.contains("org.example/missing") && error.contains("no declared requires edge"));
}
