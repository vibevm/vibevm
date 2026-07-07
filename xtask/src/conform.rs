//! `cargo xtask conform …` — a thin shim over the `conform-cli-rust` library,
//! which now ships in stack:org.vibevm/rust-ai-native (PROP-024 code-bearing
//! packages). The fact engine, the rule set, and the policy model all live in
//! the package; this shim only resolves the vibevm repo root and delegates,
//! and it keeps the vibevm-specific "every crate is gated or exempt" invariant
//! test (which is about vibevm's own `crates/` + `conform.toml`, not the
//! engine).

use anyhow::Result;

use crate::repo_root;

pub(crate) fn run_conform_check(baseline_rel: &str, scope: Option<&str>) -> Result<()> {
    conform_cli_rust::run_check(&repo_root()?, baseline_rel, scope)
}

pub(crate) fn run_conform_freeze(baseline_rel: &str) -> Result<()> {
    conform_cli_rust::run_freeze(&repo_root()?, baseline_rel)
}

#[cfg(test)]
mod tests {
    /// Every workspace crate is classified exactly once — gated or
    /// exempt-with-a-reason, never both and never neither. The invariant
    /// itself now lives in the engine (`Config::validate_against_tree`,
    /// enforced on every `conform-rust check` for every consumer); this
    /// test keeps vibevm's own policy honest at `cargo test` time too.
    #[test]
    fn every_crate_is_gated_or_exempt() {
        let root = crate::repo_root().expect("repo root");
        let config = conform_cli_rust::load_config(&root).expect("load conform.toml");
        config
            .validate_against_tree(&root)
            .expect("vibevm's conform.toml violates the gated-or-exempt tree invariant");
    }
}
