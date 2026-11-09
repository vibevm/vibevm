//! A loopback stand-in for a Git host that serves both a package
//! organisation and its repository-listing API.
//!
//! One server, two jobs: `GET /orgs/<org>/repos` answers the paged
//! repository listing (optionally only to a bearer token, as
//! `api.gitverse.ru` does), and every other path serves the
//! organisation's bare repositories over dumb HTTP. A mirror can
//! therefore read exactly the url the registry declares, with no
//! redirection and no network.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// The organisation the fixture host holds.
pub const ORG: &str = "vibespecs";

pub fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(cwd)
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=f@example.invalid",
        ])
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// The host's organisation: one bare repository per repository name,
/// prepared for dumb HTTP (`git update-server-info`).
pub struct Org {
    pub dir: PathBuf,
    work: PathBuf,
}

impl Org {
    pub fn new(root: &Path) -> Self {
        let dir = root.join("org");
        let work = root.join("work");
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir_all(&work).unwrap();
        Org { dir, work }
    }

    /// A repository `<repo>.git` with one commit per version, tagged
    /// `v<version>`. `manifest` writes a package `vibe.toml`; without it
    /// the repository is tagged but carries no manifest.
    pub fn repository(&self, repo: &str, versions: &[&str], manifest: Option<(&str, &str)>) {
        let bare = self.dir.join(format!("{repo}.git"));
        git(
            &self.dir,
            &["init", "-q", "--bare", "-b", "main", bare.to_str().unwrap()],
        );
        let work = self.work.join(repo);
        git(
            &self.work,
            &[
                "clone",
                "-q",
                bare.to_str().unwrap(),
                work.to_str().unwrap(),
            ],
        );
        // A base commit, so a repository with no version still has a
        // branch to push (an untagged organisation repository).
        fs::write(work.join("README"), format!("{repo}\n")).unwrap();
        git(&work, &["add", "-A"]);
        git(&work, &["commit", "-q", "-m", "base"]);
        for version in versions {
            if let Some((group, name)) = manifest {
                fs::write(
                    work.join("vibe.toml"),
                    format!(
                        "[package]\ngroup='{group}'\nname='{name}'\nkind='feat'\n\
                         version='{version}'\ndescription='The {name} fixture package'\n"
                    ),
                )
                .unwrap();
            }
            fs::write(work.join("payload"), format!("{repo} {version}\n")).unwrap();
            git(&work, &["add", "-A"]);
            git(&work, &["commit", "-q", "--allow-empty", "-m", version]);
            git(&work, &["tag", "-f", &format!("v{version}")]);
        }
        git(
            &work,
            &["push", "-q", "-f", "origin", "HEAD:refs/heads/main"],
        );
        if !versions.is_empty() {
            git(&work, &["push", "-q", "-f", "origin", "--tags"]);
        }
        // Dumb HTTP reads `info/refs` and `objects/info/packs`.
        git(&bare, &["update-server-info"]);
    }
}

/// The fixture host on loopback: the organisation-repository API plus the
/// organisation's repositories over dumb HTTP.
pub struct Host {
    /// `http://127.0.0.1:<port>` — no trailing slash.
    pub url: String,
    /// Every recorded `/orgs/<org>/repos` request head, lower-cased.
    listings: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Host {
    /// Serve `pages` for `/orgs/<org>/repos` and `org`'s repositories
    /// under `/<org>/`. With `needs_token`, a request to the listing
    /// without an `Authorization` header is answered 401 — as
    /// `api.gitverse.ru` answers every anonymous request.
    pub fn new(org: &Org, pages: Vec<Vec<String>>, needs_token: Option<&str>) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let listings = Arc::new(Mutex::new(Vec::new()));
        let record = Arc::clone(&listings);
        let stop = Arc::new(AtomicBool::new(false));
        let done = Arc::clone(&stop);
        let root = org.dir.clone();
        let needs_token = needs_token.map(str::to_owned);
        let worker = thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        socket.set_nonblocking(false).unwrap();
                        socket
                            .set_read_timeout(Some(Duration::from_millis(500)))
                            .unwrap();
                        let mut head = Vec::new();
                        let mut byte = [0_u8; 1];
                        while !head.ends_with(b"\r\n\r\n")
                            && socket.read(&mut byte).unwrap_or(0) == 1
                        {
                            head.push(byte[0]);
                        }
                        let head = String::from_utf8_lossy(&head).into_owned();
                        let (status, body) =
                            answer(&head, &root, &pages, needs_token.as_deref(), &record);
                        let _ = write!(
                            socket,
                            "HTTP/1.1 {status}\r\nContent-Type: application/octet-stream\r\n\
                             Content-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = socket.write_all(&body);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            }
        });
        Host {
            url,
            listings,
            stop,
            worker: Some(worker),
        }
    }

    /// The organisation root the registry declares.
    pub fn registry_url(&self) -> String {
        format!("{}/{ORG}", self.url)
    }

    pub fn listings(&self) -> Vec<String> {
        self.listings.lock().unwrap().clone()
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// The status and body one request head deserves.
fn answer(
    head: &str,
    root: &Path,
    pages: &[Vec<String>],
    needs_token: Option<&str>,
    listings: &Mutex<Vec<String>>,
) -> (&'static str, Vec<u8>) {
    const MISSING: (&str, Vec<u8>) = ("404 Not Found", Vec::new());
    let target = head.split_whitespace().nth(1).unwrap_or_default();
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path == format!("/orgs/{ORG}/repos") {
        listings.lock().unwrap().push(head.to_ascii_lowercase());
        return listing(head, query, pages, needs_token);
    }
    // Anything else is a dumb-HTTP Git read of one of the organisation's
    // bare repositories: the path names the file, and the query is
    // ignored — a smart-protocol probe (`info/refs?service=…`) is
    // answered with the plain `info/refs`, whose content type is not the
    // smart advertisement, so Git falls back to the dumb protocol this
    // server speaks (as `common::http_dir` does for the same reason).
    let Some(relative) = path.strip_prefix(&format!("/{ORG}/")) else {
        return MISSING;
    };
    if relative.split('/').any(|segment| segment == "..") {
        return MISSING;
    }
    match fs::read(root.join(relative)) {
        Ok(body) => ("200 OK", body),
        Err(_) => MISSING,
    }
}

/// One page of the organisation's repositories, as both hosts shape it.
fn listing(
    head: &str,
    query: &str,
    pages: &[Vec<String>],
    needs_token: Option<&str>,
) -> (&'static str, Vec<u8>) {
    if let Some(required) = needs_token
        && !head.to_ascii_lowercase().contains(&format!(
            "authorization: bearer {}",
            required.to_ascii_lowercase()
        ))
    {
        return (
            "401 Unauthorized",
            br#"{"error":{"code":"UNAUTHORIZED"}}"#.to_vec(),
        );
    }
    let page: usize = query
        .split('&')
        .find_map(|pair| pair.strip_prefix("page="))
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    let names = pages
        .get(page.saturating_sub(1))
        .cloned()
        .unwrap_or_default();
    let body = names
        .iter()
        .map(|name| format!(r#"{{"name":"{name}","full_name":"{ORG}/{name}"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    ("200 OK", format!("[{body}]").into_bytes())
}
