//! Public Linux fixture for structured installation (PROP-065
//! `#acceptance`, minus agent delivery): a four-step owned option
//! (`sources` → `agent` under a feature → `recipe` → `copy`) plus a
//! `finish` step; `plan` and `describe`; an unmet `command:` need refused
//! before download; a failing middle step kept for `--resume`;
//! `--from-step`; features by flag and by default; an external result whose
//! recorded shell uninstalls run in reverse; a required agent step refused.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Output;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::UserScratch;
use serde_json::Value;
use sha2::{Digest, Sha256};

const KEY: &str = "org.example/stepped@1.0.0";
const WORK: &str = "cache/app-work/org.example.stepped@1.0.0";

/// Serve `body` on a loopback port; count the requests.
fn serve(body: Vec<u8>) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hits);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = Vec::new();
            let mut buffer = [0u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => request.extend_from_slice(&buffer[..read]),
                }
            }
            counter.fetch_add(1, Ordering::SeqCst);
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    (format!("http://{address}/dist/tool-src.tar.gz"), hits)
}

fn tar_gz(name: &str, body: &str) -> Vec<u8> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut tar = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, name, body.as_bytes()).unwrap();
    tar.into_inner().unwrap().finish().unwrap()
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn header(name: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
         [application]\nid = \"{name}\"\n\n[[installer_options]]\nid = \"main\"\nname = \"Main\"\n\
         platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{}\"\n",
        std::env::consts::ARCH
    )
}

struct Fixture {
    root: tempfile::TempDir,
    registry: PathBuf,
    log: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        let log = root.path().join("steps.log");
        Self {
            root,
            registry,
            log,
        }
    }

    fn package(&self, name: &str, body: &str) -> PathBuf {
        let package = self.registry.join(format!("org.example/{name}/v1.0.0"));
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("vibe.toml"), format!("{}{body}", header(name))).unwrap();
        package
    }

    fn vibe(&self, user: &UserScratch, args: &[&str]) -> Output {
        let mut command = user.vibe();
        command.current_dir(self.root.path()).args(args);
        // An agent posture of the caller would deliver the agent step
        // (PROP-064); these proofs run without one.
        for variable in [
            "HTTP_PROXY",
            "http_proxy",
            "ALL_PROXY",
            "all_proxy",
            "VIBE_AGENT_COMMAND",
            "VIBE_AGENT_ARGS",
            "VIBE_INVOKED_BY",
        ] {
            command.env_remove(variable);
        }
        command.output().unwrap()
    }

    fn app(&self, user: &UserScratch, verb: &[&str], package: &str, extra: &[&str]) -> Output {
        let registry = self.registry.to_str().unwrap();
        let mut args = verb.to_vec();
        args.extend_from_slice(&[package, "--registry", registry]);
        args.extend_from_slice(extra);
        self.vibe(user, &args)
    }

    fn log(&self) -> String {
        fs::read_to_string(&self.log).unwrap_or_default()
    }
}

fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn failed(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "unexpected success: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn json(output: &Output) -> Value {
    serde_json::from_str(&ok(output)).unwrap()
}

fn index(user: &UserScratch) -> Value {
    serde_json::from_slice(&fs::read(user.settings.join("applications/index.json")).unwrap())
        .unwrap()
}

/// The four-step owned option plus a finish step.
fn stepped(fixture: &Fixture, url: &str, url_sha: &str) -> PathBuf {
    let log = fixture.log.display();
    let package = fixture.package(
        "stepped",
        &format!(
            r#"
[[installer_options.features]]
id = "fixes"
name = "Agent fixes"

[[installer_options.features]]
id = "docs"
name = "Documentation"
default = true

[[installer_options.steps]]
id = "fetch"
title = "Fetch the sources"
kind = "sources"
needs = ["network"]
[[installer_options.steps.sources]]
name = "upstream"
url = "{url}"
sha256 = "{url_sha}"

[[installer_options.steps]]
id = "fix"
title = "Let an agent adapt the sources"
kind = "agent"
when = {{ feature = "fixes" }}
prompt = "Adapt $sources/upstream to this host"

[[installer_options.steps]]
id = "build"
title = "Build"
kind = "recipe"
interpreter = "posix_sh"
script = "build.sh"

[[installer_options.steps]]
id = "docs"
title = "Copy the documentation"
kind = "copy"
when = {{ feature = "docs" }}
from = "docs"
to = "share/doc"

[[installer_options.steps]]
id = "announce"
title = "Announce the root"
kind = "shell"
phase = "finish"
command = "echo \"$VIBE_APP_ROOT\" > root.txt && echo FINISH >> '{log}'"

[[installer_options.launchers]]
id = "main"
name = "Tool"
executable = "$app_bin/tool"
default = true
"#
        ),
    );
    fs::write(
        package.join("build.sh"),
        format!(
            "set -e\necho RECIPE >> '{log}'\ntest ! -f \"$VIBE_PACKAGE/FAIL\" || exit 4\n\
             mkdir -p \"$VIBE_OUT/bin\"\n{{ echo '#!/bin/sh'; cat \"$VIBE_SOURCES/upstream/tool.sh.in\"; }} > \"$VIBE_OUT/bin/tool\"\n\
             chmod 755 \"$VIBE_OUT/bin/tool\"\n"
        ),
    )
    .unwrap();
    fs::create_dir_all(package.join("docs")).unwrap();
    fs::write(package.join("docs/readme.txt"), "read me").unwrap();
    package
}

#[test]
fn a_step_list_plans_installs_fails_resumes_and_restarts() {
    let tarball = tar_gz("tool.sh.in", "echo \"TOOL_RAN $*\"\n");
    let url_sha = sha(&tarball);
    let (url, hits) = serve(tarball);
    let fixture = Fixture::new();
    let package = stepped(&fixture, &url, &url_sha);
    let user = UserScratch::new();
    let name = "org.example/stepped";

    // plan: run / skip without downloading or running anything.
    let plan = json(&fixture.app(&user, &["app", "plan"], name, &["--json"]));
    assert_eq!(plan["protocol"], "vibe-app-plan/1");
    assert_eq!(plan["result"], "owned");
    let actions: Vec<(&str, &str)> = plan["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| {
            (
                step["id"].as_str().unwrap(),
                step["action"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        actions,
        [
            ("fetch", "run"),
            ("fix", "skip"),
            ("build", "run"),
            ("docs", "run"),
            ("announce", "run")
        ]
    );
    assert_eq!(plan["features"][1]["chosen"], true);
    let text = ok(&fixture.app(&user, &["app", "plan"], name, &["--feature", "fixes"]));
    assert!(
        text.contains("skip (no agent configured) fix (agent)"),
        "{text}"
    );
    assert!(text.contains("warning: no agent is configured"), "{text}");
    let only = ok(&fixture.app(&user, &["install", "-g"], name, &["--plan-only"]));
    assert!(only.contains("1. run fetch (sources)"), "{only}");
    assert_eq!(hits.load(Ordering::SeqCst), 0, "planning downloads nothing");
    assert!(!user.settings.join("applications/index.json").exists());

    // describe: features and steps as declared.
    let described = json(&fixture.app(&user, &["app", "describe"], name, &["--json"]));
    assert_eq!(described["protocol"], "vibe-app-describe/1");
    let option = &described["options"][0];
    assert_eq!(option["form"], "steps");
    assert_eq!(option["features"][0]["id"], "fixes");
    assert_eq!(option["steps"][1]["kind"], "agent");
    assert_eq!(option["steps"][1]["when"][0]["feature"], "fixes");
    let steps_text = ok(&fixture.app(&user, &["app", "steps"], name, &[]));
    assert!(steps_text.contains("kind = \"copy\""), "{steps_text}");

    // A failing middle step keeps the work directory; the index is untouched.
    fs::write(package.join("FAIL"), "").unwrap();
    let error = failed(&fixture.app(&user, &["install", "-g"], name, &[]));
    assert!(error.contains("step `build` (recipe)"), "{error}");
    assert!(
        error.contains("kept for inspection") && error.contains("--resume"),
        "{error}"
    );
    assert!(error.contains("exited with code 4"), "{error}");
    let work = user.settings.join(WORK);
    let state: Value = serde_json::from_slice(&fs::read(work.join("steps.json")).unwrap()).unwrap();
    assert_eq!(state["protocol"], "vibe-app-steps/1");
    assert_eq!(state["steps"][0]["outcome"], "done");
    assert_eq!(state["steps"][1]["outcome"], "skipped");
    assert_eq!(state["steps"][2]["outcome"], "failed");
    assert!(work.join("sources/upstream/tool.sh.in").is_file());
    assert!(!user.settings.join("applications/index.json").exists());
    assert_eq!(hits.load(Ordering::SeqCst), 1);

    // --resume continues at `build` without fetching again.
    fs::remove_file(package.join("FAIL")).unwrap();
    let report = json(&fixture.app(&user, &["install", "-g"], name, &["--resume", "--json"]));
    assert_eq!(report["installation"], KEY);
    assert_eq!(report["selectedMode"], "steps");
    assert_eq!(hits.load(Ordering::SeqCst), 1, "a resume never re-fetches");
    assert_eq!(fixture.log(), "RECIPE\nRECIPE\nFINISH\n");
    assert!(!work.exists(), "success deletes the work directory");
    let payload = user
        .settings
        .join("opt/apps/stepped/org.example.stepped@1.0.0");
    assert_eq!(
        fs::read_to_string(payload.join("root.txt")).unwrap().trim(),
        payload.display().to_string(),
        "the finish step sees VIBE_APP_ROOT and runs in the payload"
    );
    assert_eq!(
        fs::read_to_string(payload.join("share/doc/readme.txt")).unwrap(),
        "read me"
    );
    let record = &index(&user)["installations"][KEY];
    assert_eq!(record["method"], "steps");
    assert_eq!(record["features"], serde_json::json!(["docs"]));
    let outcomes: Vec<&str> = record["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| step["outcome"].as_str().unwrap())
        .collect();
    assert_eq!(outcomes, ["done", "skipped", "done", "done", "done"]);
    let ran = ok(&fixture.vibe(&user, &["app", "run", name, "main"]));
    assert!(ran.contains("TOOL_RAN"), "{ran}");

    // --resume without a kept work directory is a clear error.
    let error = failed(&fixture.app(&user, &["install", "-g"], name, &["--resume"]));
    assert!(error.contains("no kept work directory"), "{error}");

    // --from-step restarts at a named step; an unknown id is refused.
    fs::write(package.join("FAIL"), "").unwrap();
    failed(&fixture.app(&user, &["install", "-g"], name, &[]));
    let error = failed(&fixture.app(&user, &["install", "-g"], name, &["--from-step", "nope"]));
    assert!(error.contains("names no step"), "{error}");
    fs::remove_file(package.join("FAIL")).unwrap();
    let before = fixture.log();
    ok(&fixture.app(&user, &["install", "-g"], name, &["--from-step", "fetch"]));
    assert_eq!(fixture.log(), format!("{before}RECIPE\nFINISH\n"));
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "the verified source stays cached"
    );

    // Features by flag: without docs the copy step is skipped.
    ok(&fixture.app(&user, &["install", "-g"], name, &["--no-feature", "docs"]));
    assert!(!payload.join("share/doc").exists());
    let record = &index(&user)["installations"][KEY];
    assert!(record.get("features").is_none(), "no feature chosen");
    assert_eq!(record["steps"][3]["outcome"], "skipped");
    ok(&fixture.vibe(&user, &["uninstall", "-g", name]));
    assert!(!payload.exists());
}

#[test]
fn an_unmet_command_need_is_refused_before_any_download() {
    let (url, hits) = serve(b"never served".to_vec());
    let fixture = Fixture::new();
    fixture.package(
        "needy",
        &format!(
            "[[installer_options.steps]]\nid = \"fetch\"\ntitle = \"Fetch\"\nkind = \"sources\"\n\
             [[installer_options.steps.sources]]\nname = \"up\"\nurl = \"{url}\"\nsha256 = \"{}\"\n\
             [[installer_options.steps]]\nid = \"build\"\ntitle = \"Build\"\nkind = \"recipe\"\nscript = \"b.sh\"\n\
             needs = [\"command:vibe-surely-absent-tool\"]\n",
            sha(b"never served")
        ),
    );
    let user = UserScratch::new();
    let error = failed(&fixture.app(&user, &["install", "-g"], "org.example/needy", &[]));
    assert!(error.contains("step `build`"), "{error}");
    assert!(error.contains("command:vibe-surely-absent-tool"), "{error}");
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    assert!(!user.settings.join("cache/app-work").exists());
    assert!(!user.settings.join("applications/index.json").exists());
}

#[test]
fn external_steps_record_uninstalls_that_run_in_reverse() {
    let fixture = Fixture::new();
    let root = fixture.root.path().join("external-root");
    let log = fixture.log.display();
    fixture.package(
        "outside",
        &format!(
            "result = {{ external = \"{root}\" }}\n\
             [[installer_options.steps]]\nid = \"one\"\ntitle = \"One\"\nkind = \"shell\"\n\
             command = \"mkdir -p '{root}' && echo one >> '{log}'\"\nuninstall = \"echo undo-one >> '{log}'\"\n\
             [[installer_options.steps]]\nid = \"two\"\ntitle = \"Two\"\nkind = \"shell\"\n\
             command = \"echo two >> '{log}'\"\nuninstall = \"echo undo-two >> '{log}'\"\n",
            root = root.display()
        ),
    );
    let user = UserScratch::new();
    let report = json(&fixture.app(
        &user,
        &["install", "-g"],
        "org.example/outside",
        &["--json"],
    ));
    assert_eq!(report["appRoot"], root.display().to_string());
    let record = &index(&user)["installations"]["org.example/outside@1.0.0"];
    assert_eq!(record["appRoot"], root.display().to_string());
    assert_eq!(
        record["steps"][1]["uninstall"],
        format!("echo undo-two >> '{log}'")
    );
    assert_eq!(record["commands"], serde_json::json!({}));
    ok(&fixture.vibe(&user, &["uninstall", "-g", "org.example/outside"]));
    assert_eq!(fixture.log(), "one\ntwo\nundo-two\nundo-one\n");
    assert!(root.is_dir(), "an external root is never deleted");
}

#[test]
fn a_required_agent_step_is_refused_and_a_method_form_describes_its_sugar_steps() {
    let fixture = Fixture::new();
    fixture.package(
        "agentic",
        "[[installer_options.steps]]\nid = \"port\"\ntitle = \"Port\"\nkind = \"agent\"\n\
         description = \"upstream builds only after an agent ports it\"\nprompt = \"Port it\"\nrequired = true\n\
         [[installer_options.steps]]\nid = \"docs\"\ntitle = \"Docs\"\nkind = \"copy\"\nfrom = \"vibe.toml\"\nto = \"x\"\n",
    );
    let user = UserScratch::new();
    let error = failed(&fixture.app(&user, &["install", "-g"], "org.example/agentic", &[]));
    assert!(
        error.contains("required but no agent is configured"),
        "{error}"
    );
    assert!(
        error.contains("upstream builds only after an agent ports it"),
        "{error}"
    );
    assert!(!user.settings.join("applications/index.json").exists());

    fixture.package(
        "zipped",
        "pre_install = \"true\"\n[installer_options.installation]\ntype = \"zip\"\nsource = \"tool.zip\"\n\
         sha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\n",
    );
    let described = json(&fixture.app(
        &user,
        &["app", "describe"],
        "org.example/zipped",
        &["--json"],
    ));
    let option = &described["options"][0];
    assert_eq!(option["form"], "installation");
    let kinds: Vec<(&str, &str)> = option["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| (step["id"].as_str().unwrap(), step["kind"].as_str().unwrap()))
        .collect();
    assert_eq!(kinds, [("pre_install", "shell"), ("archive", "archive")]);
    assert_eq!(option["steps"][0]["hook"], "pre_install");
}
