//! Package coordinates and full fact addresses, parsed rather than
//! pattern-matched.
//!
//! A `group/name` coordinate is not «a string with a slash in it»: the
//! group is an LDH reverse-FQDN and the name is kebab-case, and both
//! grammars already exist as [`vibe_core::Group`] and
//! [`vibe_core::PackageName`] — the very types the rest of the project
//! resolves packages by. `vibe-core` is already a dependency of this
//! crate (the generated `x-rust-type` bindings need it), so using the
//! real parsers costs nothing and buys the one property a substring
//! check cannot: a coordinate that only LOOKS like one is refused
//! here rather than three layers later.
//!
//! The same parsers are what make the address→source join meaningful.
//! `spec://<group>/<package>/<path>#<fact>` carries the coordinate in
//! its first two segments, so a row that says one package in its
//! address and another in its `source` is caught by comparing two
//! things that were each parsed, not two things that were each
//! trusted.

use super::errors::AddressDefect;

/// The `spec://` scheme every full fact address carries.
pub(super) const SCHEME: &str = "spec://";

/// Whether `text` is a `<group>/<name>` coordinate under the project's
/// own grammars.
pub(super) fn is_coordinate(text: &str) -> bool {
    match text.split_once('/') {
        Some((group, name)) => {
            !name.contains('/')
                && vibe_core::Group::parse(group).is_ok()
                && vibe_core::PackageName::parse(name).is_ok()
        }
        None => false,
    }
}

/// The first thing wrong with a full
/// `spec://<group>/<package>/<path>#<fact>` address, or `None` when the
/// grammar holds. Identity is always the FULL address
/// (`##OPTIONAL-IR-FACT-EVIDENCE`), so every part is load-bearing: a
/// bare id, a missing anchor, an unparseable coordinate or an escaping
/// path each name something a second reader cannot resolve.
pub(super) fn address_defect(address: &str) -> Option<AddressDefect> {
    let Some(rest) = address.strip_prefix(SCHEME) else {
        return Some(AddressDefect::NoScheme);
    };
    if address.contains('\\') {
        return Some(AddressDefect::Backslash);
    }
    let mut halves = rest.split('#');
    let path = halves.next().unwrap_or_default();
    let Some(fact) = halves.next() else {
        return Some(AddressDefect::NoAnchor);
    };
    if halves.next().is_some() {
        return Some(AddressDefect::ExtraAnchor);
    }
    if fact.trim().is_empty() {
        return Some(AddressDefect::BlankAnchor);
    }
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() < 3 {
        return Some(AddressDefect::TooFewSegments);
    }
    for segment in &segments[2..] {
        if segment.trim().is_empty() {
            return Some(AddressDefect::BlankSegment);
        }
        if *segment == ".." {
            return Some(AddressDefect::ParentSegment);
        }
        if *segment == "." {
            return Some(AddressDefect::DotSegment);
        }
    }
    if vibe_core::Group::parse(segments[0]).is_err() {
        return Some(AddressDefect::UnparseableGroup);
    }
    if vibe_core::PackageName::parse(segments[1]).is_err() {
        return Some(AddressDefect::UnparseablePackage);
    }
    None
}

/// The `group/package` coordinate an address carries, borrowed from
/// the address itself. Only meaningful after [`address_defect`] has
/// returned `None`; the `Option` is the one honest answer for a
/// malformed address rather than a panic on a wire value.
pub(super) fn address_coordinate(address: &str) -> Option<&str> {
    let rest = address.strip_prefix(SCHEME)?;
    let path = rest.split('#').next()?;
    let mut slashes = path.match_indices('/');
    slashes.next()?;
    let (second, _) = slashes.next()?;
    Some(&path[..second])
}
