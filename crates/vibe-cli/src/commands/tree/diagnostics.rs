//! Non-fatal diagnostics over the resolved tree (PROP-036 §2.10).
//!
//! These never abort rendering — each is a fact the human needs, surfaced
//! alongside the tree. Today one check lands: **root-drift**, the lockfile's
//! recorded `meta.root_dependencies` versus the root `vibe.toml`
//! `[requires.packages]`. The **stale-artifacts** check (the committed
//! `STATIC.md` / `INDEX.md` lanes versus a fresh `EffectiveBoot` recompute)
//! is deferred — it needs the full boot recompute.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#diagnostics");

use std::collections::BTreeSet;

use vibe_core::manifest::Lockfile;

use super::model::{Diagnostic, Severity};

/// The REQ every diagnostic in this module enforces (PROP-036 §2.10). Each
/// `Diagnostic` cites it via `spec_ref` (scaffold-F) so the surfaced fact
/// carries a jump to its governing contract.
const DIAGNOSTICS_REQ: &str = "spec://vibevm/modules/vibe-cli/PROP-036#diagnostics";

/// Compute the non-fatal diagnostics for a resolved tree (PROP-036 §2.10).
///
/// `roots` is the manifest `[requires.packages]` projected to `group/name`
/// (the same set the model exposes as [`super::model::PackageTree::roots`]).
pub fn check(roots: &[String], lockfile: &Lockfile) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if let Some(drift) = root_drift(roots, lockfile) {
        out.push(drift);
    }
    out
}

/// The **root-drift** diagnostic (PROP-036 §2.10): compare the manifest roots
/// against the lockfile's recorded `meta.root_dependencies` (each mapped to
/// its `group/name` via [`vibe_core::PackageRef::qualified_name`]). On any set
/// difference emit exactly one `Warn`; `None` when the two sets agree.
fn root_drift(roots: &[String], lockfile: &Lockfile) -> Option<Diagnostic> {
    let manifest_roots: BTreeSet<String> = roots.iter().cloned().collect();
    let lock_roots: BTreeSet<String> = lockfile
        .meta
        .root_dependencies
        .iter()
        .map(|r| r.qualified_name())
        .collect();
    let drift = manifest_roots.symmetric_difference(&lock_roots).count();
    if drift == 0 {
        return None;
    }
    Some(Diagnostic {
        severity: Severity::Warn,
        code: "root-drift".to_string(),
        message: format!(
            "{drift} lock root(s) differ from vibe.toml [requires.packages]; the lockfile may be behind \u{2014} run vibe install"
        ),
        locator: Some("vibe.lock meta.root_dependencies".to_string()),
        spec_ref: DIAGNOSTICS_REQ.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_core::PackageRef;

    /// An empty lockfile whose recorded root set is `refs` (as `group/name`).
    fn lock_with_roots(refs: &[&str]) -> Lockfile {
        let mut lf = Lockfile::empty("vibe (test)", "2026-01-01T00:00:00Z");
        lf.meta.root_dependencies = refs
            .iter()
            .map(|r| PackageRef::parse(r).expect("valid pkgref"))
            .collect();
        lf
    }

    #[test]
    fn a_disjoint_root_set_yields_exactly_one_warn() {
        let lockfile = lock_with_roots(&["org.vibevm/b"]);
        let out = check(&["org.vibevm/a".to_string()], &lockfile);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].code, "root-drift");
        assert_eq!(out[0].severity, Severity::Warn);
        // {a} △ {b} = {a, b} → 2 differing roots.
        assert!(out[0].message.starts_with("2 "), "{}", out[0].message);
        assert_eq!(
            out[0].locator.as_deref(),
            Some("vibe.lock meta.root_dependencies")
        );
        // scaffold-F: the diagnostic cites the REQ it enforces.
        assert_eq!(out[0].spec_ref, DIAGNOSTICS_REQ);
        assert!(out[0].spec_ref.starts_with("spec://"));
    }

    #[test]
    fn an_equal_root_set_yields_no_diagnostic() {
        // Order does not matter — the comparison is set-wise.
        let lockfile = lock_with_roots(&["org.vibevm/a", "org.vibevm/b"]);
        let out = check(
            &["org.vibevm/b".to_string(), "org.vibevm/a".to_string()],
            &lockfile,
        );
        assert!(out.is_empty());
    }

    #[test]
    fn one_missing_lock_root_counts_as_one() {
        // The manifest declares two roots; the lock records only one.
        let lockfile = lock_with_roots(&["org.vibevm/a"]);
        let out = check(
            &["org.vibevm/a".to_string(), "org.vibevm/b".to_string()],
            &lockfile,
        );
        assert_eq!(out.len(), 1);
        assert!(out[0].message.starts_with("1 "), "{}", out[0].message);
    }
}
