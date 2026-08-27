//! Shared git-backed fixture for the R3.4 FAILURE reds: one published
//! `org.demo/tools` flow package with a REAL static boot input, whose slot
//! lifecycle rows can be made to fail HARD on demand.
//!
//! Two publications, one shape:
//!
//! * a `slot:pre-install` script that always fails (its own sentinel, its own
//!   exit code) — the whole-Ready failure family;
//! * ordered `slot:post-install` rows — an `earlier-ok` builtin log, then a
//!   `later-hard-fail` script — for the scoped-update / forced-reinstall
//!   families.
//!
//! A plain post-install nonzero is SOFT (flagged, the command stays green), so
//! the hard-post script is marker-gated: only when `.vibe/arm-hard-post`
//! exists does it sabotage `.vibe/lifecycle.toml` (file → directory, so the
//! run's next checkpoint write cannot land), print the exact secret
//! [`HARD_POST_SECRET`] to stderr, and exit 17. Without the marker the same
//! script is green, which is what makes the UNTRACED SEED of every armed
//! project legal. The sabotage makes each failed project single-use: the
//! trace-on and trace-off twins always seed SEPARATE projects.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use super::{UserScratch, run_git, slot_dir};

/// The exact secret the hard-post script prints to stderr on the armed path.
/// The secrecy reds search every byte the trace wrote for this string.
pub const HARD_POST_SECRET: &str = "TRACE-SECRET-HARD-POST";

/// The pre-install failure's own, distinct sentinel.
pub const PRE_INSTALL_SECRET: &str = "TRACE-SECRET-PRE-INSTALL";

/// The project-relative marker that arms the hard-post sabotage.
const ARM_MARKER: &str = ".vibe/arm-hard-post";

/// The package's one static boot input, named by `[boot_snippet].source`.
const BOOT_FILE: &str = "boot/40-tools.md";

/// The ordered `slot:post-install` declarations every post-install version
/// carries: the green builtin log first, the marker-gated script second.
const ORDERED_EXTENSIONS: &str = concat!(
    "[[extension]]\n",
    "id = \"earlier-ok\"\n",
    "point = \"slot:post-install\"\n",
    "handler = { kind = \"builtin\", name = \"log\" }\n",
    "config = { message = \"EARLIER-OK-LOG\" }\n",
    "\n",
    "[[extension]]\n",
    "id = \"later-hard-fail\"\n",
    "point = \"slot:post-install\"\n",
    "handler = { kind = \"script\", base = \"hooks/post\" }\n",
);

/// One published source tree plus the per-package bare registry carrying it.
pub struct Published {
    /// The org root the project's `[[registry]]` points at.
    pub registry: PathBuf,
    /// The still-writable source repo — `add_version` commits here.
    pub source: PathBuf,
}

/// A fresh, configured source tree with the boot/hook scaffolding in place.
fn init_source(root: &Path, name: &str) -> PathBuf {
    let source = root.join(name);
    fs::create_dir_all(source.join("boot")).unwrap();
    fs::create_dir_all(source.join("hooks")).unwrap();
    run_git(&source, &["init", "--initial-branch=main"]);
    run_git(&source, &["config", "user.email", "t@example.com"]);
    run_git(&source, &["config", "user.name", "Test"]);
    // One spelling of every script on every checkout, so the .sh/.ps1 pair
    // below behaves identically wherever the bare repo is cloned from.
    fs::write(source.join(".gitattributes"), "* text=auto eol=lf\n").unwrap();
    source
}

/// Rewrite the source at `version`. The boot input is version-bearing on
/// purpose: a version bump changes `boot/40-tools.md`, so the boot
/// regeneration downstream really has new bytes to recompile.
fn write_version(source: &Path, version: &str, extensions: &str) {
    fs::write(
        source.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"org.demo\"\nname = \"tools\"\nkind = \"flow\"\n\
             version = \"{version}\"\n\n\
             [boot_snippet]\nsource = \"{BOOT_FILE}\"\ncategory = \"flow\"\n\n\
             {extensions}"
        ),
    )
    .unwrap();
    fs::write(
        source.join(BOOT_FILE),
        format!("# Tools {version} {{#root}}\n\nTOOLS BOOT BODY {version}\n"),
    )
    .unwrap();
    fs::write(source.join("payload.txt"), format!("payload {version}\n")).unwrap();
}

/// Commit, tag and bare-publish the source as `v<version>` under the org root.
fn publish_bare(root: &Path, source: &Path, version: &str, message: &str) -> Published {
    run_git(source, &["add", "-A"]);
    run_git(source, &["commit", "-m", message]);
    run_git(source, &["tag", &format!("v{version}")]);
    let bare = root.join("org.demo.tools.git");
    run_git(
        root,
        &[
            "clone",
            "--bare",
            source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    Published {
        registry: root.to_path_buf(),
        source: source.to_path_buf(),
    }
}

/// Publish `org.demo/tools@0.1.0` whose `slot:pre-install` row ALWAYS fails:
/// its own sentinel on stderr, exit 13. The whole-Ready failure family.
pub fn publish_pre_install_failure(root: &Path) -> Published {
    let source = init_source(root, "src-pre");
    fs::write(
        source.join("hooks/pre.sh"),
        format!("printf {PRE_INSTALL_SECRET} >&2\nexit 13\n"),
    )
    .unwrap();
    fs::write(
        source.join("hooks/pre.ps1"),
        format!("[Console]::Error.Write('{PRE_INSTALL_SECRET}')\nexit 13\n"),
    )
    .unwrap();
    write_version(
        &source,
        "0.1.0",
        concat!(
            "[[extension]]\n",
            "id = \"pre-fail\"\n",
            "point = \"slot:pre-install\"\n",
            "handler = { kind = \"script\", base = \"hooks/pre\" }\n",
        ),
    );
    publish_bare(root, &source, "0.1.0", "pre-install failure fixture")
}

/// Publish `org.demo/tools@0.1.0` with ORDERED `slot:post-install` rows: the
/// `earlier-ok` builtin log first, then the marker-gated `later-hard-fail`
/// script. Unarmed, both rows are green — the seed of every armed project.
pub fn publish_ordered_post_install(root: &Path) -> Published {
    let source = init_source(root, "src-post");
    fs::write(
        source.join("hooks/post.sh"),
        format!(
            concat!(
                "if [ -e \"$VIBE_PROJECT_ROOT/{}\" ]; then\n",
                "  rm -f \"$VIBE_PROJECT_ROOT/.vibe/lifecycle.toml\"\n",
                "  mkdir \"$VIBE_PROJECT_ROOT/.vibe/lifecycle.toml\"\n",
                "  printf {} >&2\n",
                "  exit 17\n",
                "fi\n",
            ),
            ARM_MARKER, HARD_POST_SECRET,
        ),
    )
    .unwrap();
    fs::write(
        source.join("hooks/post.ps1"),
        format!(
            concat!(
                "$armed = Join-Path $env:VIBE_PROJECT_ROOT '{}'\n",
                "if (Test-Path -LiteralPath $armed) {{\n",
                "  $state = Join-Path $env:VIBE_PROJECT_ROOT '.vibe/lifecycle.toml'\n",
                "  if (Test-Path -LiteralPath $state) {{ Remove-Item -LiteralPath $state -Force }}\n",
                "  New-Item -ItemType Directory -Path $state | Out-Null\n",
                "  [Console]::Error.Write('{}')\n",
                "  exit 17\n",
                "}}\n",
            ),
            ARM_MARKER, HARD_POST_SECRET,
        ),
    )
    .unwrap();
    write_version(&source, "0.1.0", ORDERED_EXTENSIONS);
    publish_bare(root, &source, "0.1.0", "ordered post-install fixture")
}

/// Commit, tag and fetch one more version of the ordered fixture into the
/// bare registry — `add`/`tag` in the source, `fetch` into the bare, in that
/// order, so the registry serves the tag the moment it exists.
pub fn add_version(published: &Published, version: &str) {
    write_version(&published.source, version, ORDERED_EXTENSIONS);
    run_git(&published.source, &["add", "-A"]);
    run_git(&published.source, &["commit", "-m", "next version"]);
    run_git(&published.source, &["tag", &format!("v{version}")]);
    let bare = published.registry.join("org.demo.tools.git");
    run_git(&bare, &["fetch", "origin", "+refs/*:refs/*", "--prune"]);
}

/// The org URL a project's `[[registry]]` points at for these fixtures.
pub fn registry_url(registry: &Path) -> String {
    format!(
        "git+file://{}",
        registry.to_string_lossy().replace('\\', "/")
    )
}

/// A standalone project wired to the per-package git registry, declaring the
/// one STATIC dependency edge that makes the node really compile.
pub fn project(user: &UserScratch, registry: &Path) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let manifest = project.path().join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str(&format!(
        "\n[[registry]]\nname = \"fixture\"\nurl = \"{}\"\n\n\
         [requires]\npackages = {{ \"flow:org.demo/tools\" = {{ \
         version = \"^0.1\", link = \"static\" }} }}\n",
        registry_url(registry)
    ));
    fs::write(&manifest, text).unwrap();
    project
}

/// Install from the declared manifest over the UNARMED fixture: green, and
/// opening no trace tree — every trace the reds assert belongs to the command
/// under test, never to its seed.
pub fn seed_untraced(user: &UserScratch, project: &Path) {
    let seed = user
        .vibe()
        .args(["install", "--json", "--assume-yes"])
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert!(
        seed.status.success(),
        "the untraced seed over an unarmed fixture is green: {}",
        String::from_utf8_lossy(&seed.stderr),
    );
    assert!(
        !project.join(".vibe/trace").exists(),
        "the seed opened no trace tree of its own",
    );
}

/// Arm the hard-post sabotage for this project's NEXT slot run.
pub fn arm_hard_post(project: &Path) {
    fs::create_dir_all(project.join(".vibe")).unwrap();
    fs::write(project.join(ARM_MARKER), "armed\n").unwrap();
}

/// Corrupt the installed payload of the named slot so `Verify` integrity
/// re-materialises it instead of trusting the bytes on disk.
pub fn corrupt_payload(project: &Path, version: &str) {
    let payload = project
        .join(slot_dir("org.demo.tools", version))
        .join("payload.txt");
    fs::write(&payload, "corrupted\n")
        .unwrap_or_else(|error| panic!("corrupting {}: {error}", payload.display()));
}

/// A terminal stderr both twins can be compared under: the twin projects live
/// in different temp directories, so each one's own path is folded to
/// `<root>` (both separator spellings) and CRLF is normalised away.
pub fn normalise_stderr(project: &Path, stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr).replace("\r\n", "\n");
    let native = project.display().to_string();
    let json_escaped_native = native.replace('\\', "\\\\");
    let forward = native.replace('\\', "/");
    text.replace(&json_escaped_native, "<root>")
        .replace(&native, "<root>")
        .replace(&forward, "<root>")
}

/// Fold every occurrence of this project's absolute path — in BOTH separator
/// spellings — ANYWHERE inside a JSON value.
///
/// The top-level `project` member is only the first place a twin's temp path
/// surfaces: each contribution row carries its absolute `slot_target.root`
/// too, and two twins on distinct projects would differ there even after the
/// top-level member was folded. Strings are REWRITTEN and nothing else — the
/// structure, the keys and every non-path value are left exactly as emitted,
/// so the off/on equality still compares the whole structured surface.
pub fn normalise_json_paths(value: &mut serde_json::Value, project: &Path) {
    let native = project.display().to_string();
    let forward = native.replace('\\', "/");
    fn walk(value: &mut serde_json::Value, native: &str, forward: &str) {
        match value {
            serde_json::Value::String(text) => {
                *text = text.replace(native, "<root>").replace(forward, "<root>");
            }
            serde_json::Value::Array(rows) => {
                for row in rows {
                    walk(row, native, forward);
                }
            }
            serde_json::Value::Object(members) => {
                for (_, member) in members.iter_mut() {
                    walk(member, native, forward);
                }
            }
            _ => {}
        }
    }
    walk(value, &native, &forward);
}
