//! §7's profile-legality rule, as the pure function that decides it.
//!
//! > "`vibe deploy` without `--profile` is legal only when the manifest
//! > names an explicit default or defines exactly one profile.
//! > Environment variables and the presence of secrets never choose a
//! > profile."
//!
//! The rule is tested where it lives — over a parsed manifest, with no
//! process, no filesystem and no environment — because that is what makes
//! "environment never chooses" a property rather than a promise.

use specmark::verifies;
use vibe_core::manifest::Manifest;

use super::profile::{ProfileMode, ProfileResolution};
use super::{plan_json, resolve_profile};
use vibe_lifecycle::{DeployPlanReport, DeployResourcePlan};

fn resolve(
    deploy: Option<&vibe_core::manifest::DeploySection>,
    requested: Option<&str>,
) -> anyhow::Result<Option<ProfileResolution>> {
    resolve_profile(
        deploy,
        requested,
        ProfileMode::Forward(vibe_core::manifest::TargetOs::Windows),
    )
}

/// Parse one fixture manifest.
fn manifest(body: &str) -> Manifest {
    Manifest::parse_str(body).unwrap_or_else(|error| panic!("the fixture manifest parses: {error}"))
}

/// The artifact half every deploy fixture needs: one build target whose
/// output a deploy target may name.
const ARTIFACTS: &str = concat!(
    "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
    "[[artifacts.build]]\nid = \"tool\"\nmechanism = \"build:cargo\"\n",
    "outputs = [{ id = \"tool.exe\", kind = \"executable\" }]\n\n",
);

/// One `[[deploy.target]]` row.
fn target(id: &str) -> String {
    format!(
        "[[deploy.target]]\nid = \"{id}\"\nartifact = \"tool.exe\"\n\
         mechanism = \"deploy:vibe-bin\"\n\n"
    )
}

/// One `[deploy.profiles.<name>]` row.
fn profile(name: &str, targets: &[&str]) -> String {
    let list = targets
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[deploy.profiles.{name}]\ntargets = [{list}]\n\n")
}

/// Step 1: an explicit `--profile` wins.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_explicit_profile_wins() {
    let body = format!(
        "{ARTIFACTS}[deploy]\ndefault_profile = \"local\"\n\n{}{}{}{}",
        target("a"),
        target("b"),
        profile("local", &["a"]),
        profile("production", &["b"]),
    );
    let manifest = manifest(&body);

    let selection = resolve(manifest.deploy.as_ref(), Some("production"))
        .expect("an explicit profile resolves")
        .expect("a selection");

    assert_eq!(selection.profile, "production");
    assert_eq!(selection.targets, ["b"]);
}

/// Step 2: the manifest's own `default_profile`.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_declared_default_profile_answers_a_bare_deploy() {
    let body = format!(
        "{ARTIFACTS}[deploy]\ndefault_profile = \"local\"\n\n{}{}{}{}",
        target("a"),
        target("b"),
        profile("local", &["a"]),
        profile("production", &["b"]),
    );
    let manifest = manifest(&body);

    let selection = resolve(manifest.deploy.as_ref(), None)
        .expect("the declared default resolves")
        .expect("a selection");

    assert_eq!(selection.profile, "local");
    assert_eq!(selection.targets, ["a"]);
}

/// Step 3: exactly one defined profile answers a bare deploy.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn exactly_one_profile_answers_a_bare_deploy() {
    let body = format!("{ARTIFACTS}{}{}", target("a"), profile("local", &["a"]),);
    let manifest = manifest(&body);

    let selection = resolve(manifest.deploy.as_ref(), None)
        .expect("the exactly-one rule resolves")
        .expect("a selection");

    assert_eq!(selection.profile, "local");
}

/// Step 4: two profiles and no declared default is a typed refusal that
/// NAMES the defined profiles.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn two_profiles_without_a_default_refuse_and_name_them() {
    let body = format!(
        "{ARTIFACTS}{}{}{}{}",
        target("a"),
        target("b"),
        profile("local", &["a"]),
        profile("production", &["b"]),
    );
    let manifest = manifest(&body);

    let error = resolve(manifest.deploy.as_ref(), None)
        .expect_err("a bare deploy over two profiles is illegal");

    let rendered = error.to_string();
    assert!(rendered.contains("needs a profile"), "{rendered}");
    assert!(rendered.contains("local"), "{rendered}");
    assert!(rendered.contains("production"), "{rendered}");
    assert!(
        rendered.contains("an environment variable never chooses a profile"),
        "{rendered}",
    );
}

/// A `--profile` naming nothing defined refuses and lists what is.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_unknown_profile_refuses_and_lists_the_defined_ones() {
    let body = format!("{ARTIFACTS}{}{}", target("a"), profile("local", &["a"]));
    let manifest = manifest(&body);

    let error = resolve(manifest.deploy.as_ref(), Some("staging"))
        .expect_err("an undefined profile refuses");

    let rendered = error.to_string();
    assert!(rendered.contains("`--profile staging`"), "{rendered}");
    assert!(rendered.contains("defined: local"), "{rendered}");
}

/// A project with no `[deploy]` section deploys nothing and does NOT
/// refuse — `vibe deploy` is the ninth phase verb, and the legality rule
/// is about choosing among profiles, not about running the verb.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_project_with_no_deploy_section_selects_nothing() {
    let manifest = manifest(ARTIFACTS);

    assert!(
        resolve(manifest.deploy.as_ref(), None)
            .expect("a project with nothing to deploy still runs")
            .is_none(),
    );
}

/// …but a `--profile` on such a project is a refusal: the operator asked
/// for something that does not exist.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_profile_flag_on_a_project_with_no_profiles_refuses() {
    let manifest = manifest(ARTIFACTS);

    let error = resolve(manifest.deploy.as_ref(), Some("local"))
        .expect_err("naming a profile that cannot exist refuses");

    assert!(
        error.to_string().contains("declares no deploy profiles"),
        "{error}",
    );
}

/// The authored ORDER of a profile's targets survives resolution — §7:
/// "Profile targets are ordered as authored".
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_authored_target_order_survives() {
    let body = format!(
        "{ARTIFACTS}{}{}{}",
        target("second"),
        target("first"),
        profile("local", &["second", "first"]),
    );
    let manifest = manifest(&body);

    let selection = resolve(manifest.deploy.as_ref(), None)
        .expect("the one profile resolves")
        .expect("a selection");

    assert_eq!(selection.targets, ["second", "first"]);
}

#[test]
fn one_default_profile_projects_windows_and_posix_without_choosing_a_profile() {
    let body = format!(
        "{ARTIFACTS}[[deploy.target]]\nid='win'\nartifact='tool.exe'\nmechanism='deploy:vibe-bin'\nwhen={{os=['windows']}}\n\
         [[deploy.target]]\nid='posix'\nartifact='tool.exe'\nmechanism='deploy:vibe-bin'\nwhen={{os=['linux','macos']}}\n\
         [deploy]\ndefault_profile='local'\n{}",
        profile("local", &["win", "posix"]),
    );
    let parsed = manifest(&body);
    let deploy = parsed.deploy.as_ref();
    let windows = resolve_profile(
        deploy,
        None,
        ProfileMode::Forward(vibe_core::manifest::TargetOs::Windows),
    )
    .unwrap()
    .unwrap();
    let linux = resolve_profile(
        deploy,
        None,
        ProfileMode::Forward(vibe_core::manifest::TargetOs::Linux),
    )
    .unwrap()
    .unwrap();
    assert_eq!(windows.profile, "local");
    assert_eq!(linux.profile, "local");
    assert_eq!(windows.targets, ["win"]);
    assert_eq!(linux.targets, ["posix"]);
    assert_eq!(windows.decisions()[1].status(), "skipped");

    let reports = [DeployPlanReport {
        target: "win".into(),
        mechanism: "deploy:vibe-bin".into(),
        provider: "org.vibevm/vibe#vibe-bin".into(),
        via: "the shipped builtin default".into(),
        displaced_default: None,
        planned: true,
        reason: "new deployment".into(),
        resources: vec![DeployResourcePlan {
            resource: "bin/tool".into(),
            desired_digest: "0".repeat(64),
            recorded_digest: None,
            change: "create".into(),
        }],
        summary: "install tool".into(),
    }];
    let json = plan_json(&windows, &reports).unwrap();
    let rows = json["targets"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["status"], "active");
    assert_eq!(rows[1]["status"], "skipped");
    assert_eq!(rows[1]["observed_os"], "windows");
    assert_eq!(rows[1]["when"]["os"], serde_json::json!(["linux", "macos"]));
    assert!(rows[1]["reason"].as_str().unwrap().contains("excludes"));
}

#[test]
fn forward_dependency_and_zero_applicable_refuse_but_inverse_keeps_authored_members() {
    let body = format!(
        "{ARTIFACTS}[[deploy.target]]\nid='win'\nartifact='tool.exe'\nmechanism='deploy:vibe-bin'\nwhen={{os=['windows']}}\n\
         [[deploy.target]]\nid='posix'\nartifact='tool.exe'\nmechanism='deploy:vibe-bin'\nwhen={{os=['linux']}}\ndepends_on=['win']\n{}",
        profile("local", &["win", "posix"]),
    );
    let parsed = manifest(&body);
    let deploy = parsed.deploy.as_ref();
    let error = resolve_profile(
        deploy,
        None,
        ProfileMode::Forward(vibe_core::manifest::TargetOs::Linux),
    )
    .unwrap_err()
    .to_string();
    for expected in ["posix", "win", "linux"] {
        assert!(error.contains(expected), "{error}");
    }
    let inverse = resolve_profile(deploy, None, ProfileMode::Inverse)
        .unwrap()
        .unwrap();
    assert_eq!(inverse.targets, ["win", "posix"]);
    assert!(inverse.decisions().is_empty());

    let only_win = format!(
        "{ARTIFACTS}{}{}",
        target("win").replace("\n\n", "\nwhen={os=['windows']}\n\n"),
        profile("local", &["win"])
    );
    let manifest = manifest(&only_win);
    let error = resolve_profile(
        manifest.deploy.as_ref(),
        None,
        ProfileMode::Forward(vibe_core::manifest::TargetOs::Linux),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("NO_APPLICABLE_TARGETS"), "{error}");
}

/// The whole point, as a fence rather than a promise: the resolver's
/// source reads no environment at all.
///
/// A test that SET an environment variable and asserted the answer did
/// not change would be both weaker (it can only cover the spellings it
/// guessed) and unsound (libtest runs bodies on many threads, and
/// `set_var` from one while another reads is the exact undefined
/// behaviour that made those functions `unsafe`). Reading the cell's own
/// text covers every spelling and mutates nothing.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_resolver_reads_no_environment() {
    let source = include_str!("profile.rs");
    for forbidden in ["std::env", "env::var", "env!(", "option_env!(", "getenv"] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` appears in the profile resolver; §7 forbids the environment from              choosing a profile, and the only way to keep that true is for the cell not to be              able to read one",
        );
    }
    // And the same for the settings/secret surfaces a resolver must not
    // consult: the presence of a token never selects a destination.
    for forbidden in ["settings_dir", "publish.token", "read_to_string"] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` appears in the profile resolver"
        );
    }
}
