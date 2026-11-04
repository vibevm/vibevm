//! Package-name patterns in one fixed group and the resolver's actual source world.
//! Selected mode enumerates only selected dependencies. Ordinary installed mode
//! uses a held dependency layout and the same document/version lookup as points.
//! Only true absence excludes a member; broken or ambiguous matches refuse.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#SOURCE-GLOB-BINDING"
);

use super::{FileResolver, ResolveError};
use crate::address::{Authority, SpecAddress};
#[cfg(test)]
use std::path::Path;

/// Whether the package NAME (and only it) contains a wildcard.
pub fn is_pattern(addr: &SpecAddress) -> bool {
    matches!(&addr.authority, Authority::Package { name, .. } if name.contains('*'))
}

impl FileResolver {
    /// Expand within the fixed group and the same world used by point reads.
    /// Preserve explicit package/revision pins. Without a package pin, ordinary
    /// installed lookup uses its existing newest rule and versionless spelling.
    /// A non-pattern still denotes exactly itself; a true empty set is legal.
    pub fn expand_pattern(&self, addr: &SpecAddress) -> Result<Vec<SpecAddress>, ResolveError> {
        if !is_pattern(addr) {
            return Ok(vec![addr.clone()]);
        }
        super::paths::source(&addr.doc_path, &[])?;
        let Authority::Package {
            group,
            name: pattern,
            version,
        } = &addr.authority
        else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        if let Some(selected) = &self.selected {
            for (candidate_group, name) in selected.keys() {
                if candidate_group != group
                    || !name_matches(pattern, name)
                    || self.is_self(group, name)
                {
                    continue;
                }
                let concrete = build_address(addr, name)?;
                if member(self.locate_source(&concrete, &[]).map(|_| ()))? {
                    out.push(concrete);
                }
            }
        } else {
            if let Some(version) = version {
                super::paths::version(version)?;
            }
            let Some(slots) = self.installed_slots()? else {
                return Ok(out);
            };
            let mut names = slots
                .names()?
                .into_iter()
                .filter_map(|slot| {
                    let (candidate_group, name) = slot.rsplit_once('.')?;
                    (candidate_group == group
                        && !name.is_empty()
                        && name_matches(pattern, name)
                        && !self.is_self(group, name))
                    .then(|| name.to_owned())
                })
                .collect::<Vec<_>>();
            names.sort();
            for name in names {
                let concrete = build_address(addr, &name)?;
                if member(slots.locate_document(group, &name, version.as_deref(), &addr.doc_path))?
                {
                    out.push(concrete);
                }
            }
        }
        Ok(out)
    }
}

fn member(result: Result<(), ResolveError>) -> Result<bool, ResolveError> {
    match result {
        Ok(()) => Ok(true),
        Err(ResolveError::DocNotFound { .. } | ResolveError::PackageSlotNotFound(_)) => Ok(false),
        Err(error) => Err(error),
    }
}

/// Rebuild the concrete identity without discarding any selector data.
fn build_address(pattern: &SpecAddress, name: &str) -> Result<SpecAddress, ResolveError> {
    let mut concrete = pattern.clone();
    if let Authority::Package {
        name: selected_name,
        ..
    } = &mut concrete.authority
    {
        *selected_name = name.into();
    }
    let mut raw = concrete.without_pin();
    if let Some(revision) = concrete.pinned_r {
        raw.push_str(&format!("~r{revision}"));
    }
    SpecAddress::parse(&raw).map_err(|error| ResolveError::SourceObservation {
        path: raw,
        reason: format!("matching coordinate cannot form a concrete source address: {error}"),
    })
}

/// Does `pattern` (a package-name glob) match `name`? `*` matches any run of
/// bytes, including the empty run; several `*` are allowed; every other byte is
/// literal — there are no `?` or `[...]` metacharacters (law 1). Compared at the
/// byte level: `*` is 0x2A and is never a UTF-8 continuation byte, so byte
/// scanning is `*`-safe, and the byte comparison matches the resolver's
/// byte-order view of names (law 5).
fn name_matches(pattern: &str, name: &str) -> bool {
    let pat = pattern.as_bytes();
    let txt = name.as_bytes();
    let (mut pi, mut ti) = (0usize, 0usize);
    // The position of the last unmatched `*` and the text index at which it
    // resumed swallowing — the classic backtracking wildcard match.
    let (mut star, mut resume) = (None::<usize>, 0usize);
    while ti < txt.len() {
        if pi < pat.len() && pat[pi] == b'*' {
            star = Some(pi);
            resume = ti;
            pi += 1;
        } else if pi < pat.len() && pat[pi] == txt[ti] {
            pi += 1;
            ti += 1;
        } else if let Some(p) = star {
            // A literal failed: have the last star swallow one more byte.
            pi = p + 1;
            resume += 1;
            ti = resume;
        } else {
            return false;
        }
    }
    // A trailing run of stars matches the (now exhausted) end of the name.
    while pi < pat.len() && pat[pi] == b'*' {
        pi += 1;
    }
    pi == pat.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SelfCoordinate;
    use crate::resolver::{specs_root_under, vibedeps_root_under};
    use std::fs;

    /// A self coordinate that none of the test package names collide with — the
    /// self tree is inert for pattern expansion (only `vibedeps/` is scanned).
    fn coord() -> SelfCoordinate {
        SelfCoordinate::new(Some("org.vibevm.core".into()), "vibevm".into())
    }

    /// Build a vibedeps slot `<group>.<name>` holding the given versions, each
    /// with `spec/contract/API.md` — the document every G-test resolves.
    /// The scaffolds route through the resolver's own root probes (fresh
    /// tempdirs fall back to the legacy names; PROP-052's flip moves these
    /// fixtures with the product).
    fn make_plugin(ws: &Path, group: &str, name: &str, versions: &[&str]) {
        let slot = vibedeps_root_under(ws).join(format!("{group}.{name}"));
        for v in versions {
            let dir = specs_root_under(&slot.join(v)).join("contract");
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("API.md"), "# API\n").unwrap();
        }
    }

    /// A slot with a version directory but NO document — installed yet not a
    /// member of a set whose address names `contract/API`.
    fn make_slot_no_doc(ws: &Path, group: &str, name: &str, version: &str) {
        let dir = vibedeps_root_under(ws)
            .join(format!("{group}.{name}"))
            .join(version);
        fs::create_dir_all(&dir).unwrap();
    }

    fn names_of(got: &[SpecAddress]) -> Vec<&str> {
        got.iter()
            .map(|a| match &a.authority {
                Authority::Package { name, .. } => name.as_str(),
                _ => "",
            })
            .collect()
    }

    #[test]
    fn g1_two_matches_in_name_order() {
        // G1: `plugin-*` yields exactly two addresses, alpha → beta (sorted by
        // name, byte order). The addresses are built canonically (РТ-4).
        let ws = tempfile::TempDir::new().unwrap();
        make_plugin(ws.path(), "org.vibevm.plugins", "plugin-alpha", &["1.0.0"]);
        make_plugin(ws.path(), "org.vibevm.plugins", "plugin-beta", &["1.0.0"]);
        let r = FileResolver::new(ws.path(), coord());
        let pat = SpecAddress::parse("spec://org.vibevm.plugins/plugin-*/contract/API").unwrap();
        let got = r.expand_pattern(&pat).unwrap();
        assert_eq!(names_of(&got), vec!["plugin-alpha", "plugin-beta"]);
        assert_eq!(got[0].doc_path, "contract/API");
        assert_eq!(
            got[0].raw,
            "spec://org.vibevm.plugins/plugin-alpha/contract/API"
        );
    }

    #[test]
    fn g2_sort_independent_of_creation_order() {
        // G2: slots created in reverse order (beta before alpha) — the read
        // order must not change the result; the sort decides it.
        let ws = tempfile::TempDir::new().unwrap();
        make_plugin(ws.path(), "org.vibevm.plugins", "plugin-beta", &["1.0.0"]);
        make_plugin(ws.path(), "org.vibevm.plugins", "plugin-alpha", &["1.0.0"]);
        let r = FileResolver::new(ws.path(), coord());
        let pat = SpecAddress::parse("spec://org.vibevm.plugins/plugin-*/contract/API").unwrap();
        assert_eq!(
            names_of(&r.expand_pattern(&pat).unwrap()),
            vec!["plugin-alpha", "plugin-beta"]
        );
    }

    #[test]
    fn g3_non_matching_excluded() {
        // G3: `flow-widget` sits alongside — its name does not match `plugin-*`,
        // so it is not a member.
        let ws = tempfile::TempDir::new().unwrap();
        make_plugin(ws.path(), "org.vibevm.plugins", "plugin-alpha", &["1.0.0"]);
        make_plugin(ws.path(), "org.vibevm.plugins", "plugin-beta", &["1.0.0"]);
        make_plugin(ws.path(), "org.vibevm.plugins", "widget", &["1.0.0"]);
        let r = FileResolver::new(ws.path(), coord());
        let pat = SpecAddress::parse("spec://org.vibevm.plugins/plugin-*/contract/API").unwrap();
        assert_eq!(
            names_of(&r.expand_pattern(&pat).unwrap()),
            vec!["plugin-alpha", "plugin-beta"]
        );
    }

    #[test]
    fn g4_match_without_doc_is_not_a_member() {
        // G4: `flow-plugin-gamma` is installed (slot + version present) but has
        // no `contract/API` document — it is not a member, and that is not an
        // error (law 3: membership is name AND document).
        let ws = tempfile::TempDir::new().unwrap();
        make_plugin(ws.path(), "org.vibevm.plugins", "plugin-alpha", &["1.0.0"]);
        make_slot_no_doc(ws.path(), "org.vibevm.plugins", "plugin-gamma", "1.0.0");
        let r = FileResolver::new(ws.path(), coord());
        let pat = SpecAddress::parse("spec://org.vibevm.plugins/plugin-*/contract/API").unwrap();
        assert_eq!(
            names_of(&r.expand_pattern(&pat).unwrap()),
            vec!["plugin-alpha"]
        );
    }

    #[test]
    fn g5_empty_set_is_legal() {
        // G5: a glob matching nothing is Ok(vec![]), not a "source not found"
        // error (law 4 — patterns degrade naturally).
        let ws = tempfile::TempDir::new().unwrap();
        make_plugin(ws.path(), "org.vibevm.plugins", "widget", &["1.0.0"]); // not plugin-*
        let r = FileResolver::new(ws.path(), coord());
        let pat = SpecAddress::parse("spec://org.vibevm.plugins/plugin-*/contract/API").unwrap();
        let got = r.expand_pattern(&pat).unwrap();
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn g6_freshest_version_selected() {
        // G6: a matched package has two versions (1.0.0, 2.0.0); the address
        // points at the 2.0.0 slot — verified by resolve_file on the result
        // (law 6 — B-028's freshest rule, applied at resolve time).
        let ws = tempfile::TempDir::new().unwrap();
        make_plugin(
            ws.path(),
            "org.vibevm.plugins",
            "plugin-alpha",
            &["1.0.0", "2.0.0"],
        );
        let r = FileResolver::new(ws.path(), coord());
        let pat = SpecAddress::parse("spec://org.vibevm.plugins/plugin-*/contract/API").unwrap();
        let got = r.expand_pattern(&pat).unwrap();
        assert_eq!(got.len(), 1);
        let file = r.resolve_file(&got[0]).unwrap();
        let expected_tail = format!(
            "2.0.0/{}/contract/API.md",
            crate::resolver::LEGACY_SPECS_ROOT
        );
        assert!(file.ends_with(expected_tail), "{file:?}");
    }

    #[test]
    fn g7_qualified_slot_preserves_hyphenated_package_name() {
        // The last dot separates the qualified group from its package name;
        // a hyphen inside plugin-alpha is part of that name.
        let ws = tempfile::TempDir::new().unwrap();
        make_plugin(ws.path(), "org.vibevm.plugins", "plugin-alpha", &["1.0.0"]); // …plugins.plugin-alpha
        make_plugin(ws.path(), "org.vibevm.plugins", "alpha", &["1.0.0"]); // …plugins.alpha → name "alpha"
        let r = FileResolver::new(ws.path(), coord());
        let pat = SpecAddress::parse("spec://org.vibevm.plugins/plugin-*/contract/API").unwrap();
        assert_eq!(
            names_of(&r.expand_pattern(&pat).unwrap()),
            vec!["plugin-alpha"]
        );
    }

    #[test]
    fn g8_star_matches_empty_and_all() {
        // G8: `*` matches any run, including the empty one.
        //  (a) `plugin-*` matches a package literally named `plugin-` (empty
        //      tail); `plugin-` sorts before `plugin-alpha` (shorter prefix).
        //  (b) `*` matches every installed name.
        let ws = tempfile::TempDir::new().unwrap();
        make_plugin(ws.path(), "org.vibevm.plugins", "plugin-", &["1.0.0"]);
        make_plugin(ws.path(), "org.vibevm.plugins", "plugin-alpha", &["1.0.0"]);
        let r = FileResolver::new(ws.path(), coord());

        let pat = SpecAddress::parse("spec://org.vibevm.plugins/plugin-*/contract/API").unwrap();
        assert_eq!(
            names_of(&r.expand_pattern(&pat).unwrap()),
            vec!["plugin-", "plugin-alpha"]
        );

        let all = SpecAddress::parse("spec://org.vibevm.plugins/*/contract/API").unwrap();
        assert_eq!(
            names_of(&r.expand_pattern(&all).unwrap()),
            vec!["plugin-", "plugin-alpha"]
        );
    }

    #[test]
    fn g9_point_resolve_of_pattern_is_pattern_error() {
        // G9 (law 7): point-resolving a pattern is a loud error, not a quiet
        // PackageSlotNotFound. resolve_file refuses the pattern and points at
        // expand_pattern.
        let r = FileResolver::new(Path::new("."), coord());
        let pat = SpecAddress::parse("spec://org.vibevm.plugins/plugin-*/contract/API").unwrap();
        let err = r.resolve_file(&pat).unwrap_err();
        assert!(
            matches!(err, ResolveError::PatternNotExpanded { .. }),
            "{err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("expand_pattern"), "{msg}");
        assert!(msg.contains("plugin-*"), "{msg}"); // the pattern is visible in the text
    }

    #[test]
    fn g_non_pattern_expands_to_self() {
        // ПРАВКА 1: a non-pattern address expands to exactly itself, with NO
        // vibedeps scan — otherwise an address on the project's OWN tree (which
        // resolves under ws_root/spec, never vibedeps/) would silently drop to
        // EMPTY. A total, branch-free oracle; the point address still fails
        // loudly in resolve_file.
        let ws = tempfile::TempDir::new().unwrap();
        let r = FileResolver::new(ws.path(), coord());

        // (a) the dangerous case: a self-coordinate address, absent from
        // vibedeps/, expands to itself (one element, identical raw).
        let self_addr = SpecAddress::parse("spec://org.vibevm.core/vibevm/common/TARGET").unwrap();
        let got = r.expand_pattern(&self_addr).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].raw, self_addr.raw);

        // (b) a point address on a package that is not installed at all also
        // expands to itself.
        let missing = SpecAddress::parse("spec://org.demo/nope-such/contract/API").unwrap();
        let got2 = r.expand_pattern(&missing).unwrap();
        assert_eq!(got2.len(), 1);
        assert_eq!(got2[0].raw, missing.raw);
    }

    #[test]
    fn g_anchor_and_raw_round_trip() {
        // РТ-4: a built address carries the pattern's anchor and a consistent
        // raw, and round-trips through parse (well-formed).
        let ws = tempfile::TempDir::new().unwrap();
        make_plugin(ws.path(), "org.vibevm.plugins", "plugin-alpha", &["1.0.0"]);
        let r = FileResolver::new(ws.path(), coord());
        let pat =
            SpecAddress::parse("spec://org.vibevm.plugins/plugin-*/contract/API#root").unwrap();
        let got = r.expand_pattern(&pat).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].anchor, vec!["root"]);
        assert_eq!(
            got[0].raw,
            "spec://org.vibevm.plugins/plugin-alpha/contract/API#root"
        );
        SpecAddress::parse(&got[0].raw).unwrap();
    }

    #[test]
    fn g_is_pattern_only_name_star() {
        // РТ-1: a pattern is a `*` ONLY in the package name. A `*` in the group
        // or document is a literal; a host authority is never a pattern.
        assert!(is_pattern(
            &SpecAddress::parse("spec://org.vibevm.plugins/a-*/d").unwrap()
        ));
        assert!(!is_pattern(
            &SpecAddress::parse("spec://org.vibevm.plugins/abc/d").unwrap()
        ));
        // A `*` in the group does not make the address a pattern (the group
        // is a fixed namespace, not wildcard syntax).
        assert!(!is_pattern(
            &SpecAddress::parse("spec://org.*/abc/d").unwrap()
        ));
        // An undotted host authority is not a package, so never a pattern.
        assert!(!is_pattern(&SpecAddress::parse("spec://demo/a/b").unwrap()));
    }
}
