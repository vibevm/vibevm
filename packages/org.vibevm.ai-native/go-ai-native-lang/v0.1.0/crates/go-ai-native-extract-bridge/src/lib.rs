//! The spawn-and-parse bridge to `tools/go-extract` (protocol 1).
//!
//! One `go run` per batch: the caller hands a project root plus the
//! extractor source, and gets back typed per-file records — conform
//! facts AND the §8 `//spec:` directive markers — parsed from the
//! extractor's NDJSON stream. Every failure mode is its own error
//! class with its own fix surface, because "the gate errored" and
//! "install go" are different conversations.
//!
//! The parse half ([`parse_ndjson`]) is a pure function over recorded
//! output, so the protocol handling is testable without go on the
//! box — the replay tests freeze protocol 1 exactly as a live
//! extractor run emits it.

specmark::scope!("spec://go-ai-native-lang/go/tools/conform-frontend-go#extractor");

use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Protocol revision this bridge speaks. The extractor stamps every
/// record; a mismatch is a [`BridgeError::Protocol`], and bumping it
/// bumps the `go-extract` frontend version, retiring conform cache
/// slots.
pub const PROTOCOL: u64 = 1;

/// The environment override for the go binary — the consumer's PATH
/// `go` is the default; a machine that keeps go off PATH (this
/// project's own dev box keeps it at `C:/opt/go`) points here.
pub const GO_ENV_OVERRIDE: &str = "GO_AI_NATIVE_GO";

/// The bridge's failure taxonomy. Each variant names its fix surface —
/// the Class-F posture carried into the error layer.
#[derive(Debug, Error)]
pub enum BridgeError {
    #[error(
        "violates REQ discipline://go-ai-native-lang/guide#wiring: \
         `go` did not spawn ({source}); fix surface: install go >= 1.24 \
         and put it on PATH (or set GO_AI_NATIVE_GO to the binary) — the \
         Go structural gate parses through the language's own parser"
    )]
    GoMissing { source: std::io::Error },

    #[error(
        "violates REQ discipline://go-ai-native-lang/guide#wiring: \
         go-extract exited {status} ({stderr}); fix surface: re-run with \
         the same arguments to reproduce, then file the extractor bug — a \
         malformed SOURCE file never causes this (B5 degrades it instead)"
    )]
    ExtractorFailed { status: i32, stderr: String },

    #[error(
        "violates REQ discipline://go-ai-native-lang/guide#wiring: \
         extractor record is not protocol {PROTOCOL} ({detail}); fix \
         surface: the installed stack and this binary disagree — re-run \
         `vibe install` so the extractor and the bridge ship together"
    )]
    Protocol { detail: String },
}

/// One `go_unsafe` / `import` / `item` / `file_metrics` record, exactly
/// as the extractor emits it (serde-tagged on `fact`). `Serialize` is
/// symmetric so the oracle relay (go-ai-native-tcg) can re-emit the
/// same vocabulary it received (TCG-PROTOCOL-GO §2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "fact", rename_all = "snake_case")]
pub enum RawFact {
    GoUnsafe {
        kind: String,
        line: u32,
        #[serde(default)]
        reason: Option<String>,
    },
    Import {
        to_path: String,
        line: u32,
    },
    Item {
        kind: String,
        symbol: String,
        line: u32,
        is_exported: bool,
        has_doc_example: bool,
        /// kind=type only: the primitive underlying of a defined type
        /// (`type AccountID string` → `"string"`) — the Go brand
        /// signal (TCG-PROTOCOL-GO §2 `scope.branded`). Absent for
        /// non-type items, aliases, and non-primitive underlyings.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        underlying: Option<String>,
    },
    FileMetrics {
        lines: u32,
    },
}

/// One §8 `//spec:` directive marker. Unlike the TS twin this carries
/// the author-asserted revision `r` — the Go specmap scanner consumes
/// markers straight from this stream (one parser, one vocabulary).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawMarker {
    pub tag: String,
    pub uri: String,
    pub r: Option<u32>,
    pub reason: Option<String>,
    pub symbol: Option<String>,
    pub line: u32,
}

/// One extractor record — one source file.
#[derive(Debug, Clone, Deserialize)]
pub struct FileRecord {
    pub protocol: u64,
    pub file: String,
    pub in_test: bool,
    pub degraded: bool,
    pub facts: Vec<RawFact>,
    pub markers: Vec<RawMarker>,
}

/// Parse an NDJSON stream of extractor records. Pure — the replay
/// tests drive this against recorded output, no go required.
pub fn parse_ndjson(text: &str) -> Result<Vec<FileRecord>, BridgeError> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let record: FileRecord = serde_json::from_str(line).map_err(|e| BridgeError::Protocol {
            detail: format!("unparseable record: {e}"),
        })?;
        if record.protocol != PROTOCOL {
            return Err(BridgeError::Protocol {
                detail: format!("record says protocol {}", record.protocol),
            });
        }
        out.push(record);
    }
    Ok(out)
}

/// The extractor source, compiled into every consumer of this bridge
/// from the package's own `tools/go-extract/extract.go` — the binary is
/// self-contained, and extractor/bridge version skew is impossible
/// because they build from one tree.
pub const EXTRACTOR_SOURCE: &str = include_str!("../../../tools/go-extract/extract.go");

/// The go binary to spawn: the [`GO_ENV_OVERRIDE`] variable when set,
/// else `go` from PATH.
pub fn go_binary() -> String {
    std::env::var(GO_ENV_OVERRIDE).unwrap_or_else(|_| "go".to_string())
}

/// Materialise the embedded extractor under
/// `<project>/target/conform/go-extract/` (content-addressed: re-runs
/// are no-ops, concurrent runs agree) and return its path.
pub fn materialise_extractor(project_root: &Path) -> std::io::Result<std::path::PathBuf> {
    let digest = conform_core::content_hash(EXTRACTOR_SOURCE);
    let short = digest.trim_start_matches("sha256:");
    let dir = project_root
        .join("target")
        .join("conform")
        .join("go-extract");
    let path = dir.join(format!("extract-{}.go", &short[..16.min(short.len())]));
    if !path.exists() {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(&path, EXTRACTOR_SOURCE)?;
    }
    Ok(path)
}

/// Extract exactly one file whose HYPOTHETICAL content is handed in
/// (the overlay form the oracle relay uses, TCG-PROTOCOL-GO §3):
/// `go run extract.go --stdin-file <rel>` with the content on stdin.
pub fn extract_content(
    project_root: &Path,
    extractor: &Path,
    file_rel: &str,
    content: &str,
) -> Result<FileRecord, BridgeError> {
    use std::io::Write;
    let mut cmd = Command::new(go_binary());
    cmd.arg("run")
        .arg(extractor)
        .arg("--root")
        .arg(project_root)
        .arg("--stdin-file")
        .arg(file_rel)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|source| BridgeError::GoMissing { source })?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(content.as_bytes());
    }
    let output = child
        .wait_with_output()
        .map_err(|source| BridgeError::GoMissing { source })?;
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        let status = output.status.code().unwrap_or(-1);
        return Err(BridgeError::ExtractorFailed { status, stderr });
    }
    let mut records = parse_ndjson(&String::from_utf8_lossy(&output.stdout))?;
    records.pop().ok_or_else(|| BridgeError::Protocol {
        detail: "the overlay extraction produced no record".to_string(),
    })
}

/// Run the extractor over `project_root` (optionally narrowed to
/// `files`, repo-relative) and parse its stream.
pub fn extract_tree(
    project_root: &Path,
    extractor: &Path,
    files: Option<&[String]>,
) -> Result<Vec<FileRecord>, BridgeError> {
    let mut cmd = Command::new(go_binary());
    cmd.arg("run").arg(extractor).arg("--root").arg(project_root);
    if let Some(files) = files {
        cmd.arg("--files");
        for file in files {
            cmd.arg(file);
        }
    }
    let output = cmd
        .output()
        .map_err(|source| BridgeError::GoMissing { source })?;
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        let status = output.status.code().unwrap_or(-1);
        return Err(BridgeError::ExtractorFailed { status, stderr });
    }
    parse_ndjson(&String::from_utf8_lossy(&output.stdout))
}

/// Lower one record into the neutral engine's fact model. `in_test` is
/// file-grain (the record's flag), stamped onto every census fact; the
/// importing module for `Import` facts is the file itself — Go package
/// facts are reported per file.
pub fn conform_facts(record: &FileRecord) -> Vec<conform_core::Fact> {
    use conform_core::Fact;
    record
        .facts
        .iter()
        .map(|raw| match raw {
            RawFact::GoUnsafe { kind, line, reason } => Fact::GoUnsafe {
                kind: kind.clone(),
                line: *line,
                in_test: record.in_test,
                reason: reason.clone(),
            },
            RawFact::Import { to_path, line } => Fact::Import {
                from_module: record.file.clone(),
                to_path: to_path.clone(),
                line: *line,
            },
            RawFact::Item {
                kind,
                symbol,
                line,
                is_exported,
                has_doc_example,
                ..
            } => Fact::Item {
                kind: kind.clone(),
                symbol: symbol.clone(),
                line: *line,
                attrs: Vec::new(),
                is_pub: *is_exported,
                has_doctest: *has_doc_example,
            },
            RawFact::FileMetrics { lines } => Fact::FileMetrics { lines: *lines },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorded extractor output — byte-for-byte the shape a live
    /// `go run extract.go --root test/fixtures/dirty` emitted on
    /// 2026-07-17, trimmed to the protocol-bearing essentials, so the
    /// replay freezes the SAME protocol the extractor speaks.
    const REPLAY: &str = concat!(
        r#"{"protocol":1,"file":"internal/cells/plan/plan.go","in_test":false,"degraded":false,"#,
        r#""facts":[{"fact":"file_metrics","lines":52},"#,
        r#"{"fact":"import","to_path":"net/http/pprof","line":13},"#,
        r#"{"fact":"go_unsafe","kind":"blank_import","line":13},"#,
        r#"{"fact":"go_unsafe","kind":"seam_error_missing_req","line":17},"#,
        r#"{"fact":"item","kind":"func","symbol":"Solve","is_exported":true,"has_doc_example":false,"line":31},"#,
        r#"{"fact":"go_unsafe","kind":"ambient_call","reason":"wall clock IS the domain here","line":51}],"#,
        r#""markers":[{"tag":"scope","uri":"spec://demo/PROP-001#cells","r":1,"reason":null,"symbol":null,"line":4},"#,
        r#"{"tag":"deviates","uri":"spec://demo/PROP-001#cells","r":1,"reason":"wall clock IS the domain here","symbol":"Sanctioned","line":49}]}"#,
        "\n",
        r#"{"protocol":1,"file":"internal/cells/plan/plan_test.go","in_test":true,"degraded":false,"#,
        r#""facts":[{"fact":"file_metrics","lines":7},{"fact":"go_unsafe","kind":"t_skip","line":6}],"markers":[]}"#,
        "\n",
    );

    #[test]
    fn replay_parses_and_lowers_into_engine_facts() {
        let records = parse_ndjson(REPLAY).expect("parse");
        assert_eq!(records.len(), 2);
        assert!(records[1].in_test);

        let facts = conform_facts(&records[0]);
        assert_eq!(facts.len(), 6);
        let sanctioned = facts.iter().find(|f| {
            matches!(f, conform_core::Fact::GoUnsafe { kind, reason, .. }
                if kind == "ambient_call" && reason.is_some())
        });
        assert!(sanctioned.is_some(), "deviation testimony must survive lowering");
        let t_skip = conform_facts(&records[1]);
        assert!(t_skip.iter().any(|f| {
            matches!(f, conform_core::Fact::GoUnsafe { kind, in_test, .. }
                if kind == "t_skip" && *in_test)
        }));
        assert_eq!(records[0].markers[1].symbol.as_deref(), Some("Sanctioned"));
        assert_eq!(records[0].markers[0].r, Some(1));
    }

    #[test]
    fn protocol_mismatch_is_its_own_error_class() {
        let bad = r#"{"protocol":2,"file":"a.go","in_test":false,"degraded":false,"facts":[],"markers":[]}"#;
        let err = parse_ndjson(bad).expect_err("protocol 2 must fail");
        assert!(matches!(err, BridgeError::Protocol { .. }));
        assert!(err.to_string().contains("vibe install"));
    }

    #[test]
    fn garbage_line_is_a_protocol_error_naming_the_parse() {
        let err = parse_ndjson("not json\n").expect_err("garbage must fail");
        assert!(matches!(err, BridgeError::Protocol { .. }));
    }

    #[test]
    fn extractor_materialises_content_addressed_and_idempotent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let first = materialise_extractor(tmp.path()).expect("materialise");
        assert!(first.exists());
        let again = materialise_extractor(tmp.path()).expect("again");
        assert_eq!(first, again);
        let body = std::fs::read_to_string(&first).expect("read back");
        assert_eq!(body, EXTRACTOR_SOURCE);
    }
}
