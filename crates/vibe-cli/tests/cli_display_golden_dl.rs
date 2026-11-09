//! Golden captures of `vibe dl` on a real terminal (PROP-067 `#acceptance`,
//! PROP-068 `#dl`): the response head as a table, a successful download, an
//! untrusted certificate and a proxy that wants authentication. Each run
//! is taken under a pseudo-terminal (`pty-runner.c`), its attributes
//! stripped and its volatile parts (ports, paths, milliseconds, times)
//! normalised. `--no-progress` keeps the transfer bar's animation out of
//! the capture; the bar itself is the shared one every download draws.

#![cfg(target_os = "linux")]

mod common;
mod display_support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::UserScratch;
use common::net_fixture::{ConnectProxy, FIXTURE_HOST, HttpsServer, TestCa};
use display_support::{golden, normalise};

const PTY_RUNNER_SOURCE: &str = include_str!("common/pty-runner.c");

fn pty_runner(scope: &Path) -> PathBuf {
    let source = scope.join("pty-runner.c");
    let binary = scope.join("pty-runner");
    fs::write(&source, PTY_RUNNER_SOURCE).unwrap();
    let output = Command::new("cc")
        .args(["-O0", "-o"])
        .arg(&binary)
        .arg(&source)
        .arg("-lutil")
        .output()
        .unwrap();
    assert!(output.status.success(), "cc failed");
    binary
}

/// Attributes, carriage returns, ports and milliseconds out of a capture.
fn clean(raw: &[u8], paths: &[(&Path, &str)]) -> String {
    let text = String::from_utf8_lossy(raw).replace("\r\n", "\n");
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        if c != '\r' {
            out.push(c);
        }
    }
    let mut text = normalise(&out, paths);
    for host in ["127.0.0.1:", &format!("{FIXTURE_HOST}:")] {
        let mut result = String::new();
        let mut rest = text.as_str();
        while let Some(at) = rest.find(host) {
            result.push_str(&rest[..at + host.len()]);
            rest = &rest[at + host.len()..];
            let digits = rest.chars().take_while(char::is_ascii_digit).count();
            if digits > 0 {
                result.push_str("<port>");
            }
            rest = &rest[digits..];
        }
        result.push_str(rest);
        text = result;
    }
    text.split(" · ")
        .map(|part| {
            let trimmed = part.trim_end();
            match trimmed.strip_suffix(" ms") {
                Some(n) if n.chars().all(|c| c.is_ascii_digit()) => {
                    format!("<ms>{}", &part[trimmed.len()..])
                }
                _ => part.to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join(" · ")
        .lines()
        .map(|line| {
            // A trailing `· N ms` at the end of a row.
            match line.rsplit_once(" · ") {
                Some((head, tail))
                    if tail.ends_with(" ms")
                        && tail[..tail.len() - 3].chars().all(|c| c.is_ascii_digit()) =>
                {
                    format!("{head} · <ms>")
                }
                _ => line.to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

struct Golden {
    dir: tempfile::TempDir,
    ca: TestCa,
    user: UserScratch,
    runner: PathBuf,
}

impl Golden {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let ca = TestCa::new(dir.path());
        let root = dir.path().join("root");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.txt"), b"hello\n").unwrap();
        let user = UserScratch::new();
        fs::create_dir_all(&user.settings).unwrap();
        fs::write(
            user.settings.join("config.toml"),
            format!("[proxy]\nca_file = '{}'\n", ca.ca_pem.display()),
        )
        .unwrap();
        let runner = pty_runner(dir.path());
        Self {
            dir,
            ca,
            user,
            runner,
        }
    }

    fn run(&self, args: &[&str]) -> String {
        let home = self.dir.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let output = Command::new(&self.runner)
            .arg(env!("CARGO_BIN_EXE_vibe"))
            .arg("--no-progress")
            .arg("dl")
            .args(args)
            .current_dir(self.dir.path())
            .env("VIBE_SETTINGS", &self.user.settings)
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("HOME", &home)
            .env("TERM", "xterm-256color")
            .env_remove("NO_COLOR")
            .env_remove("HTTPS_PROXY")
            .env_remove("VIBEVM_PROXY_MODE")
            .output()
            .unwrap();
        clean(
            &output.stdout,
            &[
                (self.dir.path(), "<dir>"),
                (&self.user.settings, "<settings>"),
                (&self.ca.ca_pem, "<ca>"),
            ],
        )
    }
}

#[test]
fn the_response_head_is_a_table_on_a_terminal() {
    let g = Golden::new();
    let server = HttpsServer::start(&g.ca, &g.dir.path().join("root"));
    golden(
        "dl-headers",
        &g.run(&["--headers", &server.url("127.0.0.1", "a.txt")]),
    );
}

#[test]
fn a_successful_download_reads_as_its_golden() {
    let g = Golden::new();
    let server = HttpsServer::start(&g.ca, &g.dir.path().join("root"));
    let dest = g.dir.path().join("a-copy.txt");
    golden(
        "dl-success",
        &g.run(&[&server.url("127.0.0.1", "a.txt"), dest.to_str().unwrap()]),
    );
}

#[test]
fn an_untrusted_certificate_reads_as_its_golden() {
    let g = Golden::new();
    let other = TestCa::untrusted(g.dir.path());
    let server = HttpsServer::start(&other, &g.dir.path().join("root"));
    golden(
        "dl-untrusted",
        &g.run(&["--show", &server.url("127.0.0.1", "a.txt")]),
    );
}

#[test]
fn a_proxy_that_wants_authentication_reads_as_its_golden() {
    let g = Golden::new();
    let server = HttpsServer::start(&g.ca, &g.dir.path().join("root"));
    let proxy = ConnectProxy::start_requiring_auth();
    let mut text = g.run(&[
        "--proxy",
        &proxy.url(),
        "--show",
        &server.url(FIXTURE_HOST, "a.txt"),
    ]);
    text = text.replace(&proxy.url(), "http://127.0.0.1:<port>");
    golden("dl-proxy-407", &text);
}

#[test]
fn a_binary_body_is_refused_on_a_terminal_and_printed_with_force() {
    let g = Golden::new();
    let root = g.dir.path().join("root");
    fs::write(root.join("blob.bin"), [0u8, 159, 146, 150, 0, 1, 2]).unwrap();
    let server = HttpsServer::start(&g.ca, &root);
    let url = server.url("127.0.0.1", "blob.bin");
    let home = g.dir.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let run = |extra: &[&str]| {
        Command::new(&g.runner)
            .arg(env!("CARGO_BIN_EXE_vibe"))
            .args(["--no-progress", "dl", "--show"])
            .args(extra)
            .arg(&url)
            .current_dir(g.dir.path())
            .env("VIBE_SETTINGS", &g.user.settings)
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("HOME", &home)
            .env("TERM", "xterm-256color")
            .output()
            .unwrap()
    };
    let refused = run(&[]);
    assert_eq!(refused.status.code(), Some(19));
    let text = String::from_utf8_lossy(&refused.stdout);
    assert!(text.contains("dl_binary_terminal"), "{text}");
    let forced = run(&["--force"]);
    assert_eq!(forced.status.code(), Some(0));
}
