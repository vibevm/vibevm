//! `cargo xtask embed-doc-shell` — build the reader's shell and put it
//! where a release build of `vibe` can compile it in (PROP-057
//! `##SHELL-XTASK-EMBED`).
//!
//! Three steps and a measurement:
//!
//! 1. Build the web package with the embeddable adapter —
//!    `pnpm build:embedded`, which is the package's own two-pass order
//!    (client build, then adapter build) plus its page-count gate. The
//!    order is not a preference: the adapter's own advice to build
//!    through the app builder produces pages that resolve symbols against
//!    a stale manifest (finding P4-O1, anomaly A-2).
//! 2. Copy what a reader needs — the route template, the chunks, the
//!    styles, the fonts — into `crates/vibe-doc-shell/shell/`, and NOT
//!    the thirteen other prerendered fixture pages, which are a property
//!    of the build's page library and would be two thirds of the weight
//!    for nothing.
//! 3. Write the shell's own index beside them, and re-pin
//!    `doc-shell.lock`.
//!
//! The digest is measured by `vibe-doc-shell` itself, through the crate
//! this xtask depends on, so the number written into the pin and the
//! number `vibe doc shell status` measures later come from one
//! implementation.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use vibe_doc_shell::{digest, index::Index};

/// The web package the shell is built from, repo-relative.
const WEB_PACKAGE: &str = "vibevm/vibepacks/org.vibevm.doc/web/v0.1.0";

/// Its coordinate, which the pin and the index name.
const WEB_COORDINATE: &str = "org.vibevm.doc/web";

/// Where the embeddable adapter leaves its output, inside the package.
const BUILD_OUTPUT: &str = "site/dist-embedded";

/// The crate that compiles the shell in.
const SHELL_CRATE: &str = "crates/vibe-doc-shell";

/// The directories of the build output a reader actually serves.
///
/// `build/` is the framework's chunks, `assets/` the stylesheet and the
/// font faces. Everything else the adapter writes describes the FIXTURE
/// the package builds against — its sitemap, its web-app manifest, its
/// thirteen other prerendered pages — and a reader serves none of it.
const SERVED_DIRS: [&str; 2] = ["assets", "build"];

/// Single files taken from the root of the build output.
const SERVED_FILES: [&str; 1] = ["favicon.svg"];

/// Build the shell and embed it.
pub(crate) fn run_embed_doc_shell(repo_root: &Path, skip_build: bool) -> Result<()> {
    let package = repo_root.join(WEB_PACKAGE);
    if !package.join("package.json").is_file() {
        bail!(
            "the web package is not at `{}` — the shell is built from it and there is \
             nothing else to build it from \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-XTASK-EMBED)",
            package.display()
        );
    }

    if skip_build {
        println!("embed-doc-shell: --no-build; using the output already in {BUILD_OUTPUT}");
    } else {
        build_web_package(&package)?;
    }

    let output = package.join(BUILD_OUTPUT);
    if !output.is_dir() {
        bail!(
            "`{}` does not exist after the build; nothing to embed \
             (fix: run `pnpm --dir {} build:embedded` by hand and read what it says)",
            output.display(),
            package.display()
        );
    }

    let shell_dir = repo_root.join(SHELL_CRATE).join("shell");
    let template = choose_template(&output)?;
    let files = gather(&output, &template)?;
    write_shell(&shell_dir, &files)?;

    let sha256 = digest::of(&files);
    let index = Index {
        schema: vibe_doc_shell::index::INDEX_SCHEMA,
        package: WEB_COORDINATE.to_string(),
        version: web_package_version(&package)?,
        base: vibe_doc_shell::DEFAULT_BASE.to_string(),
        island_marker: vibe_doc_shell::ISLAND_MARKER.to_string(),
        files: files.len() as u32,
        sha256: sha256.clone(),
    };
    let index_path = shell_dir.join(vibe_doc_shell::index::INDEX_FILE);
    std::fs::write(&index_path, index.to_json())
        .with_context(|| format!("writing `{}`", index_path.display()))?;

    repin(&repo_root.join(SHELL_CRATE).join("doc-shell.lock"), &index)?;

    let bytes: u64 = files.values().map(|one| one.len() as u64).sum();
    println!(
        "embed-doc-shell: {} file(s), {bytes} byte(s) under {}",
        files.len(),
        shell_dir.display()
    );
    println!("embed-doc-shell: template {template}");
    println!("embed-doc-shell: sha256 {sha256}");
    println!(
        "embed-doc-shell: build `vibe` with `--features vibe-doc-shell/embedded-shell` to \
         carry it"
    );
    Ok(())
}

/// Run the package's own embeddable build.
fn build_web_package(package: &Path) -> Result<()> {
    // `pnpm` is a shim on Windows and a script elsewhere; the extension
    // is resolved here rather than by a shell, because no shell is
    // spawned (a command line assembled for one is a quoting bug waiting
    // for a path with a space in it).
    let program = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };
    println!(
        "embed-doc-shell: {program} --dir {} build:embedded",
        package.display()
    );
    let status = Command::new(program)
        .arg("--dir")
        .arg(package)
        .arg("build:embedded")
        .status()
        .with_context(|| {
            format!(
                "running `{program} --dir {} build:embedded` — the shell is a build of the \
                 web package, so Node and pnpm are what a release build of `vibe` needs \
                 (fix: install them, or build without `--features \
                 vibe-doc-shell/embedded-shell` for the bare fallback shell)",
                package.display()
            )
        })?;
    if !status.success() {
        bail!(
            "`{program} --dir {} build:embedded` exited {} — read its output; the page-count \
             gate is the usual reason and it prints what it counted",
            package.display(),
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

/// Which prerendered page is the route template.
///
/// Every page of the embedded build carries the island marker, so any of
/// them would serve; the choice is made deterministic rather than left to
/// directory order, because a template that changed between two runs of
/// this step would change the shell's digest for no reason a person could
/// see. Shortest address first, then lexicographic.
fn choose_template(output: &Path) -> Result<String> {
    let mut candidates: Vec<String> = Vec::new();
    for (relative, bytes) in
        digest::read_tree(output).with_context(|| format!("reading `{}`", output.display()))?
    {
        if !relative.ends_with(".html") {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        if text.contains(vibe_doc_shell::ISLAND_MARKER) {
            candidates.push(relative);
        }
    }
    candidates.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    match candidates.first() {
        Some(found) => Ok(found.clone()),
        None => bail!(
            "no page in `{}` carries the island marker `{}` — the embeddable adapter is \
             supposed to leave one where the island goes \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-XTASK-EMBED)",
            output.display(),
            vibe_doc_shell::ISLAND_MARKER
        ),
    }
}

/// The files the shell carries, keyed by the path they are served at.
fn gather(output: &Path, template: &str) -> Result<BTreeMap<String, Vec<u8>>> {
    let all =
        digest::read_tree(output).with_context(|| format!("reading `{}`", output.display()))?;
    let mut out = BTreeMap::new();
    for (relative, bytes) in all {
        if relative == template {
            out.insert(vibe_doc_shell::index::PAGE_TEMPLATE.to_string(), bytes);
            continue;
        }
        let served = SERVED_DIRS
            .iter()
            .any(|dir| relative.starts_with(&format!("{dir}/")))
            || SERVED_FILES.contains(&relative.as_str());
        if served {
            out.insert(relative, bytes);
        }
    }
    if !out.contains_key(vibe_doc_shell::index::PAGE_TEMPLATE) {
        bail!("the chosen template `{template}` vanished between two reads of the build output");
    }
    Ok(out)
}

/// Replace the shell directory with exactly these files.
fn write_shell(shell_dir: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    if shell_dir.exists() {
        std::fs::remove_dir_all(shell_dir)
            .with_context(|| format!("clearing `{}`", shell_dir.display()))?;
    }
    for (relative, bytes) in files {
        let mut path = shell_dir.to_path_buf();
        for segment in relative.split('/') {
            path.push(segment);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating `{}`", parent.display()))?;
        }
        std::fs::write(&path, bytes).with_context(|| format!("writing `{}`", path.display()))?;
    }
    Ok(())
}

/// The web package's own version, read as data.
fn web_package_version(package: &Path) -> Result<String> {
    let path = package.join("vibe.toml");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading `{}`", path.display()))?;
    let parsed: toml::Value =
        toml::from_str(&text).with_context(|| format!("`{}` does not parse", path.display()))?;
    parsed
        .get("package")
        .and_then(|table| table.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("`{}` declares no `[package].version`", path.display()))
}

/// Rewrite the pin's three generated fields, leaving its prose alone.
///
/// Line surgery rather than a TOML round-trip because the file is mostly
/// a comment explaining what the pin is for, and a serialiser would drop
/// every word of it.
fn repin(path: &Path, index: &Index) -> Result<()> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading `{}`", path.display()))?;
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let replaced = if line.starts_with("package = ") {
            format!("package = \"{}\"", index.package)
        } else if line.starts_with("version = ") {
            format!("version = \"{}\"", index.version)
        } else if line.starts_with("sha256 = ") {
            format!("sha256 = \"{}\"", index.sha256)
        } else {
            line.to_string()
        };
        out.push_str(&replaced);
        out.push('\n');
    }
    std::fs::write(path, out).with_context(|| format!("writing `{}`", path.display()))?;
    println!("embed-doc-shell: re-pinned {}", path.display());
    Ok(())
}
