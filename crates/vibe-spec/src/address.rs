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
    #[error("invalid anchor segment `{0}` (expected kebab-case `[a-z0-9]+(-[a-z0-9]+)*`)")]
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

/// One anchor tree-path segment: kebab-case `[a-z0-9]+(-[a-z0-9]+)*`. Applied
/// per-segment so a flat `spec://pkg/doc#flat-anchor` validates exactly as the
/// vendored `is_valid_anchor` does.
fn is_valid_anchor_segment(seg: &str) -> bool {
    if seg.is_empty() {
        return false;
    }
    seg.split('-').all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    })
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
        assert_eq!(
            SpecAddress::parse("spec://vibevm/x/y#Bad"),
            Err(SpecAddressError::InvalidAnchorSegment("Bad".into()))
        );
        // An empty segment between dots.
        assert_eq!(
            SpecAddress::parse("spec://vibevm/x/y#a..b"),
            Err(SpecAddressError::InvalidAnchorSegment(String::new()))
        );
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
