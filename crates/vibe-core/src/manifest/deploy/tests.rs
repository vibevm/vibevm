//! Focused tests for `[[deploy.target]]` and `[deploy.profiles]`.

use specmark::verifies;

use super::{DeployProfile, DeploySection, DeployTarget};
use crate::Error;
use crate::manifest::Manifest;

const PROJECT: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";
const VIRTUAL: &str = "[workspace]\nmembers = []\n";
const ARTIFACTS: &str = concat!(
    "[[artifacts.build]]\n",
    "id = \"helper\"\n",
    "mechanism = \"build:cargo\"\n",
    "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
);
const DEPLOY: &str = concat!(
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

fn parse(body: &str) -> Manifest {
    Manifest::parse_str(&format!("{PROJECT}\n{ARTIFACTS}\n{body}")).unwrap()
}

fn parse_error(body: &str) -> String {
    Manifest::parse_str(&format!("{PROJECT}\n{ARTIFACTS}\n{body}"))
        .unwrap_err()
        .to_string()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn full_example_parses_and_round_trips() {
    let manifest = parse(DEPLOY);
    let deploy = manifest.deploy.as_ref().unwrap();
    assert_eq!(deploy.targets.len(), 1);
    let target = &deploy.targets[0];
    assert_eq!(target.id, "local-helper");
    assert_eq!(target.artifact, "helper.exe");
    assert_eq!(target.mechanism.to_string(), "deploy:vibe-bin");
    assert_eq!(
        target.provider.as_ref().map(|pin| pin.to_string()),
        Some("org.example/installers#vibe-bin-v2".to_string())
    );
    assert_eq!(target.depends_on.as_deref(), Some(&[][..]));
    assert_eq!(
        target.config.as_ref().unwrap().as_table()["command"].as_str(),
        Some("helper")
    );
    assert_eq!(
        deploy
            .profiles
            .get("local")
            .map(|profile| profile.targets.clone()),
        Some(vec!["local-helper".to_string()])
    );

    let rendered = toml::to_string_pretty(&manifest).unwrap();
    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, reparsed);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn default_profile_is_explicit_under_deploy() {
    let full = concat!(
        "[[deploy.target]]\nid = \"ci-helper\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
        "[[deploy.target]]\nid = \"local-helper\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
        "[deploy.profiles.ci]\ntargets = [\"ci-helper\"]\n",
        "[deploy.profiles.local]\ntargets = [\"local-helper\"]\n",
        "[deploy]\ndefault_profile = \"local\"\n",
    );
    let manifest = parse(full);
    let deploy = manifest.deploy.as_ref().unwrap();
    assert_eq!(deploy.default_profile.as_deref(), Some("local"));
    assert_eq!(deploy.profiles.len(), 2);
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    assert!(rendered.contains("default_profile"), "{rendered}");
    assert_eq!(Manifest::parse_str(&rendered).unwrap(), manifest);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn unknown_and_missing_fields_refuse_at_the_shape_layer() {
    for (body, fragment) in [
        (
            "[[deploy.target]]\nid = \"x\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\nmystery = true\n",
            "unknown field",
        ),
        (
            "[[deploy.target]]\nid = \"x\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\ndepends_on = []\nconfig = 3\n",
            "config",
        ),
        (
            "[deploy.profiles.local]\ntargets = [\"local-helper\"]\nmystery = true\n",
            "unknown field",
        ),
        ("[deploy.profiles.local]\nmystery = true\n", "unknown field"),
    ] {
        // The profile rows need their target; build a minimal valid target
        // set for the profile cases.
        let with_target = format!(
            "[[deploy.target]]\nid = \"local-helper\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n\n{body}"
        );
        match Manifest::parse_str(&format!("{PROJECT}\n{ARTIFACTS}\n{with_target}")) {
            Err(Error::ParseToml { diagnostic, .. }) => {
                let detail = diagnostic.to_string();
                assert!(detail.contains(fragment), "fragment={fragment}: {detail}");
            }
            other => panic!("expected ParseToml, got {other:?}"),
        }
    }
    for (body, field) in [
        (
            "[[deploy.target]]\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
            "id",
        ),
        (
            "[[deploy.target]]\nid = \"x\"\nmechanism = \"deploy:vibe-bin\"\n",
            "artifact",
        ),
        (
            "[[deploy.target]]\nid = \"x\"\nartifact = \"helper.exe\"\n",
            "mechanism",
        ),
    ] {
        match Manifest::parse_str(&format!("{PROJECT}\n{ARTIFACTS}\n{body}")) {
            Err(Error::ParseToml { diagnostic, .. }) => {
                let detail = diagnostic.to_string();
                assert!(detail.contains(field), "field={field}: {detail}");
            }
            other => panic!("expected ParseToml for missing `{field}`, got {other:?}"),
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn empty_unsafe_and_mismatched_rows_refuse_with_remediation() {
    for (body, fragment) in [
        (
            "[[deploy.target]]\nid = \"\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
            "field `id`",
        ),
        (
            "[[deploy.target]]\nid = \"Bad Id\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
            "field `id`",
        ),
        (
            "[[deploy.target]]\nid = \"x\"\nartifact = \"ghost.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
            "names no declared artifact",
        ),
        (
            "[[deploy.target]]\nid = \"x\"\nartifact = \"helper.exe\"\nmechanism = \"build:cargo\"\n",
            "deploy targets select the `deploy:` family only",
        ),
        (
            "[[deploy.target]]\nid = \"x\"\nartifact = \"helper.exe\"\nmechanism = \"acquire:prebuilt\"\n",
            "deploy targets select the `deploy:` family only",
        ),
        (
            "[[deploy.target]]\nid = \"x\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\nprovider = \"vibe-bin-v2\"\n",
            "short id",
        ),
        (
            "[[deploy.target]]\nid = \"x\"\nartifact = \"helper.exe\"\nmechanism = \"vibe-bin\"\n",
            "field `mechanism`",
        ),
    ] {
        let error = parse_error(body);
        assert!(error.contains(fragment), "fragment={fragment}: {error}");
        assert!(error.contains("PROP-054"), "{error}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn duplicate_ids_refuse() {
    let duplicate = concat!(
        "[[deploy.target]]\nid = \"same\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
        "[[deploy.target]]\nid = \"same\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
    );
    let error = parse_error(duplicate);
    assert!(error.contains("duplicate [[deploy.target]]"), "{error}");
    assert!(error.contains("value `same`"), "{error}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn depends_on_validates_unknowns_duplicates_self_and_cycles() {
    let target = |id: &str, depends_on: &str| {
        format!(
            "[[deploy.target]]\nid = \"{id}\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\ndepends_on = {depends_on}\n"
        )
    };
    // Unknown, duplicate, self.
    for (depends_on, fragment) in [
        ("[\"ghost\"]", "unknown target `ghost`"),
        ("[\"a\", \"a\"]", "more than once"),
    ] {
        let body = format!("{}{}", target("a", "[]"), target("b", depends_on));
        let error = parse_error(&body);
        assert!(error.contains(fragment), "fragment={fragment}: {error}");
    }
    let self_dep = parse_error(&target("a", "[\"a\"]"));
    assert!(self_dep.contains("lists itself"), "{self_dep}");

    // Cycle across two targets.
    let cycle = parse_error(&format!(
        "{}{}",
        target("a", "[\"b\"]"),
        target("b", "[\"a\"]")
    ));
    assert!(cycle.contains("cyclic"), "{cycle}");
    assert!(cycle.contains("a -> b -> a"), "{cycle}");

    // Forward references are legal.
    let forward = parse(&format!("{}{}", target("a", "[\"b\"]"), target("b", "[]")));
    assert!(forward.validate().is_ok());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn profiles_validate_names_empties_duplicates_and_unknowns() {
    let targets = concat!(
        "[[deploy.target]]\nid = \"a\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
        "[[deploy.target]]\nid = \"b\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
    );
    for (profile, fragment) in [
        (
            "[deploy.profiles.\"Bad Name\"]\ntargets = [\"a\"]\n",
            "not a portable token",
        ),
        ("[deploy.profiles.empty]\ntargets = []\n", "is empty"),
        (
            "[deploy.profiles.dup]\ntargets = [\"a\", \"a\"]\n",
            "more than once",
        ),
        (
            "[deploy.profiles.ghost]\ntargets = [\"ghost\"]\n",
            "unknown deploy target `ghost`",
        ),
    ] {
        let error = parse_error(&format!("{targets}\n{profile}"));
        assert!(error.contains(fragment), "fragment={fragment}: {error}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn removing_one_dependency_from_a_profile_refuses() {
    let targets = concat!(
        "[[deploy.target]]\nid = \"a\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
        "[[deploy.target]]\nid = \"b\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\ndepends_on = [\"a\"]\n",
    );
    let complete = parse(&format!(
        "{targets}\n[deploy.profiles.full]\ntargets = [\"a\", \"b\"]\n"
    ));
    assert!(complete.validate().is_ok());

    let pruned = parse_error(&format!(
        "{targets}\n[deploy.profiles.pruned]\ntargets = [\"b\"]\n"
    ));
    assert!(
        pruned.contains("dependency `a` is not included in the profile"),
        "{pruned}"
    );
    // Order inside the selection is authored and free.
    let reordered = parse(&format!(
        "{targets}\n[deploy.profiles.reordered]\ntargets = [\"b\", \"a\"]\n"
    ));
    assert!(reordered.validate().is_ok());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn two_profiles_may_reuse_a_target_and_default_must_exist() {
    let body = concat!(
        "[[deploy.target]]\nid = \"shared\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
        "[[deploy.target]]\nid = \"extra\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
        "[deploy.profiles.one]\ntargets = [\"shared\"]\n",
        "[deploy.profiles.two]\ntargets = [\"shared\", \"extra\"]\n",
    );
    assert!(parse(body).validate().is_ok());

    let ghost = parse_error(&format!("{body}\n[deploy]\ndefault_profile = \"ghost\"\n"));
    assert!(ghost.contains("names no declared profile"), "{ghost}");

    let present = parse(&format!("{body}\n[deploy]\ndefault_profile = \"one\"\n"));
    assert_eq!(
        present.deploy.as_ref().unwrap().default_profile.as_deref(),
        Some("one")
    );
}

/// `[[deploy.target]]` rows and a profile's `targets` are **vectors**: the
/// order is the declaration and must come back unshuffled. Both are authored
/// reverse-lexicographically, so a sorted implementation could not pass by
/// accident. Profile *names* are a map: they carry no order law and are
/// rendered sorted, which this test states rather than pretends otherwise.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn authored_vector_order_survives_reverse_lexicographically() {
    let body = concat!(
        "[[deploy.target]]\nid = \"zulu\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
        "[[deploy.target]]\nid = \"mike\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
        "[[deploy.target]]\nid = \"alpha\"\nartifact = \"helper.exe\"\nmechanism = \"deploy:vibe-bin\"\n",
        "[deploy.profiles.zone]\ntargets = [\"zulu\", \"mike\", \"alpha\"]\n",
        "[deploy.profiles.alpine]\ntargets = [\"mike\", \"alpha\"]\n",
    );
    let manifest = parse(body);
    let deploy = manifest.deploy.as_ref().unwrap();
    let ids: Vec<&str> = deploy.targets.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, ["zulu", "mike", "alpha"], "target order is authored");
    assert_eq!(
        deploy.profiles["zone"].targets,
        ["zulu", "mike", "alpha"],
        "profile selection order is authored"
    );

    // Vec equality is order-sensitive, so the round trip pins both vectors;
    // the render is read too, because a re-sort must be visible in the file
    // the operator gets back, not only in memory.
    let rendered = toml::to_string_pretty(&manifest).unwrap();
    let reparsed = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, reparsed);
    let round_tripped = reparsed.deploy.as_ref().unwrap();
    assert_eq!(
        round_tripped
            .targets
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        ["zulu", "mike", "alpha"]
    );
    assert_eq!(
        round_tripped.profiles["zone"].targets,
        ["zulu", "mike", "alpha"]
    );
    let positions: Vec<usize> = ["zulu", "mike", "alpha"]
        .iter()
        .map(|id| rendered.find(&format!("id = \"{id}\"")).expect("rendered"))
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "target rows were re-sorted:\n{rendered}"
    );

    // The map half, stated rather than wished for: profile *names* render in
    // sorted order. Nothing may depend on the order they were written in.
    let zone_at = rendered.find("[deploy.profiles.zone]").expect("rendered");
    let alpine_at = rendered.find("[deploy.profiles.alpine]").expect("rendered");
    assert!(
        alpine_at < zone_at,
        "map keys are rendered sorted; the doc comment says so:\n{rendered}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn deploy_requires_project_or_package_role() {
    // Deploy-only in a virtual workspace: the deploy role law fires before
    // any artifact-ref check, so the section names its own remediation.
    let error = Manifest::parse_str(&format!("{VIRTUAL}\n{DEPLOY}"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("[deploy] desired targets require"),
        "{error}"
    );
    assert!(error.contains("pure virtual"), "{error}");

    let package =
        "[package]\ngroup = \"org.demo\"\nname = \"pkg\"\nkind = \"flow\"\nversion = \"0.1.0\"\n";
    assert!(Manifest::parse_str(&format!("{package}\n{ARTIFACTS}\n{DEPLOY}")).is_ok());
}

#[test]
fn programmatic_deploy_sections_fail_the_same_validator() {
    let mut manifest = parse(DEPLOY);
    manifest.deploy.as_mut().unwrap().targets[0].artifact = "ghost.exe".into();
    let error = manifest.validate().unwrap_err().to_string();
    assert!(error.contains("names no declared artifact"), "{error}");

    // Row-level faults also refuse at the serialization seam.
    manifest.deploy.as_mut().unwrap().targets[0].mechanism = "build:cargo".parse().unwrap();
    let error = manifest.validate().unwrap_err().to_string();
    assert!(error.contains("`deploy:` family only"), "{error}");
    assert!(
        toml::to_string_pretty(&manifest)
            .unwrap_err()
            .to_string()
            .contains("`deploy:` family only")
    );

    let artifacts = manifest.artifacts.as_ref().unwrap();
    let mut section = DeploySection::default();
    section.targets.push(DeployTarget {
        id: "x".into(),
        artifact: artifacts.build[0].outputs[0].id.clone(),
        mechanism: "deploy:vibe-bin".parse().unwrap(),
        provider: None,
        depends_on: Some(vec!["x".into()]),
        config: None,
    });
    section.profiles.insert(
        "all".into(),
        DeployProfile {
            targets: vec!["x".into()],
        },
    );
    let error = section.validate(&artifacts.output_ids()).unwrap_err();
    assert!(error.contains("lists itself"), "{error}");
}
