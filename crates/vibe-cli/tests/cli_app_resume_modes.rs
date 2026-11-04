//! Actual terminal configuration prompts and retained per-step agent delivery mode.
#![cfg(all(test, target_os = "linux"))]
mod common;
use common::app_registry::{AppRegistry, json, ok};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn pty(r: &AppRegistry, runner: &Path, args: &[&str]) -> Output {
    pty_input(r, runner, args, "")
}
fn pty_input(r: &AppRegistry, runner: &Path, args: &[&str], input: &str) -> Output {
    Command::new(runner)
        .arg(env!("CARGO_BIN_EXE_vibe"))
        .args(args)
        .current_dir(r.root.path())
        .env("VIBE_SETTINGS", r.settings())
        .env("VIBE_REGISTRY_CACHE", &r.user.cache)
        .env("VIBEVM_SEARCH_CACHE_DIR", &r.user.search_cache)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("HOME", r.home())
        .env("TERM", "xterm")
        .env("VIBE_TEST_PTY_INPUT", input)
        .env(
            "VIBE_TEST_PTY_WAIT",
            if args.contains(&"org.example/secret-default") {
                "Token (token)"
            } else {
                ""
            },
        )
        .env("VIBE_AGENT_COMMAND", "sh")
        .env("VIBE_AGENT_ARGS", "[]")
        .env_remove("VIBE_INVOKED_BY")
        .env_remove("VIBE_UNATTENDED")
        .env_remove("CI")
        .output()
        .unwrap()
}
fn compiled_runner(scope: &Path) -> std::path::PathBuf {
    let c = scope.join("pty.c");
    let runner = scope.join("pty");
    let source=include_str!("common/pty-runner.c").replace("#include <errno.h>","#include <errno.h>\n#include <poll.h>\n#include <signal.h>\n#include <time.h>").replace("    char buffer[4096];","    time_t started = time(NULL);\n    char buffer[4096];").replace("        ssize_t length = read(master, buffer, sizeof(buffer));","        if (time(NULL) - started > 20) { kill(-child, SIGKILL); waitpid(child, NULL, 0); return 131; }\n        struct pollfd ready = { .fd = master, .events = POLLIN };\n        if (poll(&ready, 1, 1000) == 0) continue;\n        ssize_t length = read(master, buffer, sizeof(buffer));");
    let source=source.replace("    if (input != NULL && write_all(master, input, strlen(input)) < 0) return 127;", "    const char *wait_for = getenv(\"VIBE_TEST_PTY_WAIT\");\n    int deferred = wait_for != NULL && strlen(wait_for) > 0;\n    int sent = !deferred;\n    char seen[8192] = {0}; size_t used = 0;\n    if (!deferred && input != NULL && write_all(master, input, strlen(input)) < 0) return 127;")
        .replace("            if (write_all(STDOUT_FILENO, buffer, (size_t)length) < 0) return 127;", "            if (write_all(STDOUT_FILENO, buffer, (size_t)length) < 0) return 127;\n            if (!sent && used + (size_t)length < sizeof(seen)) {\n                memcpy(seen + used, buffer, (size_t)length); used += (size_t)length; seen[used] = 0;\n                if (strstr(seen, wait_for) != NULL) {\n                    struct termios state;\n                    for (int i = 0; i < 100; i++) {\n                        if (tcgetattr(master, &state) == 0 && !(state.c_lflag & ECHO)) break;\n                        usleep(10000);\n                    }\n                    if (input != NULL && write_all(master, input, strlen(input)) < 0) return 127;\n                    sent = 1;\n                }\n            }");
    fs::write(&c, source).unwrap();
    let compile = Command::new("cc")
        .args(["-O0", "-o"])
        .arg(&runner)
        .arg(&c)
        .arg("-lutil")
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    runner
}
#[test]
fn inherited_mode_is_stable_across_actual_pty_and_hosted_non_tty_resume_in_both_directions() {
    let scratch = tempfile::tempdir().unwrap();
    let runner = compiled_runner(scratch.path());
    for attended in [true, false] {
        let r = AppRegistry::new();
        let package = r.package(
            "mode",
            "1.0.0",
            "mode",
            r#"
result='owned'
[[installer_options.steps]]
id='build'
title='Build'
kind='recipe'
script='build.sh'
[[installer_options.steps]]
id='finish'
title='Finish'
kind='agent'
phase='finish'
required=true
prompt='Complete the bounded inherited-mode fixture.'
"#,
        );
        fs::write(package.join("build.sh"),format!("set -e\necho built >> '{}'\nmkdir -p \"$VIBE_OUT\"\necho payload > \"$VIBE_OUT/receipt\"\n",r.root.path().join("runs").display())).unwrap();
        let registry = r.registry();
        let args = [
            "install",
            "-g",
            "org.example/mode",
            "--installer",
            "main",
            "--platform",
            "linux",
            "--arch",
            std::env::consts::ARCH,
            "--registry",
            registry.to_str().unwrap(),
            "--keep-work",
            "--offline",
            "--assume-yes",
            "--agent-mode",
            "agent",
        ];
        let output = if attended {
            pty(&r, &runner, &args)
        } else {
            r.user
                .vibe()
                .current_dir(r.root.path())
                .env("HOME", r.home())
                .env("VIBE_AGENT_COMMAND", "sh")
                .env("VIBE_AGENT_ARGS", "[]")
                .env_remove("VIBE_INVOKED_BY")
                .args(args)
                .args(["--json", "--unattended"])
                .output()
                .unwrap()
        };
        assert_eq!(
            output.status.code(),
            Some(75),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let work = json(&r.vibe(&["app", "work", "-g", "--json"]));
        let row = &work["entries"][0];
        let id = row["operation"]["target"]["operation"].as_str().unwrap();
        let envelope = Path::new(row["path"].as_str().unwrap())
            .join("steps/finish/operations")
            .join(id);
        let original = fs::read(envelope.join("context.json")).unwrap();
        let context: serde_json::Value = serde_json::from_slice(&original).unwrap();
        assert_eq!(
            context["mode"],
            if attended {
                "interactive"
            } else {
                "unattended"
            }
        );
        fs::write(
            envelope.join("result.json"),
            r#"{"protocol":"vibe-agent-hook-result/1","status":"ok","message":"done"}"#,
        )
        .unwrap();
        let args = [
            "app",
            "resume",
            "org.example/mode@1.0.0",
            "--operation",
            id,
            "-g",
            "--offline",
        ];
        let output = if attended {
            r.vibe(&[args.as_slice(), &["--json", "--unattended"]].concat())
        } else {
            pty(&r, &runner, &args)
        };
        ok(&output);
        assert_eq!(fs::read(envelope.join("context.json")).unwrap(), original);
        assert_eq!(
            fs::read_to_string(r.root.path().join("runs")).unwrap(),
            "built\n"
        );
    }
}

#[test]
fn attended_modify_enter_preserves_the_actual_rich_group_selection() {
    let scratch = tempfile::tempdir().unwrap();
    let runner = compiled_runner(scratch.path());
    let r = AppRegistry::new();
    let package = r.package(
        "choice",
        "1.0.0",
        "choice",
        r#"
result='owned'
[[installer_options.steps]]
id='build'
title='Build'
kind='recipe'
script='build.sh'
[[installer_options.configuration.features]]
id='a'
name='A'
[[installer_options.configuration.features]]
id='b'
name='B'
[[installer_options.configuration.groups]]
mode='choice'
id='g'
name='Group'
cardinality='exactly-one'
members=['a','b']
defaults=['a']
"#,
    );
    fs::write(package.join("build.sh"),format!("set -e\necho \"$VIBE_FEATURE_A|$VIBE_FEATURE_B\" >> '{}'\nmkdir -p \"$VIBE_OUT\"\necho built > \"$VIBE_OUT/receipt\"\n",r.root.path().join("effects").display())).unwrap();
    let input = r.root.path().join("input.json");
    fs::write(&input,r#"{"protocol":"vibe-configuration-input/1","input":{"intent":{"policy":"require-selection","features":[],"defaultFeatures":false,"selections":{"g":{"intent":"exact","members":["b"]}},"parameters":{}},"disabledFeatures":[]}}"#).unwrap();
    ok(&r.app(
        &["install", "-g"],
        "org.example/choice",
        &[
            "--configuration-input",
            input.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ],
    ));
    let output = pty_input(
        &r,
        &runner,
        &[
            "app",
            "modify",
            "org.example/choice",
            "-g",
            "--registry",
            r.registry().to_str().unwrap(),
            "--offline",
        ],
        "\n",
    );
    ok(&output);
    let lines = fs::read_to_string(r.root.path().join("effects")).unwrap();
    let rows: Vec<_> = lines.lines().collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], rows[1]);
    assert_eq!(
        r.index()["installations"]["org.example/choice@1.0.0"]["features"],
        serde_json::json!(["b"])
    );
}
#[test]
fn hidden_required_secret_enter_retains_its_public_declared_default() {
    let scratch = tempfile::tempdir().unwrap();
    let runner = compiled_runner(scratch.path());
    let r = AppRegistry::new();
    let package = r.package(
        "secret-default",
        "1.0.0",
        "secret-default",
        r#"
result='owned'
[[installer_options.steps]]
id='build'
title='Build'
kind='recipe'
script='build.sh'
[[installer_options.configuration.parameters]]
id='token'
name='Token'
type='string'
required=true
secret=true
default='public-authored-default'
"#,
    );
    fs::write(package.join("build.sh"),"set -e\ntest \"$VIBE_PARAM_TOKEN\" = 'public-authored-default'\nmkdir -p \"$VIBE_OUT\"\necho built > \"$VIBE_OUT/receipt\"\n").unwrap();
    let output = pty_input(
        &r,
        &runner,
        &[
            "install",
            "-g",
            "org.example/secret-default",
            "--installer",
            "main",
            "--platform",
            "linux",
            "--arch",
            std::env::consts::ARCH,
            "--registry",
            r.registry().to_str().unwrap(),
            "--offline",
            "--assume-yes",
        ],
        "\n",
    );
    ok(&output);
    let index = r.index();
    let parameter = &index["installations"]["org.example/secret-default@1.0.0"]["configuration"]["evaluation"]
        ["parameters"][0];
    assert_eq!(parameter["origin"]["kind"], "default");
    assert_eq!(parameter["provided"], false);
    assert!(parameter.get("value").is_none());
}

#[test]
fn attended_visible_edit_retains_dormant_member_in_the_record_then_reactivation_uses_it() {
    let scratch = tempfile::tempdir().unwrap();
    let runner = compiled_runner(scratch.path());
    let r = AppRegistry::new();
    let package = r.package(
        "dormant-ui",
        "1.0.0",
        "dormant-ui",
        r#"
result='owned'
[[installer_options.steps]]
id='build'
title='Build'
kind='recipe'
script='build.sh'
[[installer_options.configuration.features]]
id='a'
name='A'
[[installer_options.configuration.features]]
id='b'
name='B'
guards=[{member='gate'}]
[[installer_options.configuration.features]]
id='gate'
name='Gate'
[[installer_options.configuration.groups]]
mode='choice'
id='g'
name='Group'
cardinality='zero-or-more'
members=['a','b']
[[installer_options.configuration.groups]]
mode='choice'
id='activation'
name='Activation'
cardinality='zero-or-one'
members=['gate']
"#,
    );
    fs::write(
        package.join("build.sh"),
        "set -e\nmkdir -p \"$VIBE_OUT\"\nprintf '%s' \"$VIBE_FEATURE_B\" > \"$VIBE_OUT/b\"\n",
    )
    .unwrap();
    let input = r.root.path().join("input.json");
    fs::write(&input,r#"{"protocol":"vibe-configuration-input/1","input":{"intent":{"policy":"require-selection","features":[],"defaultFeatures":false,"selections":{"activation":{"intent":"exact","members":[]},"g":{"intent":"additive","members":["a","b"]}},"parameters":{}},"disabledFeatures":[]}}"#).unwrap();
    ok(&r.app(
        &["install", "-g"],
        "org.example/dormant-ui",
        &[
            "--configuration-input",
            input.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ],
    ));
    ok(&pty_input(
        &r,
        &runner,
        &[
            "app",
            "modify",
            "org.example/dormant-ui",
            "-g",
            "--registry",
            r.registry().to_str().unwrap(),
            "--offline",
        ],
        "\n\n",
    ));
    let index = r.index();
    let stored = &index["installations"]["org.example/dormant-ui@1.0.0"]["configuration"]["evaluation"]
        ["input"];
    assert_eq!(
        stored["intent"]["selections"]["g"]["members"],
        serde_json::json!(["a", "b"])
    );
    assert_eq!(stored["intent"]["selections"]["g"]["intent"], "additive");
    let mut next = stored.clone();
    next["intent"]["selections"]["activation"]["members"] = serde_json::json!(["gate"]);
    fs::write(
        &input,
        serde_json::to_vec(
            &serde_json::json!({"protocol":"vibe-configuration-input/1","input":next}),
        )
        .unwrap(),
    )
    .unwrap();
    ok(&r.app(
        &["app", "modify"],
        "org.example/dormant-ui",
        &[
            "-g",
            "--configuration-input",
            input.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ],
    ));
    assert_eq!(
        r.index()["installations"]["org.example/dormant-ui@1.0.0"]["features"],
        serde_json::json!(["a", "b", "gate"])
    );
}
