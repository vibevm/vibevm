//! Transfers (PROP-068 `#acceptance`): a loopback server serves
//! a throttled 16 MiB `[[sources]]` body to `vibe install -g` under
//! `--progress-format json`; the stream carries the transfer on its start,
//! its samples and its final figures; another process lists it active and
//! watches its samples while it runs; the history keeps one entry whose
//! series sums to its bytes; a killed install leaves an active file the
//! next listing turns into an `interrupted` entry; `[transfer] limit` holds
//! the average; and a run that downloads nothing leaves no journal. The
//! git half (`#acceptance-git`): `vibe cache add` from a `git+file://`
//! per-package registry streams the clone as a `reported` transfer of the
//! package's row, its history entry names what the clone left on disk, and
//! a second warm-up, whose fetch moves nothing, adds no transfer.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use common::UserScratch;
use serde_json::Value;
use sha2::{Digest, Sha256};

const KEY: &str = "org.example/blob@1.0.0";
const MIB: usize = 1 << 20;

/// Serve `body` on a loopback port, `chunk` bytes every `pause`.
fn serve(body: Vec<u8>, chunk: usize, pause: Duration) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let body = body.clone();
            std::thread::spawn(move || {
                let mut request = Vec::new();
                let mut buffer = [0u8; 1024];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => return,
                        Ok(read) => request.extend_from_slice(&buffer[..read]),
                    }
                }
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                for part in body.chunks(chunk) {
                    if stream.write_all(part).is_err() {
                        return;
                    }
                    std::thread::sleep(pause);
                }
            });
        }
    });
    format!("http://{address}/dist/blob.bin")
}

fn body(size: usize) -> Vec<u8> {
    (0..size).map(|index| (index % 251) as u8).collect()
}

/// A registry with one package whose `fetch` step names `url` (none when
/// it is empty), and whose `copy` step places the payload.
fn registry(root: &std::path::Path, url: &str, bytes: &[u8]) -> PathBuf {
    let registry = root.join("registry");
    let package = registry.join("org.example/blob/v1.0.0");
    fs::create_dir_all(&package).unwrap();
    let fetch = if url.is_empty() {
        String::new()
    } else {
        format!(
            "[[installer_options.steps]]\nid = \"fetch\"\ntitle = \"Fetch\"\nkind = \"sources\"\n\
             [[installer_options.steps.sources]]\nname = \"blob\"\nurl = \"{url}\"\nsha256 = \"{:x}\"\nsize = {}\n\n",
            Sha256::digest(bytes),
            bytes.len()
        )
    };
    fs::write(
        package.join("vibe.toml"),
        format!(
            "[package]\nname = \"blob\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
             [application]\nid = \"blob\"\n\n[[installer_options]]\nid = \"main\"\nname = \"Main\"\n\
             platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{}\"\n\n{fetch}\
             [[installer_options.steps]]\nid = \"place\"\ntitle = \"Place\"\nkind = \"copy\"\nfrom = \"bin\"\nto = \"bin\"\n\n\
             [[installer_options.launchers]]\nid = \"main\"\nname = \"Blob\"\nexecutable = \"$app_bin/blob\"\nkind = \"command\"\ndefault = true\n",
            std::env::consts::ARCH
        ),
    )
    .unwrap();
    fs::create_dir_all(package.join("bin")).unwrap();
    fs::write(package.join("bin/blob"), "#!/bin/sh\n").unwrap();
    registry
}

/// A plain process rather than the harness's `Command`, which only runs a
/// child to completion: these proofs hold some open while others look.
fn vibe(user: &UserScratch, root: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
    command
        .current_dir(root)
        .env("HOME", root)
        .env("VIBE_SETTINGS", &user.settings)
        .env(vibe_test_support::REGISTRY_CACHE_ENV, &user.cache)
        .env(vibe_test_support::SEARCH_CACHE_ENV, &user.search_cache)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1");
    for variable in [
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
        "VIBE_INVOKED_BY",
    ] {
        command.env_remove(variable);
    }
    command
}

/// The lines of a child's pipe, on a channel.
fn lines(pipe: impl Read + Send + 'static) -> mpsc::Receiver<String> {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            if send.send(line).is_err() {
                break;
            }
        }
    });
    receive
}

/// `vibe install -g` of the fixture under the machine stream, its stderr
/// lines arriving on a channel.
fn install(
    user: &UserScratch,
    root: &std::path::Path,
    registry: &std::path::Path,
) -> (Child, mpsc::Receiver<String>) {
    let mut child = vibe(user, root)
        .args([
            "install",
            "-g",
            KEY,
            "--registry",
            registry.to_str().unwrap(),
        ])
        .args(["--json", "--progress-format", "json"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stderr = lines(child.stderr.take().unwrap());
    (child, stderr)
}

/// Read stream lines until one satisfies `wanted`, keeping every line read.
fn until(
    stream: &mpsc::Receiver<String>,
    kept: &mut Vec<Value>,
    wanted: impl Fn(&Value) -> bool,
) -> Value {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let line = stream
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("the stream ended before the line came");
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        kept.push(value.clone());
        if wanted(&value) {
            return value;
        }
    }
}

fn listing(user: &UserScratch, root: &std::path::Path) -> Value {
    let output = vibe(user, root)
        .args(["transfers", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn millis(at: &Value) -> i64 {
    chrono::DateTime::parse_from_rfc3339(at.as_str().unwrap())
        .unwrap()
        .timestamp_millis()
}

fn history(user: &UserScratch) -> Vec<Value> {
    fs::read_to_string(user.settings.join("transfers/history.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn a_download_is_streamed_listed_watched_and_kept() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let bytes = body(16 * MIB);
    // About 4 MB/s: four seconds in which another process can look.
    let url = serve(bytes.clone(), 64 << 10, Duration::from_millis(15));
    let registry = registry(root.path(), &url, &bytes);
    let (mut child, stream) = install(&user, root.path(), &registry);
    let mut seen = Vec::new();
    let start = until(&stream, &mut seen, |line| {
        line["transfer_record"].is_object()
    });
    let id = start["transfer"].as_str().unwrap().to_owned();
    until(&stream, &mut seen, |line| {
        line["completed"].as_u64() >= Some(2 * MIB as u64)
    });

    // Another process lists it active with a fresh last sample.
    let listed = listing(&user, root.path());
    let active = &listed["active"][0];
    assert_eq!(active["id"], id, "{listed}");
    assert_eq!(active["purpose"], "package-source");
    let age = chrono::Utc::now().timestamp_millis() - millis(&active["last"]["at"]);
    assert!(age <= 500, "the last sample is {age} ms old");

    // A watch that connects now opens with the running transfer, then
    // streams its samples; closing its stdin ends it.
    let mut watch = vibe(&user, root.path())
        .args(["transfers", "watch", "--json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let watched = lines(watch.stdout.take().unwrap());
    let read = |n: usize| -> Vec<Value> {
        (0..n)
            .map(|_| {
                serde_json::from_str(&watched.recv_timeout(Duration::from_secs(10)).unwrap())
                    .unwrap()
            })
            .collect()
    };
    let opening = read(3);
    drop(watch.stdin.take());
    assert!(watch.wait().unwrap().success());
    assert_eq!(opening[0]["event"], "started");
    assert_eq!(opening[0]["id"], id);
    assert!(opening[0]["record"].is_object() && opening[0]["bytes"].as_u64() > Some(0));
    assert!(
        opening[1..]
            .iter()
            .all(|line| line["event"] == "sample" && line["protocol"] == "vibe-transfer-event/1")
    );
    assert!(opening[2]["bytes"].as_u64() >= opening[1]["bytes"].as_u64());
    eprintln!("LISTING {}", serde_json::to_string(&listed).unwrap());
    for line in &opening {
        eprintln!("WATCH {line}");
    }

    let finish = until(&stream, &mut seen, |line| {
        line["kind"] == "finish" && line["transfer"] == id
    });
    assert!(child.wait().unwrap().success());

    let record = &start["transfer_record"];
    assert_eq!(record["purpose"], "package-source");
    assert_eq!(record["package"], KEY);
    assert_eq!(record["step"], "fetch");
    assert_eq!(record["destination"], "cache");
    assert_eq!(record["url"], url.as_str());
    assert!(record.get("pid").is_none() && record.get("command").is_none());
    let samples: Vec<&Value> = seen
        .iter()
        .filter(|line| line["kind"] == "measurement" && line["transfer"] == id)
        .collect();
    assert!(
        samples.len() >= 16,
        "one sample a mebibyte at least: {}",
        samples.len()
    );
    for pair in samples.windows(2) {
        assert!(pair[0]["completed"].as_u64() <= pair[1]["completed"].as_u64());
        let gap = millis(&pair[1]["at"]) - millis(&pair[0]["at"]);
        assert!((0..=600).contains(&gap), "a sample {gap} ms after the last");
        assert!(pair[1]["rate_bps"].is_u64() && pair[1]["unit"] == "bytes");
    }
    assert_eq!(finish["completed"], 16 * MIB);
    assert_eq!(finish["total"], 16 * MIB);
    assert!(finish["peak_bps"].as_u64() > Some(0));
    eprintln!("START {start}");
    for line in samples.iter().skip(1).take(3) {
        eprintln!("MEASUREMENT {line}");
    }
    eprintln!("FINISH {finish}");

    let kept = history(&user);
    assert_eq!(kept.len(), 1);
    let entry = &kept[0];
    assert_eq!(entry["outcome"], "finished");
    assert_eq!(entry["bytes"], 16 * MIB);
    let sum: u64 = entry["series"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["bytes"].as_u64().unwrap())
        .sum();
    assert_eq!(sum, 16 * MIB as u64);
    assert_eq!(entry["command"], "install -g");
    assert!(
        listing(&user, root.path())["active"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_killed_install_becomes_interrupted_on_the_next_listing() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let bytes = body(8 * MIB);
    let url = serve(bytes.clone(), 16 << 10, Duration::from_millis(20));
    let registry = registry(root.path(), &url, &bytes);
    let (mut child, stream) = install(&user, root.path(), &registry);
    until(&stream, &mut Vec::new(), |line| {
        line["completed"].as_u64() > Some(0)
    });
    child.kill().unwrap();
    child.wait().unwrap();
    let active = user.settings.join("transfers/active");
    let json = |dir: &std::path::Path| {
        fs::read_dir(dir)
            .unwrap()
            .filter(|entry| {
                let path = entry.as_ref().unwrap().path();
                path.extension().is_some_and(|ext| ext == "json")
            })
            .count()
    };
    assert_eq!(json(&active), 1, "the active file stays behind");

    let listed = listing(&user, root.path());
    assert!(listed["active"].as_array().unwrap().is_empty(), "{listed}");
    let entry = &listed["history"][0];
    assert_eq!(entry["outcome"], "interrupted");
    assert_eq!(entry["resume"], format!("vibe install -g {KEY} --resume"));
    assert_eq!(
        fs::read_dir(&active).unwrap().count(),
        0,
        "the active file and any sibling it was being rewritten through are gone"
    );
}

#[test]
fn the_configured_limit_holds_the_average() {
    let user = UserScratch::new();
    fs::create_dir_all(&user.settings).unwrap();
    fs::write(
        user.settings.join("config.toml"),
        "[transfer]\nlimit = \"2 MB/s\"\n",
    )
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let bytes = body(4 * MIB);
    let url = serve(bytes.clone(), 256 << 10, Duration::ZERO);
    let registry = registry(root.path(), &url, &bytes);
    let (mut child, _stream) = install(&user, root.path(), &registry);
    assert!(child.wait().unwrap().success());
    let entry = &history(&user)[0];
    assert_eq!(entry["limited"], true);
    assert_eq!(entry["limit_bps"], 2_000_000);
    let average = entry["average_bps"].as_f64().unwrap();
    assert!(
        (1_600_000.0..=2_400_000.0).contains(&average),
        "average {average} B/s"
    );
}

#[test]
fn a_run_that_downloads_nothing_leaves_no_journal() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let registry = registry(root.path(), "", b"");
    let (mut child, _stream) = install(&user, root.path(), &registry);
    assert!(child.wait().unwrap().success());
    assert!(!user.settings.join("transfers").exists());
    let listed = listing(&user, root.path());
    assert!(listed["history"].as_array().unwrap().is_empty());
    assert!(
        !user.settings.join("transfers").exists(),
        "a listing creates nothing"
    );
}

/// A per-package registry under `root` (`<group>.<name>.git`, the default
/// `fqdn` naming) carrying `org.example/blob@0.1.0` with a few hundred KiB
/// of incompressible bytes, so git measures a throughput and names a size.
fn git_registry(root: &std::path::Path) -> String {
    let source = root.join("blob-source");
    let spec = source.join(common::spec_rel("flows/blob"));
    fs::create_dir_all(&spec).unwrap();
    common::run_git(&source, &["init", "--initial-branch=main"]);
    fs::write(
        source.join("vibe.toml"),
        "[package]\ngroup = \"org.example\"\nname = \"blob\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(spec.join("BLOB.md"), "# blob\n").unwrap();
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    let noise: Vec<u8> = (0..400_000)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect();
    fs::write(spec.join("noise.bin"), noise).unwrap();
    common::run_git(&source, &["add", "-A"]);
    common::run_git(
        &source,
        &[
            "-c",
            "user.email=f@example.test",
            "-c",
            "user.name=F",
            "commit",
            "-m",
            "blob",
        ],
    );
    common::run_git(&source, &["tag", "v0.1.0"]);
    let org = root.join("org");
    fs::create_dir_all(&org).unwrap();
    let bare = org.join("org.example.blob.git");
    common::run_git(
        root,
        &[
            "clone",
            "--bare",
            source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    format!("git+file://{}", org.to_string_lossy())
}

/// `vibe cache add` of the git fixture under the machine stream: its lines.
fn warm(user: &UserScratch, root: &std::path::Path, project: &std::path::Path) -> Vec<Value> {
    let output = vibe(user, root)
        .args(["cache", "add", "org.example/blob@0.1.0", "--path"])
        .arg(project)
        .args(["--json", "--progress-format", "json"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    stderr
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[test]
fn a_package_clone_is_a_reported_transfer_of_its_row() {
    if !common::git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let registry = git_registry(root.path());
    let project = root.path().join("project");
    fs::create_dir_all(&project).unwrap();
    common::write_project_with_per_package_registry(&project, &registry);

    let seen = warm(&user, root.path(), &project);
    let start = seen
        .iter()
        .find(|line| line["transfer_record"]["kind"] == "git")
        .unwrap_or_else(|| panic!("no git transfer started: {seen:?}"));
    let id = start["transfer"].as_str().unwrap();
    let record = &start["transfer_record"];
    assert_eq!(record["precision"], "reported");
    assert_eq!(record["purpose"], "package-clone");
    assert_eq!(record["package"], "org.example/blob");
    assert_eq!(record["destination"], "store");
    assert_eq!(record["limited"], false);
    assert!(
        record["url"]
            .as_str()
            .unwrap()
            .ends_with("org.example.blob.git")
    );
    // The transfer is a child of the package's own row.
    let parent = seen
        .iter()
        .find(|line| line["kind"] == "start" && line["task"] == start["parent"])
        .expect("the transfer's parent task is on the stream");
    assert!(
        parent["label"]
            .as_str()
            .unwrap()
            .contains("org.example/blob@0.1.0")
    );
    let measurements: Vec<&Value> = seen
        .iter()
        .filter(|line| line["kind"] == "measurement" && line["transfer"] == id)
        .collect();
    assert!(measurements.iter().all(|line| line["rate_bps"].is_u64()));
    assert!(
        measurements
            .iter()
            .any(|line| line["completed"].as_u64() > Some(0))
    );
    let finish = seen
        .iter()
        .find(|line| line["kind"] == "finish" && line["transfer"] == id)
        .expect("the transfer finished on the stream");
    assert!(finish["completed"].as_u64() > Some(100_000), "{finish}");
    assert_eq!(finish["total"], finish["completed"]);
    eprintln!("GIT START {start}");
    for line in measurements.iter().take(2) {
        eprintln!("GIT MEASUREMENT {line}");
    }
    eprintln!("GIT FINISH {finish}");

    let kept = history(&user);
    let entry = kept
        .iter()
        .find(|entry| entry["id"] == id)
        .expect("the clone is journaled");
    assert_eq!(entry["kind"], "git");
    assert_eq!(entry["precision"], "reported");
    assert_eq!(entry["outcome"], "finished");
    assert_eq!(entry["command"], "cache add");
    assert_eq!(entry["bytes"], finish["completed"]);
    assert!(entry["written"].as_u64() > Some(0), "{entry}");
    let clones = |kept: &[Value]| kept.iter().filter(|entry| entry["kind"] == "git").count();
    assert_eq!(clones(&kept), 1);

    // The clone is there and up to date: a second warm-up moves nothing.
    let again = warm(&user, root.path(), &project);
    assert!(again.iter().all(|line| line["transfer_record"].is_null()));
    assert_eq!(clones(&history(&user)), 1);
}
