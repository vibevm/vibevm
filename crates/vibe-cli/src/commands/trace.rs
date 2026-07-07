//! `vibe trace` — the product alias over `discipline-rust trace`
//! (PROP-014 §2.6). A pure delegator: the traceability engine ships in
//! `stack:org.vibevm/rust-ai-native` and versions with the PROJECT's
//! pinned stack, so embedding it into the product binary would let the
//! two skew; spawning the installed binary keeps `vibe trace` exactly
//! as new as the discipline the project actually uses. Arguments pass
//! through verbatim; the exit code passes back.

specmark::scope!("spec://discipline-core/mechanisms/PROP-014#index");

use anyhow::{Result, bail};

/// Delegate `vibe trace <args…>` to `discipline-rust trace <args…>`.
pub fn run(args: &[String]) -> Result<i32> {
    let spawned = std::process::Command::new("discipline-rust")
        .arg("trace")
        .args(args)
        .status();
    match spawned {
        Ok(status) => Ok(status.code().unwrap_or(1)),
        Err(e) => bail!(
            "vibe trace delegates to `discipline-rust` and could not spawn it ({e}).\n\
             Install the discipline toolchain once:\n\
             \x20 cargo install --path vibedeps/<stack-slot>/crates/discipline-cli\n\
             or run it in place:\n\
             \x20 cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml \\\n\
             \x20     -p discipline-cli --bin discipline-rust -- trace <args…>\n\
             (<stack-slot> is e.g. stack-rust-ai-native/<version> — check vibe.lock; \
             TypeScript trees use `discipline-typescript trace` from their stack)"
        ),
    }
}

#[cfg(test)]
mod tests {
    /// With PATH scrubbed the delegator cannot spawn, and the error is
    /// the recipe, not a raw OS code.
    #[test]
    fn missing_binary_yields_the_install_recipe() {
        let orig = std::env::var_os("PATH");
        // SAFETY / test-serial note: mutating PATH is process-global; this
        // is the module's only test, so nothing races it.
        unsafe { std::env::set_var("PATH", "") };
        let err = super::run(&["explain".into(), "spec://x/Y#z".into()])
            .expect_err("spawn must fail on an empty PATH");
        unsafe {
            match orig {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }
        let text = err.to_string();
        assert!(text.contains("cargo install --path"), "{text}");
        assert!(text.contains("discipline-rust"), "{text}");
    }
}
