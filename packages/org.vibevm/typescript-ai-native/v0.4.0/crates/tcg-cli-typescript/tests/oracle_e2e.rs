//! End-to-end over the REAL chain: SystemOracle → node → LanguageService
//! on a copy of the ts-oracle fixture project, with the discipline
//! enrichment on top. Node-dependent by design: absence of node or of
//! the tool's node_modules is a HARD failure carrying the recipe, never
//! a skip (the package's node-gated test posture).

use std::path::{Path, PathBuf};
use std::time::Duration;

use tcg_cli_typescript::{Policy, enrich_validate};
use tcg_oracle_bridge::{OracleTransport, Position, SystemOracle};

fn fixture_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tools")
        .join("ts-oracle")
        .join("test")
        .join("fixtures")
        .join("proj")
}

fn oracle_node_modules() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tools")
        .join("ts-oracle")
        .join("node_modules")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("mkdir");
    for entry in std::fs::read_dir(from).expect("read_dir") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("ft").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy");
        }
    }
}

/// Junction (Windows) / symlink (unix) the tool's node_modules into the
/// temp project so the CONSUMER-resolution path works offline — the
/// fresh_ts_project lesson, reused. Verbatim prefixes stripped.
fn link_node_modules(project: &Path) {
    let target = oracle_node_modules();
    assert!(
        target.exists(),
        "tools/ts-oracle/node_modules missing — run `npm install` there once \
         (the oracle tests need a resolvable `typescript`)"
    );
    let link = project.join("node_modules");
    #[cfg(windows)]
    {
        let strip = |p: &Path| {
            let s = p.to_string_lossy().into_owned();
            s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(s)
        };
        let status = std::process::Command::new("cmd")
            .args([
                "/c",
                "mklink",
                "/J",
                &strip(&link),
                &strip(&target.canonicalize().expect("canon target")),
            ])
            .status()
            .expect("mklink spawn");
        assert!(status.success(), "mklink /J failed");
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
    }
}

#[test]
fn the_real_chain_validates_enriches_scopes_and_completes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("proj");
    copy_tree(&fixture_src(), &root);
    link_node_modules(&root);
    // A real policy for the fixture: cells under src/cells, seam index.
    std::fs::write(
        root.join("conform.toml"),
        "max_file_lines = 600\n\n[typescript]\nroots = [\"src\"]\ncells_dir = \"src/cells\"\nseam = \"index\"\n",
    )
    .expect("conform.toml");

    let policy = Policy::load(&root).expect("policy");
    let mut oracle =
        SystemOracle::spawn(&root, Duration::from_secs(60)).expect("spawn (node on PATH?)");
    let init = oracle
        .init(&root, Some("src/cells"), "index")
        .expect("init against the fixture");
    assert!(init.ts_version.starts_with(|c: char| c.is_ascii_digit()));

    // 1) clean file: no error diagnostics; the sanctioned brand cast is
    // a finding (no baseline in the fixture → not baselined) + advice.
    let clean = oracle
        .validate("src/cells/greet/index.ts", None)
        .expect("validate clean");
    assert!(!clean.degraded);
    assert_eq!(
        clean
            .diagnostics
            .iter()
            .filter(|d| d.category == "error")
            .count(),
        0
    );
    let enriched = enrich_validate(&policy, "src/cells/greet/index.ts", clean);
    assert_eq!(enriched.conform_findings.len(), 1);
    assert_eq!(enriched.conform_findings[0].rule, "ts-unsafe-in-domain");
    assert!(!enriched.conform_findings[0].baselined);
    assert!(enriched.advice.iter().any(|a| a.contains("guide s8")));

    // 2) a seeded type error in an overlay is caught without disk writes
    let original =
        std::fs::read_to_string(root.join("src/cells/greet/index.ts")).expect("read");
    let seeded = format!("{original}\nconst bad: number = \"oops\";\n");
    let v = oracle
        .validate("src/cells/greet/index.ts", Some(&seeded))
        .expect("validate seeded");
    assert!(v.diagnostics.iter().any(|d| d.code == 2322), "{:?}", v.diagnostics);
    let after =
        std::fs::read_to_string(root.join("src/cells/greet/index.ts")).expect("re-read");
    assert_eq!(after, original, "the overlay never reaches disk");

    // 3) scope: cell + seam + the branded heuristic
    let scope = oracle
        .scope("src/cells/greet/index.ts", None)
        .expect("scope");
    assert_eq!(scope.cell.as_deref(), Some("greet"));
    assert_eq!(scope.seam_file.as_deref(), Some("src/cells/greet/index.ts"));
    assert!(scope.branded.iter().any(|b| b.name == "GuestName" && b.heuristic));

    // 4) complete with a prefix carries type text; the any-typed entry
    // in the dirty cell is flagged unsafe
    let dirty =
        std::fs::read_to_string(root.join("src/cells/dirty/index.ts")).expect("dirty");
    let probe = format!("{dirty}\nexport function p(): number {{\n  return anyTh\n}}\n");
    let line = (probe.lines().position(|l| l.contains("return anyTh")).expect("probe line")
        + 1) as u64;
    let character =
        (probe.lines().nth(line as usize - 1).expect("line").find("anyTh").expect("col")
            + "anyTh".len()) as u64;
    let comp = oracle
        .complete(
            "src/cells/dirty/index.ts",
            Position { line, character },
            Some(&probe),
            Some("anyTh"),
            10,
        )
        .expect("complete");
    let hit = comp
        .entries
        .iter()
        .find(|e| e.name == "anyThing")
        .expect("anyThing completes");
    assert!(hit.unsafe_, "any-typed completion must be flagged");

    oracle.shutdown().expect("graceful shutdown");
}
