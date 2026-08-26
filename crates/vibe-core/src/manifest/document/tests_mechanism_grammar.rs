//! Document-level tests for the mechanism/artifact/deploy grammar: the full
//! accepted example, role matrix, byte-identity of old fixtures, comment
//! survival through the rewrite path, and programmatic revalidation.

use std::fs;

use specmark::verifies;
use tempfile::tempdir;

use crate::manifest::{ArtifactInput, ArtifactOutput, ArtifactTarget, ArtifactsSection, Manifest};

const PROJECT: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";
const PACKAGE: &str =
    "[package]\ngroup = \"org.demo\"\nname = \"pkg\"\nkind = \"flow\"\nversion = \"0.1.0\"\n";
const VIRTUAL: &str = "[workspace]\nmembers = []\n";

/// The accepted fixed grammar, verbatim from the packet.
const FULL_EXAMPLE: &str = concat!(
    "[[mechanism]]\n",
    "id = \"cargo-v2\"\n",
    "role = \"build\"\n",
    "name = \"cargo\"\n",
    "handler = { kind = \"native\", crate_dir = \"crates/cargo-provider\" }\n",
    "protocol = 1\n",
    "config_schema = \"schemas/cargo-build-v1.jtd.json\"\n",
    "freshness = \"provider\"\n",
    "\n",
    "[mechanisms]\n",
    "\"build:cargo\" = \"org.example/build-tools#cargo-v2\"\n",
    "\n",
    "[[artifacts.build]]\n",
    "id = \"helper\"\n",
    "mechanism = \"build:cargo\"\n",
    "provider = \"org.example/build-tools#cargo-v2\"\n",
    "inputs = [{ path = \"Cargo.toml\" }, { path = \"Cargo.lock\" }, { path = \"crates/helper/**\" }]\n",
    "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
    "config = { package = \"helper\", bin = \"helper\", profile = \"release\", locked = true }\n",
    "\n",
    "[[artifacts.package]]\n",
    "id = \"helper-windows\"\n",
    "mechanism = \"package:windows-zip\"\n",
    "inputs = [{ artifact = \"helper.exe\" }]\n",
    "outputs = [{ id = \"helper.zip\", kind = \"archive\" }]\n",
    "config = { layout = \"distribution/windows\" }\n",
    "\n",
    "[[deploy.target]]\n",
    "id = \"local-helper\"\n",
    "artifact = \"helper.exe\"\n",
    "mechanism = \"deploy:vibe-bin\"\n",
    "provider = \"org.example/installers#vibe-bin-v2\"\n",
    "depends_on = []\n",
    "config = { command = \"helper\" }\n",
    "\n",
    "[deploy.profiles.local]\n",
    "targets = [\"local-helper\"]\n",
);

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn exact_full_example_parses_and_round_trips() {
    let manifest = Manifest::parse_str(&format!("{PROJECT}\n{FULL_EXAMPLE}")).unwrap();
    manifest.validate().unwrap();
    assert_eq!(manifest.mechanism_decls.len(), 1);
    assert_eq!(manifest.mechanism_routes.len(), 1);
    let artifacts = manifest.artifacts.as_ref().unwrap();
    assert_eq!(artifacts.build.len(), 1);
    assert_eq!(artifacts.package.len(), 1);
    let deploy = manifest.deploy.as_ref().unwrap();
    assert_eq!(deploy.targets.len(), 1);
    assert_eq!(deploy.profiles.len(), 1);
    assert!(deploy.default_profile.is_none());

    let rendered = toml::to_string_pretty(&manifest).unwrap();
    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, reparsed);
    // Spelling survives: the wire names appear exactly as authored.
    assert!(rendered.contains("[[mechanism]]"), "{rendered}");
    assert!(rendered.contains("[mechanisms]"), "{rendered}");
    assert!(
        rendered.contains("\"build:cargo\" = \"org.example/build-tools#cargo-v2\""),
        "{rendered}"
    );
    assert!(rendered.contains("[[artifacts.build]]"), "{rendered}");
    assert!(rendered.contains("[[artifacts.package]]"), "{rendered}");
    assert!(rendered.contains("[[deploy.target]]"), "{rendered}");
    assert!(rendered.contains("[deploy.profiles.local]"), "{rendered}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn default_profile_spelling_rides_the_deploy_section() {
    let body = format!("{FULL_EXAMPLE}\n[deploy]\ndefault_profile = \"local\"\n");
    let manifest = Manifest::parse_str(&format!("{PROJECT}\n{body}")).unwrap();
    assert_eq!(
        manifest.deploy.as_ref().unwrap().default_profile.as_deref(),
        Some("local")
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn role_matrix_is_pinned_across_project_package_and_virtual() {
    let example = |body: &str| format!("{body}\n{FULL_EXAMPLE}");

    // Project: everything is legal.
    assert!(Manifest::parse_str(&example(PROJECT)).is_ok());
    // Package: everything is legal (a root package is a valid host).
    assert!(Manifest::parse_str(&example(PACKAGE)).is_ok());
    // Pure virtual workspace: routes survive, everything else refuses.
    let virtual_error = Manifest::parse_str(&example(VIRTUAL))
        .unwrap_err()
        .to_string();
    assert!(
        virtual_error.contains("[[mechanism]] is legal only"),
        "{virtual_error}"
    );

    let routes_only = "[mechanisms]\n\"build:cargo\" = \"org.example/build-tools#cargo-v2\"\n";
    assert!(Manifest::parse_str(&format!("{VIRTUAL}\n{routes_only}")).is_ok());

    // Decls / artifacts / deploy each refuse alone in a virtual workspace.
    let decl_only = FULL_EXAMPLE.split("\n[mechanisms]\n").next().unwrap();
    let decl_error = Manifest::parse_str(&format!("{VIRTUAL}\n{decl_only}"))
        .unwrap_err()
        .to_string();
    assert!(
        decl_error.contains("[[mechanism]] is legal only"),
        "{decl_error}"
    );

    let artifacts_only = concat!(
        "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:cargo\"\n",
        "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
    );
    let artifacts_error = Manifest::parse_str(&format!("{VIRTUAL}\n{artifacts_only}"))
        .unwrap_err()
        .to_string();
    assert!(
        artifacts_error.contains("[artifacts] desired targets require"),
        "{artifacts_error}"
    );

    // Deploy: legal without profiles in a project manifest; refused in a
    // pure virtual workspace by its own role law (before any artifact-ref
    // check fires).
    let with_artifacts = concat!(
        "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:cargo\"\n",
        "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
        "[[deploy.target]]\nid = \"local-helper\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
    );
    assert!(Manifest::parse_str(&format!("{PROJECT}\n{with_artifacts}")).is_ok());
    let deploy_only = "[[deploy.target]]\nid = \"local-helper\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n";
    let deploy_error = Manifest::parse_str(&format!("{VIRTUAL}\n{deploy_only}"))
        .unwrap_err()
        .to_string();
    assert!(
        deploy_error.contains("[deploy] desired targets require"),
        "{deploy_error}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn old_fixture_serializes_byte_identically_when_new_sections_absent() {
    // The `full_project_parses` fixture — a rich manifest with no new
    // sections. The render must be byte-stable and carry none of the new
    // section keys.
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
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    let rendered_again = toml::to_string_pretty(&manifest).unwrap();
    assert_eq!(rendered, rendered_again);
    for absent in ["mechanism", "mechanisms", "artifacts", "deploy"] {
        assert!(!rendered.contains(absent), "leaked `{absent}`:\n{rendered}");
    }
    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, reparsed);
    assert_eq!(toml::to_string_pretty(&reparsed).unwrap(), rendered);

    // A package-role fixture exercises the ManifestWire field order beyond
    // the extension controls too.
    let package_raw = r#"[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.3.0"
[compatibility]
min_vibe_version = "0.1.0"
"#;
    let package = Manifest::parse_str(package_raw).unwrap();
    let package_rendered = toml::to_string_pretty(&package).unwrap();
    assert_eq!(
        toml::to_string_pretty(&Manifest::parse_str(&package_rendered).unwrap()).unwrap(),
        package_rendered
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn comments_survive_merge_rewrite_through_the_new_sections() {
    let project = tempdir().unwrap();
    let path = project.path().join(Manifest::FILENAME);
    // Written in the canonical render shapes (header sub-tables for handler
    // and outputs rows) so the decoration walker pairs matching forms.
    fs::write(
        &path,
        format!(
            "{PROJECT}\n\
             [[mechanism]]\n\
             id = \"cargo-v2\"\n\
             role = \"build\"\n\
             name = \"cargo\"\n\
             protocol = 1\n\
             config_schema = \"schemas/cargo-build-v1.jtd.json\"\n\
             freshness = \"provider\"\n\
             \n\
             # KEEP-HANDLER
             [mechanism.handler]\n\
             kind = \"native\"\n\
             crate_dir = \"crates/cargo-provider\"\n\
             \n\
             [mechanisms]\n\
             # KEEP-ROUTE
             \"build:cargo\" = \"org.example/build-tools#cargo-v2\"\n\
             \n\
             [[artifacts.build]]\n\
             id = \"helper\"\n\
             mechanism = \"build:cargo\"\n\
             \n\
             # KEEP-OUTPUT
             [[artifacts.build.outputs]]\n\
             id = \"helper.exe\"\n\
             kind = \"executable\"\n\
             \n\
             [[deploy.target]]\n\
             id = \"local-helper\"\n\
             artifact = \"helper.exe\"\n\
             mechanism = \"deploy:vibe-bin\"\n\
             \n\
             # KEEP-PROFILE
             [deploy.profiles.local]\n\
             targets = [\"local-helper\"]\n"
        ),
    )
    .unwrap();

    let mut manifest = Manifest::read(&path).unwrap();
    manifest.project.as_mut().unwrap().version = "0.2.0".into();
    manifest.write(&path).unwrap();

    let written = fs::read_to_string(&path).unwrap();
    for marker in ["KEEP-HANDLER", "KEEP-ROUTE", "KEEP-OUTPUT", "KEEP-PROFILE"] {
        assert!(written.contains(marker), "lost {marker}:\n{written}");
    }
    assert!(written.contains("version = \"0.2.0\""), "{written}");
    let reparsed = Manifest::read(&path).unwrap();
    assert_eq!(reparsed.project.unwrap().version, "0.2.0");
    assert_eq!(reparsed.mechanism_decls.len(), 1);
}

/// The packet's accepted example is written with **inline** `inputs`,
/// `outputs`, `config` and `handler` rows. The serializer canonicalises those
/// into header tables — a spelling change the operator did not ask for — so
/// the decoration seam has to carry each comment onto the row that inherited
/// it. Losing a comment because the shape was rewritten underneath it is the
/// exact failure this pins.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn inline_authored_rows_keep_their_comments_through_write() {
    let project = tempdir().unwrap();
    let path = project.path().join(Manifest::FILENAME);
    let authored = format!(
        "{PROJECT}\n\
         [[mechanism]]\n\
         id = \"cargo-v2\"\n\
         role = \"build\"\n\
         name = \"cargo\"\n\
         # KEEP-INLINE-HANDLER\n\
         handler = {{ kind = \"native\", crate_dir = \"crates/cargo-provider\" }}\n\
         protocol = 1\n\
         config_schema = \"schemas/cargo-build-v1.jtd.json\"\n\
         freshness = \"provider\"\n\
         \n\
         [[artifacts.build]]\n\
         id = \"helper\"\n\
         mechanism = \"build:cargo\"\n\
         # KEEP-INLINE-INPUTS\n\
         inputs = [{{ path = \"Cargo.toml\" }}, {{ path = \"crates/helper/**\" }}]\n\
         # KEEP-INLINE-OUTPUTS\n\
         outputs = [{{ id = \"helper.exe\", kind = \"executable\" }}]\n\
         # KEEP-INLINE-CONFIG\n\
         config = {{ package = \"helper\", locked = true }}\n\
         \n\
         [[deploy.target]]\n\
         id = \"local-helper\"\n\
         artifact = \"helper.exe\"\n\
         mechanism = \"deploy:vibe-bin\"\n\
         # KEEP-INLINE-DEPLOY-CONFIG\n\
         config = {{ command = \"helper\" }}\n"
    );
    fs::write(&path, &authored).unwrap();

    let mut manifest = Manifest::read(&path).unwrap();
    manifest.project.as_mut().unwrap().version = "0.2.0".into();
    manifest.write(&path).unwrap();

    let written = fs::read_to_string(&path).unwrap();
    for marker in [
        "KEEP-INLINE-HANDLER",
        "KEEP-INLINE-INPUTS",
        "KEEP-INLINE-OUTPUTS",
        "KEEP-INLINE-CONFIG",
        "KEEP-INLINE-DEPLOY-CONFIG",
    ] {
        assert!(written.contains(marker), "lost {marker}:\n{written}");
    }
    assert!(written.contains("version = \"0.2.0\""), "{written}");

    // Values survive the canonicalisation unchanged, and the rewritten file
    // is stable: a second write moves nothing further.
    let reparsed = Manifest::read(&path).unwrap();
    let build = &reparsed.artifacts.as_ref().unwrap().build[0];
    assert_eq!(build.inputs.as_ref().map(Vec::len), Some(2));
    assert_eq!(build.outputs[0].id, "helper.exe");
    assert_eq!(
        build.config.as_ref().unwrap().as_table()["locked"].as_bool(),
        Some(true)
    );
    reparsed.write(&path).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), written);
}

/// The note that followed the whole array — `inputs = [...] # KEEP` — has no
/// bracket to sit after once the array becomes headers. It moves to the last
/// header the array expanded into, exactly once, alongside the key-prefix and
/// per-element notes.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn whole_array_trailing_notes_survive_canonicalisation() {
    let project = tempdir().unwrap();
    let path = project.path().join(Manifest::FILENAME);
    // Single-row array, multi-row array, and all three comment positions at
    // once: key prefix, per-element note, and the array's own trailing note.
    let authored = format!(
        "{PROJECT}\n\
         [[artifacts.build]]\n\
         id = \"helper\"\n\
         mechanism = \"build:cargo\"\n\
         # KEEP-KEY-PREFIX\n\
         inputs = [\n\
         \x20 # KEEP-ELEMENT-NOTE\n\
         \x20 {{ path = \"Cargo.toml\" }},\n\
         \x20 {{ path = \"crates/helper/**\" }},\n\
         ] # KEEP-ARRAY-SUFFIX\n\
         outputs = [{{ id = \"helper.exe\", kind = \"executable\" }}] # KEEP-SINGLE-ROW-SUFFIX\n"
    );
    fs::write(&path, &authored).unwrap();

    let mut manifest = Manifest::read(&path).unwrap();
    manifest.project.as_mut().unwrap().version = "0.2.0".into();
    manifest.write(&path).unwrap();

    let written = fs::read_to_string(&path).unwrap();
    for marker in [
        "KEEP-KEY-PREFIX",
        "KEEP-ELEMENT-NOTE",
        "KEEP-ARRAY-SUFFIX",
        "KEEP-SINGLE-ROW-SUFFIX",
    ] {
        assert_eq!(
            written.matches(marker).count(),
            1,
            "`{marker}` must survive exactly once:\n{written}"
        );
    }

    // The array note landed on a header of the array it belonged to.
    let suffix_line = written
        .lines()
        .find(|line| line.contains("KEEP-ARRAY-SUFFIX"))
        .expect("rendered");
    assert!(
        suffix_line.contains("[[artifacts.build.inputs]]"),
        "{written}"
    );

    // Values are untouched and the rewrite is a fixpoint.
    let reparsed = Manifest::read(&path).unwrap();
    assert_eq!(
        reparsed.artifacts.as_ref().unwrap().build[0]
            .inputs
            .as_ref()
            .map(Vec::len),
        Some(2)
    );
    reparsed.write(&path).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), written);
}

/// The last row's own note and the whole array's note are two different
/// comments by the same operator. Canonicalisation must keep **both**, in
/// source order, on the header that replaced the last row — and must not
/// duplicate either on a second write.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn row_and_array_notes_both_survive_in_source_order() {
    let project = tempdir().unwrap();
    let path = project.path().join(Manifest::FILENAME);
    let authored = format!(
        "{PROJECT}\n\
         [[artifacts.build]]\n\
         id = \"helper\"\n\
         mechanism = \"build:cargo\"\n\
         inputs = [\n\
         \x20 {{ path = \"Cargo.toml\" }}, # KEEP-ROW\n\
         ] # KEEP-ARRAY\n\
         # KEEP-OUTPUTS-KEY\n\
         outputs = [\n\
         \x20 # KEEP-OUTPUT-ELEMENT\n\
         \x20 {{ id = \"helper.exe\", kind = \"executable\" }},\n\
         \x20 {{ id = \"helper.pdb\", kind = \"debug-info\" }}, # KEEP-LAST-OUTPUT-ROW\n\
         ] # KEEP-OUTPUTS-ARRAY\n"
    );
    fs::write(&path, &authored).unwrap();

    let mut manifest = Manifest::read(&path).unwrap();
    manifest.project.as_mut().unwrap().version = "0.2.0".into();
    manifest.write(&path).unwrap();

    let written = fs::read_to_string(&path).unwrap();
    for marker in [
        "KEEP-ROW",
        "KEEP-ARRAY",
        "KEEP-OUTPUTS-KEY",
        "KEEP-OUTPUT-ELEMENT",
        "KEEP-LAST-OUTPUT-ROW",
        "KEEP-OUTPUTS-ARRAY",
    ] {
        assert_eq!(
            written.matches(marker).count(),
            1,
            "`{marker}` must survive exactly once:\n{written}"
        );
    }
    // Source order on the merged line: the row's note precedes the array's.
    let merged = written
        .lines()
        .find(|line| line.contains("KEEP-ROW"))
        .expect("rendered");
    assert!(
        merged.find("KEEP-ROW") < merged.find("KEEP-ARRAY"),
        "notes must keep source order:\n{written}"
    );

    // Values parse unchanged and a second write is a byte fixpoint.
    let reparsed = Manifest::read(&path).unwrap();
    let build = &reparsed.artifacts.as_ref().unwrap().build[0];
    assert_eq!(build.inputs.as_ref().map(Vec::len), Some(1));
    assert_eq!(build.outputs.len(), 2);
    assert_eq!(build.outputs[1].id, "helper.pdb");
    reparsed.write(&path).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), written);
}

#[test]
fn programmatic_documents_fail_the_same_validators() {
    // Build a valid document, then corrupt each plane programmatically: the
    // same validators fire as through TOML.
    let mut manifest = Manifest::parse_str(&format!("{PROJECT}\n{FULL_EXAMPLE}")).unwrap();

    manifest.artifacts.as_mut().unwrap().build[0].mechanism = "package:zip".parse().unwrap();
    let error = manifest.validate().unwrap_err().to_string();
    assert!(
        error.contains("must equal the target's phase family"),
        "{error}"
    );
    assert!(
        toml::to_string_pretty(&manifest)
            .unwrap_err()
            .to_string()
            .contains("must equal the target's phase family")
    );

    manifest.artifacts.as_mut().unwrap().build[0].mechanism = "build:cargo".parse().unwrap();
    manifest.deploy.as_mut().unwrap().targets[0].artifact = "ghost.exe".into();
    let error = manifest.validate().unwrap_err().to_string();
    assert!(error.contains("names no declared artifact"), "{error}");

    // A document-level artifact cycle constructed in Rust.
    let mut cycle = ArtifactsSection::default();
    cycle.build.push(ArtifactTarget {
        id: "first".into(),
        mechanism: "build:cargo".parse().unwrap(),
        provider: None,
        inputs: Some(vec![ArtifactInput::Artifact {
            artifact: "second.out".into(),
        }]),
        outputs: vec![ArtifactOutput {
            id: "first.out".into(),
            kind: "executable".into(),
        }],
        config: None,
    });
    cycle.build.push(ArtifactTarget {
        id: "second".into(),
        mechanism: "build:cargo".parse().unwrap(),
        provider: None,
        inputs: Some(vec![ArtifactInput::Artifact {
            artifact: "first.out".into(),
        }]),
        outputs: vec![ArtifactOutput {
            id: "second.out".into(),
            kind: "executable".into(),
        }],
        config: None,
    });
    let error = cycle.validate().unwrap_err();
    assert!(error.contains("cyclic"), "{error}");
}
