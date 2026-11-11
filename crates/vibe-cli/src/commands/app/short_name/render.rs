//! What the short-name boundary *says*: the one provenance line, the two
//! refusals, and the two spelling helpers that keep what the operator
//! typed intact.
//!
//! Pure over values — nothing here opens a registry, reads a file or
//! learns anything this run did — so the wording is tested against a
//! `Lookup` built by hand rather than against whatever this machine holds.

use vibe_core::Group;
use vibe_registry::short_name::AliasTarget;

use super::{Candidate, Lookup};

/// `node → org.vibevm.node/node` — the one line that says what vibe
/// decided on the operator's behalf. An alias adds who decided it:
/// `qwen → org.vibevm.qwen/qwen-code (alias in registry vibespecs)`,
/// because a spelling that lands on a differently-named package is a
/// bigger step than supplying a group, and the operator is owed the
/// name of the registry that took it.
pub(super) fn provenance(name: &str, candidate: &Candidate) -> String {
    format!("{name} → {}", candidate.spelling(name))
}

/// The PROP-008 §2.7 collision refusal in the application commands'
/// spelling: the ambiguous bare name, the qualified candidates numbered
/// and marked, and the re-run hint in the command the operator typed.
pub(super) fn render_collision(command: &str, name: &str, lookup: &Lookup) -> String {
    use std::fmt::Write as _;
    let eligible = lookup.eligible();
    let mut message = format!(
        "the short name `{name}` is ambiguous — {} packages match it:\n",
        eligible.len(),
    );
    for (index, candidate) in lookup.candidates.iter().enumerate() {
        let _ = writeln!(message, "  {}. {}", index + 1, candidate.spelling(name));
    }
    // A collision carries at least two eligible candidates, so `first()` is
    // always `Some`; render the hint when present rather than panic on the
    // unreachable empty case.
    if let Some(first) = eligible.first() {
        let _ = write!(
            message,
            "Re-run with the qualified form, e.g. `{command} {}/{}`.",
            first.group,
            first.package_name(name),
        );
    }
    message
}

/// The not-found sentence: what was looked in, what was deliberately not
/// looked in, which candidates exist but are never picked, and the
/// qualified form that always works.
pub(super) fn render_not_found(command: &str, name: &str, lookup: &Lookup) -> String {
    use std::fmt::Write as _;
    // A verb that acts on installed state asked no registry, so its
    // refusal names none: the answer sits in the file beside it, and the
    // remedy is local (`#INSTALLED-STATE-RESOLVES-LOCALLY`).
    if lookup.local {
        return format!(
            "the short name `{name}` is not installed — no package of that name is in this \
             home's application index. This command acts only on installed applications; if it \
             is installed under a group you did not name, give the qualified form \
             `{command} <group>/{name}`, and if it is new to this home, add it with \
             `vibe install -g <group>/{name}`.",
        );
    }
    let mut message = format!(
        "could not resolve the short name `{name}` — no package of that name is installed in \
         this home or carried by a consulted registry's index.\n",
    );
    let _ = match lookup.consulted.as_slice() {
        [] => writeln!(message, "consulted: (no registry answered)"),
        consulted => writeln!(message, "consulted: {}", consulted.join(", ")),
    };
    if !lookup.skipped.is_empty() {
        let _ = writeln!(
            message,
            "not consulted, excluded by the effective source policy: {}",
            lookup.skipped.join(", "),
        );
    }
    let untrusted: Vec<String> = lookup
        .candidates
        .iter()
        .filter(|candidate| !candidate.eligible)
        .map(|candidate| format!("{}/{}", candidate.group, candidate.package_name(name)))
        .collect();
    if !untrusted.is_empty() {
        let _ = writeln!(
            message,
            "never picked for you, their group is untrusted: {}. Name one in full, or add its \
             group to `trusted_groups` in `<settings>/agents.toml`.",
            untrusted.join(", "),
        );
    }
    if lookup.offline {
        let _ = writeln!(
            message,
            "`--offline` reads the cached index alone, so a registry whose index is not cached \
             contributes no candidate.",
        );
    }
    let _ = write!(
        message,
        "Give the qualified form instead, e.g. `{command} <group>/{name}`.",
    );
    message
}

/// Splice a resolved `group` into the spelling the operator typed,
/// **textually**, so a version requirement, a channel name and a `kind:`
/// prefix all survive byte for byte — `node@22` must not come back as
/// `node@^22`.
pub(super) fn splice(spelling: &str, group: &Group) -> String {
    let spelling = spelling.trim();
    match spelling.split_once(':') {
        Some((kind, rest)) if !kind.contains('/') && !kind.contains('@') => {
            format!("{kind}:{group}/{rest}")
        }
        _ => format!("{group}/{spelling}"),
    }
}

/// Splice an alias TARGET into the spelling the operator typed: both
/// the group and the name change, while a `kind:` prefix and a version
/// requirement survive byte for byte — `qwen@0.27` must come back as
/// `org.vibevm.qwen/qwen-code@0.27` and not as `@^0.27`.
pub(super) fn splice_alias(spelling: &str, target: &AliasTarget) -> String {
    let spelling = spelling.trim();
    let (kind, rest) = match spelling.split_once(':') {
        Some((kind, rest)) if !kind.contains('/') && !kind.contains('@') => (Some(kind), rest),
        _ => (None, spelling),
    };
    let version = rest.split_once('@').map(|(_, version)| version);
    let mut out = String::new();
    if let Some(kind) = kind {
        out.push_str(kind);
        out.push(':');
    }
    out.push_str(&format!("{}/{}", target.group, target.name));
    if let Some(version) = version {
        out.push('@');
        out.push_str(version);
    }
    out
}

/// The bare name a spelling carries, or `None` when it is already
/// qualified or is not a pkgref at all — in which case the command keeps
/// exactly the behaviour it has today and raises its own error.
pub(super) fn bare_name(spelling: &str) -> Option<String> {
    let parsed = vibe_core::PackageRef::parse(spelling.trim()).ok()?;
    (!parsed.is_qualified()).then(|| parsed.name.as_str().to_owned())
}
