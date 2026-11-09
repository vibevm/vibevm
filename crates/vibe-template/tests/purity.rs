//! The purity contract of PROP-075 `#TEMPLATE-RENDER-LOCATION`, as a
//! test rather than a comment: this crate renders and nothing else.
//!
//! Rendering happens at resolution time **on the client, always**, and the
//! renderer is *one* function so that a second implementation cannot
//! disagree with it undetectably. That argument only holds while the
//! renderer has no way to reach a file, a socket, a registry, a cache or a
//! clock — so the dependency table and the crate's own sources are checked
//! here, and a later edit that widens either fails this test.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Everything `Cargo.toml` may name under `[dependencies]`. Adding a row
/// here is a deliberate, reviewed decision about the seam, which is the
/// point of the list.
const ALLOWED: &[&str] = &[
    "specmark",
    "vibe-core",
    "serde",
    "serde_json",
    "semver",
    "sha2",
    "thiserror",
    "toml",
    "toml_edit",
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn the_dependency_list_is_exactly_the_pure_one() {
    let manifest = std::fs::read_to_string(crate_dir().join("Cargo.toml")).expect("own Cargo.toml");
    let document: toml::Value = toml::from_str(&manifest).expect("own Cargo.toml parses");
    let declared: BTreeSet<String> = document
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("[dependencies]")
        .keys()
        .cloned()
        .collect();
    let allowed: BTreeSet<String> = ALLOWED.iter().map(|name| (*name).to_string()).collect();
    assert_eq!(
        declared.difference(&allowed).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "a dependency outside the pure list reached vibe-template"
    );
    // No optional, dev-time back door either: the renderer must not be able
    // to reach a transport or a registry even in a test build.
    for table in ["dev-dependencies", "build-dependencies"] {
        for name in document
            .get(table)
            .and_then(toml::Value::as_table)
            .map(|rows| rows.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
        {
            assert!(
                allowed.contains(&name) || name == "tempfile",
                "`{name}` in [{table}] is outside the pure list"
            );
        }
    }
}

/// The forbidden spellings, each with the capability it would smuggle in.
const FORBIDDEN: &[(&str, &str)] = &[
    ("std::fs", "the filesystem"),
    ("std::net", "the network"),
    ("std::env", "the ambient environment"),
    ("std::process", "a child process"),
    ("std::time", "the clock"),
    ("File::open", "the filesystem"),
    ("include_str!", "the filesystem at compile time"),
    ("include_bytes!", "the filesystem at compile time"),
    ("vibe_net", "the transport"),
    ("vibe_registry", "the registry"),
    ("vibe_resolver", "the resolver"),
];

#[test]
fn the_sources_reach_neither_a_file_nor_a_socket_nor_a_clock() {
    let mut checked = 0usize;
    for file in rust_files(&crate_dir().join("src")) {
        let text = std::fs::read_to_string(&file).expect("own source");
        checked += 1;
        for (spelling, capability) in FORBIDDEN {
            assert!(
                !text.contains(spelling),
                "{}: `{spelling}` would give the renderer {capability}",
                file.display()
            );
        }
    }
    assert!(checked >= 5, "the scan found only {checked} source files");
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory).expect("a readable source directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}
