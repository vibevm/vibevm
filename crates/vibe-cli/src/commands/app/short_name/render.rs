//! What the short-name boundary *says*: the one provenance line, the two
//! refusals, and the two spelling helpers that keep what the operator
//! typed intact.
//!
//! Pure over values — nothing here opens a registry, reads a file or
//! learns anything this run did — so the wording is tested against a
//! `Lookup` built by hand rather than against whatever this machine holds.

use vibe_core::Group;

use super::Lookup;

/// `node → org.vibevm.node/node` — the one line that says what vibe
/// decided on the operator's behalf.
pub(super) fn provenance(name: &str, group: &Group) -> String {
    format!("{name} → {group}/{name}")
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
            "Re-run with the qualified form, e.g. `{command} {}/{name}`.",
            first.group,
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
        .map(|candidate| format!("{}/{name}", candidate.group))
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

/// The bare name a spelling carries, or `None` when it is already
/// qualified or is not a pkgref at all — in which case the command keeps
/// exactly the behaviour it has today and raises its own error.
pub(super) fn bare_name(spelling: &str) -> Option<String> {
    let parsed = vibe_core::PackageRef::parse(spelling.trim()).ok()?;
    (!parsed.is_qualified()).then(|| parsed.name.as_str().to_owned())
}
