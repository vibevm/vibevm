//! The `spec://` address grammar (PROP-035 §6).
//!
//! Canonical form:
//!
//! ```text
//! spec://<group>/<name>[@<version>]/<doc-path>#<anchor>[.<sub>…][~r<N>]
//! ```
//!
//! - **authority** — either a package coordinate `<group>/<name>` or the host
//!   project's reserved single-token namespace (e.g. `vibevm`). The two are
//!   told apart syntactically: a first segment containing a `.` is a
//!   reverse-DNS **group** (so `<group>/<name>` follows); a first segment
//!   without a `.` is the **host** authority (no `name`). Demo/fixture packages
//!   therefore use dotted groups (`com.example.demo`), matching PROP-029's
//!   `com.example.shop` illustrations. The group↔name joiner is `/`, never `.`
//!   (PROP-029).
//! - **`@<version>`** — optional, attached to `<name>`; a raw version spec, not
//!   parsed to semver here (the router resolves the concrete slot from the
//!   lockfile later, PROP-035 §6). Absent, the version is the lockfile's.
//! - **`<doc-path>`** — the document path under the package/host `spec/` root,
//!   genre segments included (`flows/…`, `modules/…`). Required: an address
//!   always names at least a document.
//! - **`#<anchor>.<sub>…`** — a **tree path** into the document IR (§5): the
//!   dots descend levels (`a.b.c` = `c` inside `b` inside `a`). Optional —
//!   absent, the address denotes the whole document (the "`spec://` link to a
//!   whole file without specifics" of PROP-035 §7.1).
//! - **`~r<N>`** — optional spec-unit revision pin (PROP-014), `N ≥ 1`.

use std::fmt;

/// A parsed `spec://` address. Purely syntactic: it records what the address
/// names, not where it lands on disk (that is the router's job, with the
/// install context).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecAddress {
    /// The address exactly as written, including any `~rN`.
    pub raw: String,
    /// Host project or package coordinate.
    pub authority: Authority,
    /// Document path under the `spec/` root, `/`-joined, genre included.
    pub doc_path: String,
    /// Tree path into the document (`a.b.c` → `["a", "b", "c"]`). Empty means
    /// the whole document.
    pub anchor: Vec<String>,
    /// Optional spec-unit revision pin (`~rN`).
    pub pinned_r: Option<u32>,
}

/// The authority half of a `spec://` address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Authority {
    /// The root project's reserved namespace (e.g. `vibevm`) — not a package,
    /// has no group (PROP-029 §scope).
    Host(String),
    /// A package coordinate. `version` is the raw `@`-spec, unparsed.
    Package {
        group: String,
        name: String,
        version: Option<String>,
    },
}

/// Why a `spec://` string is not a well-formed address.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpecAddressError {
    #[error("not a spec:// address (missing `spec://` scheme)")]
    MissingScheme,
    #[error("spec:// address contains whitespace")]
    ContainsWhitespace,
    #[error("spec:// address has an empty authority")]
    EmptyAuthority,
    #[error("package address has a group but no name")]
    MissingName,
    #[error("package address has an empty name")]
    EmptyName,
    #[error("spec:// address names no document (authority only)")]
    MissingDocPath,
    #[error("spec:// address has an empty path segment (a leading, trailing, or doubled `/`)")]
    EmptyPathSegment,
    #[error("spec:// address has a `#` but an empty anchor")]
    EmptyAnchor,
    #[error("invalid anchor segment `{0}` (expected an id `[A-Za-z][A-Za-z0-9_-]*`)")]
    InvalidAnchorSegment(String),
    #[error("invalid revision pin `{0}` (expected `~rN` with N ≥ 1)")]
    InvalidRevision(String),
}

impl SpecAddress {
    /// Parse a `spec://` address. Deterministic and context-free: it does not
    /// consult the lockfile, the filesystem, or the installed package set.
    pub fn parse(raw: &str) -> Result<Self, SpecAddressError> {
        let body = raw
            .strip_prefix("spec://")
            .ok_or(SpecAddressError::MissingScheme)?;
        if body.chars().any(char::is_whitespace) {
            return Err(SpecAddressError::ContainsWhitespace);
        }

        // Split the fragment (`#anchor[~rN]`) off the path part.
        let (path_part, frag) = match body.split_once('#') {
            Some((p, f)) => (p, Some(f)),
            None => (body, None),
        };
        if path_part.is_empty() {
            return Err(SpecAddressError::EmptyAuthority);
        }

        let segs: Vec<&str> = path_part.split('/').collect();
        let (authority, doc_segs) = classify_authority(&segs)?;

        if doc_segs.is_empty() {
            return Err(SpecAddressError::MissingDocPath);
        }
        if doc_segs.iter().any(|s| s.is_empty()) {
            return Err(SpecAddressError::EmptyPathSegment);
        }
        let doc_path = doc_segs.join("/");

        let (anchor, pinned_r) = parse_fragment(frag)?;

        Ok(SpecAddress {
            raw: raw.to_string(),
            authority,
            doc_path,
            anchor,
            pinned_r,
        })
    }

    /// The address with any `~rN` pin dropped, rebuilt canonically. Useful as a
    /// stable key (the pin is a revision selector, not part of the identity).
    pub fn without_pin(&self) -> String {
        let mut s = String::from("spec://");
        match &self.authority {
            Authority::Host(h) => s.push_str(h),
            Authority::Package {
                group,
                name,
                version,
            } => {
                s.push_str(group);
                s.push('/');
                s.push_str(name);
                if let Some(v) = version {
                    s.push('@');
                    s.push_str(v);
                }
            }
        }
        s.push('/');
        s.push_str(&self.doc_path);
        if !self.anchor.is_empty() {
            s.push('#');
            s.push_str(&self.anchor.join("."));
        }
        s
    }
}

impl fmt::Display for SpecAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

/// Decide whether the leading segments name a package (`group/name`) or the
/// host, and return the authority plus the remaining document segments.
fn classify_authority<'a>(
    segs: &'a [&'a str],
) -> Result<(Authority, &'a [&'a str]), SpecAddressError> {
    let first = *segs.first().ok_or(SpecAddressError::EmptyAuthority)?;
    if first.is_empty() {
        return Err(SpecAddressError::EmptyAuthority);
    }

    // A dotted first segment is a reverse-DNS group → `<group>/<name>/…`.
    if first.contains('.') {
        let name_seg = segs.get(1).copied().ok_or(SpecAddressError::MissingName)?;
        let (name, version) = match name_seg.split_once('@') {
            Some((n, v)) => (n.to_string(), Some(v.to_string())),
            None => (name_seg.to_string(), None),
        };
        if name.is_empty() {
            return Err(SpecAddressError::EmptyName);
        }
        let authority = Authority::Package {
            group: first.to_string(),
            name,
            version,
        };
        Ok((authority, &segs[2.min(segs.len())..]))
    } else {
        // Undotted first segment is the host namespace.
        Ok((Authority::Host(first.to_string()), &segs[1..]))
    }
}

/// Parse the `#anchor[~rN]` fragment into a tree path and an optional pin.
fn parse_fragment(frag: Option<&str>) -> Result<(Vec<String>, Option<u32>), SpecAddressError> {
    let Some(frag) = frag else {
        return Ok((Vec::new(), None));
    };
    if frag.is_empty() {
        return Err(SpecAddressError::EmptyAnchor);
    }

    let (anchor_str, pinned_r) = match frag.split_once('~') {
        Some((a, rev)) => (a, Some(parse_revision(rev)?)),
        None => (frag, None),
    };
    if anchor_str.is_empty() {
        return Err(SpecAddressError::EmptyAnchor);
    }

    let anchor: Vec<String> = anchor_str.split('.').map(str::to_string).collect();
    for seg in &anchor {
        if !is_valid_anchor_segment(seg) {
            return Err(SpecAddressError::InvalidAnchorSegment(seg.clone()));
        }
    }
    Ok((anchor, pinned_r))
}

/// `~rN` → `N`, with `N ≥ 1` (matching the vendored grammar's rule).
fn parse_revision(rev: &str) -> Result<u32, SpecAddressError> {
    let bad = || SpecAddressError::InvalidRevision(rev.to_string());
    let digits = rev.strip_prefix('r').ok_or_else(bad)?;
    let n: u32 = digits.parse().map_err(|_| bad())?;
    if n == 0 { Err(bad()) } else { Ok(n) }
}

/// One anchor tree-path segment: an id `[A-Za-z][A-Za-z0-9_-]*`. Applied
/// per-segment so a flat `spec://pkg/doc#flat-anchor` validates exactly as the
/// vendored `is_valid_fact_id` does — and `.`, the one character the id
/// grammar excludes, is exactly what separates the segments.
///
/// A document's headings and its `##<ID>` facts share one address space
/// (PROP-014 §2.1), so an address must name either: `#SOME-NORMATIVE-FACT` as
/// readily as `#a-heading`. The kebab-only law still governs where a *heading*
/// anchor is minted; it is not this parser's business. Mirrored, not shared,
/// across the separability seam (PROP-035 §4) — the twin is
/// `core-ai-native-specmark-grammar::is_valid_fact_id`, and the convention is
/// held by tests on both sides.
fn is_valid_anchor_segment(seg: &str) -> bool {
    let mut chars = seg.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(group: &str, name: &str, version: Option<&str>) -> Authority {
        Authority::Package {
            group: group.to_string(),
            name: name.to_string(),
            version: version.map(str::to_string),
        }
    }

    #[test]
    fn host_address_with_anchor() {
        let a = SpecAddress::parse("spec://vibevm/common/PROP-000#commits").unwrap();
        assert_eq!(a.authority, Authority::Host("vibevm".into()));
        assert_eq!(a.doc_path, "common/PROP-000");
        assert_eq!(a.anchor, vec!["commits"]);
        assert_eq!(a.pinned_r, None);
    }

    #[test]
    fn package_address() {
        let a = SpecAddress::parse("spec://org.vibevm.world/redbook/flows/redbook/REDBOOK#root")
            .unwrap();
        assert_eq!(a.authority, pkg("org.vibevm.world", "redbook", None));
        assert_eq!(a.doc_path, "flows/redbook/REDBOOK");
        assert_eq!(a.anchor, vec!["root"]);
    }

    #[test]
    fn package_address_with_version() {
        let a = SpecAddress::parse("spec://org.vibevm.world/redbook@0.2/flows/x#a").unwrap();
        assert_eq!(a.authority, pkg("org.vibevm.world", "redbook", Some("0.2")));
        assert_eq!(a.doc_path, "flows/x");
    }

    #[test]
    fn dotted_anchor_is_a_tree_path() {
        let a = SpecAddress::parse(
            "spec://vibevm/modules/vibe-workspace/PROP-035#pipeline.embed-order",
        )
        .unwrap();
        assert_eq!(a.anchor, vec!["pipeline", "embed-order"]);
    }

    #[test]
    fn whole_document_has_no_anchor() {
        let a = SpecAddress::parse("spec://vibevm/common/PROP-000").unwrap();
        assert!(a.anchor.is_empty());
        assert_eq!(a.doc_path, "common/PROP-000");
    }

    #[test]
    fn revision_pin() {
        let a = SpecAddress::parse("spec://vibevm/x/y#a~r3").unwrap();
        assert_eq!(a.anchor, vec!["a"]);
        assert_eq!(a.pinned_r, Some(3));
    }

    #[test]
    fn without_pin_round_trips_canonically() {
        let a = SpecAddress::parse("spec://org.vibevm.world/redbook@0.2/flows/x#a.b~r3").unwrap();
        assert_eq!(
            a.without_pin(),
            "spec://org.vibevm.world/redbook@0.2/flows/x#a.b"
        );
        // A pinless, versionless host address is its own canonical form.
        let b = SpecAddress::parse("spec://vibevm/common/PROP-000#commits").unwrap();
        assert_eq!(b.without_pin(), "spec://vibevm/common/PROP-000#commits");
    }

    #[test]
    fn rejects_missing_scheme() {
        assert_eq!(
            SpecAddress::parse("http://x/y#z"),
            Err(SpecAddressError::MissingScheme)
        );
    }

    #[test]
    fn rejects_whitespace() {
        assert_eq!(
            SpecAddress::parse("spec://vibevm/a b/c#d"),
            Err(SpecAddressError::ContainsWhitespace)
        );
    }

    #[test]
    fn rejects_authority_only() {
        assert_eq!(
            SpecAddress::parse("spec://vibevm"),
            Err(SpecAddressError::MissingDocPath)
        );
        assert_eq!(
            SpecAddress::parse("spec://org.vibevm.world/redbook"),
            Err(SpecAddressError::MissingDocPath)
        );
    }

    #[test]
    fn rejects_group_without_name() {
        // A dotted-only authority with nothing after it is a group with no name.
        assert_eq!(
            SpecAddress::parse("spec://org.vibevm.world"),
            Err(SpecAddressError::MissingName)
        );
    }

    #[test]
    fn rejects_empty_path_segment() {
        assert_eq!(
            SpecAddress::parse("spec://vibevm//PROP-000#x"),
            Err(SpecAddressError::EmptyPathSegment)
        );
    }

    #[test]
    fn rejects_bad_anchor_segment() {
        // A non-letter head — the id grammar's one shape rule.
        assert_eq!(
            SpecAddress::parse("spec://vibevm/x/y#9lives"),
            Err(SpecAddressError::InvalidAnchorSegment("9lives".into()))
        );
        assert_eq!(
            SpecAddress::parse("spec://vibevm/x/y#_lead"),
            Err(SpecAddressError::InvalidAnchorSegment("_lead".into()))
        );
        // A character outside the id charset.
        assert_eq!(
            SpecAddress::parse("spec://vibevm/x/y#has!bang"),
            Err(SpecAddressError::InvalidAnchorSegment("has!bang".into()))
        );
        // An empty segment between dots.
        assert_eq!(
            SpecAddress::parse("spec://vibevm/x/y#a..b"),
            Err(SpecAddressError::InvalidAnchorSegment(String::new()))
        );
    }

    /// An `UPPER-SLUG` names a normative fact and a `kebab-case` one a service
    /// unit; both live in the document's one address space, so an address must
    /// carry either. `#Bad` moved here from the rejection set above — the
    /// owner ruled the behaviour changes.
    #[test]
    fn anchor_segments_carry_both_id_registers() {
        let a = SpecAddress::parse("spec://vibevm/x/y#Bad").unwrap();
        assert_eq!(a.anchor, vec!["Bad"]);

        let f = SpecAddress::parse(
            "spec://org.vibevm.ai-native/core-ai-native/00-MANIFESTO#SINGLE-DESIGN-TARGET",
        )
        .unwrap();
        assert_eq!(f.anchor, vec!["SINGLE-DESIGN-TARGET"]);

        // Underscores, digits, a revision pin, and a tree path all compose.
        let p = SpecAddress::parse("spec://vibevm/x/y#R_040.sub-a~r2").unwrap();
        assert_eq!(p.anchor, vec!["R_040", "sub-a"]);
        assert_eq!(p.pinned_r, Some(2));
        assert_eq!(p.without_pin(), "spec://vibevm/x/y#R_040.sub-a");
    }

    /// The seam convention (PROP-035 §4): `vibe-spec` shares no code with
    /// `core-ai-native-specmark-grammar`, so the two must be held to one input
    /// set by tests on both sides. This is the host half — the same strings
    /// the package's `parse_spec_uri` tests assert, with the same verdicts.
    #[test]
    fn host_twin_agrees_with_the_package_grammar_on_anchor_ids() {
        // Accepted by `is_valid_fact_id`, so accepted here.
        for ok in [
            "FACT-A",
            "my-fact",
            "R_040",
            "a",
            "Z9",
            "x-y_z-1",
            "A-b",
            "req-conditional-fixpoint",
            "SINGLE-DESIGN-TARGET",
        ] {
            let raw = format!("spec://vibevm/x/y#{ok}");
            assert!(
                SpecAddress::parse(&raw).is_ok(),
                "package grammar accepts `{ok}`; host must too"
            );
        }
        // Rejected by `is_valid_fact_id`, so rejected here. `a.b` is absent on
        // purpose: `.` descends the tree path here, and each side splits before
        // it validates.
        for bad in ["", "9lives", "-lead", "_lead", "has space", "a!", "café"] {
            let raw = format!("spec://vibevm/x/y#{bad}");
            assert!(
                SpecAddress::parse(&raw).is_err(),
                "package grammar rejects `{bad}`; host must too"
            );
        }
    }

    #[test]
    fn rejects_bad_revision() {
        assert_eq!(
            SpecAddress::parse("spec://vibevm/x/y#a~r0"),
            Err(SpecAddressError::InvalidRevision("r0".into()))
        );
        assert_eq!(
            SpecAddress::parse("spec://vibevm/x/y#a~3"),
            Err(SpecAddressError::InvalidRevision("3".into()))
        );
    }

    #[test]
    fn rejects_empty_anchor() {
        assert_eq!(
            SpecAddress::parse("spec://vibevm/x/y#"),
            Err(SpecAddressError::EmptyAnchor)
        );
    }
}
