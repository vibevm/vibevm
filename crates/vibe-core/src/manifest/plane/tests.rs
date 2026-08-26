//! The one-validator law: every corruption refuses through `validate()` and
//! through serialisation, so a document that cannot be read back can never
//! be written. Plus the local-pin cross-check and the iterative cycle walk.

use specmark::verifies;

use super::local_provider_owner;
use crate::manifest::{
    ArtifactInput, ArtifactOutput, ArtifactTarget, ArtifactsSection, Manifest, ProviderOwner,
    WorkspaceSection,
};

const PROJECT: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";
const PACKAGE: &str = "[package]\ngroup = \"org.example\"\nname = \"build-tools\"\nkind = \"flow\"\nversion = \"0.1.0\"\n";
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
const PLANE: &str = concat!(
    "[[artifacts.build]]\n",
    "id = \"helper\"\n",
    "mechanism = \"build:cargo\"\n",
    "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
    "\n",
    "[[artifacts.package]]\n",
    "id = \"helper-windows\"\n",
    "mechanism = \"package:windows-zip\"\n",
    "inputs = [{ artifact = \"helper.exe\" }]\n",
    "outputs = [{ id = \"helper.zip\", kind = \"archive\" }]\n",
    "\n",
    "[[deploy.target]]\n",
    "id = \"local-helper\"\n",
    "artifact = \"helper.exe\"\n",
    "mechanism = \"deploy:vibe-bin\"\n",
    "\n",
    "[[deploy.target]]\n",
    "id = \"after-helper\"\n",
    "artifact = \"helper.zip\"\n",
    "mechanism = \"deploy:vibe-bin\"\n",
    "depends_on = [\"local-helper\"]\n",
    "\n",
    "[deploy.profiles.local]\n",
    "targets = [\"local-helper\", \"after-helper\"]\n",
);

fn valid() -> Manifest {
    Manifest::parse_str(&format!("{PROJECT}\n{PLANE}")).unwrap()
}

/// Both directions must refuse, with the same sentence.
#[track_caller]
fn refuses_both_ways(manifest: &Manifest, fragment: &str) {
    let validated = manifest
        .validate()
        .expect_err("validate() must refuse")
        .to_string();
    assert!(validated.contains(fragment), "validate(): {validated}");
    let serialised = toml::to_string_pretty(manifest)
        .expect_err("serialisation must refuse the same document")
        .to_string();
    assert!(serialised.contains(fragment), "serialize: {serialised}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn ghost_refs_refuse_through_both_paths() {
    let mut manifest = valid();
    manifest.deploy.as_mut().unwrap().targets[0].artifact = "ghost.exe".into();
    refuses_both_ways(&manifest, "names no declared artifact");

    let mut manifest = valid();
    manifest.artifacts.as_mut().unwrap().package[0].inputs = Some(vec![ArtifactInput::Artifact {
        artifact: "ghost.exe".into(),
    }]);
    refuses_both_ways(&manifest, "references unknown artifact `ghost.exe`");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn cycles_refuse_through_both_paths() {
    let mut manifest = valid();
    let artifacts = manifest.artifacts.as_mut().unwrap();
    artifacts.build[0].inputs = Some(vec![ArtifactInput::Artifact {
        artifact: "helper.exe".into(),
    }]);
    refuses_both_ways(
        &manifest,
        "artifact target graph is cyclic: helper -> helper",
    );

    let mut manifest = valid();
    let deploy = manifest.deploy.as_mut().unwrap();
    deploy.targets[0].depends_on = Some(vec!["after-helper".into()]);
    refuses_both_ways(
        &manifest,
        "deploy target graph is cyclic: after-helper -> local-helper -> after-helper",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn duplicates_refuse_through_both_paths() {
    let mut manifest = valid();
    let artifacts = manifest.artifacts.as_mut().unwrap();
    let clone = artifacts.build[0].clone();
    artifacts.build.push(clone);
    refuses_both_ways(&manifest, "duplicate [[artifacts.build]] field `id`");

    let mut manifest = valid();
    manifest.deploy.as_mut().unwrap().targets[1].id = "local-helper".into();
    refuses_both_ways(&manifest, "duplicate [[deploy.target]] field `id`");

    let mut manifest = valid();
    manifest.artifacts.as_mut().unwrap().package[0].outputs[0].id = "helper.exe".into();
    refuses_both_ways(&manifest, "duplicate artifact id `helper.exe`");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn profile_faults_refuse_through_both_paths() {
    let mut manifest = valid();
    manifest.deploy.as_mut().unwrap().default_profile = Some("ghost".into());
    refuses_both_ways(&manifest, "names no declared profile");

    // Pruning a dependency out of the selection breaks the closure.
    let mut manifest = valid();
    manifest
        .deploy
        .as_mut()
        .unwrap()
        .profiles
        .get_mut("local")
        .unwrap()
        .targets = vec!["after-helper".into()];
    refuses_both_ways(
        &manifest,
        "dependency `local-helper` is not included in the profile",
    );

    let mut manifest = valid();
    manifest
        .deploy
        .as_mut()
        .unwrap()
        .profiles
        .get_mut("local")
        .unwrap()
        .targets = vec!["ghost".into()];
    refuses_both_ways(&manifest, "unknown deploy target `ghost`");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn virtual_workspace_role_refuses_through_both_paths() {
    // Only reachable programmatically: parsing already refuses it.
    let mut manifest = valid();
    manifest.project = None;
    manifest.workspace = Some(WorkspaceSection::default());
    refuses_both_ways(&manifest, "[artifacts] desired targets require");

    manifest.artifacts = None;
    refuses_both_ways(&manifest, "[deploy] desired targets require");

    manifest.deploy = None;
    manifest.mechanism_decls = Manifest::parse_str(&format!("{PROJECT}\n{DECL}"))
        .unwrap()
        .mechanism_decls;
    refuses_both_ways(&manifest, "[[mechanism]] is legal only");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn local_pins_are_cross_checked_against_the_declaration() {
    let route = "[mechanisms]\n\"build:cargo\" = \"org.example/build-tools#cargo-v2\"\n";
    assert!(Manifest::parse_str(&format!("{PACKAGE}\n{DECL}\n{route}")).is_ok());

    // Wrong role for the same declared provider.
    let mismatched_role = "[mechanisms]\n\"deploy:cargo\" = \"org.example/build-tools#cargo-v2\"\n";
    let error = Manifest::parse_str(&format!("{PACKAGE}\n{DECL}\n{mismatched_role}"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("declared in this manifest as `build:cargo`"),
        "{error}"
    );
    assert!(
        error.contains("the logical key is `deploy:cargo`"),
        "{error}"
    );

    // Wrong mechanism name for the same declared provider.
    let mismatched_name = "[mechanisms]\n\"build:rustc\" = \"org.example/build-tools#cargo-v2\"\n";
    let error = Manifest::parse_str(&format!("{PACKAGE}\n{DECL}\n{mismatched_name}"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("the logical key is `build:rustc`"),
        "{error}"
    );

    // Self-coordinate pin naming an id nobody declares.
    let ghost = "[mechanisms]\n\"build:cargo\" = \"org.example/build-tools#ghost\"\n";
    let error = Manifest::parse_str(&format!("{PACKAGE}\n{DECL}\n{ghost}"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("no `[[mechanism]]` with id `ghost`"),
        "{error}"
    );

    // A foreign coordinate stays runtime debt — never judged here.
    let foreign = "[mechanisms]\n\"deploy:vibe-bin\" = \"org.other/installers#anything\"\n";
    assert!(Manifest::parse_str(&format!("{PACKAGE}\n{DECL}\n{foreign}")).is_ok());
}

/// Declaring `group` claims a real self-coordinate `<group>/<name>`, so the
/// name has to be a real package name — enforced for **every** grouped
/// manifest, through both the parser and the writer, whether or not any
/// mechanism happens to exist. Dropping `group` drops the claim: the same
/// name is then arbitrary and travels through the `__host__` percent codec.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_grouped_project_name_must_form_its_coordinate() {
    let grouped = |name: &str| {
        format!("[project]\nname = \"{name}\"\ngroup = \"org.example\"\nversion = \"0.1.0\"\n")
    };
    let ungrouped = |name: &str| format!("[project]\nname = \"{name}\"\nversion = \"0.1.0\"\n");

    for name in ["Bad Name", "", "a/b", "Upper", "a--b", "-lead", "trail-"] {
        // No mechanism, no artifact, no deploy: a bare grouped manifest.
        let manifest = grouped(name);
        let parsed = Manifest::parse_str(&manifest)
            .expect_err("a grouped node must form its coordinate")
            .to_string();
        assert!(parsed.contains("[project].name"), "{name:?}: {parsed}");
        assert!(parsed.contains("is not a valid package name"), "{parsed}");
        assert!(parsed.contains("org.example"), "{parsed}");
        assert!(parsed.contains("drop `group`"), "{parsed}");

        // The writer refuses the same document with the same sentence: a
        // manifest that cannot be read back can never be written.
        let mut document = Manifest::parse_str(&ungrouped(name)).unwrap();
        document.project.as_mut().unwrap().group = Some("org.example".parse().unwrap());
        let written = toml::to_string_pretty(&document)
            .expect_err("serialisation must refuse it too")
            .to_string();
        assert!(written.contains("is not a valid package name"), "{written}");
        assert!(written.contains("[project].name"), "{written}");

        // Identical name WITHOUT `group`: still legal, and it round-trips
        // through the opaque host codec.
        let host = Manifest::parse_str(&ungrouped(name)).expect("ungrouped stays arbitrary");
        assert_eq!(
            local_provider_owner(&host).unwrap(),
            Some(ProviderOwner::Host {
                project: name.to_string()
            }),
            "{name:?}"
        );
        let rendered = toml::to_string_pretty(&host).unwrap();
        assert_eq!(Manifest::parse_str(&rendered).unwrap(), host, "{name:?}");
    }

    // A valid grouped name builds — and checks — its own package pin.
    let valid = Manifest::parse_str(&grouped("demo")).unwrap();
    assert_eq!(
        local_provider_owner(&valid).unwrap(),
        Some(ProviderOwner::Package {
            group: "org.example".parse().unwrap(),
            package: "demo".parse().unwrap(),
        })
    );
    let pinned = format!(
        "{}\n{DECL}\n[mechanisms]\n\"build:cargo\" = \"org.example/demo#ghost\"\n",
        grouped("demo")
    );
    let error = Manifest::parse_str(&pinned).unwrap_err().to_string();
    assert!(
        error.contains("no `[[mechanism]]` with id `ghost`"),
        "{error}"
    );
    assert!(error.contains("this package's own coordinate"), "{error}");
}

/// The owner is **total**: `None` means "declares no provider" and nothing
/// else. A virtual workspace is the only `None`; every other shape either
/// yields an owner or an error, so no cross-check can be skipped in silence.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_local_owner_is_total_over_a_validated_manifest() {
    let mut virtual_workspace = valid();
    virtual_workspace.project = None;
    virtual_workspace.artifacts = None;
    virtual_workspace.deploy = None;
    virtual_workspace.workspace = Some(WorkspaceSection::default());
    assert_eq!(local_provider_owner(&virtual_workspace), Ok(None));

    // Every other shape resolves to an owner, never to a silent `None`.
    for body in [
        PROJECT,
        PACKAGE,
        "[project]\nname = \"demo\"\ngroup = \"org.example\"\nversion = \"0.1.0\"\n",
        "[project]\nname = \"Awkward Name\"\nversion = \"0.1.0\"\n",
    ] {
        let manifest = Manifest::parse_str(body).unwrap();
        assert!(
            matches!(local_provider_owner(&manifest), Ok(Some(_))),
            "{body}"
        );
    }

    // The impossible coordinate is an error, not an absence — this is the
    // assertion that goes red if the owner ever returns to `.parse().ok()?`.
    let mut broken =
        Manifest::parse_str("[project]\nname = \"Bad Name\"\nversion = \"0.1.0\"\n").unwrap();
    broken.project.as_mut().unwrap().group = Some("org.example".parse().unwrap());
    let fault = local_provider_owner(&broken).expect_err("must not be a silent None");
    assert!(fault.contains("[project].name"), "{fault}");
    assert!(fault.contains("is not a valid package name"), "{fault}");
}

/// The landed R2 `HostIdentity` law decides which spelling a manifest owns:
/// a package and a **grouped** project own `<group>/<name>`; only an
/// **ungrouped** project owns `__host__/<project-name>`. `[[mechanism]]` is
/// legal in a project, so both owner kinds are locally checkable.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn project_hosts_own_the_landed_identity_spelling() {
    const UNGROUPED: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";
    const GROUPED: &str =
        "[project]\nname = \"demo\"\ngroup = \"org.example\"\nversion = \"0.1.0\"\n";

    // Ungrouped project: the host spelling is its own, and is checked.
    let route = |value: &str| format!("[mechanisms]\n\"build:cargo\" = \"{value}\"\n");
    assert!(
        Manifest::parse_str(&format!(
            "{UNGROUPED}\n{DECL}\n{}",
            route("__host__/demo#cargo-v2")
        ))
        .is_ok()
    );
    // Unknown id under the host owner: the diagnostic names the owner kind.
    let error = Manifest::parse_str(&format!(
        "{UNGROUPED}\n{DECL}\n{}",
        route("__host__/demo#ghost")
    ))
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("no `[[mechanism]]` with id `ghost`"),
        "{error}"
    );
    assert!(
        error.contains("this project's own host coordinate"),
        "{error}"
    );

    // Wrong role/name against the host-owned declaration.
    let mismatched = "[mechanisms]\n\"deploy:cargo\" = \"__host__/demo#cargo-v2\"\n";
    let error = Manifest::parse_str(&format!("{UNGROUPED}\n{DECL}\n{mismatched}"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("declared in this manifest as `build:cargo`"),
        "{error}"
    );
    assert!(
        error.contains("the logical key is `deploy:cargo`"),
        "{error}"
    );

    // Another project's host is foreign — runtime debt, never judged here.
    assert!(
        Manifest::parse_str(&format!(
            "{UNGROUPED}\n{DECL}\n{}",
            route("__host__/other-project#anything")
        ))
        .is_ok()
    );

    // A GROUPED project owns its real coordinate, not the host token: the
    // coordinate spelling is checked and the host spelling is foreign.
    let error = Manifest::parse_str(&format!(
        "{GROUPED}\n{DECL}\n{}",
        route("org.example/demo#ghost")
    ))
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("no `[[mechanism]]` with id `ghost`"),
        "{error}"
    );
    assert!(error.contains("this package's own coordinate"), "{error}");
    assert!(
        Manifest::parse_str(&format!(
            "{GROUPED}\n{DECL}\n{}",
            route("org.example/demo#cargo-v2")
        ))
        .is_ok()
    );
    // ...and `__host__/demo` is NOT this grouped project's identity.
    assert!(
        Manifest::parse_str(&format!(
            "{GROUPED}\n{DECL}\n{}",
            route("__host__/demo#ghost")
        ))
        .is_ok()
    );

    // Target pins answer to the same owner law.
    let build = concat!(
        "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:rustc\"\n",
        "provider = \"__host__/demo#cargo-v2\"\n",
        "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
    );
    let error = Manifest::parse_str(&format!("{UNGROUPED}\n{DECL}\n{build}"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("[[artifacts.build]] `helper` field `provider`"),
        "{error}"
    );
    assert!(
        error.contains("the logical key is `build:rustc`"),
        "{error}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn target_provider_pins_are_cross_checked_too() {
    let build = concat!(
        "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:cargo\"\n",
        "provider = \"org.example/build-tools#cargo-v2\"\n",
        "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
    );
    assert!(Manifest::parse_str(&format!("{PACKAGE}\n{DECL}\n{build}")).is_ok());

    let wrong = build.replace("mechanism = \"build:cargo\"", "mechanism = \"build:rustc\"");
    let error = Manifest::parse_str(&format!("{PACKAGE}\n{DECL}\n{wrong}"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("[[artifacts.build]] `helper` field `provider`"),
        "{error}"
    );
    assert!(
        error.contains("the logical key is `build:rustc`"),
        "{error}"
    );

    let deploy_decl = DECL
        .replace("role = \"build\"", "role = \"deploy\"")
        .replace("name = \"cargo\"", "name = \"vibe-bin\"")
        .replace("id = \"cargo-v2\"", "id = \"vibe-bin-v2\"");
    let deploy = concat!(
        "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:cargo\"\n",
        "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
        "[[deploy.target]]\nid = \"local\"\nartifact = \"helper.exe\"\n",
        "mechanism = \"deploy:other\"\nprovider = \"org.example/build-tools#vibe-bin-v2\"\n",
    );
    let error = Manifest::parse_str(&format!("{PACKAGE}\n{deploy_decl}\n{deploy}"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("[[deploy.target]] `local` field `provider`"),
        "{error}"
    );
    assert!(
        error.contains("the logical key is `deploy:other`"),
        "{error}"
    );
}

/// A chain far deeper than any recursive walker survives. The point is that
/// a deep authored graph produces a verdict, not a stack overflow.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn deep_chains_are_walked_iteratively() {
    const DEPTH: usize = 50_000;
    let chain = |looped: bool| {
        let mut section = ArtifactsSection::default();
        for index in 0..DEPTH {
            let previous = if index == 0 {
                if looped { Some(DEPTH - 1) } else { None }
            } else {
                Some(index - 1)
            };
            section.build.push(ArtifactTarget {
                id: format!("t{index}"),
                mechanism: "build:cargo".parse().unwrap(),
                provider: None,
                inputs: previous.map(|previous| {
                    vec![ArtifactInput::Artifact {
                        artifact: format!("o{previous}"),
                    }]
                }),
                outputs: vec![ArtifactOutput {
                    id: format!("o{index}"),
                    kind: "executable".into(),
                }],
                config: None,
            });
        }
        section
    };
    assert!(chain(false).validate().is_ok());
    let error = chain(true).validate().expect_err("the loop must be caught");
    assert!(error.contains("cyclic"), "deep cycle must report a cycle");
}
