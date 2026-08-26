//! Focused tests for `[[mechanism]]`, mechanism keys, provider pins, and
//! `[mechanisms]` routes.

use specmark::verifies;

use super::{
    HOST_OWNER, MechanismFreshness, MechanismKey, MechanismRole, MechanismRoutes, ProviderOwner,
    ProviderPin, validate_mechanism_declarations,
};
use crate::Error;
use crate::manifest::Manifest;

const PROJECT: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";
const VIRTUAL: &str = "[workspace]\nmembers = []\n";
const DECL: &str = concat!(
    "[[mechanism]]\n",
    "id = \"cargo-v2\"\n",
    "role = \"build\"\n",
    "name = \"cargo\"\n",
    "handler = { kind = \"native\", crate_dir = \"crates/cargo-provider\" }\n",
    "protocol = 1\n",
    "config_schema = \"schemas/cargo-build-v1.jtd.json\"\n",
    "freshness = \"provider\"\n",
);

fn parse_error(role: &str, body: &str) -> String {
    Manifest::parse_str(&format!("{role}\n{body}"))
        .unwrap_err()
        .to_string()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn full_declaration_parses_and_round_trips() {
    let manifest = Manifest::parse_str(&format!("{PROJECT}\n{DECL}")).unwrap();
    assert_eq!(manifest.mechanism_decls.len(), 1);
    let declaration = &manifest.mechanism_decls[0];
    assert_eq!(declaration.id, "cargo-v2");
    assert_eq!(declaration.role, MechanismRole::Build);
    assert_eq!(declaration.name, "cargo");
    assert_eq!(declaration.protocol, 1);
    assert_eq!(declaration.freshness, MechanismFreshness::Provider);
    assert_eq!(
        declaration.config_schema.to_str(),
        Some("schemas/cargo-build-v1.jtd.json")
    );

    let rendered = toml::to_string_pretty(&manifest).unwrap();
    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, reparsed);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn unknown_fields_on_the_declaration_refuse() {
    match Manifest::parse_str(&format!("{PROJECT}\n{DECL}mystery = true\n")) {
        Err(Error::ParseToml { diagnostic, .. }) => {
            let detail = diagnostic.to_string();
            assert!(detail.contains("unknown field"), "{detail}");
            assert!(detail.contains("mystery"), "{detail}");
        }
        other => panic!("expected ParseToml, got {other:?}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn required_fields_fail_as_toml_shape_errors() {
    for (row, field) in [
        (
            "[[mechanism]]\nrole = \"build\"\nname = \"cargo\"\nhandler = { kind = \"native\", crate_dir = \"crates/x\" }\nprotocol = 1\nconfig_schema = \"s.jtd.json\"\nfreshness = \"engine\"\n",
            "id",
        ),
        (
            "[[mechanism]]\nid = \"x\"\nname = \"cargo\"\nhandler = { kind = \"native\", crate_dir = \"crates/x\" }\nprotocol = 1\nconfig_schema = \"s.jtd.json\"\nfreshness = \"engine\"\n",
            "role",
        ),
        (
            "[[mechanism]]\nid = \"x\"\nrole = \"build\"\nhandler = { kind = \"native\", crate_dir = \"crates/x\" }\nprotocol = 1\nconfig_schema = \"s.jtd.json\"\nfreshness = \"engine\"\n",
            "name",
        ),
        (
            "[[mechanism]]\nid = \"x\"\nrole = \"build\"\nname = \"cargo\"\nprotocol = 1\nconfig_schema = \"s.jtd.json\"\nfreshness = \"engine\"\n",
            "handler",
        ),
        (
            "[[mechanism]]\nid = \"x\"\nrole = \"build\"\nname = \"cargo\"\nhandler = { kind = \"native\", crate_dir = \"crates/x\" }\nconfig_schema = \"s.jtd.json\"\nfreshness = \"engine\"\n",
            "protocol",
        ),
        (
            "[[mechanism]]\nid = \"x\"\nrole = \"build\"\nname = \"cargo\"\nhandler = { kind = \"native\", crate_dir = \"crates/x\" }\nprotocol = 1\nfreshness = \"engine\"\n",
            "config_schema",
        ),
        (
            "[[mechanism]]\nid = \"x\"\nrole = \"build\"\nname = \"cargo\"\nhandler = { kind = \"native\", crate_dir = \"crates/x\" }\nprotocol = 1\nconfig_schema = \"s.jtd.json\"\n",
            "freshness",
        ),
    ] {
        match Manifest::parse_str(&format!("{PROJECT}\n{row}")) {
            Err(Error::ParseToml { diagnostic, .. }) => {
                let detail = diagnostic.to_string();
                assert!(detail.contains(field), "field={field}: {detail}");
            }
            other => panic!("expected ParseToml for missing `{field}`, got {other:?}"),
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn unknown_role_and_freshness_values_refuse() {
    for row in [
        "[[mechanism]]\nid = \"x\"\nrole = \"test\"\nname = \"cargo\"\nhandler = { kind = \"native\", crate_dir = \"crates/x\" }\nprotocol = 1\nconfig_schema = \"s.jtd.json\"\nfreshness = \"engine\"\n",
        "[[mechanism]]\nid = \"x\"\nrole = \"build\"\nname = \"cargo\"\nhandler = { kind = \"native\", crate_dir = \"crates/x\" }\nprotocol = 1\nconfig_schema = \"s.jtd.json\"\nfreshness = \"whenever\"\n",
    ] {
        match Manifest::parse_str(&format!("{PROJECT}\n{row}")) {
            Err(Error::ParseToml { diagnostic, .. }) => {
                let detail = diagnostic.to_string();
                assert!(detail.contains("variant"), "{detail}");
            }
            other => panic!("expected ParseToml for unknown enum value, got {other:?}"),
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn semantic_declaration_faults_name_the_field_and_remediation() {
    let mechanism = |id: &str, name: &str, protocol: u32, schema: &str| {
        format!(
            "[[mechanism]]\nid = \"{id}\"\nrole = \"build\"\nname = \"{name}\"\nhandler = {{ kind = \"native\", crate_dir = \"crates/x\" }}\nprotocol = {protocol}\nconfig_schema = \"{schema}\"\nfreshness = \"engine\"\n"
        )
    };
    for (body, fragment) in [
        (mechanism("", "cargo", 1, "s.jtd.json"), "field `id`"),
        (mechanism("Bad_Id", "cargo", 1, "s.jtd.json"), "field `id`"),
        (mechanism("x", "", 1, "s.jtd.json"), "field `name`"),
        (mechanism("x", "car go", 1, "s.jtd.json"), "field `name`"),
        (mechanism("x", "cargo", 0, "s.jtd.json"), "field `protocol`"),
        (mechanism("x", "cargo", 1, ""), "field `config_schema`"),
        (
            mechanism("x", "cargo", 1, "../escape.jtd.json"),
            "field `config_schema`",
        ),
        // Four Rust backslashes reach TOML as `\\`, which parses to the single
        // backslash of a Windows drive path — the value the law refuses.
        (
            mechanism("x", "cargo", 1, "C:\\\\escape.jtd.json"),
            "field `config_schema`",
        ),
        (
            mechanism("x", "cargo", 1, "/abs.jtd.json"),
            "field `config_schema`",
        ),
        // The shared declarant-path law, not a weaker local copy.
        (mechanism("x", "cargo", 1, "nul"), "reserved device name"),
        (
            mechanism("x", "cargo", 1, "schemas/con.json"),
            "reserved device name",
        ),
        (
            mechanism("x", "cargo", 1, "schemas/a.json:evil"),
            "alternate data stream",
        ),
        (mechanism("x", "cargo", 1, "."), "`.` or `..` segment"),
        (
            mechanism("x", "cargo", 1, "schemas//a.json"),
            "empty path segment",
        ),
        (
            mechanism("x", "cargo", 1, "schemas/a.json. "),
            "silently strips",
        ),
    ] {
        let error = parse_error(PROJECT, &body);
        assert!(error.contains(fragment), "fragment={fragment}: {error}");
        assert!(error.contains("PROP-054"), "{error}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#REF-HANDLER-TABLES")]
fn handler_forms_reuse_the_extension_handler_laws() {
    for (handler, fragment) in [
        (
            "handler = { kind = \"script\", base = \"hooks/prepare.sh\" }\n",
            "must omit its script extension",
        ),
        (
            "handler = { kind = \"script\", base = \"../escape\" }\n",
            "declarant-root-relative",
        ),
        (
            "handler = { kind = \"native\" }\n",
            "requires field `crate_dir` or field `prebuilt`",
        ),
        (
            "handler = { kind = \"native\", crate_dir = \"../escape\" }\n",
            "declarant-root-relative",
        ),
    ] {
        let body = format!(
            "[[mechanism]]\nid = \"x\"\nrole = \"build\"\nname = \"cargo\"\n{handler}protocol = 1\nconfig_schema = \"s.jtd.json\"\nfreshness = \"engine\"\n"
        );
        let error = parse_error(PROJECT, &body);
        assert!(error.contains(fragment), "fragment={fragment}: {error}");
        assert!(error.contains("[[mechanism]] `x`"), "{error}");
    }
}

/// `builtin` and `agent` are not authorable provider implementations.
/// A builtin mechanism is an engine-synthetic descriptor; an agent prompt
/// cannot honour a numbered protocol or a deterministic freshness probe.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#HANDLER-KINDS")]
fn builtin_and_agent_handler_kinds_are_not_authorable() {
    let with = |handler: &str| {
        format!(
            "[[mechanism]]\nid = \"x\"\nrole = \"build\"\nname = \"cargo\"\n{handler}protocol = 1\nconfig_schema = \"s.jtd.json\"\nfreshness = \"engine\"\n"
        )
    };
    for (handler, fragment) in [
        (
            "handler = { kind = \"builtin\", name = \"log\" }\n",
            "handler kind `builtin` (name `log`) is not authorable",
        ),
        // The reserved engine internal `[[extension]]` already refuses by
        // name is refused here as a whole kind, not one blocked spelling.
        (
            "handler = { kind = \"builtin\", name = \"package-skill-project\" }\n",
            "handler kind `builtin` (name `package-skill-project`) is not authorable",
        ),
        (
            "handler = { kind = \"agent\", prompt = \"build it somehow\" }\n",
            "handler kind `agent` is not authorable",
        ),
    ] {
        let error = parse_error(PROJECT, &with(handler));
        assert!(error.contains(fragment), "fragment={fragment}: {error}");
        assert!(error.contains("script"), "{error}");
        assert!(error.contains("PROP-054"), "{error}");
    }

    // The three implementation kinds stay legal.
    for handler in [
        "handler = { kind = \"native\", crate_dir = \"crates/x\" }\n",
        "handler = { kind = \"script\", base = \"scripts/build\" }\n",
        "handler = { kind = \"binary\", name = \"cargo-provider\" }\n",
    ] {
        assert!(
            Manifest::parse_str(&format!("{PROJECT}\n{}", with(handler))).is_ok(),
            "{handler}"
        );
    }

    // `[[extension]]` is unaffected: both kinds remain legal there.
    let extension = concat!(
        "[[extension]]\nid = \"greet\"\npoint = \"phase:build\"\n",
        "handler = { kind = \"agent\", prompt = \"say hello\" }\n",
    );
    assert!(Manifest::parse_str(&format!("{PROJECT}\n{extension}")).is_ok());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn duplicate_ids_refuse() {
    let error = parse_error(PROJECT, &format!("{DECL}{DECL}"));
    assert!(error.contains("duplicate [[mechanism]]"), "{error}");
    assert!(error.contains("value `cargo-v2`"), "{error}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn role_law_pinned_virtual_workspace_cannot_declare() {
    assert!(Manifest::parse_str(&format!("{PROJECT}\n{DECL}")).is_ok());
    let package =
        "[package]\ngroup = \"org.demo\"\nname = \"pkg\"\nkind = \"flow\"\nversion = \"0.1.0\"\n";
    assert!(Manifest::parse_str(&format!("{package}\n{DECL}")).is_ok());

    let error = parse_error(VIRTUAL, DECL);
    assert!(error.contains("pure virtual `[workspace]`"), "{error}");
    assert!(error.contains("[[mechanism]]"), "{error}");

    // The programmatic validator agrees with the TOML path.
    let declaration = Manifest::parse_str(&format!("{PROJECT}\n{DECL}"))
        .unwrap()
        .mechanism_decls
        .pop()
        .unwrap();
    assert!(validate_mechanism_declarations(&[declaration], true, false).is_ok());
    assert!(validate_mechanism_declarations(&[], false, false).is_ok());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn logical_keys_parse_strictly() {
    for spelling in [
        "build:cargo",
        "package:windows-zip",
        "deploy:vibe-bin",
        "acquire:prebuilt",
    ] {
        let key = spelling.parse::<MechanismKey>().unwrap();
        assert_eq!(key.to_string(), spelling);
    }
    for invalid in [
        "cargo",
        "build:",
        ":cargo",
        "build",
        "",
        "BUILD:cargo",
        "build:Cargo",
        "test:cargo",
        "build:car go",
        "build:cargo:extra",
        "build:/cargo",
        "build:cargo#x",
        " build:cargo",
        "build:cargo ",
    ] {
        let error = invalid.parse::<MechanismKey>().expect_err("must reject");
        assert_eq!(error.input(), invalid);
        assert!(error.to_string().contains("`<role>:<portable-name>`"));
    }
    assert_eq!(
        "build:cargo".parse::<MechanismKey>().unwrap().role(),
        MechanismRole::Build
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn provider_pins_accept_only_group_qualified_exact_identities() {
    let pin: ProviderPin = "org.example/build-tools#cargo-v2".parse().unwrap();
    assert_eq!(pin.group().map(crate::Group::as_str), Some("org.example"));
    assert_eq!(
        pin.package().map(crate::PackageName::as_str),
        Some("build-tools")
    );
    assert!(pin.host_project().is_none());
    assert_eq!(pin.id(), "cargo-v2");
    assert_eq!(pin.to_string(), "org.example/build-tools#cargo-v2");

    for invalid in [
        "cargo-v2",
        "org.example/build-tools",
        "org.example/build-tools#",
        "#cargo-v2",
        "",
        "org.example/build-tools#cargo-v2@1.0",
        "org.example/build-tools@1.0#cargo-v2",
        "flow:org.example/build-tools#cargo-v2",
        "org.example/build-tools#Bad_Id",
        "org.example/build-tools#car go",
        "org.example/build tools#cargo-v2",
        "org.example#cargo-v2",
    ] {
        let error = ProviderPin::parse(invalid).expect_err("must reject");
        assert_eq!(error.input(), invalid);
        assert!(error.to_string().contains("PROP-054"), "{error}");
    }

    // The host head never leaks into the package branch.
    for host_shaped in [
        "__host__/demo#cargo-v2",
        "__host__/My%20Project#cargo-v2",
        "__host__/#cargo-v2",
    ] {
        let pin = ProviderPin::parse(host_shaped);
        assert!(
            pin.as_ref()
                .map(ProviderPin::group)
                .unwrap_or(None)
                .is_none(),
            "{host_shaped}"
        );
    }

    let versioned = ProviderPin::parse("org.example/build-tools#cargo-v2@1.0")
        .unwrap_err()
        .to_string();
    assert!(versioned.contains("PackageRef syntax"), "{versioned}");
    let short = ProviderPin::parse("cargo-v2").unwrap_err().to_string();
    assert!(short.contains("short id"), "{short}");
}

/// The reserved host branch: opaque by construction, structurally distinct
/// from a package coordinate, and spelled through the one host-owner codec —
/// so an arbitrary `[project].name` has exactly one pin and every pin decodes
/// back to exactly one name.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn host_provider_identities_parse_print_and_round_trip() {
    for (spelling, project) in [
        // Ordinary names stay byte-identical.
        ("__host__/demo#cargo-v2", "demo"),
        ("__host__/My.Project_2#cargo-v2", "My.Project_2"),
        ("__host__/#cargo-v2", ""),
        // Everything else is reversibly escaped.
        ("__host__/my%20app#cargo-v2", "my app"),
        (
            "__host__/a%2Fb%23c%40d%3Ae%5Cf%25g#cargo-v2",
            "a/b#c@d:e\\f%g",
        ),
        ("__host__/line%0Abreak#cargo-v2", "line\nbreak"),
        (
            "__host__/%D1%83%D0%BD%D0%B8%D0%BA%D0%BE%D0%B4#cargo-v2",
            "уникод",
        ),
        ("__host__/%2520#cargo-v2", "%20"),
    ] {
        let pin: ProviderPin = spelling.parse().unwrap();
        assert_eq!(pin.host_project(), Some(project), "{spelling}");
        assert_eq!(pin.id(), "cargo-v2");
        assert!(pin.group().is_none(), "{spelling}");
        assert!(pin.package().is_none(), "{spelling}");
        assert!(
            matches!(pin.owner(), ProviderOwner::Host { .. }),
            "{spelling}"
        );
        // Display is canonical and round-trips.
        assert_eq!(pin.to_string(), spelling);
        assert_eq!(pin.to_string().parse::<ProviderPin>().unwrap(), pin);
    }

    // `__host__` is not, and can never become, a real group: `_` is not an LDH
    // character, so the two owner spellings cannot collide.
    assert!(HOST_OWNER.parse::<crate::Group>().is_err());
    let package: ProviderPin = "org.example/build-tools#cargo-v2".parse().unwrap();
    let host: ProviderPin = "__host__/build-tools#cargo-v2".parse().unwrap();
    assert_ne!(package, host);
    assert_ne!(package.owner(), host.owner());

    // No two raw project names may print one pin: the pair that used to
    // collide through raw interpolation now cannot.
    let ambiguous: ProviderPin = "__host__/odd%2F%23%20project#x".parse().unwrap();
    assert_eq!(ambiguous.host_project(), Some("odd/# project"));
    assert_eq!(ambiguous.id(), "x");
    assert_ne!(ambiguous, "__host__/odd#x".parse::<ProviderPin>().unwrap());

    for invalid in [
        "__host__#cargo-v2",
        // Unescaped bytes, non-canonical and malformed escapes all refuse.
        "__host__/de mo#cargo-v2",
        "__host__/a/b#cargo-v2",
        "__host__/my%2fapp#cargo-v2",
        "__host__/%2D#cargo-v2",
        "__host__/%ZZ#cargo-v2",
        "__host__/%8_#cargo-v2",
        "__host__/%80#cargo-v2",
        "__host__/demo#Bad_Id",
        "__host__/demo",
        "__host__/demo#cargo-v2@1.0",
    ] {
        let error = ProviderPin::parse(invalid).expect_err("must reject");
        assert_eq!(error.input(), invalid);
        assert!(error.to_string().contains("PROP-054"), "{error}");
    }
}

/// `[mechanisms]` is a map: one entry answers one key and none shadows
/// another, so entry order carries no meaning — and is not preserved: map
/// keys render sorted. What is pinned is lookup and round-trip. The routes
/// are authored reverse-lexicographically so the sort is visible rather than
/// assumed away.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn routes_parse_look_up_and_round_trip() {
    let body = concat!(
        "[mechanisms]\n",
        "\"deploy:vibe-bin\" = \"org.example/installers#my-bin-layout\"\n",
        "\"build:cargo\" = \"org.example/build-tools#cargo-v2\"\n",
        "\"acquire:prebuilt\" = \"org.example/tools#fetch-v1\"\n",
    );
    let manifest = Manifest::parse_str(&format!("{PROJECT}\n{body}")).unwrap();
    assert_eq!(manifest.mechanism_routes.len(), 3);
    for (key, expected) in [
        ("build:cargo", "org.example/build-tools#cargo-v2"),
        ("deploy:vibe-bin", "org.example/installers#my-bin-layout"),
        ("acquire:prebuilt", "org.example/tools#fetch-v1"),
    ] {
        assert_eq!(
            manifest
                .mechanism_routes
                .get(key)
                .map(ProviderPin::to_string),
            Some(expected.to_string()),
            "{key}"
        );
    }

    let rendered = toml::to_string_pretty(&manifest).unwrap();
    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, reparsed);
    // Stated, not wished for: a map renders sorted, so no law may depend on
    // the order the operator wrote the routes in.
    let positions: Vec<usize> = ["acquire:prebuilt", "build:cargo", "deploy:vibe-bin"]
        .iter()
        .map(|key| rendered.find(key).expect("route rendered"))
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "map keys render sorted; the doc comment says so:\n{rendered}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn route_faults_name_key_or_value_with_remediation() {
    for (table, fragment) in [
        (
            "[mechanisms]\n\"cargo\" = \"org.example/build-tools#cargo-v2\"\n",
            "route key `cargo` is invalid",
        ),
        ("[mechanisms]\n\"build:cargo\" = \"cargo-v2\"\n", "short id"),
        (
            "[mechanisms]\n\"build:cargo\" = \"\"\n",
            "invalid provider identity",
        ),
        (
            "[mechanisms]\n\"build:cargo\" = \"org.example/build-tools#x@1.0\"\n",
            "PackageRef syntax",
        ),
    ] {
        let error = parse_error(PROJECT, table);
        assert!(error.contains(fragment), "fragment={fragment}: {error}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn routes_are_legal_in_a_pure_virtual_workspace() {
    let body = "[mechanisms]\n\"build:cargo\" = \"org.example/build-tools#cargo-v2\"\n";
    let manifest = Manifest::parse_str(&format!("{VIRTUAL}\n{body}")).unwrap();
    assert_eq!(manifest.mechanism_routes.len(), 1);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn acquire_is_a_provider_role_but_not_a_target_role() {
    let declaration = DECL.replace("role = \"build\"", "role = \"acquire\"");
    let manifest = Manifest::parse_str(&format!("{PROJECT}\n{declaration}")).unwrap();
    assert_eq!(manifest.mechanism_decls[0].role, MechanismRole::Acquire);

    let route = "[mechanisms]\n\"acquire:prebuilt\" = \"org.example/tools#fetch-v1\"\n";
    assert!(Manifest::parse_str(&format!("{PROJECT}\n{route}")).is_ok());

    // The target arrays reject it — pinned in the artifact/deploy tests; the
    // key parser itself keeps acquire first-class for routes.
    assert_eq!(
        "acquire:prebuilt".parse::<MechanismKey>().unwrap().role(),
        MechanismRole::Acquire
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn routes_wire_is_strict_about_unknown_shape() {
    // `[mechanisms]` is a flat string map; a nested table refuses at the
    // TOML-shape layer.
    let body = "[mechanisms]\n\"build:cargo\" = { nested = true }\n";
    match Manifest::parse_str(&format!("{PROJECT}\n{body}")) {
        Err(Error::ParseToml { .. }) => {}
        other => panic!("expected ParseToml for non-string route, got {other:?}"),
    }
}

#[test]
fn programmatic_invalid_declarations_fail_the_same_validator() {
    let mut manifest = Manifest::parse_str(&format!("{PROJECT}\n{DECL}")).unwrap();
    manifest.mechanism_decls[0].protocol = 0;
    let error = manifest.validate().unwrap_err().to_string();
    assert!(error.contains("field `protocol`"), "{error}");
    assert!(
        toml::to_string_pretty(&manifest)
            .unwrap_err()
            .to_string()
            .contains("field `protocol`")
    );

    let mut routes = MechanismRoutes::default();
    routes.insert(
        "build:cargo".parse().unwrap(),
        "org.example/build-tools#cargo-v2".parse().unwrap(),
    );
    assert_eq!(routes.len(), 1);
}
