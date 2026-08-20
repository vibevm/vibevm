//! The fresh-TypeScript-project acceptance, frozen (the Phase 6 §9
//! walk, engine calls end to end): bootstrap a temp project, tag one
//! export, and drive init → conform → specmap → orphan gate through
//! the REAL extractor and node. `typescript` resolves through a
//! directory junction/symlink onto tools/ts-extract's own install, so
//! the test needs no network and no per-run npm install.

use std::path::{Path, PathBuf};

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, body).expect("write");
}

/// Link the packaged extractor's node_modules into the temp project.
fn link_node_modules(root: &Path) -> bool {
    let target =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/ts-extract/node_modules");
    // mklink refuses `..` segments; flatten to a plain absolute path.
    let target = std::path::absolute(&target)
        .and_then(|p| p.canonicalize())
        .map(strip_verbatim)
        .unwrap_or(target);
    if !target.exists() {
        eprintln!("fresh-ts: tools/ts-extract/node_modules missing — run `npm install` there");
        return false;
    }
    let link = root.join("node_modules");
    #[cfg(windows)]
    {
        // A junction needs no privileges, unlike a directory symlink.
        let out = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .output()
            .expect("spawning mklink");
        if !out.status.success() {
            eprintln!(
                "fresh-ts: mklink failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        out.status.success()
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(&target, &link).is_ok()
    }
}

/// `canonicalize` on Windows yields a `\\?\` verbatim path `cmd.exe`
/// cannot take; strip it (the PROP-019 `derive_self` lesson).
fn strip_verbatim(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(stripped) => PathBuf::from(stripped),
        None => path,
    }
}

#[test]
fn init_then_gates_catch_violations_and_the_tagged_tree_passes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    assert!(link_node_modules(root), "node_modules link failed");
    write(
        root,
        "package.json",
        "{ \"name\": \"fresh\", \"type\": \"module\" }\n",
    );
    write(
        root,
        "spec/PROP-001.md",
        "# PROP-001 — the demo spec {#root}\n\n## Hello {#req-hello}\n`req r1`\n\nIt MUST greet.\n",
    );
    // A materialised package slot with a spec tree (what vibe install
    // makes) so init discovers [[external_specs]].
    write(
        root,
        "vibedeps/flow-some-core/0.4.0/vibe.toml",
        "[package]\nname = \"some-core\"\n",
    );
    write(
        root,
        "vibedeps/flow-some-core/0.4.0/spec/mechanisms/ENGINE-X.md",
        "## Rules {#rules}\n`req r1`\n\nbody\n",
    );
    // One tagged export citing the local unit, one UNTAGGED export, and
    // one unsafe-set token.
    write(
        root,
        "src/cells/hello/index.ts",
        "/** @implements spec://fresh/PROP-001#req-hello */\n\
         export function hello(name: string): string {\n  return `hello, ${name}`;\n}\n\
         export const NAKED = 1;\n",
    );
    write(
        root,
        "src/cells/other/index.ts",
        "/** @scope spec://fresh/PROP-001#req-hello */\n\
         export const leak: any = 1;\n",
    );

    // 1. Bootstrap.
    typescript_ai_native_cli::run_init(
        root,
        &typescript_ai_native_cli::InitOptions {
            namespace: Some("fresh".into()),
            force: false,
        },
    )
    .expect("init");
    let policy = std::fs::read_to_string(root.join("specmap.toml")).expect("policy");
    assert!(
        policy.contains("root = \"vibedeps/flow-some-core/0.4.0/spec\""),
        "{policy}"
    );

    // 2. The conform gate catches the `any` through the real node run.
    let err = typescript_ai_native_conform::run_check(
        root,
        typescript_ai_native_conform::DEFAULT_TS_BASELINE,
        None,
    )
    .expect_err("the any must fail the gate");
    assert!(err.to_string().contains("1 new finding(s)"), "{err}");

    // 3. The index mints (the ratchet is warn-only on a mint), then
    // `--check` blocks on the naked export (the scoped file passes
    // whole — mirroring `rust-ai-native-specmap`).
    typescript_ai_native_specmap::run_specmap_typescript(root, false).expect("mint warns only");
    let err = typescript_ai_native_specmap::run_specmap_typescript(root, true)
        .expect_err("one naked export must block the check");
    assert!(err.to_string().contains("1 untagged export(s)"), "{err}");

    // 4. Tag it; mint again; check reproduces; the gate is green.
    write(
        root,
        "src/cells/hello/index.ts",
        "/** @implements spec://fresh/PROP-001#req-hello */\n\
         export function hello(name: string): string {\n  return `hello, ${name}`;\n}\n\
         /** @documents spec://fresh/PROP-001#req-hello */\n\
         export const NAKED = 1;\n",
    );
    typescript_ai_native_specmap::run_specmap_typescript(root, false).expect("mint");
    typescript_ai_native_specmap::run_specmap_typescript(root, true).expect("check clean");
}
