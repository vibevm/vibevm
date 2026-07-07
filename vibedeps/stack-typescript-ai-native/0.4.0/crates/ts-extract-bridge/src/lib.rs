//! The spawn-and-parse bridge to `tools/ts-extract` (protocol 1).
//!
//! One node run per batch: the caller hands a project root (where the
//! consumer's `typescript` resolves) plus the extractor script, and gets
//! back typed per-file records — conform facts AND the §9 JSDoc spec
//! markers — parsed from the extractor's NDJSON stream. Every failure
//! mode is its own error class with its own fix surface, because "the
//! gate errored" and "install node" are different conversations.
//!
//! The parse half ([`parse_ndjson`]) is a pure function over recorded
//! output, so the protocol handling is testable without node on the
//! box — the replay tests freeze protocol 1 exactly as the extractor's
//! own node:test suite emits it.

use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Protocol revision this bridge speaks. The extractor stamps every
/// record; a mismatch is a [`BridgeError::Protocol`], and bumping it
/// bumps the `ts-tsc` frontend version, retiring conform cache slots.
pub const PROTOCOL: u64 = 1;

/// The bridge's failure taxonomy. Each variant names its fix surface —
/// the Class-F posture (`violates REQ …; fix surface: …`) carried into
/// the error layer.
#[derive(Debug, Error)]
pub enum BridgeError {
    #[error(
        "violates REQ discipline://typescript-ai-native/guide#tooling: \
         `node` did not spawn ({source}); fix surface: install node >= 22.6 \
         and put it on PATH — the TypeScript structural gate parses through \
         the project's own compiler"
    )]
    NodeMissing { source: std::io::Error },

    #[error(
        "violates REQ discipline://typescript-ai-native/guide#tooling: \
         the project cannot resolve `typescript` ({stderr}); fix surface: \
         `npm install -D typescript` in the project root — the tsc floor \
         step needs the same install"
    )]
    TypescriptUnresolvable { stderr: String },

    #[error(
        "violates REQ discipline://typescript-ai-native/guide#tooling: \
         ts-extract exited {status} ({stderr}); fix surface: re-run with the \
         same arguments to reproduce, then file the extractor bug — a \
         malformed SOURCE file never causes this (B5 degrades it instead)"
    )]
    ExtractorFailed { status: i32, stderr: String },

    #[error(
        "violates REQ discipline://typescript-ai-native/guide#tooling: \
         extractor record is not protocol {PROTOCOL} ({detail}); fix \
         surface: the installed stack and this binary disagree — re-run \
         `vibe install` so the extractor and the bridge ship together"
    )]
    Protocol { detail: String },
}

/// One `ts_unsafe` / `import` / `item` / `file_metrics` record, exactly
/// as the extractor emits it (serde-tagged on `fact`). `Serialize` is
/// symmetric so the oracle relay (tcg-oracle-bridge) can re-emit the
/// same vocabulary it received (TCG-PROTOCOL §2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "fact", rename_all = "snake_case")]
pub enum RawFact {
    TsUnsafe {
        kind: String,
        line: u32,
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
    },
    FileMetrics {
        lines: u32,
    },
}

/// One §9 JSDoc spec marker (`@implements spec://…` and friends).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawMarker {
    pub tag: String,
    pub uri: String,
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
/// tests drive this against recorded output, no node required.
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
/// from the package's own `tools/ts-extract/extract.ts` — the binary is
/// self-contained, and extractor/bridge version skew is impossible
/// because they build from one tree.
pub const EXTRACTOR_SOURCE: &str = include_str!("../../../tools/ts-extract/extract.ts");

/// Materialise the embedded extractor under
/// `<project>/target/conform/ts-extract/` (content-addressed: re-runs
/// are no-ops, concurrent runs agree) and return its path.
pub fn materialise_extractor(project_root: &Path) -> std::io::Result<std::path::PathBuf> {
    let digest = conform_core::content_hash(EXTRACTOR_SOURCE);
    let short = digest.trim_start_matches("sha256:");
    let dir = project_root
        .join("target")
        .join("conform")
        .join("ts-extract");
    let path = dir.join(format!("extract-{}.ts", &short[..16.min(short.len())]));
    if !path.exists() {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(&path, EXTRACTOR_SOURCE)?;
    }
    Ok(path)
}

/// Run the extractor over `project_root` (optionally narrowed to
/// `files`, repo-relative) and parse its stream.
pub fn extract_tree(
    project_root: &Path,
    extractor: &Path,
    files: Option<&[String]>,
) -> Result<Vec<FileRecord>, BridgeError> {
    let mut cmd = Command::new("node");
    cmd.arg(extractor).arg("--root").arg(project_root);
    if let Some(files) = files {
        cmd.arg("--files");
        for file in files {
            cmd.arg(file);
        }
    }
    let output = cmd
        .output()
        .map_err(|source| BridgeError::NodeMissing { source })?;
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        let status = output.status.code().unwrap_or(-1);
        if status == 3 {
            return Err(BridgeError::TypescriptUnresolvable { stderr });
        }
        return Err(BridgeError::ExtractorFailed { status, stderr });
    }
    parse_ndjson(&String::from_utf8_lossy(&output.stdout))
}

/// Lower one record into the neutral engine's fact model. `in_test` is
/// file-grain (the record's flag), stamped onto every unsafe fact; the
/// importing module for `Import` facts is the file itself — TypeScript
/// modules ARE paths.
pub fn conform_facts(record: &FileRecord) -> Vec<conform_core::Fact> {
    use conform_core::Fact;
    record
        .facts
        .iter()
        .map(|raw| match raw {
            RawFact::TsUnsafe { kind, line, reason } => Fact::TsUnsafe {
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

    /// Recorded extractor output — byte-for-byte the shape the node:test
    /// suite in tools/ts-extract asserts, so the two suites freeze the
    /// SAME protocol from both ends.
    const REPLAY: &str = concat!(
        r#"{"protocol":1,"file":"src/a.ts","in_test":false,"degraded":false,"#,
        r#""facts":[{"fact":"file_metrics","lines":10},"#,
        r#"{"fact":"import","to_path":"../greet/internal.js","line":2},"#,
        r#"{"fact":"ts_unsafe","kind":"any_type","line":7,"reason":null},"#,
        r#"{"fact":"ts_unsafe","kind":"ts_expect_error","line":11,"reason":"why"},"#,
        r#"{"fact":"item","kind":"function","symbol":"parse","line":5,"is_exported":true,"has_doc_example":false}],"#,
        r#""markers":[{"tag":"implements","uri":"spec://demo/PROP-001#req","reason":null,"symbol":"parse","line":4}]}"#,
        "\n",
        r#"{"protocol":1,"file":"src/a.test.ts","in_test":true,"degraded":false,"facts":[{"fact":"file_metrics","lines":3}],"markers":[]}"#,
        "\n",
    );

    #[test]
    fn replay_parses_and_lowers_into_engine_facts() {
        let records = parse_ndjson(REPLAY).expect("parse");
        assert_eq!(records.len(), 2);
        assert!(records[1].in_test);

        let facts = conform_facts(&records[0]);
        assert_eq!(facts.len(), 5);
        let unsafe_any = facts.iter().find(|f| {
            matches!(f, conform_core::Fact::TsUnsafe { kind, in_test, .. }
                if kind == "any_type" && !in_test)
        });
        assert!(unsafe_any.is_some());
        let import = facts.iter().find(|f| {
            matches!(f, conform_core::Fact::Import { from_module, .. }
                if from_module == "src/a.ts")
        });
        assert!(import.is_some());
        assert_eq!(records[0].markers[0].uri, "spec://demo/PROP-001#req");
    }

    #[test]
    fn protocol_mismatch_is_its_own_error_class() {
        let bad = r#"{"protocol":2,"file":"a.ts","in_test":false,"degraded":false,"facts":[],"markers":[]}"#;
        let err = parse_ndjson(bad).expect_err("protocol 2 must fail");
        assert!(matches!(err, BridgeError::Protocol { .. }));
        assert!(err.to_string().contains("vibe install"));
    }

    #[test]
    fn garbage_line_is_a_protocol_error_naming_the_parse() {
        let err = parse_ndjson("not json\n").expect_err("garbage must fail");
        assert!(matches!(err, BridgeError::Protocol { .. }));
    }
}
