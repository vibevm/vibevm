//! Index-backed short-name resolution at the CLI input boundary
//! (PROP-008 §2.6).
//!
//! `vibe install wal` — a bare, unqualified pkgref — is resolved here
//! to the qualified `org.vibevm.world/wal` *once*, before the depsolver runs
//! and before the pkgref is merged into `[requires].packages`.
//! Manifests and the lockfile only ever store the qualified form; the
//! short name is CLI sugar (PROP-008 §2.4). Resolution never recurses
//! into the dependency graph — every transitive `[requires.packages]`
//! key is already group-qualified by construction.
//!
//! The lookup consults the lockfile first — a locked package of that
//! name wins outright ("the short name prefers what is already
//! locked", PROP-008 §2.6) — then enumerates registry candidates: a
//! local-directory registry is scanned, a multi-registry resolver
//! walks each registry's index. Two groups under one bare name is a
//! collision the resolver refuses to guess past (PROP-008 §2.7).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-008#short-name");

use anyhow::{Result, bail};
use vibe_core::manifest::Lockfile;
use vibe_core::{Group, PackageRef};

use super::install::InstallResolver;
use crate::exit_code::InstallError;

/// The outcome of resolving one bare package name to its `group`.
enum ShortNameOutcome {
    /// Exactly one `(group, name)` package matched.
    Resolved(Group),
    /// No package of that name was found in any source.
    NotFound,
    /// More than one group publishes the name — a collision
    /// (PROP-008 §2.7). Groups are sorted and de-duplicated; the
    /// variant carries at least two.
    Ambiguous(Vec<Group>),
}

/// De-duplicate a stream of groups and sort the result, so a caller
/// gets a stable candidate list regardless of registry-walk or
/// lockfile order.
fn distinct_sorted<'a>(groups: impl Iterator<Item = &'a Group>) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for g in groups {
        if !out.contains(g) {
            out.push(g.clone());
        }
    }
    out.sort();
    out
}

/// Distinct groups carrying a locked package of `name`.
fn locked_groups(lockfile: &Lockfile, name: &str) -> Vec<Group> {
    distinct_sorted(
        lockfile
            .packages
            .iter()
            .filter(|p| p.name == name)
            .map(|p| &p.group),
    )
}

/// Resolve a bare `name` to its `group`.
///
/// The lockfile is consulted first: a single locked entry of that
/// name wins; two locked entries under different groups are already a
/// collision the lockfile cannot break. With nothing locked, the
/// registries reachable through `resolver` are enumerated.
fn resolve(
    resolver: &InstallResolver,
    name: &str,
    lockfile: &Lockfile,
) -> Result<ShortNameOutcome> {
    let locked = locked_groups(lockfile, name);
    if let [only] = locked.as_slice() {
        return Ok(ShortNameOutcome::Resolved(only.clone()));
    }
    if locked.len() > 1 {
        return Ok(ShortNameOutcome::Ambiguous(locked));
    }
    let candidates = resolver.candidate_groups(name)?;
    if let [only] = candidates.as_slice() {
        return Ok(ShortNameOutcome::Resolved(only.clone()));
    }
    Ok(if candidates.is_empty() {
        ShortNameOutcome::NotFound
    } else {
        ShortNameOutcome::Ambiguous(candidates)
    })
}

/// Render the PROP-008 §2.7 collision message: the ambiguous bare
/// name, a numbered list of the qualified candidates, and a re-run
/// hint. Pure — unit-tested without a registry.
fn render_collision(name: &str, candidates: &[Group]) -> String {
    use std::fmt::Write as _;
    let mut msg = format!(
        "the short name `{name}` is ambiguous — {} packages match it:\n",
        candidates.len(),
    );
    for (i, group) in candidates.iter().enumerate() {
        let _ = writeln!(msg, "  {}. {group}/{name}", i + 1);
    }
    // A collision carries at least two candidates, so `first()` is
    // always `Some` here; render the re-run hint when present and skip
    // it (rather than panic) on the unreachable empty case.
    if let Some(first) = candidates.first() {
        let _ = write!(
            msg,
            "Re-run with the qualified form, e.g. `vibe install {first}/{name}`.",
        );
    }
    msg
}

/// Qualify one CLI-supplied pkgref. A pkgref that already carries a
/// `group` passes through untouched. A bare pkgref is resolved
/// through [`resolve`]; the discovered `group` is spliced in, with
/// `kind`, `name`, and `version` preserved.
///
/// An unresolvable or ambiguous short name fails the command — the
/// resolver never guesses (PROP-008 §2.7).
pub fn qualify(
    resolver: &InstallResolver,
    pkgref: &PackageRef,
    lockfile: &Lockfile,
) -> Result<PackageRef> {
    if pkgref.is_qualified() {
        return Ok(pkgref.clone());
    }
    match resolve(resolver, &pkgref.name, lockfile)? {
        ShortNameOutcome::Resolved(group) => Ok(PackageRef {
            kind: pkgref.kind,
            group: Some(group),
            name: pkgref.name.clone(),
            version: pkgref.version.clone(),
        }),
        ShortNameOutcome::NotFound => bail!(
            "could not resolve the short name `{name}` — no package of that name \
             is in `vibe.lock` or any configured registry's index. If a registry \
             has no package index, short names cannot be enumerated against it; \
             give the qualified form instead, e.g. `vibe install <group>/{name}`.",
            name = pkgref.name,
        ),
        // PROP-008 §2.7 — a short name matching two groups is a
        // collision the resolver refuses to guess past. It carries a
        // dedicated exit code (`7`, via `InstallError::AmbiguousPackage`)
        // distinct from a `3` dependency conflict, and the rendered
        // message lists the qualified alternatives so the operator can
        // pick one.
        ShortNameOutcome::Ambiguous(groups) => {
            Err(InstallError::AmbiguousPackage(render_collision(&pkgref.name, &groups)).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(s: &str) -> Group {
        Group::parse(s).unwrap()
    }

    #[test]
    fn distinct_sorted_dedups_and_orders() {
        let input = [g("org.vibevm"), g("com.acme"), g("org.vibevm")];
        assert_eq!(
            distinct_sorted(input.iter()),
            vec![g("com.acme"), g("org.vibevm")]
        );
    }

    #[test]
    fn distinct_sorted_empty_is_empty() {
        assert!(distinct_sorted(std::iter::empty::<&Group>()).is_empty());
    }

    #[test]
    fn distinct_sorted_single_group_collapses() {
        let input = [g("org.vibevm"), g("org.vibevm"), g("org.vibevm")];
        assert_eq!(distinct_sorted(input.iter()), vec![g("org.vibevm")]);
    }

    #[test]
    fn render_collision_numbers_every_candidate() {
        let msg = render_collision("wal", &[g("com.acme"), g("org.vibevm")]);
        assert!(
            msg.contains("`wal` is ambiguous — 2 packages match"),
            "missing the header line:\n{msg}"
        );
        assert!(msg.contains("  1. com.acme/wal"), "missing item 1:\n{msg}");
        assert!(
            msg.contains("  2. org.vibevm.world/wal"),
            "missing item 2:\n{msg}"
        );
        assert!(
            msg.contains("`vibe install com.acme/wal`"),
            "missing the re-run hint:\n{msg}"
        );
    }
}
