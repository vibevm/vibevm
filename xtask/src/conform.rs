//! `cargo xtask conform …` — a thin shim over the `conform-cli` library,
//! which now ships in stack:org.vibevm/rust-ai-native (PROP-024 code-bearing
//! packages). The fact engine, the rule set, and the policy model all live in
//! the package; this shim only resolves the vibevm repo root and delegates,
//! and it keeps the vibevm-specific "every crate is gated or exempt" invariant
//! test (which is about vibevm's own `crates/` + `conform.toml`, not the
//! engine).

use std::path::Path;

use anyhow::Result;
use conform_core::Config;

use crate::repo_root;

/// Load vibevm's conform policy (`conform.toml` at the repo root). Kept here so
/// `health.rs` reads the policy through one path; delegates to the engine.
pub(crate) fn load_config(root: &Path) -> Result<Config> {
    conform_cli::load_config(root)
}

pub(crate) fn run_conform_check(baseline_rel: &str, scope: Option<&str>) -> Result<()> {
    conform_cli::run_check(&repo_root()?, baseline_rel, scope)
}

pub(crate) fn run_conform_freeze(baseline_rel: &str) -> Result<()> {
    conform_cli::run_freeze(&repo_root()?, baseline_rel)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    /// Every workspace crate is classified exactly once — gated by the
    /// config's `gated_crates` or exempt-with-a-reason by `[[exempt]]`,
    /// never both and never neither. This turns the exemption *table*
    /// into an enforced *invariant*: add a crate and forget to place it,
    /// or delete one and leave a phantom entry, and this fails.
    #[test]
    fn every_crate_is_gated_or_exempt() {
        let root = crate::repo_root().expect("repo root");
        let config = super::load_config(&root).expect("load conform.toml");

        let gated: BTreeSet<&str> = config.gated_crates.iter().map(|s| s.as_str()).collect();
        let exempt: BTreeSet<&str> = config
            .exempt
            .iter()
            .map(|e| e.crate_name.as_str())
            .collect();

        assert_eq!(
            gated.len(),
            config.gated_crates.len(),
            "gated_crates carries a duplicate crate name"
        );
        assert_eq!(
            exempt.len(),
            config.exempt.len(),
            "exempt carries a duplicate crate name"
        );

        let both: Vec<&str> = gated.intersection(&exempt).copied().collect();
        assert!(both.is_empty(), "crates both gated and exempt: {both:?}");

        for e in &config.exempt {
            assert!(
                !e.reason.trim().is_empty(),
                "{} is exempt without a recorded reason — the one thing this \
                 table exists to forbid",
                e.crate_name
            );
        }

        // Coverage against the real workspace: every crate dir under
        // `crates/` is named in exactly one set, and every listed name
        // except the workspace-root `xtask` is a real crate (no typos).
        let crates_dir = root.join("crates");
        let mut on_disk: BTreeSet<String> = BTreeSet::new();
        for entry in std::fs::read_dir(&crates_dir).expect("read crates/") {
            let entry = entry.expect("dir entry");
            if entry.file_type().expect("file type").is_dir()
                && entry.path().join("Cargo.toml").exists()
            {
                on_disk.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
        for c in &on_disk {
            assert!(
                gated.contains(c.as_str()) || exempt.contains(c.as_str()),
                "crate `{c}` is neither gated nor exempt — classify it in conform.toml"
            );
        }
        for c in gated.union(&exempt) {
            if *c == "xtask" {
                continue; // the tooling crate lives at the workspace root, not under crates/
            }
            assert!(
                on_disk.contains(*c),
                "`{c}` is listed in conform.toml but has no crates/{c} directory — typo?"
            );
        }
    }
}
