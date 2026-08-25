use std::collections::BTreeMap;
use std::path::PathBuf;

use specmark::verifies;

use super::*;

fn decl(point: &str, handler: ExtensionHandler) -> ExtensionDecl {
    ExtensionDecl {
        id: "demo".to_string(),
        point: point.parse().unwrap(),
        handler,
        config: None,
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    }
}

fn table(body: &str) -> toml::Table {
    toml::from_str(body).unwrap()
}

fn assert_eq_type<T: Eq>() {}

#[test]
fn authored_extension_id_cannot_use_the_vibe_prefix() {
    let error = crate::manifest::Manifest::parse_str(
        r#"
[project]
name = "demo"
version = "0.0.1"

[[extension]]
id = "@vibe/hooks/pre-install"
point = "slot:pre-install"
handler = { kind = "builtin", name = "log" }
"#,
    )
    .expect_err("@vibe/ is reserved");
    let error = error.to_string();
    assert!(error.contains("reserved `@vibe/` prefix"), "{error}");
    assert!(error.contains("@vibe/hooks/pre-install"), "{error}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-FIELDS")]
fn semantic_toml_wrappers_keep_manifest_eq_and_make_nan_reflexive() {
    assert_eq_type::<crate::manifest::Manifest>();
    assert_eq_type::<ExtensionConfig>();
    assert_eq_type::<ExtensionWhen>();

    let mut positive = toml::Table::new();
    positive.insert("value".into(), toml::Value::Float(f64::NAN));
    let mut negative_payload = toml::Table::new();
    negative_payload.insert(
        "value".into(),
        toml::Value::Float(f64::from_bits(0xfff8_0000_0000_0042)),
    );
    let a = ExtensionConfig::from_table(positive);
    let b = ExtensionConfig::from_table(negative_payload);
    assert_eq!(a, a.clone(), "Eq must be reflexive over NaN");
    assert_eq!(a, b, "NaN sign/payload are not semantic TOML values");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-FIELDS")]
fn semantic_toml_equality_keeps_signed_zero_datetime_and_nested_types() {
    let negative = ExtensionConfig::from_table(table(
        "zero = -0.0\nat = 1979-05-27T07:32:00Z\n[nested]\nvalues = [1, 1.0, true]\n",
    ));
    let positive = ExtensionConfig::from_table(table(
        "zero = 0.0\nat = 1979-05-27T07:32:00Z\n[nested]\nvalues = [1, 1.0, true]\n",
    ));
    assert_ne!(negative, positive, "signed zero has distinct TOML bits");
    assert!(matches!(
        negative.as_table().get("at"),
        Some(toml::Value::Datetime(_))
    ));

    let reordered = ExtensionConfig::from_table(table(
        "at = 1979-05-27T07:32:00Z\nzero = -0.0\n[nested]\nvalues = [1, 1.0, true]\n",
    ));
    assert_eq!(negative, reordered, "table key order is not semantic");

    let reversed_array = ExtensionConfig::from_table(table(
        "zero = -0.0\nat = 1979-05-27T07:32:00Z\n[nested]\nvalues = [1.0, 1, true]\n",
    ));
    assert_ne!(
        negative, reversed_array,
        "array order and scalar kind matter"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#REF-HANDLER-TABLES")]
fn script_and_native_paths_are_portably_declarant_root_relative() {
    for base in [
        "",
        ".",
        "./hooks/prepare",
        "hooks/prepare",
        "dir.with.dot/prepare",
    ] {
        let valid = decl(
            "phase:build",
            ExtensionHandler::Script { base: base.into() },
        );
        assert!(valid.validate().is_ok(), "valid path `{base}`");
    }

    for base in [
        "../prepare",
        "hooks/../prepare",
        "/root/prepare",
        "//server/share/prepare",
        "C:/hooks/prepare",
        "c:hooks/prepare",
        r"hooks\prepare",
    ] {
        let invalid = decl(
            "phase:build",
            ExtensionHandler::Script { base: base.into() },
        );
        let error = invalid.validate().unwrap_err();
        assert!(error.contains("handler.base"), "{error}");
        assert!(error.contains(base), "{error}");
        assert!(error.contains("declarant-root-relative"), "{error}");
    }

    let extension = decl(
        "phase:build",
        ExtensionHandler::Script {
            base: "hooks/prepare.sh".into(),
        },
    );
    assert!(
        extension
            .validate()
            .unwrap_err()
            .contains("omit its script extension")
    );

    let neither = decl(
        "phase:build",
        ExtensionHandler::Native {
            crate_dir: None,
            prebuilt: None,
        },
    );
    assert!(neither.validate().unwrap_err().contains("requires field"));

    let empty_prebuilt = decl(
        "phase:build",
        ExtensionHandler::Native {
            crate_dir: None,
            prebuilt: Some(BTreeMap::new()),
        },
    );
    assert!(
        empty_prebuilt.validate().is_ok(),
        "field presence is enough"
    );

    let source_only = decl(
        "phase:build",
        ExtensionHandler::Native {
            crate_dir: Some("ext/native".into()),
            prebuilt: None,
        },
    );
    let prebuilt_only = decl(
        "phase:build",
        ExtensionHandler::Native {
            crate_dir: None,
            prebuilt: Some(BTreeMap::from([(
                "future-platform-key".to_string(),
                PathBuf::from("ext/bin/plugin.bin"),
            )])),
        },
    );
    let both = decl(
        "phase:build",
        ExtensionHandler::Native {
            crate_dir: Some("ext/native".into()),
            prebuilt: Some(BTreeMap::new()),
        },
    );
    assert!(source_only.validate().is_ok());
    assert!(prebuilt_only.validate().is_ok());
    assert!(both.validate().is_ok());

    let bad_crate = decl(
        "phase:build",
        ExtensionHandler::Native {
            crate_dir: Some(PathBuf::from("../ext")),
            prebuilt: None,
        },
    );
    assert!(bad_crate.validate().unwrap_err().contains("crate_dir"));

    let bad_prebuilt = decl(
        "phase:build",
        ExtensionHandler::Native {
            crate_dir: None,
            prebuilt: Some(BTreeMap::from([(
                "opaque-platform".to_string(),
                PathBuf::from("../plugin.dll"),
            )])),
        },
    );
    assert!(
        bad_prebuilt
            .validate()
            .unwrap_err()
            .contains("opaque-platform")
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HANDLER-KINDS")]
fn compile_points_accept_only_builtin_and_native() {
    for handler in [
        ExtensionHandler::Builtin {
            name: String::new(),
        },
        ExtensionHandler::Native {
            crate_dir: Some("ext".into()),
            prebuilt: None,
        },
    ] {
        assert!(decl("compile:emitted", handler).validate().is_ok());
    }
    for handler in [
        ExtensionHandler::Script {
            base: "hook".into(),
        },
        ExtensionHandler::Binary {
            name: String::new(),
        },
        ExtensionHandler::Agent {
            prompt: String::new(),
        },
    ] {
        let error = decl("compile:source", handler).validate().unwrap_err();
        assert!(error.contains("builtin` or `native"), "{error}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILER-INTERNALS-FLAG")]
fn pass_flag_and_optional_pass_table_follow_presence_laws() {
    let mut pass = decl(
        "compile:pass",
        ExtensionHandler::Builtin { name: "x".into() },
    );
    assert!(pass.validate().unwrap_err().contains("requires field"));
    pass.compiler_internals = Some(false);
    assert!(pass.validate().unwrap_err().contains("requires field"));
    pass.compiler_internals = Some(true);
    assert!(pass.validate().is_ok(), "the pass table itself is optional");

    pass.pass = Some(ExtensionPass {
        kind: ExtensionPassKind::Transform,
        level: Some(ExtensionIrLevel::Closure),
        from: None,
        to: None,
        after: Some("qualify".into()),
        before: Some("link".into()),
        replace: Some("anything".into()),
        formats: Some(Vec::new()),
        artifact: Some(String::new()),
    });
    assert!(
        pass.validate().is_ok(),
        "kind-specific placement conflicts belong to R6"
    );

    let mut ordinary = decl("phase:test", ExtensionHandler::Builtin { name: "x".into() });
    ordinary.compiler_internals = Some(false);
    assert!(ordinary.validate().unwrap_err().contains("forbidden"));
    ordinary.compiler_internals = None;
    ordinary.pass = pass.pass;
    assert!(ordinary.validate().unwrap_err().contains("field `pass`"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn optional_fields_are_gated_by_point_family_and_presence() {
    let mut phase = decl(
        "phase:test",
        ExtensionHandler::Agent {
            prompt: "anything".into(),
        },
    );
    phase.inputs = Some(Vec::new());
    assert!(phase.validate().is_ok(), "agent is legal outside create");
    phase.auto = Some(false);
    assert!(phase.validate().unwrap_err().contains("field `auto`"));

    let mut source = decl(
        "compile:source",
        ExtensionHandler::Builtin { name: "x".into() },
    );
    source.applies_to = Some(ExtensionAppliesTo {
        packages: Some(vec!["org.x/*".into()]),
        paths: Some(vec!["spec/**".into()]),
    });
    source.auto = Some(true);
    assert!(source.validate().is_ok());
    source.inputs = Some(Vec::new());
    assert!(source.validate().unwrap_err().contains("field `inputs`"));

    let mut lane = decl(
        "compile:lane",
        ExtensionHandler::Builtin { name: "x".into() },
    );
    lane.applies_to = source.applies_to;
    assert!(lane.validate().unwrap_err().contains("applies_to"));

    let mut slot = decl(
        "slot:pre-install",
        ExtensionHandler::Binary { name: "x".into() },
    );
    slot.applies_to = Some(ExtensionAppliesTo::default());
    assert!(slot.validate().is_ok());
}
