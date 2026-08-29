//! Shape tests for `[[artifacts.build]]` / `[[artifacts.package]]` in the
//! amended A1 spelling: round-trips, the exact `provider` pin, `workdir`,
//! `select`, the closed `kind` vocabulary, strict unknown/missing fields and
//! the tagged one-of input rows. The pure validation laws live next door in
//! `tests_validation.rs`.

use crate::Error;
use crate::manifest::{ArtifactInput, ArtifactKind, Manifest};

const PROJECT: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";
const BUILD: &str = concat!(
    "[[artifacts.build]]\n",
    "id = \"vibe-helper\"\n",
    "mechanism = \"build:cargo\"\n",
    "inputs = [{ path = \"Cargo.toml\" }, { path = \"Cargo.lock\" }, { path = \"crates/vibe-helper/**\" }]\n",
    "outputs = [\n",
    "  { id = \"vibe-helper.exe\", kind = \"executable\",\n",
    "    select = { package = \"vibe-helper\", bin = \"vibe-helper\" } },\n",
    "]\n",
    "config = { profile = \"release\", locked = true }\n",
);
const PACKAGE: &str = concat!(
    "[[artifacts.package]]\n",
    "id = \"vibe-helper-windows\"\n",
    "mechanism = \"package:windows-zip\"\n",
    "inputs = [{ artifact = \"vibe-helper.exe\" }, { path = \"assets/readme.md\" }]\n",
    "outputs = [{ id = \"vibe-helper.zip\", kind = \"archive\" }]\n",
    "config = { layout = \"distribution/windows\" }\n",
);

fn parse(body: &str) -> Manifest {
    Manifest::parse_str(&format!("{PROJECT}\n{body}")).unwrap()
}

fn parse_error(body: &str) -> String {
    Manifest::parse_str(&format!("{PROJECT}\n{body}"))
        .unwrap_err()
        .to_string()
}

/// Acceptance 1 (artifact half): the §4 architecture example parses, every
/// field is reachable through an accessor, and the document round-trips.
#[test]
fn full_example_parses_and_round_trips() {
    let manifest = parse(&format!("{BUILD}\n{PACKAGE}"));
    let artifacts = manifest.artifacts.as_ref().unwrap();
    assert_eq!(artifacts.build.len(), 1);
    assert_eq!(artifacts.package.len(), 1);

    let build = &artifacts.build[0];
    assert_eq!(build.id, "vibe-helper");
    assert_eq!(build.mechanism.to_string(), "build:cargo");
    assert!(build.provider.is_none());
    // `workdir` was not authored: the amended default `"."` fills it in.
    assert_eq!(build.workdir, ".");
    let inputs = build.inputs.as_ref().unwrap();
    assert_eq!(inputs.len(), 3);
    assert!(
        matches!(&inputs[0], ArtifactInput::Path { path } if path.to_str() == Some("Cargo.toml"))
    );
    assert!(
        matches!(&inputs[2], ArtifactInput::Path { path } if path.to_str() == Some("crates/vibe-helper/**"))
    );
    assert_eq!(build.outputs[0].id, "vibe-helper.exe");
    assert_eq!(build.outputs[0].kind, ArtifactKind::Executable);
    assert_eq!(build.outputs[0].kind.as_str(), "executable");
    assert_eq!(
        build.outputs[0].select.as_ref().unwrap().as_table()["bin"].as_str(),
        Some("vibe-helper")
    );
    assert_eq!(
        build.config.as_ref().unwrap().as_table()["locked"].as_bool(),
        Some(true)
    );

    let package = &artifacts.package[0];
    assert_eq!(package.id, "vibe-helper-windows");
    assert_eq!(package.mechanism.to_string(), "package:windows-zip");
    let inputs = package.inputs.as_ref().unwrap();
    assert_eq!(inputs.len(), 2);
    assert!(
        matches!(&inputs[0], ArtifactInput::Artifact { artifact } if artifact == "vibe-helper.exe")
    );
    assert!(
        matches!(&inputs[1], ArtifactInput::Path { path } if path.to_str() == Some("assets/readme.md"))
    );
    assert_eq!(package.outputs[0].id, "vibe-helper.zip");
    assert_eq!(package.outputs[0].kind, ArtifactKind::Archive);
    assert!(package.outputs[0].select.is_none());
    assert_eq!(
        package.config.as_ref().unwrap().as_table()["layout"].as_str(),
        Some("distribution/windows")
    );

    let rendered = toml::to_string_pretty(&manifest).unwrap();
    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, reparsed);
}

/// An authored non-default `workdir` survives the round trip; the default
/// stays implicit and does not leak into the render.
#[test]
fn explicit_workdir_round_trips_and_default_stays_implicit() {
    let body = concat!(
        "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:cargo\"\n",
        "workdir = \"crates/helper\"\ninputs = [{ path = \"Cargo.toml\" }]\n",
        "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
    );
    let manifest = parse(body);
    assert_eq!(
        manifest.artifacts.as_ref().unwrap().build[0].workdir,
        "crates/helper"
    );
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    assert!(
        rendered.contains("workdir = \"crates/helper\""),
        "{rendered}"
    );
    assert_eq!(Manifest::parse_str(&rendered).unwrap(), manifest);

    let default = parse(BUILD);
    let rendered = toml::to_string_pretty(&default).unwrap();
    assert!(!rendered.contains("workdir"), "{rendered}");
}

/// The exact `provider` pin parses and round-trips on both families.
#[test]
fn optional_exact_pins_parse_and_round_trip_on_both_families() {
    let body = concat!(
        "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:cargo\"\n",
        "provider = \"org.example/build-tools#cargo-v2\"\n",
        "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
        "[[artifacts.package]]\nid = \"bundle\"\nmechanism = \"package:zip\"\n",
        "provider = \"org.example/build-tools#zip-v1\"\n",
        "outputs = [{ id = \"bundle.zip\", kind = \"archive\" }]\n",
    );
    let manifest = parse(body);
    let artifacts = manifest.artifacts.as_ref().unwrap();
    assert_eq!(
        artifacts.build[0]
            .provider
            .as_ref()
            .map(|pin| pin.to_string()),
        Some("org.example/build-tools#cargo-v2".to_string())
    );
    assert_eq!(
        artifacts.package[0]
            .provider
            .as_ref()
            .map(|pin| pin.to_string()),
        Some("org.example/build-tools#zip-v1".to_string())
    );
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    assert_eq!(Manifest::parse_str(&rendered).unwrap(), manifest);

    // A short id is not a pin.
    let short = concat!(
        "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:cargo\"\n",
        "provider = \"cargo-v2\"\n",
        "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
    );
    let error = parse_error(short);
    assert!(error.contains("short id"), "{error}");
}

/// Acceptance 2: an unknown field in every structural table refuses naming
/// the field; the opaque `select`/`config` tables stay opaque on purpose.
#[test]
fn unknown_and_missing_fields_refuse_at_the_shape_layer() {
    for (body, fragment) in [
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.exe\", kind = \"executable\" }]\nmystery = true\n",
            "unknown field",
        ),
        (
            "[[artifacts.package]]\nid = \"x\"\nmechanism = \"package:zip\"\noutputs = [{ id = \"x.zip\", kind = \"archive\" }]\nmystery = 1\n",
            "unknown field",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.exe\", kind = \"executable\", mystery = 1 }]\n",
            "unknown field",
        ),
        (
            "[[artifacts.deploy]]\nid = \"x\"\nmechanism = \"deploy:vibe-bin\"\noutputs = [{ id = \"x\", kind = \"file\" }]\n",
            "unknown field",
        ),
        (
            "[[artifacts.acquire]]\nid = \"x\"\nmechanism = \"acquire:prebuilt\"\noutputs = [{ id = \"x\", kind = \"file\" }]\n",
            "unknown field",
        ),
    ] {
        match Manifest::parse_str(&format!("{PROJECT}\n{body}")) {
            Err(Error::ParseToml { diagnostic, .. }) => {
                let detail = diagnostic.to_string();
                assert!(detail.contains(fragment), "fragment={fragment}: {detail}");
            }
            other => panic!("expected ParseToml, got {other:?}"),
        }
    }
    for (body, field) in [
        (
            "[[artifacts.build]]\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.exe\", kind = \"executable\" }]\n",
            "id",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\noutputs = [{ id = \"x.exe\", kind = \"executable\" }]\n",
            "mechanism",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\n",
            "outputs",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{ kind = \"executable\" }]\n",
            "id",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.exe\" }]\n",
            "kind",
        ),
    ] {
        match Manifest::parse_str(&format!("{PROJECT}\n{body}")) {
            Err(Error::ParseToml { diagnostic, .. }) => {
                let detail = diagnostic.to_string();
                assert!(detail.contains(field), "field={field}: {detail}");
            }
            other => panic!("expected ParseToml for missing `{field}`, got {other:?}"),
        }
    }
    // `inputs` is optional (absent means "no inputs") and stays distinct
    // from an authored empty list through the round trip.
    let absent = "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.exe\", kind = \"executable\" }]\n";
    let manifest = parse(absent);
    assert!(
        manifest.artifacts.as_ref().unwrap().build[0]
            .inputs
            .is_none()
    );
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    assert_eq!(Manifest::parse_str(&rendered).unwrap(), manifest);

    // The opaque tables refuse no key: `select` and `config` are provider
    // surface, not structural grammar.
    let opaque = concat!(
        "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\n",
        "outputs = [{ id = \"x.exe\", kind = \"executable\", select = { anything = true } }]\n",
        "config = { also = \"anything\" }\n",
    );
    assert!(parse(opaque).validate().is_ok());
}

/// Input rows are a strict tagged one-of, in both families.
#[test]
fn input_rows_are_strict_tagged_one_of() {
    for (row, fragment) in [
        (
            "{ path = \"a\", artifact = \"b\" }",
            "both `path` and `artifact`",
        ),
        ("{}", "neither `path` nor `artifact`"),
        ("{ mystery = true }", "neither `path` nor `artifact`"),
        ("{ path = \"a\", mystery = true }", "plus unknown field"),
        ("{ artifact = \"a\", mystery = true }", "plus unknown field"),
        ("{ path = 3 }", "must be a string"),
    ] {
        for table in ["build", "package"] {
            let body = format!(
                "[[artifacts.{table}]]\nid = \"x\"\nmechanism = \"{table}:cargo\"\ninputs = [{row}]\noutputs = [{{ id = \"x.exe\", kind = \"file\" }}]\n"
            );
            let error = parse_error(&body);
            assert!(error.contains(fragment), "fragment={fragment}: {error}");
            assert!(error.contains("row 0"), "{error}");
        }
    }
}
