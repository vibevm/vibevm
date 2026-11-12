//! Index-backed short-name resolution at the CLI input boundary
//! (PROP-008 §2.6).
//!
//! `vibe install wal` — a bare, unqualified pkgref — is resolved here
//! to the qualified `org.vibevm/wal` *once*, before the depsolver runs
//! and before the pkgref is merged into `[requires].packages`.
//! Manifests and the lockfile only ever store the qualified form; the
//! short name is CLI sugar (PROP-008 §2.4). Resolution never recurses
//! into the dependency graph — every transitive `[requires.packages]`
//! key is already group-qualified by construction.
//!
//! **A short name is one a registry ISSUED.** The owner's ruling of
//! 2026-11-12: «если мы разрешим назначать короткие имена держателям
//! пакетов, то всё сломается, потому что все будут бороться за эти
//! короткие имена. Это то же самое что с доменами в интернете - мы не
//! можем дать людям самим выбирать любое имя.» So the registries'
//! alias tables are the whole of the registry tier
//! (`vibe_index::aliases`, written by the registry OWNER and never by a
//! package author), and a package merely *called* `opencode` is not a
//! candidate for the spelling `opencode`. The groups that publish the
//! name are still enumerated — the refusal prints them, so an operator
//! can copy a qualified name out of the message — but nothing picks
//! one.
//!
//! **What still resolves locally.** The lockfile is consulted first: a
//! locked package of that name wins outright ("the short name prefers
//! what is already locked", `##LOCKFILE-AUTHORITATIVE`). That tier is
//! untouched by the ruling, which is about a registry-wide namespace:
//! `vibe.lock` is this project's own record of what it installed, under
//! names it already qualified once, and resolving against it asks no
//! registry anything.
//!
//! Two issued targets under one spelling is a collision the resolver
//! refuses to guess past (PROP-008 §2.7, exit `7`); two registries
//! issuing the same target are one candidate.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name");

use anyhow::{Result, bail};
use vibe_core::manifest::Lockfile;
use vibe_core::{Group, PackageRef};
use vibe_registry::short_name::AliasCandidate;

use super::install::InstallResolver;
use crate::exit_code::InstallError;

/// The outcome of resolving one bare package name.
enum ShortNameOutcome {
    /// Exactly one `(group, name)` package matched — from `vibe.lock`,
    /// the one tier that still answers a bare name without an issued
    /// spelling behind it.
    Resolved(Group),
    /// Exactly one registry had ISSUED this spelling (PROP-008 §2.6):
    /// the name stands for that package, and this is it. The whole of
    /// the registry tier since the owner's 2026-11-12 ruling.
    ResolvedAlias(Box<AliasCandidate>),
    /// No registry issued the spelling. The variant carries what the
    /// refusal needs: the qualified packages that merely BEAR the name
    /// (a hint to copy, never a choice), and the registries consulted
    /// and deliberately skipped.
    NotIssued(NotIssued),
    /// More than one candidate matches the name — a collision
    /// (PROP-008 §2.7). The variant carries at least two qualified
    /// spellings, sorted and de-duplicated, and the form to re-run with.
    Ambiguous(Vec<String>, String),
}

/// What a refusal says when no consulted registry issued the spelling.
struct NotIssued {
    /// `<group>/<name>` for every package that bears the bare name —
    /// the hint, printed so the operator can copy one, never picked.
    hint: Vec<String>,
    /// The registries that were walked, in declared order.
    consulted: Vec<String>,
    /// The registries the effective source policy leaves declared and
    /// never contacts.
    skipped: Vec<String>,
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
/// registries reachable through `resolver` are asked which spellings
/// they have ISSUED — and only that.
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
        return Ok(ShortNameOutcome::Ambiguous(
            qualified(&locked, name),
            format!("{}/{name}", locked[0]),
        ));
    }
    let found = resolver.short_name_candidates(name)?;
    match found.aliases.as_slice() {
        [] => Ok(ShortNameOutcome::NotIssued(NotIssued {
            hint: qualified(&found.named, name),
            consulted: resolver.consulted_registries(),
            skipped: resolver.policy_skipped_registries(),
        })),
        [only] => Ok(ShortNameOutcome::ResolvedAlias(Box::new(only.clone()))),
        many => Ok(ShortNameOutcome::Ambiguous(
            many.iter().map(AliasCandidate::provenance).collect(),
            many[0].target.to_string(),
        )),
    }
}

/// `<group>/<name>` for each group — the spellings a collision lists.
fn qualified(groups: &[Group], name: &str) -> Vec<String> {
    groups.iter().map(|g| format!("{g}/{name}")).collect()
}

/// Render the PROP-008 §2.7 collision message: the ambiguous bare
/// name, a numbered list of the qualified candidates, and a re-run
/// hint. Pure — unit-tested without a registry.
fn render_collision(name: &str, candidates: &[String], hint: &str) -> String {
    use std::fmt::Write as _;
    let mut msg = format!(
        "the short name `{name}` is ambiguous — {} packages match it:\n",
        candidates.len(),
    );
    for (i, spelling) in candidates.iter().enumerate() {
        let _ = writeln!(msg, "  {}. {spelling}", i + 1);
    }
    let _ = write!(
        msg,
        "Re-run with the qualified form, e.g. `vibe install {hint}`.",
    );
    msg
}

/// The refusal for a spelling no registry issued (PROP-008 §2.6
/// `##ISSUED-NAMES-ONLY`).
///
/// It says the one thing that is true — nobody issued this name — then
/// names who was asked, who was deliberately not asked, and the
/// qualified packages that merely bear the spelling. The hint is a list
/// to copy from, and the sentence says so: a name that is not issued
/// must not be resolvable by accident just because the message happens
/// to mention one package.
fn render_not_issued(name: &str, refusal: &NotIssued) -> String {
    use std::fmt::Write as _;
    let mut msg = format!(
        "the short name `{name}` is not issued by any consulted registry; give the full name.\n\
         A short name exists only when a registry's own alias table issues it — being named \
         `{name}` grants a package nothing.\n",
    );
    let _ = match refusal.consulted.as_slice() {
        [] => writeln!(msg, "consulted: (no registry answered)"),
        consulted => writeln!(msg, "consulted: {}", consulted.join(", ")),
    };
    if !refusal.skipped.is_empty() {
        let _ = writeln!(
            msg,
            "not consulted, excluded by the effective source policy: {}",
            refusal.skipped.join(", "),
        );
    }
    match refusal.hint.as_slice() {
        [] => {
            let _ = write!(
                msg,
                "Give the qualified form instead, e.g. `vibe install <group>/{name}`.",
            );
        }
        hint => {
            let _ = writeln!(
                msg,
                "packages named `{name}`, none of them chosen for you — copy one in full: {}",
                hint.join(", "),
            );
            let _ = write!(msg, "e.g. `vibe install {}`.", hint[0]);
        }
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
        // A spelling the registry ISSUED for the package (PROP-008
        // §2.6). Both halves of the identity may change, and the
        // operator is told which registry decided that — the resolved
        // pkgref that leaves here is fully qualified, so nothing
        // downstream ever sees the short name (PROP-029
        // `#ADDR-SHORT-NAMES`).
        ShortNameOutcome::ResolvedAlias(candidate) => {
            eprintln!("{} → {}", pkgref.name, candidate.provenance());
            Ok(PackageRef {
                kind: pkgref.kind,
                group: Some(candidate.target.group.clone()),
                name: candidate.target.name.parse()?,
                version: pkgref.version.clone(),
            })
        }
        ShortNameOutcome::NotIssued(refusal) => {
            bail!(render_not_issued(&pkgref.name, &refusal))
        }
        // PROP-008 §2.7 — a short name matching two issued targets is a
        // collision the resolver refuses to guess past. It carries a
        // dedicated exit code (`7`, via `InstallError::AmbiguousPackage`)
        // distinct from a `3` dependency conflict, and the rendered
        // message lists the qualified alternatives so the operator can
        // pick one.
        ShortNameOutcome::Ambiguous(spellings, hint) => Err(InstallError::AmbiguousPackage(
            render_collision(&pkgref.name, &spellings, &hint),
        )
        .into()),
    }
}

/// Qualify a CLI-supplied pkgref using **only the lockfile** — no
/// registry index, no network. This is the resolution path for commands
/// that act on an already-installed package (`vibe uninstall`, `vibe
/// update`): `vibe.lock` is the authority for what is installed
/// (PROP-008 §2.6 LOCKFILE-AUTHORITATIVE), so a bare short name resolves
/// against it alone and the answer never leaves the file.
///
/// - A group-qualified ref passes through untouched.
/// - Exactly one locked entry of that name → its `group` is spliced in.
/// - No locked entry → "not installed", a clear local failure that does
///   not reach the network (the common `uninstall`/`update` case where the
///   package simply is not there).
/// - Two locked groups under one name → the same PROP-008 §2.7 collision
///   [`qualify`] reports.
pub fn qualify_locked(pkgref: &PackageRef, lockfile: &Lockfile) -> Result<PackageRef> {
    if pkgref.is_qualified() {
        return Ok(pkgref.clone());
    }
    match locked_groups(lockfile, &pkgref.name).as_slice() {
        [only] => Ok(PackageRef {
            kind: pkgref.kind,
            group: Some(only.clone()),
            name: pkgref.name.clone(),
            version: pkgref.version.clone(),
        }),
        [] => bail!(
            "the short name `{name}` is not installed — no package of that name is in \
             `vibe.lock`. This command acts only on installed packages; if it is installed \
             under a group you did not name, give the qualified form `<group>/{name}`, and \
             if it is new to this project, add it with `vibe install <group>/{name}`.",
            name = pkgref.name,
        ),
        // PROP-008 §2.7 — a short name matching two locked groups is a
        // collision the lockfile cannot break (same verdict as `qualify`).
        many => Err(InstallError::AmbiguousPackage(render_collision(
            &pkgref.name,
            &qualified(many, &pkgref.name),
            &format!("{}/{}", many[0], pkgref.name),
        ))
        .into()),
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
        let msg = render_collision(
            "wal",
            &qualified(&[g("com.acme"), g("org.vibevm")], "wal"),
            "com.acme/wal",
        );
        assert!(
            msg.contains("`wal` is ambiguous — 2 packages match"),
            "missing the header line:\n{msg}"
        );
        assert!(msg.contains("  1. com.acme/wal"), "missing item 1:\n{msg}");
        assert!(
            msg.contains("  2. org.vibevm/wal"),
            "missing item 2:\n{msg}"
        );
        assert!(
            msg.contains("`vibe install com.acme/wal`"),
            "missing the re-run hint:\n{msg}"
        );
    }

    /// The refusal of the new law: a package bearing the name is named
    /// as a hint to copy, and the sentence makes clear nothing was
    /// chosen from it.
    #[test]
    fn a_name_nobody_issued_is_refused_with_the_qualified_names_as_a_hint() {
        let msg = render_not_issued(
            "opencode",
            &NotIssued {
                hint: qualified(&[g("org.vibevm.opencode")], "opencode"),
                consulted: vec!["vibespecs".into()],
                skipped: vec!["corp".into()],
            },
        );
        assert!(
            msg.contains("the short name `opencode` is not issued by any consulted registry"),
            "{msg}"
        );
        assert!(msg.contains("give the full name"), "{msg}");
        assert!(msg.contains("consulted: vibespecs"), "{msg}");
        assert!(
            msg.contains("excluded by the effective source policy: corp"),
            "{msg}"
        );
        assert!(
            msg.contains("none of them chosen for you"),
            "the hint must not read as a choice: {msg}"
        );
        assert!(
            msg.contains("org.vibevm.opencode/opencode"),
            "the qualified name must be copyable: {msg}"
        );
    }

    /// With nothing publishing the name either, the refusal falls back
    /// to the shape of the qualified form rather than inventing a hint.
    #[test]
    fn a_name_nothing_carries_asks_for_the_qualified_form() {
        let msg = render_not_issued(
            "nope",
            &NotIssued {
                hint: Vec::new(),
                consulted: Vec::new(),
                skipped: Vec::new(),
            },
        );
        assert!(msg.contains("consulted: (no registry answered)"), "{msg}");
        assert!(msg.contains("`vibe install <group>/nope`"), "{msg}");
        assert!(!msg.contains("copy one in full"), "{msg}");
    }

    /// A minimal locked entry — `locked_groups` reads only `name` and
    /// `group`, so the rest is fixture mass.
    fn locked_pkg(group: &str, name: &str) -> vibe_core::manifest::LockedPackage {
        toml::from_str(&format!(
            "kind = \"flow\"\n\
             name = \"{name}\"\n\
             group = \"{group}\"\n\
             version = \"0.3.0\"\n\
             source_url = \"git@example.invalid/x.git\"\n\
             content_hash = \"sha256:abc\"\n"
        ))
        .unwrap()
    }

    #[test]
    fn qualify_locked_passes_a_qualified_ref_through() {
        let lf = Lockfile::empty("test", "2026-01-01");
        let r = PackageRef::parse("org.vibevm/wal").unwrap();
        let q = qualify_locked(&r, &lf).unwrap();
        assert_eq!(q.qualified_name(), "org.vibevm/wal");
    }

    #[test]
    fn qualify_locked_resolves_one_locked_entry() {
        let mut lf = Lockfile::empty("test", "2026-01-01");
        lf.packages.push(locked_pkg("org.vibevm", "wal"));
        let r = PackageRef::parse("wal").unwrap();
        let q = qualify_locked(&r, &lf).unwrap();
        assert_eq!(q.group.as_ref().unwrap().as_str(), "org.vibevm");
        assert_eq!(q.name.as_str(), "wal");
    }

    #[test]
    fn qualify_locked_unknown_is_not_installed_and_offline() {
        // The common `uninstall`/`update` case: the package is simply not
        // there. The failure must name `vibe.lock` and stay local — never
        // reach for a registry or its index.
        let lf = Lockfile::empty("test", "2026-01-01");
        let r = PackageRef::parse("wal").unwrap();
        let err = qualify_locked(&r, &lf).unwrap_err().to_string();
        assert!(
            err.contains("not installed"),
            "must say not installed: {err}"
        );
        assert!(err.contains("vibe.lock"), "must name the lockfile: {err}");
        assert!(
            !err.to_lowercase().contains("registry") && !err.to_lowercase().contains("index"),
            "must not reach for a registry/index: {err}"
        );
    }

    #[test]
    fn qualify_locked_collision_is_ambiguous() {
        let mut lf = Lockfile::empty("test", "2026-01-01");
        lf.packages.push(locked_pkg("com.acme", "wal"));
        lf.packages.push(locked_pkg("org.vibevm", "wal"));
        let r = PackageRef::parse("wal").unwrap();
        let err = qualify_locked(&r, &lf).unwrap_err();
        assert!(
            err.downcast_ref::<InstallError>().is_some(),
            "a collision must surface as InstallError (exit 7): {err}"
        );
        assert!(
            err.to_string().contains("ambiguous"),
            "must say ambiguous: {err}"
        );
    }
}
