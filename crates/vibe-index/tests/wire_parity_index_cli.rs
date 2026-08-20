//! Differential wire-parity oracle for the seven `vibe-index` CLI
//! report schemas (`schemas/index_cli/e1/*.jtd.json` →
//! `vibe_wire::generated::index_cli::e1::*`, minted by B-079).
//!
//! Same law as the other wire-parity oracles (`wire_parity_repomd.rs`):
//! the hand-written envelopes in `crates/vibe-index/src/cli/*.rs` and
//! the JTD-generated readers are meant to meet on ONE wire. The three
//! halves proved here, per envelope:
//!
//! 1. **The corpus survives the generated reader.** Every authored
//!    golden document under `formats/corpora/index_cli/e1/` parses into
//!    its generated type and back at the `Value` level without loss —
//!    a field the schema forgot is dropped (or rejected) by the reader,
//!    and the equality catches both.
//! 2. **The writer emits the corpus bytes.** The corpus is not an
//!    aspiration: the real binary, run against the fixture index this
//!    file builds from authored bytes, prints exactly those bytes. The
//!    fixture is deterministic by construction — every value in the
//!    JSON comes from this file, never from the clock, the network or
//!    the filesystem's opinion — and the paths the envelopes echo
//!    (`lockfile`, `data_dir`) are pinned by running the child with a
//!    relative path against its current directory, so the comparison
//!    is byte-for-byte, not structural.
//! 3. **Broken forms are rejected loudly.** One negative case per
//!    envelope: a required field with a wrong shape (and, for the two
//!    closed inline vocabularies, an unknown enum value) must make the
//!    generated reader fail, never silently coerce. An UNKNOWN FIELD is
//!    deliberately NOT the negative case: `foreign_parsers = "many"`
//!    rules these readers permissive (PROP-044 §4.4 — a foreign parser
//!    may be newer than this build), so unknown fields are read and
//!    ignored; the loud refusal belongs to shape violations, and that
//!    is what is proven.
//!
//! The writers themselves are untouchable in this landing (the schema
//! describes what IS): the fixture below is the same shape
//! `tests/cli_read.rs` builds with live git repos, authored statically
//! so no git and no network is needed, and so the bytes cannot drift
//! between machines.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use vibe_wire::generated::index_cli::e1::capabilities_report::CapabilitiesReport;
use vibe_wire::generated::index_cli::e1::get_report::GetReport;
use vibe_wire::generated::index_cli::e1::list_report::ListReport;
use vibe_wire::generated::index_cli::e1::outdated_report::OutdatedReport;
use vibe_wire::generated::index_cli::e1::purls_report::PurlsReport;
use vibe_wire::generated::index_cli::e1::search_report::SearchReport;
use vibe_wire::generated::index_cli::e1::verify_report::VerifyReport;

// ── The fixture index ──────────────────────────────────────────────────────
//
// Three packages, six versions, one quarantined version on each of `wal`
// and `sqlx-skin` (`must_understand` names a capability no build has), so
// every envelope's `unavailable` region is populated on the same fixture
// that populates its usable rows. All timestamps are fixed instants; the
// `verify` manifest carries one honest row (primary.jsonl), one stale row
// (stale-projection.json — bytes on disk, wrong size and hash in the
// manifest) and one row whose file is absent (missing-artifact.json), so
// the verify corpus exercises the mismatch, missing and ok branches at
// once.

const PRIMARY_JSONL: &str = "{\"v\":\"wal 0.1.0\"}\n{\"v\":\"wal 0.2.0\"}\n";
const STALE_PROJECTION: &str = "{\"note\":\"this file drifted from its manifest row\"}\n";

const REPOMD: &str = r#"{
  "schema_version": 1,
  "registry": "vibespecs",
  "registry_url": "https://gitverse.ru/vibespecs",
  "naming": "fqdn",
  "generated_at": "2026-08-20T09:00:00Z",
  "generator": "vibe-index 0.1.0-dev",
  "package_count": 3,
  "version_count": 6,
  "files": {
    "by-name": {
      "kind": "directory",
      "entries": 3
    },
    "primary.jsonl": {
      "kind": "file",
      "size": "36",
      "sha256": "sha256:509d4e23be389910123b4525d9242f49971200b8ec3440d9edb032067501cedb"
    },
    "stale-projection.json": {
      "kind": "file",
      "size": "999",
      "sha256": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
    },
    "missing-artifact.json": {
      "kind": "file",
      "size": "2",
      "sha256": "sha256:1111111111111111111111111111111111111111111111111111111111111111"
    }
  }
}
"#;

const WAL_BY_NAME: &str = r#"{
  "indexed_at": "2026-08-20T09:00:00Z",
  "name": "wal",
  "packages": [
    {
      "group": "org.vibevm",
      "indexed_at": "2026-08-20T09:00:00Z",
      "name": "wal",
      "versions": [
        {
          "schema_version": 1,
          "kind": "flow",
          "group": "org.vibevm",
          "name": "wal",
          "version": "0.1.0",
          "content_hash": "sha256:9f2c4a7b51e83d6f0aa2c4e9d7b83f15c6e0d2a8b9f4c1e7d3a5b8f2e6c9d4a1",
          "source_url": "https://gitverse.ru/vibevm/org.vibevm.wal.git",
          "source_ref": "v0.1.0",
          "registry": "vibespecs",
          "files_count": 281,
          "indexed_at": "2026-08-18T10:00:00Z",
          "indexed_by": "vibe-index 0.1.0-dev",
          "resolved_commit": "0123456789abcdef0123456789abcdef01234567",
          "authors": ["Oleg Chirukhin"],
          "description": "Write-ahead log discipline for the specspaces world",
          "homepage": "https://gitverse.ru/vibevm/vibevm",
          "keywords": ["wal", "checkpoint", "journal"],
          "license": "UPL-1.0",
          "provides": { "capabilities": ["interface:wal"] }
        },
        {
          "schema_version": 1,
          "kind": "flow",
          "group": "org.vibevm",
          "name": "wal",
          "version": "0.2.0",
          "content_hash": "sha256:3ab76f0d29c4e1a8b5d3f7c9e2a4d6b8f1c3e5a7d9b2f4e6c8a0d3b5f7e9c1a2",
          "source_url": "https://gitverse.ru/vibevm/org.vibevm.wal.git",
          "source_ref": "v0.2.0",
          "registry": "vibespecs",
          "files_count": 290,
          "indexed_at": "2026-08-19T10:00:00Z",
          "indexed_by": "vibe-index 0.1.0-dev",
          "description": "Second release: stabilised the replay path"
        },
        {
          "schema_version": 1,
          "kind": "flow",
          "group": "org.vibevm",
          "name": "wal",
          "version": "1.0.0-rc.1",
          "content_hash": "sha256:5d9e2c7a4b8f1d3e6c9a2b5d8f1e4c7a0b3d6f9e2c5a8b1d4f7e0c3a6b9d2f5e8",
          "source_url": "https://gitverse.ru/vibevm/org.vibevm.wal.git",
          "source_ref": "v1.0.0-rc.1",
          "registry": "vibespecs",
          "files_count": 296,
          "indexed_at": "2026-08-20T08:00:00Z",
          "indexed_by": "vibe-index 0.1.0-dev",
          "description": "Release candidate: hash-chain verification",
          "yanked": true,
          "must_understand": ["org.vibevm/wal/tombstone@1"],
          "provides": { "capabilities": ["interface:wal"] }
        }
      ],
      "latest_stable": "0.2.0"
    }
  ]
}
"#;

const RUST_BY_NAME: &str = r#"{
  "indexed_at": "2026-08-20T09:00:00Z",
  "name": "rust",
  "packages": [
    {
      "group": "org.vibevm",
      "indexed_at": "2026-08-20T09:00:00Z",
      "name": "rust",
      "versions": [
        {
          "schema_version": 1,
          "kind": "stack",
          "group": "org.vibevm",
          "name": "rust",
          "version": "0.1.0",
          "content_hash": "sha256:7ce1f4b8a2d6e0c3b5a79d1f3e5c7b9a1d3f5e7c9b1a3d5f7e9c1b3a5d7f9e2c4",
          "source_url": "https://gitverse.ru/vibevm/org.vibevm.rust.git",
          "source_ref": "v0.1.0",
          "registry": "vibespecs",
          "files_count": 12,
          "indexed_at": "2026-08-18T11:00:00Z",
          "indexed_by": "vibe-index 0.1.0-dev"
        },
        {
          "schema_version": 1,
          "kind": "stack",
          "group": "org.vibevm",
          "name": "rust",
          "version": "0.2.0",
          "content_hash": "sha256:c1b3a5d7f9e2c416e0a8c2d4f6b8a0c3e5a79d1b3f5e7c9b1a3d5f7e9c1b3a5d7f",
          "source_url": "https://gitverse.ru/vibevm/org.vibevm.rust.git",
          "source_ref": "v0.2.0",
          "registry": "vibespecs",
          "files_count": 15,
          "indexed_at": "2026-08-19T11:00:00Z",
          "indexed_by": "vibe-index 0.1.0-dev"
        }
      ],
      "latest_stable": "0.2.0"
    }
  ]
}
"#;

const SQLX_SKIN_BY_NAME: &str = r#"{
  "indexed_at": "2026-08-20T09:00:00Z",
  "name": "sqlx-skin",
  "packages": [
    {
      "group": "org.vibevm",
      "indexed_at": "2026-08-20T09:00:00Z",
      "name": "sqlx-skin",
      "versions": [
        {
          "schema_version": 1,
          "kind": "feat",
          "group": "org.vibevm",
          "name": "sqlx-skin",
          "version": "0.1.0",
          "content_hash": "sha256:aa11bb22cc33dd44ee55ff6600aa11bb22cc33dd44ee55ff6600aa11bb22cc33dd",
          "source_url": "https://gitverse.ru/vibevm/org.vibevm.sqlx-skin.git",
          "source_ref": "v0.1.0",
          "registry": "vibespecs",
          "files_count": 34,
          "indexed_at": "2026-08-18T12:00:00Z",
          "indexed_by": "vibe-index 0.1.0-dev",
          "description": "Binds to pkg:cargo/sqlx@0.8.0",
          "describes": "pkg:cargo/sqlx@0.8.0",
          "provides": { "capabilities": ["interface:sqlx"] },
          "subskills": [
            {
              "path": "skills/sqlx/skin",
              "delivery": "eager",
              "description": "The cargo sqlx skin"
            }
          ]
        },
        {
          "schema_version": 1,
          "kind": "feat",
          "group": "org.vibevm",
          "name": "sqlx-skin",
          "version": "0.2.0",
          "content_hash": "sha256:bb22cc33dd44ee55ff660011aa22bb33cc44dd55ee66ff77001122bb33cc44dd55",
          "source_url": "https://gitverse.ru/vibevm/org.vibevm.sqlx-skin.git",
          "source_ref": "v0.2.0",
          "registry": "vibespecs",
          "files_count": 38,
          "indexed_at": "2026-08-19T12:00:00Z",
          "indexed_by": "vibe-index 0.1.0-dev",
          "describes": "pkg:cargo/sqlx@0.8.0",
          "provides": { "capabilities": ["interface:sqlx"] },
          "must_understand": ["org.vibevm/subskills/lazy-materialisation@1"]
        },
        {
          "schema_version": 1,
          "kind": "feat",
          "group": "org.vibevm",
          "name": "sqlx-skin",
          "version": "0.3.0",
          "content_hash": "sha256:cc33dd44ee55ff660022bb33cc44dd55ee66ff77001122bb33cc44dd55ee66ff77",
          "source_url": "https://gitverse.ru/vibevm/org.vibevm.sqlx-skin.git",
          "source_ref": "v0.3.0",
          "registry": "vibespecs",
          "files_count": 41,
          "indexed_at": "2026-08-20T07:00:00Z",
          "indexed_by": "vibe-index 0.1.0-dev",
          "subskills": [
            {
              "path": "skills/sqlx/skin",
              "delivery": "lazy-pull",
              "describes": "pkg:cargo/sqlx@0.8.0"
            }
          ]
        }
      ],
      "latest_stable": "0.3.0"
    }
  ]
}
"#;

const LOCKFILE: &str = r#"[[package]]
kind = "flow"
group = "org.vibevm"
name = "wal"
version = "0.2.0"

[[package]]
kind = "stack"
group = "org.vibevm"
name = "rust"
version = "0.1.0"

[[package]]
kind = "feat"
group = "org.vibevm"
name = "sqlx-skin"
version = "0.1.0"

[[package]]
kind = "tool"
group = "org.vibevm"
name = "ghost-pkg"
version = "0.3.0"
"#;

/// Write the fixture index under `root`: `root/index/**` plus the lockfile
/// `root/vibe.lock`. Children run with `root` as their current directory
/// and address the index as `index`, the lockfile as `vibe.lock` — the
/// relative spellings that pin `data_dir` and `lockfile` in the envelopes.
fn write_fixture(root: &Path) {
    let index = root.join("index");
    let by_name = index.join("by-name");
    std::fs::create_dir_all(&by_name).expect("creating the fixture by-name directory");
    std::fs::write(index.join("repomd.json"), REPOMD).expect("writing the fixture repomd");
    std::fs::write(index.join("primary.jsonl"), PRIMARY_JSONL).expect("writing primary.jsonl");
    std::fs::write(index.join("stale-projection.json"), STALE_PROJECTION)
        .expect("writing stale-projection.json");
    std::fs::write(by_name.join("wal.json"), WAL_BY_NAME).expect("writing wal.json");
    std::fs::write(by_name.join("rust.json"), RUST_BY_NAME).expect("writing rust.json");
    std::fs::write(by_name.join("sqlx-skin.json"), SQLX_SKIN_BY_NAME)
        .expect("writing sqlx-skin.json");
    std::fs::write(root.join("vibe.lock"), LOCKFILE).expect("writing vibe.lock");
}

/// Run `vibe-index <args> --json` against the fixture and return its
/// stdout bytes. `verify` is the one verb that exits non-zero on this
/// fixture (the stale and missing manifest rows are deliberate), so the
/// exit status is reported, not asserted — the contract under test here
/// is the bytes, and `verify` prints its full report before failing.
fn run_writer(args: &[&str]) -> Vec<u8> {
    let work = tempfile::tempdir().expect("a scratch directory for the fixture");
    write_fixture(work.path());
    let mut cmd = vibe_test_support::cargo_bin("vibe-index");
    cmd.args(args).arg("--json").current_dir(work.path());
    let output = cmd.output().expect("the vibe-index binary runs");
    assert!(
        !output.stdout.is_empty(),
        "the writer printed nothing for `{args:?}` (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

/// The corpus home this oracle reads its golden documents from.
fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("formats")
        .join("corpora")
        .join("index_cli")
        .join("e1")
}

fn corpus(name: &str) -> Vec<u8> {
    std::fs::read(corpus_dir().join(name))
        .unwrap_or_else(|e| panic!("reading the corpus document `{name}`: {e}"))
}

fn corpus_value(name: &str) -> Value {
    serde_json::from_slice(&corpus(name))
        .unwrap_or_else(|e| panic!("the corpus document `{name}` parses: {e}"))
}

/// One envelope's corpus document through its generated reader and back:
/// the equality is at the `Value` level (key order is not part of the
/// contract — the hand-written writer serialises in declaration order,
/// the generated type alphabetically — but key SET and values are).
fn round_trips<T>(name: &str)
where
    T: for<'de> Deserialize<'de> + serde::Serialize,
{
    let j1 = corpus_value(name);
    let typed: T = serde_json::from_value(j1.clone())
        .unwrap_or_else(|e| panic!("the generated reader accepts `{name}`: {e}"));
    let j2 = serde_json::to_value(&typed)
        .unwrap_or_else(|e| panic!("the generated reader's output serialises (`{name}`): {e}"));
    assert_eq!(
        j1, j2,
        "wire drift between the corpus document `{name}` and its round-trip — \
         a field the schema misses is dropped (or rejected) by the generated reader"
    );
}

/// Half 1: every corpus document survives its generated reader without
/// loss — eight documents over the seven envelopes (`get` carries a
/// found:false twin whose optional region is empty).
#[test]
fn corpus_documents_round_trip_through_the_generated_readers() {
    round_trips::<GetReport>("get_report.json");
    round_trips::<GetReport>("get_report-minimal.json");
    round_trips::<ListReport>("list_report.json");
    round_trips::<SearchReport>("search_report.json");
    round_trips::<CapabilitiesReport>("capabilities_report.json");
    round_trips::<PurlsReport>("purls_report.json");
    round_trips::<OutdatedReport>("outdated_report.json");
    round_trips::<VerifyReport>("verify_report.json");
}

/// Half 2: the real writer emits the corpus bytes — byte for byte, on
/// every envelope. The fixture is authored, so nothing here can drift
/// between machines or runs; if this fails, either the writer or the
/// corpus changed, and the corpus is the one that was frozen.
#[test]
fn the_real_writer_emits_the_corpus_bytes() {
    let cases: [(&str, Vec<&str>); 8] = [
        ("get_report.json", vec!["get", "index", "org.vibevm", "wal"]),
        (
            "get_report-minimal.json",
            vec!["get", "index", "org.vibevm", "definitely-absent"],
        ),
        ("list_report.json", vec!["list", "index"]),
        ("search_report.json", vec!["search", "index", "wal rust"]),
        (
            "capabilities_report.json",
            vec!["capabilities", "index", "interface:sqlx"],
        ),
        (
            "purls_report.json",
            vec!["purls", "index", "pkg:cargo/sqlx@0.8.0"],
        ),
        (
            "outdated_report.json",
            vec!["outdated", "index", "--lockfile", "vibe.lock"],
        ),
        ("verify_report.json", vec!["verify", "index"]),
    ];
    let mut failures: Vec<String> = Vec::new();
    for (name, args) in cases {
        let wrote = run_writer(&args);
        let golden = corpus(name);
        if wrote != golden {
            failures.push(format!(
                "`{name}`: the writer printed {} byte(s), the corpus holds {} — first \
                 divergence at byte {}",
                wrote.len(),
                golden.len(),
                wrote
                    .iter()
                    .zip(golden.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(wrote.len().min(golden.len()))
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "the writer and the corpus disagree:\n  {}",
        failures.join("\n  ")
    );
}

/// Half 3: broken forms are rejected loudly — one negative case per
/// envelope. Unknown FIELDS are not among the cases: these formats are
/// `foreign_parsers = "many"`, whose generated readers are permissive by
/// policy (PROP-044 §4.4 — a foreign parser may be newer than this
/// build); the loud refusal belongs to shape violations, and the two
/// closed inline vocabularies get theirs from an unknown enum VALUE.
#[test]
fn broken_documents_are_rejected_loudly() {
    let get = serde_json::json!({
        "command": "get", "found": "yes", "group": "org.vibevm",
        "name": "wal", "versions": []
    });
    assert!(
        serde_json::from_value::<GetReport>(get).is_err(),
        "a boolean as a string must be refused, not coerced"
    );

    let list = serde_json::json!({
        "command": "list", "registry": "vibespecs", "package_count": "three",
        "returned": 0, "offset": 0, "limit": 50, "packages": []
    });
    assert!(
        serde_json::from_value::<ListReport>(list).is_err(),
        "a count as a string must be refused, not coerced"
    );

    let search = serde_json::json!({
        "command": "search", "query": "wal", "hit_count": 1,
        "hits": [{ "kind": "flow", "name": "wal", "latest_stable": null,
                   "score": "high", "matched_tokens": ["wal"], "description": null }]
    });
    assert!(
        serde_json::from_value::<SearchReport>(search).is_err(),
        "a score as a string must be refused, not coerced"
    );

    let capabilities = serde_json::json!({
        "command": "capabilities", "capability": "interface:sqlx",
        "hit_count": 0, "hits": 7
    });
    assert!(
        serde_json::from_value::<CapabilitiesReport>(capabilities).is_err(),
        "a collection as a number must be refused, not coerced"
    );

    let purls = serde_json::json!({
        "command": "purls", "purl": "pkg:cargo/sqlx@0.8.0", "hit_count": 1,
        "hits": [{ "kind": "feat", "name": "sqlx-skin", "version": "0.1.0",
                   "binding_site": "workspace" }]
    });
    assert!(
        serde_json::from_value::<PurlsReport>(purls).is_err(),
        "an unknown binding_site value must be refused — the vocabulary is closed"
    );

    let outdated = serde_json::json!({
        "command": "outdated", "lockfile": "vibe.lock", "update_available": 0,
        "rows": [{ "kind": "flow", "group": "org.vibevm", "name": "wal",
                   "installed": "0.2.0", "latest": null, "status": "stale" }]
    });
    assert!(
        serde_json::from_value::<OutdatedReport>(outdated).is_err(),
        "an unknown status value must be refused — the vocabulary is closed"
    );

    let verify = serde_json::json!({
        "command": "verify", "data_dir": "index", "registry": "vibespecs",
        "package_count": 3, "version_count": 6, "files_checked": 3,
        "mismatches": [], "missing": [], "ok": "nope"
    });
    assert!(
        serde_json::from_value::<VerifyReport>(verify).is_err(),
        "a verdict as a string must be refused, not coerced"
    );
}
