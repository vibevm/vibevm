//! Public Linux fixture for the wizard surface of the installer engine
//! (PROP-065 `#feature-tree`, `#profiles`, `#parameters`, `#license`,
//! `#sizes`, `#icon`, `#runs`, `#cancellation`, `#events`, `#error-codes`,
//! `#exit-codes`), driven the way a graphical tool drives it: `describe`
//! → `plan` with a profile, features and parameters (and its refusals) →
//! `install --json --progress-format json` parsing the event stream →
//! `runs` listing the record → an install killed with SIGTERM, recorded
//! `interrupted` and resumed → `app errors` listing the codes met.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use common::UserScratch;
use serde_json::Value;
use sha2::{Digest, Sha256};

const NAME: &str = "org.example/wizard";
const KEY: &str = "org.example/wizard@1.0.0";
const WORK: &str = "cache/app-work/org.example.wizard@1.0.0";

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
    (format!("http://{address}/dist/wizard-src.tar.gz"), hits)
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

/// The wizard package: a license, a feature tree with an exclusive group,
/// two profiles, typed parameters, a sized download, an icon, and a recipe
/// that sleeps while `SLEEP` exists.
fn package(root: &Path, url: &str, sha: &str, size: usize) -> PathBuf {
    let package = root.join("registry/org.example/wizard/v1.0.0");
    fs::create_dir_all(package.join("share")).unwrap();
    fs::create_dir_all(package.join("lang")).unwrap();
    let pid = root.join("build.pid");
    fs::write(
        package.join("vibe.toml"),
        format!(
            r#"[package]
name = "wizard"
group = "org.example"
kind = "app"
version = "1.0.0"

[application]
id = "wizard"
icon = "share/wizard.png"

[[installer_options]]
id = "main"
name = "Main"
platform = "linux"
platform_name = "Linux"
arch = "{arch}"

[installer_options.license]
file = "LICENSE.txt"
name = "Wizard License 1.0"

[[installer_options.features]]
id = "core"
name = "Core"
required = true

[[installer_options.features]]
id = "gui"
name = "GUI"
default = true

[[installer_options.features]]
id = "gtk"
name = "GTK"
parent = "gui"
exclusive_group = "toolkit"
default = true

[[installer_options.features]]
id = "qt"
name = "Qt"
parent = "gui"
exclusive_group = "toolkit"
requires = ["extras"]

[[installer_options.features]]
id = "extras"
name = "Extras"

[[installer_options.profiles]]
id = "typical"
name = "Typical"
features = ["core", "gui"]
default = true

[[installer_options.profiles]]
id = "complete"
name = "Complete"
features = ["core", "gui", "extras"]

[[installer_options.parameters]]
id = "port"
name = "Port"
type = "integer"
default = 8080
min = 1
max = 65535

[[installer_options.parameters]]
id = "lang"
name = "Language"
type = "choice"
default = "en"
choices = [{{ value = "en", name = "English" }}, {{ value = "ru", name = "Russian" }}]

[[installer_options.parameters]]
id = "token"
name = "Token"
type = "string"
required = true
secret = true

[[installer_options.steps]]
id = "fetch"
title = "Fetch the sources"
kind = "sources"
[[installer_options.steps.sources]]
name = "upstream"
url = "{url}"
sha256 = "{sha}"
size = {size}

[[installer_options.steps]]
id = "build"
title = "Build"
kind = "recipe"
interpreter = "posix_sh"
script = "build.sh"

[[installer_options.steps]]
id = "icon"
title = "Copy the icon"
kind = "copy"
from = "share"
to = "share"

[[installer_options.steps]]
id = "lang"
title = "Copy the Russian texts"
kind = "copy"
from = "lang"
to = "share/lang-$param:lang"
when = {{ param_equals = {{ id = "lang", value = "ru" }} }}

[[installer_options.launchers]]
id = "main"
name = "Wizard"
executable = "$app_bin/wizard"
kind = "command"
default = true
"#,
            arch = std::env::consts::ARCH,
        ),
    )
    .unwrap();
    fs::write(
        package.join("build.sh"),
        format!(
            "set -e\necho \"building with port $VIBE_PARAM_PORT\"\n\
             if [ -f \"$VIBE_PACKAGE/SLEEP\" ]; then\n\
             sleep 30 & sleeper=$!\n\
             trap 'kill $sleeper; exit 143' TERM\n\
             echo $$ > '{pid}'\n\
             wait $sleeper\n\
             fi\n\
             mkdir -p \"$VIBE_OUT/bin\"\n\
             cp \"$VIBE_SOURCES/upstream/wizard.sh\" \"$VIBE_OUT/bin/wizard\" && chmod 755 \"$VIBE_OUT/bin/wizard\"\n\
             env | grep -E '^VIBE_(PARAM|FEATURE)_' | sort > \"$VIBE_OUT/env.txt\"\n",
            pid = pid.display()
        ),
    )
    .unwrap();
    fs::write(package.join("LICENSE.txt"), "Use it kindly.\n").unwrap();
    fs::write(package.join("share/wizard.png"), b"\x89PNG").unwrap();
    fs::write(package.join("lang/hello.txt"), "privet").unwrap();
    package
}

struct Fixture {
    root: tempfile::TempDir,
    user: UserScratch,
}

impl Fixture {
    fn command(&self, args: &[&str]) -> Command {
        let mut command = self.user.vibe();
        let registry = self.root.path().join("registry");
        command
            .current_dir(self.root.path())
            .args(args)
            .arg("--registry")
            .arg(registry);
        for variable in ["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
            command.env_remove(variable);
        }
        command.env_remove("VIBE_PROGRESS_FORMAT");
        // A spawnable std command: the fixture signals the running vibe.
        let mut spawnable = Command::new(command.get_program());
        spawnable.args(command.get_args());
        for (key, value) in command.get_envs() {
            match value {
                Some(value) => spawnable.env(key, value),
                None => spawnable.env_remove(key),
            };
        }
        if let Some(dir) = command.get_current_dir() {
            spawnable.current_dir(dir);
        }
        spawnable
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    fn plain(&self, args: &[&str]) -> Output {
        let mut command = self.user.vibe();
        command.current_dir(self.root.path()).args(args);
        command.output().unwrap()
    }
}

fn ok(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// The `{"ok":false,…}` envelope of a refused JSON command, with its exit.
fn refused(output: &Output) -> (Option<i32>, Value) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let envelope = stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value["ok"] == false)
        .unwrap_or_else(|| panic!("no error envelope in: {stderr}"));
    (output.status.code(), envelope)
}

fn events(stderr: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(stderr)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<Value>(line)
                .unwrap_or_else(|error| panic!("stderr line is not JSON ({error}): {line}"))
        })
        .filter(|value| value["protocol"] == "vibe-progress-event/1")
        .collect()
}

fn labels_of(events: &[Value]) -> Vec<String> {
    events
        .iter()
        .filter(|event| event["kind"] == "start")
        .filter_map(|event| event["label"].as_str().map(str::to_owned))
        .collect()
}

/// A step task is labelled `<id> (<kind>)  <title>` (PROP-065 `#display`).
fn step_labels(events: &[Value]) -> Vec<String> {
    events
        .iter()
        .filter(|event| event["kind"] == "start")
        .filter_map(|event| event["label"].as_str())
        .filter(|label| label.contains(" (") && !label.starts_with("Command"))
        .map(|label| label.split(" (").next().unwrap_or_default().to_owned())
        .collect()
}

#[test]
fn a_tool_drives_describe_plan_install_runs_cancel_and_errors() {
    let source = tar_gz("wizard.sh", "#!/bin/sh\necho WIZARD_RAN\n");
    let sha = format!("{:x}", Sha256::digest(&source));
    let size = source.len();
    let (url, hits) = serve(source);
    let fixture = Fixture {
        root: tempfile::tempdir().unwrap(),
        user: UserScratch::new(),
    };
    let package = package(fixture.root.path(), &url, &sha, size);

    // describe: tree, relations, profiles, parameters, license, icon.
    let described = ok(&fixture.run(&["app", "describe", NAME, "--json"]));
    assert_eq!(
        described["application"]["icon"],
        package.join("share/wizard.png").display().to_string()
    );
    let option = &described["options"][0];
    assert_eq!(option["features"][2]["parent"], "gui");
    assert_eq!(option["features"][3]["exclusiveGroup"], "toolkit");
    assert_eq!(
        option["features"][3]["requires"],
        serde_json::json!(["extras"])
    );
    assert_eq!(option["features"][0]["required"], true);
    assert_eq!(
        option["defaultFeatures"],
        serde_json::json!(["core", "gui", "gtk"])
    );
    assert_eq!(option["profiles"][0]["id"], "typical");
    assert_eq!(option["profiles"][0]["default"], true);
    assert_eq!(option["parameters"][0]["type"], "integer");
    assert_eq!(option["parameters"][0]["default"], "8080");
    assert_eq!(option["parameters"][1]["choices"][1]["value"], "ru");
    assert_eq!(option["parameters"][2]["secret"], true);
    assert_eq!(option["license"]["name"], "Wizard License 1.0");
    assert_eq!(option["steps"][0]["sources"][0]["size"], size);

    // plan: the profile, an explicit group member and its requirement.
    let choice = [
        "--profile",
        "typical",
        "--feature",
        "qt",
        "--set",
        "port=9090",
        "--set",
        "lang=ru",
        "--set",
        "token=s3cret",
    ];
    let mut args = vec!["app", "plan", NAME, "--json", "--accept-license"];
    args.extend_from_slice(&choice);
    let plan = ok(&fixture.run(&args));
    assert_eq!(plan["profile"], "typical");
    let chosen: Vec<&str> = plan["features"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|feature| feature["chosen"] == true)
        .map(|feature| feature["id"].as_str().unwrap())
        .collect();
    assert_eq!(chosen, ["core", "gui", "qt", "extras"]);
    assert_eq!(plan["features"][4]["implied"]["rule"], "requires");
    assert_eq!(plan["features"][4]["implied"]["by"], "qt");
    assert_eq!(plan["downloadBytes"], size);
    assert_eq!(plan["unknownSizes"], 0);
    assert_eq!(plan["parameters"][0]["value"], "9090");
    assert_eq!(plan["parameters"][2]["secret"], true);
    assert!(
        plan["parameters"][2].get("value").is_none(),
        "a secret is never shown"
    );
    assert_eq!(plan["license"]["accepted"], true);
    assert_eq!(plan["steps"][3]["action"], "run");
    // The drawing belongs to the text plan alone: `vibe-app-plan/1` keeps
    // its flat step array (PROP-065 `#plan`, PROP-067 `#list`).
    let document = fixture.run(&args);
    assert!(
        !String::from_utf8_lossy(&document.stdout).contains("──"),
        "the JSON plan carries no rule"
    );
    let mut text_args = vec!["app", "plan", NAME, "--accept-license"];
    text_args.extend_from_slice(&choice);
    let text = String::from_utf8(fixture.run(&text_args).stdout).unwrap();
    assert!(
        text.starts_with("── plan for org.example/wizard@1.0.0 ─")
            && text.contains("\n  1.  fetch  (sources)  Fetch the sources\n")
            && text.contains("\n  4.  lang   (copy)     Copy the Russian texts"),
        "the text plan opens with its rule and lists its steps: {text}"
    );

    // plan refusals: exit 2 with a stable code, nothing downloaded.
    let mut met = Vec::new();
    for (extra, code) in [
        (
            vec!["--accept-license", "--set", "token=t", "--feature", "nope"],
            "unknown_feature",
        ),
        (
            vec![
                "--accept-license",
                "--set",
                "token=t",
                "--set",
                "port=99999",
            ],
            "parameter_invalid",
        ),
        (vec!["--set", "token=t"], "license_required"),
        (
            vec![
                "--accept-license",
                "--set",
                "token=t",
                "--no-feature",
                "extras",
                "--feature",
                "qt",
            ],
            "feature_conflict",
        ),
    ] {
        let mut args = vec!["app", "plan", NAME, "--json"];
        args.extend_from_slice(&extra);
        let (status, envelope) = refused(&fixture.run(&args));
        assert_eq!(status, Some(2), "{envelope}");
        assert_eq!(envelope["code"], code, "{envelope}");
        met.push(code);
    }
    assert_eq!(hits.load(Ordering::SeqCst), 0, "planning downloads nothing");

    // install: the event stream on stderr, the envelope on stdout.
    let mut args = vec![
        "install",
        "-g",
        NAME,
        "--json",
        "--progress-format",
        "json",
        "--assume-yes",
        "--accept-license",
        "--profile",
        "typical",
        "--feature",
        "qt",
        "--set",
        "lang=ru",
        "--set",
        "token=s3cret",
    ];
    args.truncate(args.len());
    let output = fixture.run(&args);
    let report = ok(&output);
    assert_eq!(report["installation"], KEY);
    let stream = events(&output.stderr);
    assert_eq!(step_labels(&stream), ["fetch", "build", "icon", "lang"]);
    let fetch_task = stream
        .iter()
        .find(|event| {
            event["kind"] == "start"
                && event["label"]
                    .as_str()
                    .unwrap_or("")
                    .starts_with("fetch (sources)")
        })
        .map(|event| event["task"].clone())
        .unwrap_or_else(|| panic!("no fetch start in {:#?}", labels_of(&stream)));
    assert_eq!(
        labels_of(&stream)[1],
        "fetch (sources) Fetch the sources",
        "a step row names its id, its kind and its title"
    );
    let measured = stream
        .iter()
        .find(|event| event["kind"] == "measurement" && event["task"] == fetch_task)
        .unwrap();
    assert_eq!(
        (measured["completed"].as_u64(), measured["total"].as_u64()),
        (Some(size as u64), Some(size as u64))
    );
    assert_eq!(measured["unit"], "bytes");
    assert!(
        stream.iter().any(|event| event["kind"] == "detail"
            && event["message"]
                .as_str()
                .is_some_and(|text| text.contains("building with port 8080"))),
        "child output is a detail line"
    );
    assert!(
        stream
            .iter()
            .any(|event| event["kind"] == "finish" && event["task"] == fetch_task)
    );
    let seqs: Vec<u64> = stream
        .iter()
        .map(|event| event["seq"].as_u64().unwrap())
        .collect();
    assert!(
        seqs.windows(2).all(|pair| pair[1] == pair[0] + 1),
        "unthrottled, in order"
    );
    assert_eq!(hits.load(Ordering::SeqCst), 1);

    // The record keeps choices except the secret; children saw everything.
    let index: Value = serde_json::from_slice(
        &fs::read(fixture.user.settings.join("applications/index.json")).unwrap(),
    )
    .unwrap();
    let record = &index["installations"][KEY];
    assert_eq!(
        record["parameters"],
        serde_json::json!({ "lang": "ru", "port": "8080" })
    );
    assert_eq!(record["license"]["name"], "Wizard License 1.0");
    assert_eq!(
        record["features"],
        serde_json::json!(["core", "gui", "qt", "extras"])
    );
    let payload = fixture
        .user
        .settings
        .join("opt/apps/wizard/org.example.wizard@1.0.0");
    let env = fs::read_to_string(payload.join("env.txt")).unwrap();
    for line in [
        "VIBE_FEATURE_GTK=0",
        "VIBE_FEATURE_QT=1",
        "VIBE_PARAM_LANG=ru",
        "VIBE_PARAM_PORT=8080",
        "VIBE_PARAM_TOKEN=s3cret",
    ] {
        assert!(env.contains(line), "{line} in {env}");
    }
    assert!(
        payload.join("share/lang-ru/hello.txt").is_file(),
        "$param:lang expanded"
    );
    let listed = ok(&fixture.plain(&["app", "list", "--json"]));
    let copies = fixture.user.settings.join("applications/icons");
    assert_eq!(
        listed["installations"][0]["icon"],
        copies
            .join("org.example.wizard@1.0.0/share/wizard.png")
            .display()
            .to_string()
    );

    // runs: the record with its per-step log.
    let runs = ok(&fixture.plain(&["app", "runs", NAME, "--json"]));
    assert_eq!(runs["protocol"], "vibe-app-runs/1");
    let run = &runs["runs"][0];
    assert_eq!(
        (run["command"].as_str(), run["outcome"].as_str()),
        (Some("install"), Some("done"))
    );
    let log = run["logs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|log| log["step"] == "build")
        .unwrap();
    let text = fs::read_to_string(log["path"].as_str().unwrap()).unwrap();
    assert!(text.contains("building with port 8080"), "{text}");
    assert!(
        Path::new(run["path"].as_str().unwrap())
            .join("plan.json")
            .is_file()
    );

    // cancellation: SIGTERM reaches the child; the step is `interrupted`.
    ok(&fixture.plain(&["uninstall", "-g", NAME, "--json"]));
    fs::write(package.join("SLEEP"), "").unwrap();
    let pid_file = fixture.root.path().join("build.pid");
    let stderr_file = fixture.root.path().join("cancel.stderr");
    let mut vibe = fixture
        .command(&[
            "install",
            "-g",
            NAME,
            "--json",
            "--accept-license",
            "--set",
            "token=t",
        ])
        .stdout(Stdio::null())
        .stderr(fs::File::create(&stderr_file).unwrap())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let child_pid = loop {
        if let Ok(text) = fs::read_to_string(&pid_file)
            && text.ends_with('\n')
        {
            break text.trim().to_owned();
        }
        assert!(Instant::now() < deadline, "the recipe never started");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(
        Command::new("kill")
            .args(["-TERM", &vibe.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let status = vibe.wait().unwrap();
    assert_eq!(status.code(), Some(1), "an interrupted step exits 1");
    let alive = Command::new("kill")
        .args(["-0", &child_pid])
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success();
    assert!(!alive, "the child received the signal and died");
    let work = fixture.user.settings.join(WORK);
    let state: Value = serde_json::from_slice(&fs::read(work.join("steps.json")).unwrap()).unwrap();
    assert_eq!(state["steps"][0]["outcome"], "done");
    assert_eq!(state["steps"][1]["outcome"], "interrupted");
    let index: Value = serde_json::from_slice(
        &fs::read(fixture.user.settings.join("applications/index.json")).unwrap(),
    )
    .unwrap();
    assert!(
        index["installations"].get(KEY).is_none(),
        "the index is untouched"
    );
    let (_, envelope) = refused(&Output {
        status,
        stdout: Vec::new(),
        stderr: fs::read(&stderr_file).unwrap(),
    });
    assert_eq!(envelope["code"], "interrupted");
    assert_eq!(envelope["step"], "build");
    assert_eq!(envelope["work"], work.display().to_string());
    assert!(envelope["error"].as_str().unwrap().contains("--resume"));
    met.push("interrupted");
    let runs = ok(&fixture.plain(&["app", "runs", NAME, "--json"]));
    assert_eq!(runs["runs"][0]["outcome"], "interrupted");

    // --resume continues at the interrupted step.
    fs::remove_file(package.join("SLEEP")).unwrap();
    let resumed = ok(&fixture.run(&[
        "install",
        "-g",
        NAME,
        "--json",
        "--accept-license",
        "--set",
        "token=t",
        "--resume",
    ]));
    assert_eq!(resumed["installation"], KEY);
    assert!(payload.join("bin/wizard").is_file());
    assert!(!work.exists());
    let pruned = ok(&fixture.plain(&["app", "runs", NAME, "--json", "--prune", "1"]));
    assert_eq!(pruned["runs"].as_array().unwrap().len(), 1);
    assert!(!pruned["pruned"].as_array().unwrap().is_empty());

    // errors: every code met above, with its exit status.
    let table = ok(&fixture.plain(&["app", "errors", "--json"]));
    assert_eq!(table["protocol"], "vibe-app-errors/1");
    for code in met {
        let row = table["codes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["code"] == code)
            .unwrap_or_else(|| panic!("`{code}` is listed"));
        let exit = if code == "interrupted" { 1 } else { 2 };
        assert_eq!(row["exit"], exit, "{code}");
    }
}
