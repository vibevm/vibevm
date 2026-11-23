//! Human keyring metadata contract with isolated synthetic settings.
#![cfg(unix)]

use std::{
    io::Write,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Stdio},
};

const SENTINEL: &str = "SYNTHETIC_HUMAN_SECRET_DO_NOT_PRINT";
const ROTATED: &str = "SYNTHETIC_ROTATED_VALUE_DO_NOT_PRINT";
const SPEC: &str = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING";

struct Fixture(tempfile::TempDir);

impl Fixture {
    fn new() -> Self {
        Self(
            tempfile::Builder::new()
                .permissions(std::fs::Permissions::from_mode(0o700))
                .tempdir()
                .unwrap(),
        )
    }

    fn run(&self, args: &[&str], input: Option<&[u8]>) -> std::process::Output {
        self.run_term(args, input, "xterm")
    }

    fn run_term(&self, args: &[&str], input: Option<&[u8]>, term: &str) -> std::process::Output {
        let out = self.attempt_term(args, input, term);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }

    fn command(&self, term: &str) -> Command {
        let settings: &Path = self.0.path();
        let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
        command
            .env_clear()
            .env("HOME", settings)
            .env("VIBE_SETTINGS", settings)
            .env("PATH", "/usr/bin:/bin")
            .env("TERM", term)
            .env("NO_COLOR", "1")
            .env("COLUMNS", "80")
            .args(["secrets"]);
        command
    }

    fn attempt(&self, args: &[&str], input: Option<&[u8]>) -> std::process::Output {
        self.attempt_term(args, input, "xterm")
    }

    fn attempt_term(
        &self,
        args: &[&str],
        input: Option<&[u8]>,
        term: &str,
    ) -> std::process::Output {
        let mut child = self
            .command(term)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(bytes) = input {
            child.stdin.take().unwrap().write_all(bytes).unwrap();
        } else {
            drop(child.stdin.take());
        }
        let out = child.wait_with_output().unwrap();
        no_values(&out);
        out
    }

    fn state(&self) -> serde_json::Value {
        serde_json::from_slice(&self.run(&["list", "--json"], None).stdout).unwrap()
    }

    fn disk_state(&self) -> Vec<(std::path::PathBuf, u32, Vec<u8>)> {
        fn visit(root: &Path, path: &Path, rows: &mut Vec<(std::path::PathBuf, u32, Vec<u8>)>) {
            if !path.exists() {
                return;
            }
            let metadata = std::fs::symlink_metadata(path).unwrap();
            let bytes = if metadata.is_file() {
                std::fs::read(path).unwrap()
            } else {
                Vec::new()
            };
            rows.push((
                path.strip_prefix(root).unwrap().to_owned(),
                metadata.permissions().mode(),
                bytes,
            ));
            if metadata.is_dir() {
                for entry in std::fs::read_dir(path).unwrap() {
                    visit(root, &entry.unwrap().path(), rows);
                }
            }
        }
        let mut rows = Vec::new();
        visit(self.0.path(), &self.0.path().join("secrets"), &mut rows);
        rows.sort();
        rows
    }
}

fn no_values(out: &std::process::Output) {
    for stream in [&out.stdout, &out.stderr] {
        let text = String::from_utf8_lossy(stream);
        for value in [SENTINEL, ROTATED] {
            assert!(!text.contains(value), "protected synthetic value leaked");
        }
    }
}

fn trailer(out: &std::process::Output, expected: &str) {
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<_> = text.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(lines.len(), 1, "{text}");
    let elapsed = lines[0]
        .strip_prefix(&format!("✻ {expected} · "))
        .unwrap_or_else(|| panic!("{text}"));
    assert!(
        elapsed.ends_with('s') && elapsed.chars().any(|c| c.is_ascii_digit()),
        "{text}"
    );
    assert!(out.stderr.is_empty());
}

#[test]
fn secrets_writes_end_with_a_trailer() {
    for mode in ["--json", "--plain", "--quiet"] {
        let fixture = Fixture::new();
        let created = fixture.run(
            &[
                "create",
                "--kind",
                "api-key",
                "--label",
                "Synthetic",
                "--value-stdin",
                mode,
            ],
            Some(SENTINEL.as_bytes()),
        );
        let first = fixture.state();
        let id = first["items"][0]["id"].as_str().unwrap();
        let current = first["items"][0]["current_revision"].as_str().unwrap();
        assert_eq!(first["store_revision"], 1);
        if mode == "--json" {
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&created.stdout).unwrap(),
                first
            );
            assert_eq!(first["protocol"], "vibe-secrets/1");
            assert_eq!(first["ok"], true);
        } else {
            trailer(
                &created,
                &format!("created keyring item Synthetic · {id} · store revision 1"),
            );
        }
        let renamed = fixture.run(
            &[
                "edit",
                id,
                "--label",
                "Renamed",
                "--expect-store",
                "1",
                "--expect-current",
                current,
                mode,
            ],
            None,
        );
        let second = fixture.state();
        assert_eq!(second["store_revision"], 2);
        assert_eq!(second["items"][0]["label"], "Renamed");
        assert_eq!(second["items"][0]["current_revision"], current);
        if mode == "--json" {
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&renamed.stdout).unwrap(),
                second
            );
        } else {
            trailer(
                &renamed,
                &format!("renamed {id} to Renamed · store revision 2"),
            );
        }
        let rotated = fixture.run(
            &[
                "rotate",
                id,
                "--expect-current",
                current,
                "--value-stdin",
                mode,
            ],
            Some(ROTATED.as_bytes()),
        );
        let third = fixture.state();
        let revision = third["items"][0]["current_revision"].as_str().unwrap();
        assert_ne!(revision, current);
        assert_eq!(third["store_revision"], 3);
        assert_eq!(third["items"][0]["id"], id);
        if mode == "--json" {
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&rotated.stdout).unwrap(),
                third
            );
        } else {
            trailer(
                &rotated,
                &format!("rotated the value of {id} · revision {revision} · store revision 3"),
            );
        }
        let before = fixture.disk_state();
        let refused = fixture.attempt(
            &[
                "remove",
                id,
                "--expect-store",
                "3",
                "--expect-current",
                revision,
            ],
            None,
        );
        refusal(
            &refused,
            "consent_required",
            &format!("vibe secrets remove {id} --expect-store 3 --expect-current {revision} --yes"),
        );
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert!(
            stderr.contains(&format!(
                "✗ consent_required  removing keyring item {id} needs --yes"
            )),
            "{stderr}"
        );
        assert!(
            stderr.contains("  why   removal is logical; the retained bytes are not erased"),
            "{stderr}"
        );
        assert!(
            before == fixture.disk_state(),
            "consent refusal mutated private state"
        );
        assert_eq!(fixture.state(), third);
        let removed = fixture.run(
            &[
                "remove",
                id,
                "--expect-store",
                "3",
                "--expect-current",
                revision,
                "--yes",
                mode,
            ],
            None,
        );
        let fourth = fixture.state();
        assert_eq!(fourth["store_revision"], 4);
        assert_eq!(fourth["items"], serde_json::json!([]));
        if mode == "--json" {
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&removed.stdout).unwrap(),
                fourth
            );
        } else {
            trailer(
                &removed,
                &format!(
                    "removed keyring item {id} · retained bytes not erased · store revision 4"
                ),
            );
        }
    }
}

fn refusal(out: &std::process::Output, code: &str, fix: &str) {
    no_values(out);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.starts_with(&format!("✗ {code}  ")), "{stderr}");
    assert!(stderr.contains(&format!("  fix   {fix}")), "{stderr}");
    assert!(stderr.contains(&format!("  see   {SPEC}")), "{stderr}");
}

#[test]
fn human_list_details_and_references_render_measured_rows() {
    let fixture = Fixture::new();
    let empty = fixture.run(&["list"], None);
    assert!(
        String::from_utf8_lossy(&empty.stdout)
            .contains("The local keyring is empty. Add an item with")
    );

    let created = fixture.run(
        &[
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
            "--json",
        ],
        Some(SENTINEL.as_bytes()),
    );
    let created: serde_json::Value = serde_json::from_slice(&created.stdout).unwrap();
    let id = created["items"][0]["id"].as_str().unwrap();

    let details = fixture.run(&["details", id], None);
    let details = String::from_utf8_lossy(&details.stdout);
    for expected in [
        "store revision 1",
        id,
        "Synthetic",
        "api-key",
        "[value hidden]",
    ] {
        assert!(details.contains(expected), "missing {expected}: {details}");
    }
    assert!(!details.contains("\"protocol\""));

    let list = fixture.run(&["list"], None);
    let list = String::from_utf8_lossy(&list.stdout);
    assert!(list.contains(id));
    assert!(list.contains("[value hidden]"));

    let quiet = fixture.run(&["list", "--quiet"], None);
    assert_eq!(
        String::from_utf8_lossy(&quiet.stdout),
        "1 keyring item · store revision 1\n"
    );

    let references = fixture.run(&["references", id], None);
    let references = String::from_utf8_lossy(&references.stdout);
    assert!(references.contains("No registered consumer references this item."));
    assert!(
        references.contains(&format!("consumers of {id}")),
        "{references}"
    );
    assert!(!references.contains("\"protocol\""));

    let json = fixture.run(&["details", id, "--json"], None);
    let json: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(json["protocol"], "vibe-secrets/1");
    assert_eq!(json, created);
}

#[test]
#[cfg(target_os = "linux")]
fn secrets_writes_input_refusals() {
    for (case, code, fix) in [
        ("empty", "value_empty", "pass a nonempty value"),
        (
            "file",
            "value_file_unsafe",
            "use a 0600 file you own in a 0700 directory you own",
        ),
        (
            "directory",
            "value_input_unreadable",
            "pipe the value in again",
        ),
        (
            "unattended",
            "attended_terminal_required",
            "vibe secrets create --kind KIND --label LABEL --value-stdin",
        ),
        (
            "terminal",
            "value_input_terminal",
            "pipe the value in, or pass --value-file PATH",
        ),
    ] {
        let fixture = Fixture::new();
        fixture.run(
            &[
                "create",
                "--kind",
                "api-key",
                "--label",
                "Existing",
                "--value-stdin",
                "--json",
            ],
            Some(SENTINEL.as_bytes()),
        );
        let state = fixture.state();
        let before = fixture.disk_state();
        let args = [
            "create",
            "--kind",
            "api-key",
            "--label",
            "Refused",
            "--value-stdin",
        ];
        let out = match case {
            "empty" => fixture.attempt(&args, Some(b"")),
            "file" => {
                let path = fixture.0.path().join("synthetic-input");
                std::fs::write(&path, SENTINEL).unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
                fixture.attempt(
                    &[
                        "create",
                        "--kind",
                        "api-key",
                        "--label",
                        "Refused",
                        "--value-file",
                        path.to_str().unwrap(),
                    ],
                    None,
                )
            }
            "directory" => fixture
                .command("xterm")
                .args(args)
                .stdin(std::fs::File::open(fixture.0.path()).unwrap())
                .output()
                .unwrap(),
            "unattended" => fixture.attempt(&args[..5], Some(SENTINEL.as_bytes())),
            "terminal" => {
                let source = fixture.0.path().join("pty.c");
                let runner = fixture.0.path().join("pty");
                // Only stdin is a size-initialized PTY; keep diagnostic streams separate.
                let c = include_str!("common/pty-runner.c").replace(
                    "if (dup2(slave, STDIN_FILENO) < 0 ||\n            dup2(slave, STDOUT_FILENO) < 0 ||\n            dup2(slave, STDERR_FILENO) < 0) _exit(125);",
                    "if (dup2(slave, STDIN_FILENO) < 0) _exit(125);");
                std::fs::write(&source, c).unwrap();
                let compile = Command::new("cc")
                    .args(["-O0", "-o"])
                    .arg(&runner)
                    .arg(&source)
                    .arg("-lutil")
                    .output()
                    .unwrap();
                assert!(
                    compile.status.success(),
                    "{}",
                    String::from_utf8_lossy(&compile.stderr)
                );
                Command::new(runner)
                    .arg(env!("CARGO_BIN_EXE_vibe"))
                    .arg("secrets")
                    .args(args)
                    .env_clear()
                    .env("HOME", fixture.0.path())
                    .env("VIBE_SETTINGS", fixture.0.path())
                    .env("PATH", "/usr/bin:/bin")
                    .env("TERM", "xterm")
                    .env("NO_COLOR", "1")
                    .output()
                    .unwrap()
            }
            _ => unreachable!(),
        };
        refusal(&out, code, fix);
        assert!(
            before == fixture.disk_state(),
            "input refusal mutated private state"
        );
        assert_eq!(fixture.state(), state);
    }
}

#[test]
fn secrets_plain_keyring_vocabulary() {
    let fixture = Fixture::new();
    let mut created = serde_json::Value::Null;
    for index in 0..5 {
        let output = fixture.run(
            &[
                "create",
                "--kind",
                "api-key",
                "--label",
                &format!("third-{index}"),
                "--value-stdin",
                "--json",
            ],
            Some(SENTINEL.as_bytes()),
        );
        created = serde_json::from_slice(&output.stdout).unwrap();
    }
    let id = created["items"][0]["id"].as_str().unwrap();
    let revision = created["items"][0]["current_revision"].as_str().unwrap();
    let store_revision = created["store_revision"].as_u64().unwrap();
    assert_eq!(store_revision, 5);
    for (term, stroke, separator, child) in [("xterm", "─", "·", "└"), ("dumb", "-", "*", "`")]
    {
        let output = fixture.run_term(&["list", "--plain"], None, term);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let lines: Vec<_> = stdout.lines().collect();
        assert_eq!(
            lines[0],
            format!(
                "{stroke}{stroke} local keyring {separator} store revision {store_revision} {}",
                stroke.repeat(44)
            ),
        );
        let index = lines.iter().position(|line| line.contains(id)).unwrap();
        assert_eq!(lines[index + 1], format!("  {child} revision {revision}"));
        assert!(!stdout.contains('\u{1b}'), "{stdout}");
    }
}
