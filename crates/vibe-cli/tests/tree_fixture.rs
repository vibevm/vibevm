//! Full-engine reference oracle for `vibe tree` (PROP-036; scaffold-d/h).
//!
//! `classify_origin` (src/commands/tree/build/tests.rs) is the pure decision
//! table in isolation. This is the whole analyzer wired end-to-end: a
//! synthetic, self-contained project — root manifest, lockfile, `STATIC.md`,
//! `INDEX.md` — run through `vibe tree --json`, with **every** classification
//! lane asserted on the emitted model.
//!
//! Unlike `tree_json.rs` (which validates the *real repo's* output against the
//! schema, and so drifts with the repo), this fixture is hermetic: it depends
//! on nothing but the six files it writes, so its expected classification is
//! fixed forever. That makes it the Class-D safety net for the engine — a
//! silent change to how the lockfile + boot artifacts fold into `LoadType` /
//! `LoadOrigin` breaks a named lane here (AINATIVE-ANALYSIS-RAID Phase 1, D5).
//!
//! The six lanes (PROP-036 §2.3–§2.4), one package each:
//!   crit     — `STATIC.md` marker + root `link = "static"`   → static / declared
//!   dyn      — `INDEX.md` entry, root plain (no link)        → dynamic / default
//!   umbrella — `STATIC.md` + root `link = "static-transitive"` → static / declared (declarer)
//!   member   — `STATIC.md`, pulled into umbrella's closure   → static / static-transitive (transitive)
//!   gated    — `INDEX.md` entry carrying `when = "os:…"`     → dynamic / when-forced
//!   orphan   — in neither boot lane                          → none / none

use assert_cmd::Command;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use tempfile::TempDir;

/// Root manifest: declares the three lanes a root edge can express (plain
/// dynamic, static, static-transitive). `member` and `orphan` are deliberately
/// absent — they reach the tree transitively / as leftovers, not by declaration.
const MANIFEST: &str = r#"[project]
name = "tree-fixture"
version = "0.1.0"

[requires.packages]
"flow:org.vibevm/crit" = { version = "^1.0", link = "static" }
"flow:org.vibevm/dyn" = "^1.0"
"flow:org.vibevm/umbrella" = { version = "^1.0", link = "static-transitive" }
"#;

/// Effective static lane. The separator is space + U+2014 em-dash + space
/// (`artifacts::MARKER_SEP`); each marker's origin is the package `group/name`.
const STATIC_MD: &str = "\
<!-- vibe:static org.vibevm/crit \u{2014} vibedeps/flow-crit/1.0.0/boot.md -->

# crit boot body

<!-- vibe:static org.vibevm/umbrella \u{2014} vibedeps/flow-umbrella/1.0.0/boot.md -->

# umbrella boot body

<!-- vibe:static org.vibevm/member \u{2014} vibedeps/flow-member/1.0.0/boot.md -->

# member boot body
";

/// Effective dynamic lane. `dyn` is plain; `gated` carries a `when` gate, which
/// classification must let win over the (absent) declaration → `when-forced`.
const INDEX_MD: &str = r#"schema = 1
static = "spec/boot/STATIC.md"

[[entry]]
path = "vibedeps/flow-dyn/1.0.0/boot.md"
kind = "dynamic"

[[entry]]
path = "vibedeps/flow-gated/1.0.0/boot.md"
kind = "dynamic"
when = "os:linux"
"#;

/// Build the lockfile text. The schema version is injected from `vibe-core` so
/// this fixture survives a lockfile-schema bump (the reader rejects any other
/// version). `umbrella → member` is the one static-transitive edge; `orphan` is
/// a resolved package that lands in no boot lane (the `none` case).
fn lockfile() -> String {
    let v = vibe_core::manifest::CURRENT_SCHEMA_VERSION;
    format!(
        r#"[meta]
generated_by = "tree-fixture"
generated_at = "2026-01-01T00:00:00Z"
schema_version = {v}
root_dependencies = [
    "flow:org.vibevm/crit@^1.0",
    "flow:org.vibevm/dyn@^1.0",
    "flow:org.vibevm/umbrella@^1.0",
]

[[package]]
kind = "flow"
name = "crit"
group = "org.vibevm"
version = "1.0.0"
source_url = "file:///fixture/crit"
content_hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

[[package]]
kind = "flow"
name = "dyn"
group = "org.vibevm"
version = "1.0.0"
source_url = "file:///fixture/dyn"
content_hash = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"

[[package]]
kind = "flow"
name = "umbrella"
group = "org.vibevm"
version = "1.0.0"
source_url = "file:///fixture/umbrella"
content_hash = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
dependencies = [
    "flow:org.vibevm/member@=1.0.0",
]

[[package]]
kind = "flow"
name = "member"
group = "org.vibevm"
version = "1.0.0"
source_url = "file:///fixture/member"
content_hash = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"

[[package]]
kind = "flow"
name = "gated"
group = "org.vibevm"
version = "1.0.0"
source_url = "file:///fixture/gated"
content_hash = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"

[[package]]
kind = "flow"
name = "orphan"
group = "org.vibevm"
version = "1.0.0"
source_url = "file:///fixture/orphan"
content_hash = "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
"#
    )
}

/// Materialise the six-file fixture project under `root`.
fn write_fixture(root: &Path) {
    std::fs::create_dir_all(root.join("spec/boot")).expect("mk spec/boot");
    std::fs::write(root.join("vibe.toml"), MANIFEST).expect("write vibe.toml");
    std::fs::write(root.join("vibe.lock"), lockfile()).expect("write vibe.lock");
    std::fs::write(root.join("spec/boot/STATIC.md"), STATIC_MD).expect("write STATIC.md");
    std::fs::write(root.join("spec/boot/INDEX.md"), INDEX_MD).expect("write INDEX.md");
}

/// Run `vibe tree --json --path <root>` and parse stdout.
fn run_tree_json(root: &Path) -> Value {
    let mut cmd = Command::cargo_bin("vibe").expect("vibe binary built");
    cmd.arg("tree").arg("--json").arg("--path").arg(root);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf-8 stdout");
    serde_json::from_str(&stdout).expect("stdout is JSON")
}

/// Index the `packages` array by `id` for lane-by-lane assertions.
fn by_id(tree: &Value) -> BTreeMap<String, Value> {
    tree["packages"]
        .as_array()
        .expect("packages is an array")
        .iter()
        .map(|p| (p["id"].as_str().expect("id string").to_string(), p.clone()))
        .collect()
}

#[test]
fn build_tree_classifies_every_lane_on_a_hermetic_fixture() {
    let dir = TempDir::new().expect("tempdir");
    write_fixture(dir.path());
    let tree = run_tree_json(dir.path());
    let pkgs = by_id(&tree);

    // Lane 1 — declared static: a `STATIC.md` marker and a root `link = "static"`.
    let crit = &pkgs["org.vibevm/crit"];
    assert_eq!(crit["load"]["type"], "static", "crit is in the static lane");
    assert_eq!(
        crit["load"]["origin"], "declared",
        "crit's static-ness is declared"
    );
    assert_eq!(crit["load"]["transitive"], false);
    assert_eq!(crit["load"]["in_static_md"], true);
    assert_eq!(crit["load"]["in_index_md"], false);

    // Lane 2 — plain dynamic: an `INDEX.md` entry, no root link → default origin.
    let dynamic = &pkgs["org.vibevm/dyn"];
    assert_eq!(dynamic["load"]["type"], "dynamic");
    assert_eq!(
        dynamic["load"]["origin"], "default",
        "an undeclared dynamic is default"
    );
    assert_eq!(dynamic["load"]["in_index_md"], true);

    // Lane 3 — static-transitive declarer: its static-ness is its own (declared),
    // never attributed to the closure it opens.
    let umbrella = &pkgs["org.vibevm/umbrella"];
    assert_eq!(umbrella["load"]["type"], "static");
    assert_eq!(
        umbrella["load"]["origin"], "declared",
        "the declarer owns its static-ness"
    );
    assert_eq!(umbrella["load"]["transitive"], false);

    // Lane 4 — transitive member: pulled into umbrella's closure, not suggested on
    // its own — the only `transitive = true` row.
    let member = &pkgs["org.vibevm/member"];
    assert_eq!(member["load"]["type"], "static");
    assert_eq!(member["load"]["origin"], "static-transitive");
    assert_eq!(
        member["load"]["transitive"], true,
        "the member is transitively static"
    );

    // Lane 5 — when-forced dynamic: the `when` gate wins over the (absent) declaration.
    let gated = &pkgs["org.vibevm/gated"];
    assert_eq!(gated["load"]["type"], "dynamic");
    assert_eq!(
        gated["load"]["origin"], "when-forced",
        "a when-gated entry is when-forced"
    );

    // Lane 6 — neither lane: a resolved package with no boot presence to attribute.
    let orphan = &pkgs["org.vibevm/orphan"];
    assert_eq!(orphan["load"]["type"], "none");
    assert_eq!(orphan["load"]["origin"], "none");
    assert_eq!(orphan["load"]["in_static_md"], false);
    assert_eq!(orphan["load"]["in_index_md"], false);
}
