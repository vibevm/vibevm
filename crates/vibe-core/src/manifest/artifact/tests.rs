//! Focused tests for `[[artifacts.build]]` / `[[artifacts.package]]`.

use specmark::verifies;

use super::{ArtifactInput, ArtifactOutput, ArtifactTarget, ArtifactsSection};
use crate::Error;
use crate::manifest::Manifest;

const PROJECT: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";
const VIRTUAL: &str = "[workspace]\nmembers = []\n";
const BUILD: &str = concat!(
    "[[artifacts.build]]\n",
    "id = \"helper\"\n",
    "mechanism = \"build:cargo\"\n",
    "inputs = [{ path = \"Cargo.toml\" }, { path = \"Cargo.lock\" }, { path = \"crates/helper/**\" }]\n",
    "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
    "config = { package = \"helper\", bin = \"helper\", profile = \"release\", locked = true }\n",
);
const PACKAGE: &str = concat!(
    "[[artifacts.package]]\n",
    "id = \"helper-windows\"\n",
    "mechanism = \"package:windows-zip\"\n",
    "inputs = [{ artifact = \"helper.exe\" }]\n",
    "outputs = [{ id = \"helper.zip\", kind = \"archive\" }]\n",
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

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn full_example_parses_and_round_trips() {
    let manifest = parse(&format!("{BUILD}\n{PACKAGE}"));
    let artifacts = manifest.artifacts.as_ref().unwrap();
    assert_eq!(artifacts.build.len(), 1);
    assert_eq!(artifacts.package.len(), 1);
    let build = &artifacts.build[0];
    assert_eq!(build.id, "helper");
    assert_eq!(build.mechanism.to_string(), "build:cargo");
    assert!(build.provider.is_none());
    assert_eq!(build.inputs.as_ref().map(Vec::len), Some(3));
    assert!(
        matches!(&build.inputs.as_ref().unwrap()[0], ArtifactInput::Path { path }
        if path.to_str() == Some("Cargo.toml"))
    );
    assert!(
        matches!(&build.inputs.as_ref().unwrap()[2], ArtifactInput::Path { path }
        if path.to_str() == Some("crates/helper/**"))
    );
    assert_eq!(build.outputs[0].id, "helper.exe");
    assert_eq!(build.outputs[0].kind, "executable");
    assert_eq!(
        build.config.as_ref().unwrap().as_table()["locked"].as_bool(),
        Some(true)
    );

    let rendered = toml::to_string_pretty(&manifest).unwrap();
    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, reparsed);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn optional_exact_pin_parses_and_round_trips() {
    let body = concat!(
        "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:cargo\"\n",
        "provider = \"org.example/build-tools#cargo-v2\"\n",
        "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
    );
    let manifest = parse(body);
    assert_eq!(
        manifest.artifacts.as_ref().unwrap().build[0]
            .provider
            .as_ref()
            .map(|pin| pin.to_string()),
        Some("org.example/build-tools#cargo-v2".to_string())
    );
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    assert_eq!(Manifest::parse_str(&rendered).unwrap(), manifest);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn unknown_and_missing_fields_refuse_at_the_shape_layer() {
    for (body, fragment) in [
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\nmystery = true\n",
            "unknown field",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.out\", kind = \"executable\", mystery = 1 }]\n",
            "unknown field",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\nconfig = 3\n",
            "config",
        ),
        (
            "[[artifacts.deploy]]\nid = \"x\"\nmechanism = \"deploy:vibe-bin\"\noutputs = []\n",
            "unknown field",
        ),
        (
            "[[artifacts.acquire]]\nid = \"x\"\nmechanism = \"acquire:prebuilt\"\noutputs = []\n",
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
            "[[artifacts.build]]\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "id",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
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
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.out\" }]\n",
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
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn empty_and_unsafe_ids_paths_and_providers_refuse_with_remediation() {
    for (body, fragment) in [
        (
            "[[artifacts.build]]\nid = \"\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "field `id`",
        ),
        (
            "[[artifacts.build]]\nid = \"Bad Id\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "field `id`",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"\", kind = \"executable\" }]\n",
            "portable token",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.out\", kind = \"Bad Kind\" }]\n",
            "field `kind`",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = []\n",
            "field `outputs` is empty",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\ninputs = [{ path = \"../escape\" }]\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "declarant-root-relative",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\ninputs = [{ path = \"C:\\\\escape\" }]\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "declarant-root-relative",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\ninputs = [{ path = \"\" }]\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "nonempty declarant-root-relative",
        ),
        // The shared declarant-path law: an input answers exactly what a
        // `[[skill]]` path answers, so nothing refused there can arrive here.
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\ninputs = [{ path = \"crates/h/x.rs:zone.identifier\" }]\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "alternate data stream",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\ninputs = [{ path = \"dist/aux\" }]\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "reserved device name",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\ninputs = [{ path = \"dist/CON.txt\" }]\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "reserved device name",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\ninputs = [{ path = \"out/secret.txt. \" }]\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "silently strips",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\ninputs = [{ path = \"a//b\" }]\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "empty path segment",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\ninputs = [{ path = \"./Cargo.toml\" }]\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "`.` or `..` segment",
        ),
        // An unspellable artifact ref is a shape fault, not a missing
        // declaration: it refuses on the token law, never as `unknown
        // artifact \`\``.
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\ninputs = [{ artifact = \"\" }]\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "field `inputs` artifact value `` is not a portable token",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\ninputs = [{ artifact = \"Bad Ref\" }]\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "field `inputs` artifact value `Bad Ref` is not a portable token",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\nprovider = \"cargo-v2\"\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "short id",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"cargo\"\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n",
            "field `mechanism`",
        ),
    ] {
        let error = parse_error(body);
        assert!(error.contains(fragment), "fragment={fragment}: {error}");
        assert!(error.contains("PROP-054"), "{error}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
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
        let body = format!(
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\ninputs = [{row}]\noutputs = [{{ id = \"x.out\", kind = \"executable\" }}]\n"
        );
        let error = parse_error(&body);
        assert!(error.contains(fragment), "fragment={fragment}: {error}");
        assert!(error.contains("row 0"), "{error}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn duplicate_ids_refuse_globally() {
    // Duplicate target id within build.
    let duplicate = concat!(
        "[[artifacts.build]]\nid = \"same\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"a.out\", kind = \"executable\" }]\n",
        "[[artifacts.build]]\nid = \"same\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"b.out\", kind = \"executable\" }]\n",
    );
    let error = parse_error(duplicate);
    assert!(error.contains("duplicate [[artifacts.build]]"), "{error}");
    assert!(error.contains("value `same`"), "{error}");

    // Target id colliding across phases.
    let cross_phase = concat!(
        "[[artifacts.build]]\nid = \"same\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"a.out\", kind = \"executable\" }]\n",
        "[[artifacts.package]]\nid = \"same\"\nmechanism = \"package:zip\"\noutputs = [{ id = \"a.zip\", kind = \"archive\" }]\n",
    );
    let error = parse_error(cross_phase);
    assert!(error.contains("duplicate [[artifacts.package]]"), "{error}");

    // Duplicate output id.
    let duplicate_output = concat!(
        "[[artifacts.build]]\nid = \"first\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"same.out\", kind = \"executable\" }]\n",
        "[[artifacts.build]]\nid = \"second\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"same.out\", kind = \"executable\" }]\n",
    );
    let error = parse_error(duplicate_output);
    assert!(
        error.contains("duplicate artifact id `same.out`"),
        "{error}"
    );

    // Target id colliding with another target's output id.
    let collision = concat!(
        "[[artifacts.build]]\nid = \"first\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"second\", kind = \"executable\" }]\n",
        "[[artifacts.build]]\nid = \"second\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"other.out\", kind = \"executable\" }]\n",
    );
    let error = parse_error(collision);
    assert!(error.contains("duplicate artifact id `second`"), "{error}");
    assert!(
        error.contains("collides with a declared output id"),
        "{error}"
    );

    // Output id colliding with a later target id.
    let reverse_collision = concat!(
        "[[artifacts.build]]\nid = \"first\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"second.out\", kind = \"executable\" }]\n",
        "[[artifacts.build]]\nid = \"second.out\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"other.out\", kind = \"executable\" }]\n",
    );
    let error = parse_error(reverse_collision);
    assert!(
        error.contains("duplicate artifact id `second.out`"),
        "{error}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn role_mismatch_and_backward_phase_edges_refuse() {
    let mismatch = "[[artifacts.build]]\nid = \"x\"\nmechanism = \"package:zip\"\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n";
    let error = parse_error(mismatch);
    assert!(error.contains("role `package`"), "{error}");
    assert!(
        error.contains("must equal the target's phase family"),
        "{error}"
    );

    let acquire = "[[artifacts.build]]\nid = \"x\"\nmechanism = \"acquire:prebuilt\"\noutputs = [{ id = \"x.out\", kind = \"executable\" }]\n";
    let error = parse_error(acquire);
    assert!(error.contains("role `acquire`"), "{error}");

    // Build consuming a package output is a backward edge.
    let backward = concat!(
        "[[artifacts.package]]\nid = \"pkg\"\nmechanism = \"package:zip\"\noutputs = [{ id = \"dist.zip\", kind = \"archive\" }]\n",
        "[[artifacts.build]]\nid = \"bin\"\nmechanism = \"build:cargo\"\ninputs = [{ artifact = \"dist.zip\" }]\noutputs = [{ id = \"bin.exe\", kind = \"executable\" }]\n",
    );
    let error = parse_error(backward);
    assert!(error.contains("phase-forward"), "{error}");
    assert!(error.contains("`dist.zip`"), "{error}");

    // Package consuming build output is legal.
    assert!(parse(&format!("{BUILD}\n{PACKAGE}")).validate().is_ok());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn unknown_artifact_refs_refuse() {
    let unknown = "[[artifacts.package]]\nid = \"pkg\"\nmechanism = \"package:zip\"\ninputs = [{ artifact = \"ghost.exe\" }]\noutputs = [{ id = \"pkg.zip\", kind = \"archive\" }]\n";
    let error = parse_error(unknown);
    assert!(error.contains("unknown artifact `ghost.exe`"), "{error}");

    // A target id is not an artifact ref: outputs are the currency.
    let target_id_ref = concat!(
        "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
        "[[artifacts.package]]\nid = \"pkg\"\nmechanism = \"package:zip\"\ninputs = [{ artifact = \"helper\" }]\noutputs = [{ id = \"pkg.zip\", kind = \"archive\" }]\n",
    );
    let error = parse_error(target_id_ref);
    assert!(error.contains("unknown artifact `helper`"), "{error}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn artifact_cycles_refuse_in_both_phases() {
    let build_cycle = concat!(
        "[[artifacts.build]]\nid = \"first\"\nmechanism = \"build:cargo\"\ninputs = [{ artifact = \"second.out\" }]\noutputs = [{ id = \"first.out\", kind = \"executable\" }]\n",
        "[[artifacts.build]]\nid = \"second\"\nmechanism = \"build:cargo\"\ninputs = [{ artifact = \"first.out\" }]\noutputs = [{ id = \"second.out\", kind = \"executable\" }]\n",
    );
    let error = parse_error(build_cycle);
    assert!(error.contains("cyclic"), "{error}");
    assert!(error.contains("first -> second -> first"), "{error}");

    let package_cycle = concat!(
        "[[artifacts.package]]\nid = \"outer\"\nmechanism = \"package:zip\"\ninputs = [{ artifact = \"inner.zip\" }]\noutputs = [{ id = \"outer.zip\", kind = \"archive\" }]\n",
        "[[artifacts.package]]\nid = \"inner\"\nmechanism = \"package:zip\"\ninputs = [{ artifact = \"outer.zip\" }]\noutputs = [{ id = \"inner.zip\", kind = \"archive\" }]\n",
    );
    let error = parse_error(package_cycle);
    assert!(error.contains("cyclic"), "{error}");

    // Self-cycle through own output.
    let self_cycle = "[[artifacts.build]]\nid = \"looped\"\nmechanism = \"build:cargo\"\ninputs = [{ artifact = \"looped.out\" }]\noutputs = [{ id = \"looped.out\", kind = \"executable\" }]\n";
    let error = parse_error(self_cycle);
    assert!(error.contains("cyclic"), "{error}");
    assert!(error.contains("looped -> looped"), "{error}");

    // A legal multi-stage chain passes.
    let chain = concat!(
        "[[artifacts.build]]\nid = \"core\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"core.lib\", kind = \"library\" }]\n",
        "[[artifacts.build]]\nid = \"app\"\nmechanism = \"build:cargo\"\ninputs = [{ artifact = \"core.lib\" }]\noutputs = [{ id = \"app.exe\", kind = \"executable\" }]\n",
        "[[artifacts.package]]\nid = \"bundle\"\nmechanism = \"package:zip\"\ninputs = [{ artifact = \"app.exe\" }, { artifact = \"core.lib\" }]\noutputs = [{ id = \"bundle.zip\", kind = \"archive\" }]\n",
    );
    assert!(parse(chain).validate().is_ok());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn artifacts_require_project_or_package_role() {
    let error = Manifest::parse_str(&format!("{VIRTUAL}\n{BUILD}"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("[artifacts] desired targets require"),
        "{error}"
    );
    assert!(error.contains("pure virtual"), "{error}");

    let package =
        "[package]\ngroup = \"org.demo\"\nname = \"pkg\"\nkind = \"flow\"\nversion = \"0.1.0\"\n";
    assert!(Manifest::parse_str(&format!("{package}\n{BUILD}")).is_ok());
}

#[test]
fn programmatic_sections_fail_the_same_validator() {
    let mut manifest = parse(BUILD);
    let artifacts = manifest.artifacts.as_mut().unwrap();
    artifacts.build[0].outputs.clear();
    let error = manifest.validate().unwrap_err().to_string();
    assert!(error.contains("field `outputs` is empty"), "{error}");
    assert!(
        toml::to_string_pretty(&manifest)
            .unwrap_err()
            .to_string()
            .contains("field `outputs` is empty")
    );

    let mut bad_role = ArtifactsSection::default();
    bad_role.build.push(ArtifactTarget {
        id: "x".into(),
        mechanism: "deploy:vibe-bin".parse().unwrap(),
        provider: None,
        inputs: None,
        outputs: vec![ArtifactOutput {
            id: "x.out".into(),
            kind: "executable".into(),
        }],
        config: None,
    });
    let error = bad_role.validate().unwrap_err();
    assert!(
        error.contains("must equal the target's phase family"),
        "{error}"
    );
}
