//! `vibe trace` — the product alias over `rust-ai-native trace`
//! (PROP-014 §2.6). A pure delegator: the traceability engine ships in
//! `stack:org.vibevm/rust-ai-native-lang` and versions with the PROJECT's
//! pinned stack, so embedding it into the product binary would let the
//! two skew; spawning the installed binary keeps `vibe trace` exactly
//! as new as the discipline the project actually uses. Arguments pass
//! through verbatim; the exit code passes back.

specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#index");

use anyhow::{Result, bail};

/// Delegate `vibe trace <args…>` to `rust-ai-native trace <args…>`.
pub fn run(args: &[String]) -> Result<i32> {
    run_with("rust-ai-native", args)
}

/// The delegation seam, binary-parameterised so the missing-binary path
/// is testable without mutating the process environment.
fn run_with(binary: &str, args: &[String]) -> Result<i32> {
    let spawned = std::process::Command::new(binary)
        .arg("trace")
        .args(args)
        .status();
    match spawned {
        Ok(status) => Ok(status.code().unwrap_or(1)),
        Err(e) => bail!(
            "vibe trace delegates to `rust-ai-native` and could not spawn it ({e}).\n\
             Install the discipline toolchain once:\n\
             \x20 cargo install --path vibedeps/<stack-slot>/crates/rust-ai-native-cli\n\
             or run it in place:\n\
             \x20 cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml \\\n\
             \x20     -p rust-ai-native-cli --bin rust-ai-native -- trace <args…>\n\
             (<stack-slot> is e.g. stack-rust-ai-native-lang/<version> — check vibe.lock; \
             TypeScript trees use `typescript-ai-native trace` from their stack)"
        ),
    }
}

#[cfg(test)]
mod tests {
    /// A binary that cannot exist anywhere: the spawn fails and the
    /// error is the recipe, not a raw OS code.
    #[test]
    fn missing_binary_yields_the_install_recipe() {
        let err = super::run_with(
            "vibe-trace-test-missing-binary-2f6e",
            &["explain".into(), "spec://x/Y#z".into()],
        )
        .expect_err("spawn must fail for a nonexistent binary");
        let text = err.to_string();
        assert!(text.contains("cargo install --path"), "{text}");
        assert!(text.contains("rust-ai-native"), "{text}");
    }
}
