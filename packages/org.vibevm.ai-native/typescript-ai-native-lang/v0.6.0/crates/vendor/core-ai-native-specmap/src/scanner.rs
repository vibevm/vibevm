//! The code-scanner seam (DEFERRALS-CLOSEOUT D3) — the specmap analog
//! of conform's `Frontend` trait. The index builder consumes scanners
//! through this trait; `RustScanner` (rscan over specmark tags) is the
//! built-in implementation, and per-language stacks ship their own
//! (`typescript-ai-native-specmap-scan` reads the §9 JSDoc markers through the
//! ts-extract bridge). The neutral core never learns about node.
//!
//! `CompositeScanner` is the canonical mixed-tree shape: one index,
//! several languages, each scanner contributing its `(items, edges,
//! warnings)` triple — the index builder sorts and dedups downstream,
//! so contribution order never leaks into the committed bytes.

specmark::scope!("spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#index");

use std::path::Path;

use crate::config::Config;
use crate::generated::specmap::{CodeItem, Edge, Warning};
use crate::rscan;

/// One language's code scan: items + edges + warnings for the tree at
/// `root` under the policy `cfg`.
pub trait CodeScanner {
    /// A short identifier for diagnostics (`rust-specmark`, `ts-tsc`).
    fn id(&self) -> &'static str;
    fn scan(&self, root: &Path, cfg: &Config) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>);
}

/// The built-in Rust scanner: specmark `#[spec]` / `#[verifies]` /
/// `scope!` tags over `syn`, exactly the scan `index::build` has always
/// run — [`crate::index::build`] delegates here, so Rust-only trees
/// stay byte-stable through the seam introduction.
pub struct RustScanner;

impl CodeScanner for RustScanner {
    fn id(&self) -> &'static str {
        "rust-specmark"
    }
    fn scan(&self, root: &Path, cfg: &Config) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
        rscan::scan_workspace(root, cfg)
    }
}

/// Several scanners, one index — the mixed-tree composition.
pub struct CompositeScanner<'a> {
    scanners: Vec<&'a dyn CodeScanner>,
}

impl<'a> CompositeScanner<'a> {
    pub fn new(scanners: Vec<&'a dyn CodeScanner>) -> CompositeScanner<'a> {
        CompositeScanner { scanners }
    }
}

impl CodeScanner for CompositeScanner<'_> {
    fn id(&self) -> &'static str {
        "composite"
    }
    fn scan(&self, root: &Path, cfg: &Config) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
        let mut items = Vec::new();
        let mut edges = Vec::new();
        let mut warnings = Vec::new();
        for scanner in &self.scanners {
            let (mut i, mut e, mut w) = scanner.scan(root, cfg);
            items.append(&mut i);
            edges.append(&mut e);
            warnings.append(&mut w);
        }
        (items, edges, warnings)
    }
}
