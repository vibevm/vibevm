//! Structured-installation manifest grammar (`PROP-065`): features, steps,
//! conditions, needs, templates, result and the sugar mapping.

use super::Manifest;
use crate::manifest::{
    Condition, InstallerOptionDecl, Need, ResultDecl, StepKind, StepOrigin, StepPhase, result_of,
    steps_of,
};

const HEADER: &str = r#"
[package]
name = "demo"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "demo"

[[installer_options]]
id = "main"
name = "Main"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"
"#;

const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn four_steps() -> String {
    format!(
        r#"{HEADER}
[[installer_options.features]]
id = "fixes"
name = "Agent fixes"
description = "Let an agent adapt the sources"

[[installer_options.features]]
id = "docs"
name = "Documentation"
default = true

[[installer_options.steps]]
id = "fetch"
title = "Fetch sources"
kind = "sources"
needs = ["network"]
[[installer_options.steps.sources]]
name = "upstream"
url = "https://example.com/src.tar.gz"
sha256 = "{SHA}"

[[installer_options.steps]]
id = "fix"
title = "Agent fixes"
kind = "agent"
when = {{ feature = "fixes" }}
needs = ["agent"]
prompt = "Fix $sources for ${{feature:docs}}"
mode = "unattended"

[[installer_options.steps]]
id = "build"
title = "Build"
kind = "recipe"
interpreter = "posix_sh"
script = "build.sh"
unless = [{{ offline = true }}, {{ env_present = "SKIP_BUILD" }}]
needs = ["command:make", "terminal"]

[[installer_options.steps]]
id = "docs"
title = "Copy docs"
kind = "copy"
from = "docs"
to = "share/doc"
optional = true
when = [{{ feature = "docs" }}, {{ file_present = "$package/docs" }}]

[[installer_options.steps]]
id = "notify"
title = "Say done"
kind = "shell"
phase = "finish"
command = "echo done"
uninstall = "echo bye"
"#
    )
}

fn option(text: &str) -> InstallerOptionDecl {
    Manifest::parse_str(text)
        .unwrap()
        .installer_options
        .remove(0)
}

fn refused(text: &str) -> String {
    Manifest::parse_str(text).unwrap_err().to_string()
}

#[test]
fn a_step_list_parses_round_trips_and_keeps_file_order() {
    let manifest = Manifest::parse_str(&four_steps()).unwrap();
    let option = &manifest.installer_options[0];
    assert!(option.installation.is_none());
    assert_eq!(option.features.len(), 2);
    assert!(option.features[1].default && !option.features[0].default);
    let ids: Vec<&str> = option.steps.iter().map(|step| step.id.as_str()).collect();
    assert_eq!(ids, ["fetch", "fix", "build", "docs", "notify"]);
    assert_eq!(option.steps[0].needs, [Need::Network]);
    assert_eq!(option.steps[1].when, [Condition::Feature("fixes".into())]);
    assert!(matches!(
        option.steps[1].kind,
        StepKind::Agent {
            required: false,
            ..
        }
    ));
    assert_eq!(
        option.steps[2].unless,
        [
            Condition::Offline(true),
            Condition::EnvPresent("SKIP_BUILD".into())
        ]
    );
    assert_eq!(
        option.steps[2].needs,
        [Need::Command("make".into()), Need::Terminal]
    );
    assert!(option.steps[3].optional);
    assert_eq!(option.steps[4].phase, StepPhase::Finish);
    assert_eq!(result_of(option), ResultDecl::owned());
    assert_eq!(steps_of(option), option.steps);
    let text = toml::to_string(&manifest).unwrap();
    assert_eq!(Manifest::parse_str(&text).unwrap(), manifest);
}

#[test]
fn every_method_maps_to_its_stated_step_list() {
    let kinds = |body: &str| -> (Vec<(String, &'static str, StepPhase)>, ResultDecl) {
        let option = option(&format!("{HEADER}{body}"));
        let steps = steps_of(&option)
            .into_iter()
            .map(|step| (step.id, step.kind.name(), step.phase))
            .collect();
        (steps, result_of(&option))
    };
    let prepare = StepPhase::Prepare;
    let zip = kinds(&format!(
        "[installer_options.installation]\ntype = \"zip\"\nsource = \"a.zip\"\nsha256 = \"{SHA}\"\n"
    ));
    assert_eq!(zip.0, [("archive".into(), "archive", prepare)]);
    assert!(zip.1.is_owned());
    let tgz = kinds(&format!(
        "[installer_options.installation]\ntype = \"tar_gz\"\nsource = \"a.tgz\"\nsha256 = \"{SHA}\"\n"
    ));
    assert_eq!(tgz.0, [("archive".into(), "archive", prepare)]);
    let shell = kinds(
        "pre_install = \"echo pre\"\npost_install = \"echo post\"\n[installer_options.installation]\ntype = \"shell\"\napp_root = \"/opt/x\"\ninstall = \"i\"\nforce_install = \"f\"\nuninstall = \"u\"\nforce_uninstall = \"fu\"\n",
    );
    assert_eq!(
        shell.0,
        [
            ("pre_install".into(), "shell", prepare),
            ("install".into(), "shell", prepare),
            ("post_install".into(), "shell", StepPhase::Finish),
        ]
    );
    assert_eq!(shell.1, ResultDecl::external("/opt/x"));
    let source = kinds(
        "[installer_options.installation]\ntype = \"source_build\"\nrecipe = { script = \"b.sh\" }\n",
    );
    assert_eq!(
        source.0,
        [
            ("sources".into(), "sources", prepare),
            ("recipe".into(), "recipe", prepare)
        ]
    );
    assert!(source.1.is_owned());
    let windows = format!("{HEADER}[installer_options.installation]\ntype = \"msi\"\nsource = \"a.msi\"\nsha256 = \"{SHA}\"\napp_root = \"C:/App\"\n")
        .replace("platform = \"linux\"", "platform = \"windows\"");
    let native = option(&windows);
    let steps = steps_of(&native);
    assert_eq!(steps.len(), 1);
    assert!(matches!(steps[0].kind, StepKind::Native { .. }));
    assert_eq!(result_of(&native), ResultDecl::external("C:/App"));
}

#[test]
fn hooks_become_leading_steps_that_remember_their_origin() {
    let text = four_steps().replace(
        "[[installer_options.features]]\nid = \"fixes\"",
        "pre_install = \"echo pre\"\npost_install = \"echo post\"\n\n[[installer_options.features]]\nid = \"fixes\"",
    );
    let steps = steps_of(&option(&text));
    assert_eq!(steps[0].id, "pre_install");
    assert_eq!(steps[0].origin, StepOrigin::PreInstall);
    assert_eq!(steps[5].id, "post_install");
    assert_eq!(steps[5].origin, StepOrigin::PostInstall);
    assert_eq!(steps[6].id, "notify", "post_install leads the finish phase");
    let collision = text.replace("id = \"notify\"", "id = \"post_install\"");
    assert!(refused(&collision).contains("duplicated"));
}

#[test]
fn step_rules_are_enforced_with_named_reasons() {
    let base = four_steps();
    let cases: Vec<(String, &str)> = vec![
        (
            base.replace(
                "[[installer_options.features]]\nid = \"fixes\"",
                &format!("[installer_options.installation]\ntype = \"zip\"\nsource = \"a.zip\"\nsha256 = \"{SHA}\"\n\n[[installer_options.features]]\nid = \"fixes\""),
            ),
            "mutually exclusive",
        ),
        (HEADER.to_owned(), "neither installation nor"),
        (base.replace("id = \"build\"", "id = \"fetch\""), "duplicated"),
        (base.replace("feature = \"fixes\"", "feature = \"nope\""), "undeclared feature"),
        (base.replace("kind = \"copy\"", "kind = \"script\""), "unknown variant"),
        (base.replace("to = \"share/doc\"", "to = \"share/doc\"\nscript = \"x\""), "unknown field"),
        (base.replace("{ feature = \"fixes\" }", "{ feature = \"fixes\", offline = true }"), "steps.when"),
        (base.replace("\"command:make\"", "\"command:\""), "command:NAME"),
        (base.replace("needs = [\"network\"]", "needs = [\"gpu\"]"), "command:NAME"),
        (base.replace("id = \"build\"\ntitle = \"Build\"", "id = \"build\"\ntitle = \"Build\"\nphase = \"finish\""), "finish phase"),
        (base.replace("$package/docs", "$home/docs"), "unknown template variable"),
        (base.replace("${feature:docs}", "${feature:nope}"), "undeclared feature"),
        (base.replace("prompt = \"Fix", "prompt_file = \"p.md\"\nprompt = \"Fix"), "exactly one"),
        (base.replace("to = \"share/doc\"", "to = \"../doc\""), "without `..`"),
        (base.replace("id = \"fixes\"\nname", "id = \"Fixes\"\nname"), "portable lowercase token"),
        (base.replace("title = \"Build\"", "title = \" \""), "title must be nonempty"),
    ];
    for (text, needle) in cases {
        let error = refused(&text);
        assert!(error.contains(needle), "expected `{needle}` in: {error}");
    }
}

#[test]
fn the_result_must_be_producible() {
    let shell_only = format!(
        "{HEADER}[[installer_options.steps]]\nid = \"a\"\ntitle = \"A\"\nkind = \"shell\"\ncommand = \"true\"\n"
    );
    assert!(refused(&shell_only).contains("owned result but no archive"));
    let external = format!(
        "{HEADER}result = {{ external = \"$vibe_home/tools/a\" }}\n{}",
        &shell_only[HEADER.len()..]
    );
    let option = option(&external);
    assert_eq!(
        result_of(&option),
        ResultDecl::external("$vibe_home/tools/a")
    );
    let relative = external.replace("$vibe_home/tools/a", "tools/a");
    assert!(refused(&relative).contains("absolute"));
    let with_method = format!(
        "{HEADER}result = \"owned\"\n[installer_options.installation]\ntype = \"zip\"\nsource = \"a.zip\"\nsha256 = \"{SHA}\"\n"
    );
    assert!(refused(&with_method).contains("apply only to [[steps]]"));
}
