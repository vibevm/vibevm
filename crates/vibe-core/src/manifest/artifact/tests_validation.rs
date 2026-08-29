//! Validation-law tests for `[[artifacts.build]]` / `[[artifacts.package]]`:
//! identity uniqueness, resolution, the mechanism family law, the incumbent
//! phase-forward edge law, named cycles, the id grammar, the path laws and
//! the legacy-compatibility pin.

use crate::Error;
use crate::manifest::{
    ArtifactBuildTarget, ArtifactInput, ArtifactKind, ArtifactOutput, ArtifactsSection, Manifest,
};

const PROJECT: &str = "[project]
name = \"demo\"
version = \"0.1.0\"
";
const VIRTUAL: &str = "[workspace]
members = []
";
const BUILD: &str = concat!(
    "[[artifacts.build]]
",
    "id = \"vibe-helper\"
",
    "mechanism = \"build:cargo\"
",
    "inputs = [{ path = \"Cargo.toml\" }, { path = \"Cargo.lock\" }, { path = \"crates/vibe-helper/**\" }]
",
    "outputs = [
",
    "  { id = \"vibe-helper.exe\", kind = \"executable\",
",
    "    select = { package = \"vibe-helper\", bin = \"vibe-helper\" } },
",
    "]
",
    "config = { profile = \"release\", locked = true }
",
);
const PACKAGE: &str = concat!(
    "[[artifacts.package]]
",
    "id = \"vibe-helper-windows\"
",
    "mechanism = \"package:windows-zip\"
",
    "inputs = [{ artifact = \"vibe-helper.exe\" }, { path = \"assets/readme.md\" }]
",
    "outputs = [{ id = \"vibe-helper.zip\", kind = \"archive\" }]
",
    "config = { layout = \"distribution/windows\" }
",
);

fn parse(body: &str) -> Manifest {
    Manifest::parse_str(&format!(
        "{PROJECT}
{body}"
    ))
    .unwrap()
}

fn parse_error(body: &str) -> String {
    Manifest::parse_str(&format!(
        "{PROJECT}
{body}"
    ))
    .unwrap_err()
    .to_string()
}

/// Acceptance 3 (artifact half): the duplicate target id, the duplicate
/// output id and the dangling artifact input are three separate typed
/// refusals.
#[test]
fn duplicates_and_dangling_inputs_refuse_separately() {
    // Duplicate target id within one family.
    let duplicate = format!(
        "{}{}",
        "[[artifacts.build]]\nid = \"same\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"a.exe\", kind = \"executable\" }]\n",
        "[[artifacts.build]]\nid = \"same\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"b.exe\", kind = \"executable\" }]\n",
    );
    let error = parse_error(&duplicate);
    assert!(
        error.contains("duplicate [[artifacts.build]] field `id`"),
        "{error}"
    );
    assert!(error.contains("value `same`"), "{error}");

    // Duplicate output id across two targets.
    let duplicate_output = format!(
        "{}{}",
        "[[artifacts.build]]\nid = \"first\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"same.exe\", kind = \"executable\" }]\n",
        "[[artifacts.build]]\nid = \"second\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"same.exe\", kind = \"executable\" }]\n",
    );
    let error = parse_error(&duplicate_output);
    assert!(
        error.contains("duplicate artifact id `same.exe`"),
        "{error}"
    );
    assert!(
        error.contains("output of [[artifacts.build]] `second`"),
        "{error}"
    );

    // A target id colliding with a declared output id.
    let collision = format!(
        "{}{}",
        "[[artifacts.build]]\nid = \"first\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"second\", kind = \"executable\" }]\n",
        "[[artifacts.build]]\nid = \"second\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"other.exe\", kind = \"executable\" }]\n",
    );
    let error = parse_error(&collision);
    assert!(error.contains("duplicate artifact id `second`"), "{error}");
    assert!(
        error.contains("collides with a declared output id"),
        "{error}"
    );

    // An artifact input that resolves to no declared output id.
    let dangling = concat!(
        "[[artifacts.package]]\nid = \"pkg\"\nmechanism = \"package:zip\"\n",
        "inputs = [{ artifact = \"ghost.exe\" }]\noutputs = [{ id = \"pkg.zip\", kind = \"archive\" }]\n",
    );
    let error = parse_error(dangling);
    assert!(
        error.contains("references unknown artifact `ghost.exe`"),
        "{error}"
    );
}

/// Acceptance 4: a mechanism key whose role prefix disagrees with the table
/// family refuses, naming the key and both roles.
#[test]
fn mechanism_prefix_must_match_the_table_family() {
    for (body, fragment) in [
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"package:zip\"\noutputs = [{ id = \"x.exe\", kind = \"executable\" }]\n",
            "has role `package`",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"deploy:vibe-bin\"\noutputs = [{ id = \"x.exe\", kind = \"executable\" }]\n",
            "has role `deploy`",
        ),
        (
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"acquire:prebuilt\"\noutputs = [{ id = \"x.exe\", kind = \"executable\" }]\n",
            "has role `acquire`",
        ),
        (
            "[[artifacts.package]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"x.zip\", kind = \"archive\" }]\n",
            "has role `build`",
        ),
    ] {
        let error = parse_error(body);
        assert!(
            error.contains("must equal the target's phase family"),
            "fragment={fragment}: {error}"
        );
        assert!(error.contains(fragment), "{error}");
        assert!(error.contains("PROP-054"), "{error}");
    }
}

/// The incumbent phase-forward law: package may consume build outputs, build
/// never consumes package. A lone backward edge refuses naming the artifact
/// and the producing phase.
#[test]
fn backward_phase_edges_refuse_with_the_incumbent_law() {
    let backward = format!(
        "{}{}",
        "[[artifacts.package]]\nid = \"pkg\"\nmechanism = \"package:zip\"\noutputs = [{ id = \"dist.zip\", kind = \"archive\" }]\n",
        "[[artifacts.build]]\nid = \"bin\"\nmechanism = \"build:cargo\"\ninputs = [{ artifact = \"dist.zip\" }]\noutputs = [{ id = \"bin.exe\", kind = \"executable\" }]\n",
    );
    let error = parse_error(&backward);
    assert!(error.contains("phase-forward"), "{error}");
    assert!(error.contains("`dist.zip`"), "{error}");
    assert!(
        error.contains("build cannot consume package or deploy"),
        "{error}"
    );

    // Package consuming a build output stays legal, in either authoring
    // order.
    assert!(parse(&format!("{BUILD}\n{PACKAGE}")).validate().is_ok());
}

/// Acceptance 5 (artifact half): the build→package→build cycle refuses
/// naming the closed sequence of ids, as do same-family cycles.
#[test]
fn cycles_refuse_and_name_the_cycle() {
    // build -> package -> build through input references.
    let cross_family = format!(
        "{}{}",
        "[[artifacts.build]]\nid = \"bin\"\nmechanism = \"build:cargo\"\ninputs = [{ artifact = \"dist.zip\" }]\noutputs = [{ id = \"bin.exe\", kind = \"executable\" }]\n",
        "[[artifacts.package]]\nid = \"pkg\"\nmechanism = \"package:zip\"\ninputs = [{ artifact = \"bin.exe\" }]\noutputs = [{ id = \"dist.zip\", kind = \"archive\" }]\n",
    );
    let error = parse_error(&cross_family);
    assert!(error.contains("cyclic"), "{error}");
    // Deterministic: the walk starts at the sorted-first root (`bin`).
    assert!(error.contains("bin -> pkg -> bin"), "{error}");

    // A same-family package cycle.
    let package_cycle = concat!(
        "[[artifacts.package]]\nid = \"outer\"\nmechanism = \"package:zip\"\ninputs = [{ artifact = \"inner.zip\" }]\noutputs = [{ id = \"outer.zip\", kind = \"archive\" }]\n",
        "[[artifacts.package]]\nid = \"inner\"\nmechanism = \"package:zip\"\ninputs = [{ artifact = \"outer.zip\" }]\noutputs = [{ id = \"inner.zip\", kind = \"archive\" }]\n",
    );
    let error = parse_error(package_cycle);
    assert!(error.contains("cyclic"), "{error}");
    assert!(error.contains("inner -> outer -> inner"), "{error}");

    // Self-cycle through own output.
    let self_cycle = concat!(
        "[[artifacts.build]]\nid = \"looped\"\nmechanism = \"build:cargo\"\n",
        "inputs = [{ artifact = \"looped.exe\" }]\noutputs = [{ id = \"looped.exe\", kind = \"executable\" }]\n",
    );
    let error = parse_error(self_cycle);
    assert!(error.contains("cyclic"), "{error}");
    assert!(error.contains("looped -> looped"), "{error}");

    // A legal multi-stage chain passes: build -> build -> package.
    let chain = format!(
        "{}{}{}",
        "[[artifacts.build]]\nid = \"core\"\nmechanism = \"build:cargo\"\noutputs = [{ id = \"core.lib\", kind = \"file\" }]\n",
        "[[artifacts.build]]\nid = \"app\"\nmechanism = \"build:cargo\"\ninputs = [{ artifact = \"core.lib\" }]\noutputs = [{ id = \"app.exe\", kind = \"executable\" }]\n",
        "[[artifacts.package]]\nid = \"bundle\"\nmechanism = \"package:zip\"\ninputs = [{ artifact = \"app.exe\" }, { artifact = \"core.lib\" }]\noutputs = [{ id = \"bundle.zip\", kind = \"archive\" }]\n",
    );
    assert!(parse(&chain).validate().is_ok());
}

/// Acceptance 7: an id outside the portable-token grammar refuses — for
/// target ids, output ids and artifact-input ids alike. Underscores, a
/// leading dash, a trailing dash/dot, `..` runs and uppercase are all out.
#[test]
fn id_grammar_refuses_uppercase_leading_dash_and_friends() {
    let build = |id: &str| {
        format!(
            "[[artifacts.build]]\nid = \"{id}\"\nmechanism = \"build:cargo\"\noutputs = [{{ id = \"x.exe\", kind = \"executable\" }}]\n"
        )
    };
    let output = |id: &str| {
        format!(
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{{ id = \"{id}\", kind = \"executable\" }}]\n"
        )
    };
    let input = |id: &str| {
        format!(
            "[[artifacts.package]]\nid = \"pkg\"\nmechanism = \"package:zip\"\ninputs = [{{ artifact = \"{id}\" }}]\noutputs = [{{ id = \"pkg.zip\", kind = \"archive\" }}]\n"
        )
    };
    for invalid in [
        "Bad-Id",
        "-lead",
        "trail-",
        "under_score",
        "a..b",
        "",
        "x@y",
        "a b",
    ] {
        for error in [parse_error(&build(invalid)), parse_error(&output(invalid))] {
            assert!(
                error.contains("is not a portable token"),
                "id={invalid:?}: {error}"
            );
            assert!(error.contains("PROP-054"), "{error}");
        }
    }
    // An artifact-input id with a forbidden spelling refuses on the token
    // law, never as a misleading "unknown artifact".
    for invalid in ["Bad Ref", "", "Under_Score"] {
        let error = parse_error(&input(invalid));
        assert!(
            error.contains(&format!(
                "field `inputs` artifact value `{invalid}` is not a portable token"
            )),
            "id={invalid:?}: {error}"
        );
    }
    // Long but well-formed ids stay legal: the plane's grammar has no length
    // cap.
    let long = "a".repeat(200);
    assert!(parse(&build(&long)).validate().is_ok());
}

/// Acceptance 8: a backslash or an escape in `workdir`/`inputs` path rows
/// refuses on the one declarant-path law.
#[test]
fn workdir_and_inputs_refuse_backslash_and_escape() {
    let build_with = |workdir: &str, inputs: &str| {
        format!(
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\nworkdir = \"{workdir}\"\ninputs = [{inputs}]\noutputs = [{{ id = \"x.exe\", kind = \"executable\" }}]\n"
        )
    };
    for (workdir, fragment) in [
        ("src\\\\helper", "backslash"),
        ("../escape", "`.` or `..` segment"),
        ("", "value is empty"),
        ("a//b", "empty path segment"),
    ] {
        let error = parse_error(&build_with(workdir, "{ path = \"a\" }"));
        assert!(
            error.contains("field `workdir`"),
            "workdir={workdir:?}: {error}"
        );
        assert!(error.contains(fragment), "workdir={workdir:?}: {error}");
    }
    // `.` itself stays legal: the authored default names the root.
    assert!(
        parse(&build_with(".", "{ path = \"a\" }"))
            .validate()
            .is_ok()
    );
    for (inputs, fragment) in [
        ("{ path = \"src\\\\x\" }", "backslash"),
        ("{ path = \"../escape\" }", "`.` or `..` segment"),
        ("{ path = \"\" }", "value is empty"),
        ("{ path = \"dist/nul\" }", "reserved device name"),
    ] {
        let error = parse_error(&build_with("src", inputs));
        assert!(
            error.contains("field `inputs`"),
            "inputs={inputs:?}: {error}"
        );
        assert!(error.contains(fragment), "inputs={inputs:?}: {error}");
    }
}

/// `kind` is the closed lowercase vocabulary; a value outside it refuses at
/// the shape layer, and all six members round-trip.
#[test]
fn kind_is_a_closed_vocabulary() {
    for kind in [
        "executable",
        "archive",
        "file",
        "directory",
        "skill",
        "agent-plugin",
    ] {
        let body = format!(
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{{ id = \"x.{kind}\", kind = \"{kind}\" }}]\n"
        );
        let manifest = parse(&body);
        assert_eq!(
            manifest.artifacts.as_ref().unwrap().build[0].outputs[0]
                .kind
                .as_str(),
            kind
        );
        let rendered = toml::to_string_pretty(&manifest).unwrap();
        assert_eq!(Manifest::parse_str(&rendered).unwrap(), manifest);
    }
    for kind in ["library", "debug-info", "Executable", ""] {
        let body = format!(
            "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = [{{ id = \"x.out\", kind = \"{kind}\" }}]\n"
        );
        match Manifest::parse_str(&format!("{PROJECT}\n{body}")) {
            Err(Error::ParseToml { diagnostic, .. }) => {
                assert!(diagnostic.to_string().contains("kind"), "kind={kind:?}");
            }
            other => panic!("expected ParseToml for kind={kind:?}, got {other:?}"),
        }
    }
}

/// Empty `outputs` refuses where the freeze requires nonempty.
#[test]
fn empty_outputs_refuse() {
    let empty_outputs =
        "[[artifacts.build]]\nid = \"x\"\nmechanism = \"build:cargo\"\noutputs = []\n";
    let error = parse_error(empty_outputs);
    assert!(error.contains("field `outputs` is empty"), "{error}");
}

/// Desired targets need a `[project]` or `[package]` role.
#[test]
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

/// Acceptance 9: a fixture manifest written before the new sections existed
/// parses exactly as before — no new section required, none leaked into the
/// render, and the render is a byte-stable fixpoint.
#[test]
fn legacy_manifests_without_new_sections_parse_unchanged() {
    let raw = r#"[project]
name = "my-client"
version = "0.0.1"
authors = ["Oleg <oleg@example.com>"]
[requires.packages]
"org.vibevm/wal" = "^0.3"
"org.vibevm/rust-cli" = "^0.1.0"
[active]
stack = "rust-cli"
[[registry]]
name = "vibespecs"
url = "https://github.com/vibespecs"
[[mirror]]
of = "vibespecs"
url = "https://mirror.internal/vibespecs"
priority = 1
"#;
    let manifest = Manifest::parse_str(raw).unwrap();
    manifest.validate().unwrap();
    assert!(manifest.artifacts.is_none());
    assert!(manifest.deploy.is_none());
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    for absent in ["artifacts", "deploy"] {
        assert!(!rendered.contains(absent), "leaked `{absent}`:\n{rendered}");
    }
    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, reparsed);
    assert_eq!(toml::to_string_pretty(&reparsed).unwrap(), rendered);

    // A package-role fixture behaves the same.
    let package_raw = r#"[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.3.0"
[compatibility]
min_vibe_version = "0.1.0"
"#;
    let package = Manifest::parse_str(package_raw).unwrap();
    assert!(package.validate().is_ok());
    let package_rendered = toml::to_string_pretty(&package).unwrap();
    assert_eq!(
        toml::to_string_pretty(&Manifest::parse_str(&package_rendered).unwrap()).unwrap(),
        package_rendered
    );
}

/// Programmatic sections answer to the same validator as TOML-authored ones,
/// through `validate()` and through serialisation.
#[test]
fn programmatic_sections_fail_the_same_validator() {
    let mut manifest = parse(&format!("{BUILD}\n{PACKAGE}"));
    manifest.artifacts.as_mut().unwrap().build[0]
        .outputs
        .clear();
    let error = manifest.validate().unwrap_err().to_string();
    assert!(error.contains("field `outputs` is empty"), "{error}");
    assert!(
        toml::to_string_pretty(&manifest)
            .unwrap_err()
            .to_string()
            .contains("field `outputs` is empty")
    );

    let mut section = ArtifactsSection::default();
    section.build.push(ArtifactBuildTarget {
        id: "x".into(),
        mechanism: "deploy:vibe-bin".parse().unwrap(),
        provider: None,
        workdir: ".".into(),
        inputs: Some(vec![ArtifactInput::Path { path: "a".into() }]),
        outputs: vec![ArtifactOutput {
            id: "x.exe".into(),
            kind: ArtifactKind::Executable,
            select: None,
        }],
        config: None,
    });
    let error = section.validate().unwrap_err().to_string();
    assert!(
        error.contains("must equal the target's phase family"),
        "{error}"
    );
}
