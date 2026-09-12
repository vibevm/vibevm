//! The `embedded-shell` gate (PROP-057 `##SHELL-XTASK-EMBED`).
//!
//! Off, nothing is required: no shell directory has to exist and the
//! binary ships the bare fallback. On, the built shell must be there, or
//! the build stops with the recipe in the message — because neither
//! `include_dir` nor `rust-embed` can tell an empty directory from a full
//! one, and a release binary that silently embedded nothing would only be
//! found by a reader.
//!
//! It computes nothing about the bytes. The digest it exports is the one
//! `cargo xtask embed-doc-shell` wrote into the shell's own index; what
//! the binary ACTUALLY carries is measured at run time by the crate
//! itself, and `vibe doc shell status` compares the three. One
//! measurement, one algorithm, one place — a build script that hashed the
//! directory too would be a second opinion about the same bytes.

use std::path::PathBuf;
use std::process::exit;

fn main() {
    // Neither library asks for a rebuild when the directory changes, so
    // this is not an optimisation: without it, a rebuilt shell does not
    // reach the binary at all (measured in the phase-0 finding A0.11).
    println!("cargo:rerun-if-changed=shell");
    println!("cargo:rerun-if-changed=doc-shell.lock");

    let Some(manifest) = std::env::var_os("CARGO_MANIFEST_DIR") else {
        eprintln!("error: CARGO_MANIFEST_DIR is not set; this is a cargo build script");
        exit(1);
    };
    let shell = PathBuf::from(manifest).join("shell");

    if std::env::var_os("CARGO_FEATURE_EMBEDDED_SHELL").is_none() {
        // The empty digest is what «this binary carries no shell of its
        // own» looks like from the inside; the crate reads it that way.
        println!("cargo:rustc-env=VIBE_DOC_SHELL_SHA256=");
        return;
    }

    let index = shell.join("shell.json");
    if !index.is_file() {
        eprintln!(
            "error: feature `embedded-shell` is on, but the built doc shell is missing: \
             `{}` does not exist.\n\
             note: the shell is produced by `cargo xtask embed-doc-shell`, which builds the \
             web package with the embeddable adapter and copies its output here.\n\
             fix: run `cargo xtask embed-doc-shell` before a release build, or build without \
             `--features embedded-shell` to get the bare fallback shell.",
            index.display()
        );
        exit(1);
    }

    let template = shell.join("page-template.html");
    if !template.is_file() {
        eprintln!(
            "error: the doc shell at `{}` carries an index but no page template \
             (`page-template.html`).\n\
             note: the template is the prerendered route the server glues the island into; \
             without it the shell has nothing to serve.\n\
             fix: re-run `cargo xtask embed-doc-shell`.",
            shell.display()
        );
        exit(1);
    }

    let text = match std::fs::read_to_string(&index) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("error: cannot read `{}`: {error}", index.display());
            exit(1);
        }
    };
    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "error: `{}` is not readable as JSON: {error}",
                index.display()
            );
            exit(1);
        }
    };
    let Some(sha256) = parsed.get("sha256").and_then(serde_json::Value::as_str) else {
        eprintln!(
            "error: `{}` declares no `sha256`.\n\
             fix: re-run `cargo xtask embed-doc-shell`, which writes it.",
            index.display()
        );
        exit(1);
    };
    println!("cargo:rustc-env=VIBE_DOC_SHELL_SHA256={sha256}");
}
