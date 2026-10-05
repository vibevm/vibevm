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

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#index");

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

/// The project's default scanner set — every built-in language the engine
/// knows, composed through [`CompositeScanner`]. The single construction
/// site [`crate::index::build`] / [`write`](crate::index::write) /
/// [`check`](crate::index::check) share, so the three entry points cannot
/// diverge on which scanners feed the index (a divergence of two
/// implementations of one law is silent by nature, and this project has
/// already paid for one). Today: Rust (`#[spec]` tags via [`RustScanner`])
/// and JTD schemas (`metadata.spec` via [`crate::jtd::JtdScanner`]). The
/// JTD scanner is a no-op when
/// [`schema_roots`](crate::config::Config::schema_roots) is empty, so a
/// project with no schema roots is byte-stable against the Rust-only scan.
pub struct DefaultScanner {
    rust: RustScanner,
    jtd: crate::jtd::JtdScanner,
}

impl DefaultScanner {
    /// The shared construction site. Stateless scanners — the policy is read
    /// at `scan` time — so construction takes no [`Config`].
    pub fn new() -> Self {
        Self {
            rust: RustScanner,
            jtd: crate::jtd::JtdScanner,
        }
    }
}

impl Default for DefaultScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeScanner for DefaultScanner {
    fn id(&self) -> &'static str {
        "default"
    }
    fn scan(&self, root: &Path, cfg: &Config) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
        // Compose through `CompositeScanner` at call time — the borrows are
        // valid for the call, so no self-referential reference is stored on
        // the struct. The index sorts and dedups downstream, so the
        // contribution order never leaks into the committed bytes.
        let scanners: Vec<&dyn CodeScanner> = vec![&self.rust, &self.jtd];
        CompositeScanner::new(scanners).scan(root, cfg)
    }
}
