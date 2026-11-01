//! `PROP-068#coverage`: every place the workspace reads an HTTP response
//! or runs `git clone|fetch` goes through the transfer wrapper or is named
//! here with the reason it does not. A new reader fails this test until it
//! is wrapped or listed; a listed one that became wrapped, or went away,
//! fails it too, so the list never outlives what it excuses.
//!
//! The scan reads the sources as text. An HTTP reader is a file that sends
//! a `reqwest` request (every response it gets has a body it may read) or
//! begins a counted transfer; a git reader is a file whose argument list
//! starts with `"clone"` or `"fetch"`. Wrapped means the file names a
//! `TransferRecord` (HTTP) or runs git through `run_transferring` (git).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Reader {
    Http,
    Git,
}

/// The readers that are not transfers, one line of why each.
const EXEMPT: &[(&str, Reader, &str)] = &[
    (
        "vibe-cli/src/commands/aiui/control.rs",
        Reader::Http,
        "the aiui control client: JSON replies of the local terminal, not a download",
    ),
    (
        "vibe-cli/src/commands/doc/shell/fetch.rs",
        Reader::Http,
        "the doc shell's fetch of the reader's page shell, asked of the operator each time (PROP-057)",
    ),
    (
        "vibe-cli/src/commands/remap/probe.rs",
        Reader::Http,
        "the remap probe's HEAD requests: no body is read",
    ),
    (
        "vibe-cli/src/commands/vvm/git.rs",
        Reader::Git,
        "the VVM build pipeline's checkout of vibe's own source (PROP-019), not a package",
    ),
    (
        "vibe-index/src/scanner/from_github.rs",
        Reader::Http,
        "the index scanner of vibe-index, which runs on a server",
    ),
    (
        "vibe-index/src/scanner/from_github.rs",
        Reader::Git,
        "the index scanner of vibe-index, which runs on a server",
    ),
    (
        "vibe-index/src/scanner/git_cli.rs",
        Reader::Git,
        "the index scanner of vibe-index, which runs on a server",
    ),
    (
        "vibe-llm/src/transport.rs",
        Reader::Http,
        "the LLM transport: a model's reply is not a download",
    ),
    (
        "vibe-publish/src/git_publish.rs",
        Reader::Git,
        "the publish side, which uploads: it fetches only the branch it pushes onto",
    ),
    (
        "vibe-publish/src/github.rs",
        Reader::Http,
        "the publish side, which uploads: the bodies are the host's API replies",
    ),
    (
        "vibe-publish/src/github_release.rs",
        Reader::Http,
        "the publish side, which uploads: the bodies are the host's API replies",
    ),
    (
        "vibe-publish/src/gitverse.rs",
        Reader::Http,
        "the publish side, which uploads: the bodies are the host's API replies",
    ),
    (
        "vibe-publish/src/post_hook.rs",
        Reader::Http,
        "the publish side's post hook: the reply to a notification it sends",
    ),
    (
        "vibe-registry/src/git_backend/shell/raw_http.rs",
        Reader::Http,
        "one manifest under a kilobyte in place of `git archive`, dozens per resolution walk",
    ),
    (
        "vibe-registry/src/index_client/handshake.rs",
        Reader::Http,
        "the index's hello.json handshake, read before any catalog file",
    ),
    (
        "vibe-registry/src/index_client/mod.rs",
        Reader::Http,
        "the index server's per-package lookup replies; the catalog files are wrapped",
    ),
    (
        "vibe-registry/src/search/full_scan.rs",
        Reader::Http,
        "`vibe search --full-scan`: a host's listing read for a search, not a download",
    ),
];

fn crates_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate lives under crates/")
        .to_path_buf()
}

/// Whether `relative` is test code, which drives fixtures, not downloads,
/// or the wrapper itself.
fn skipped(relative: &str) -> bool {
    let name = relative.rsplit('/').next().unwrap_or(relative);
    relative.starts_with("vibe-core/src/progress/transfer")
        || relative.contains("/tests/")
        || name == "tests.rs"
        || name == "test_support.rs"
        || name.ends_with("_tests.rs")
        || name.starts_with("tests_")
}

fn sources(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

/// Whether an argument list in `text` starts with `"clone"` or `"fetch"`.
fn runs_clone_or_fetch(text: &str) -> bool {
    ["\"clone\"", "\"fetch\""].iter().any(|word| {
        text.match_indices(word)
            .any(|(at, _)| text[..at].trim_end().ends_with('['))
    })
}

/// Every reader of the workspace and whether it is wrapped.
fn readers() -> BTreeMap<(String, Reader), bool> {
    let root = crates_root();
    let mut found = BTreeMap::new();
    let Ok(crates) = std::fs::read_dir(&root) else {
        return found;
    };
    for krate in crates.flatten() {
        let mut files = Vec::new();
        sources(&krate.path().join("src"), &mut files);
        for file in files {
            let relative = file
                .strip_prefix(&root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            if skipped(&relative) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let sends = text.contains("reqwest")
                && (text.contains(".send()") || text.contains(".execute("));
            if sends || text.contains("TransferRecord::http") {
                found.insert(
                    (relative.clone(), Reader::Http),
                    text.contains("TransferRecord"),
                );
            }
            if runs_clone_or_fetch(&text) {
                found.insert((relative, Reader::Git), text.contains("run_transferring("));
            }
        }
    }
    found
}

#[test]
fn every_download_is_a_transfer_or_named_with_its_reason() {
    let found = readers();
    assert!(
        found.values().any(|wrapped| *wrapped),
        "the scan found no wrapped reader at all; it is reading the wrong tree"
    );
    let mut problems = Vec::new();
    for ((file, reader), wrapped) in &found {
        let listed = EXEMPT
            .iter()
            .any(|(path, kind, _)| path == file && kind == reader);
        match (wrapped, listed) {
            (false, false) => problems.push(format!(
                "{file} ({reader:?}) reads bytes outside the transfer wrapper: wrap it or list it"
            )),
            (true, true) => problems.push(format!(
                "{file} ({reader:?}) is wrapped now: take it off the list"
            )),
            _ => {}
        }
    }
    for (path, kind, reason) in EXEMPT {
        assert!(!reason.is_empty(), "{path} is listed without a reason");
        if !found.contains_key(&((*path).to_owned(), *kind)) {
            problems.push(format!(
                "{path} ({kind:?}) is listed but no longer reads: take it off the list"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_package_clones_and_the_downloads_are_wrapped() {
    let found = readers();
    for wrapped in [
        (
            "vibe-registry/src/git_backend/shell/checkout.rs",
            Reader::Git,
        ),
        ("vibe-registry/src/index_client/catalog.rs", Reader::Http),
        ("vibe-app/src/installer/fetch.rs", Reader::Http),
        ("vibe-cli/src/commands/vvm/bundle/download.rs", Reader::Http),
    ] {
        assert_eq!(
            found.get(&(wrapped.0.to_owned(), wrapped.1)),
            Some(&true),
            "{wrapped:?}"
        );
    }
}
