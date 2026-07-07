//! `cargo xtask specmap` — regenerate (or `--check`) the canonical
//! `specmap.json` traceability index (PROP-014 §2.5), plus the orphan ratchet
//! gate. A thin shim over the `specmap-cli-rust` library, which ships in
//! stack:org.vibevm/rust-ai-native-lang (PROP-024 code-bearing packages) — the same
//! relationship `cargo xtask conform` has with `conform-cli-rust`.

use anyhow::Result;

use crate::repo_root;

pub(crate) fn run_specmap(check: bool) -> Result<()> {
    rust_ai_native_specmap::run_specmap(&repo_root()?, check)
}
