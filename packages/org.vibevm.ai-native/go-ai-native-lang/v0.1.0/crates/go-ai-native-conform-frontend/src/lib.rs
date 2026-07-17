//! `go-ai-native-conform-frontend` — the `go-extract` frontend the
//! conform-frontend-go brief specified: a [`conform_core::Frontend`]
//! whose facts come from `go/parser` + `go/ast`, via the packaged
//! `tools/go-extract` extractor and the `go-ai-native-extract-bridge`
//! protocol.
//!
//! The extractor source is EMBEDDED in the bridge (`include_str!`) and
//! written content-addressed under the consumer's
//! `target/conform/go-extract/` at construction time — the binary is
//! self-contained, and the extractor version can never skew from the
//! frontend version because they compile from one tree.
//!
//! Process economics: `warm()` (the store's batch hook) runs ONE
//! `go run` for every cache-missed file and parks the lowered facts in
//! memory; `extract()` then serves per-file from that cache. A file the
//! store never warmed (defensive path) costs one single-file go run.

specmark::scope!("spec://go-ai-native-lang/go/tools/conform-frontend-go#frontend");

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use conform_core::{Fact, Frontend};

/// The `go-extract` frontend (conform-frontend-go brief §3). Construct
/// once per run.
pub struct GoExtractFrontend {
    project_root: PathBuf,
    extractor: PathBuf,
    warmed: Mutex<HashMap<String, Vec<Fact>>>,
}

impl GoExtractFrontend {
    /// Materialise the bridge's embedded extractor and return the
    /// frontend.
    pub fn new(project_root: &Path) -> Result<GoExtractFrontend> {
        let path =
            go_ai_native_extract_bridge::materialise_extractor(project_root).with_context(
                || format!("materialising go-extract under {}", project_root.display()),
            )?;
        Ok(GoExtractFrontend {
            project_root: project_root.to_path_buf(),
            extractor: path,
            warmed: Mutex::new(HashMap::new()),
        })
    }

    /// Run the extractor for `files` (or the whole tree) and park the
    /// lowered facts. Failures surface on stderr here and again as an
    /// empty fact set per file — the gate itself stays running (B5);
    /// the CLI drivers probe the bridge FIRST so a broken toolchain is
    /// a hard error there, not a silent green here.
    fn warm_batch(&self, files: Option<&[String]>) {
        let records = match go_ai_native_extract_bridge::extract_tree(
            &self.project_root,
            &self.extractor,
            files,
        ) {
            Ok(records) => records,
            Err(error) => {
                eprintln!("conform go-extract: extraction failed — {error}");
                return;
            }
        };
        let mut warmed = match self.warmed.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        for record in &records {
            warmed.insert(
                record.file.clone(),
                go_ai_native_extract_bridge::conform_facts(record),
            );
        }
    }

    /// Probe the extraction path end-to-end (go, the extractor, the
    /// protocol) so drivers can fail hard with the taxonomy's message
    /// BEFORE a gate run silently yields zero facts.
    pub fn probe(&self) -> std::result::Result<(), go_ai_native_extract_bridge::BridgeError> {
        go_ai_native_extract_bridge::extract_tree(&self.project_root, &self.extractor, Some(&[]))
            .map(|_| ())
    }
}

impl Frontend for GoExtractFrontend {
    fn id(&self) -> &'static str {
        "go-extract"
    }
    fn version(&self) -> &'static str {
        // Bump with the extractor protocol / fact schema: retires every
        // cached slot wholesale (the brief's cache contract).
        "1"
    }
    fn warm(&self, pending_files: &[String]) {
        self.warm_batch(Some(pending_files));
    }
    fn extract(&self, file: &str, _crate_name: &str, _module: &str, _text: &str) -> Vec<Fact> {
        {
            let mut warmed = match self.warmed.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            if let Some(facts) = warmed.remove(file) {
                return facts;
            }
        }
        // Defensive single-file path: the store always warms first, but
        // a direct caller may not.
        self.warm_batch(Some(&[file.to_string()]));
        let mut warmed = match self.warmed.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        warmed.remove(file).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extractor_materialises_content_addressed_and_idempotent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let front = GoExtractFrontend::new(tmp.path()).expect("frontend");
        assert!(front.extractor.exists());
        let first = front.extractor.clone();
        let again = GoExtractFrontend::new(tmp.path()).expect("frontend again");
        assert_eq!(first, again.extractor);
        let body = std::fs::read_to_string(&first).expect("read back");
        assert_eq!(body, go_ai_native_extract_bridge::EXTRACTOR_SOURCE);
    }
}
