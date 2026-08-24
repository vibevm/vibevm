//! Pattern expansion — a `*` in a package name enumerates the installed set
//! (B-056, the plugin form: `#source spec://org.vibevm.plugins/plugin-*`).
//!
//! A point `spec://` address names one file; a pattern
//! `spec://<group>/<name-with-*>/<doc>` names the **set** of installed packages
//! whose name matches the glob and which carry that document. This module turns
//! the pattern into the sorted, deterministic list of concrete addresses it
//! denotes.
//!
//! The laws (the owner's decisions, implemented literally):
//!
//! 1. The pattern lives only in the package **name**. `*` matches any run of
//!    bytes, including the empty run; several `*` are allowed; there are no
//!    other metacharacters — `?` and `[...]` are ordinary characters.
//! 2. The group does not participate in matching. The resolver finds a slot by
//!    name suffix (the group is absent from a `vibedeps/` directory name), so a
//!    group in the pattern could not affect which slots match — it is carried
//!    into the result addresses unchanged, nothing more.
//! 3. Membership is two halves: the name matches **and** the document is
//!    present. A package whose name matched but which lacks the address's
//!    document is simply not a member — the set is *defined* by both halves of
//!    the address, exactly as the name defines the first.
//! 4. An empty set is legal. A glob that matches nothing is `Ok(vec![])`, never
//!    a "source not found" error.
//! 5. Sorting is mandatory — by package name, byte order, before the addresses
//!    are built. One tree + one lockfile ⇒ one result.
//! 6. Each member's version is the freshest installed (B-028). No second
//!    version rule is invented.
//! 7. Point-resolving a pattern is a loud error, not a quiet miss: `resolve_file`
//!    refuses a pattern and points at `expand_pattern`.

use std::path::{Path, PathBuf};

use crate::address::{Authority, SpecAddress};

use super::{
    FileResolver, ResolveError, read_dir_or_empty, resolve_doc, specs_root_under, version_order,
    vibedeps_root_under,
};

/// Is `addr` a pattern — a `spec://` address whose package **name** carries a
/// glob `*`? Only the name may carry a pattern (law 1); a `*` anywhere else is
/// a literal character. Point resolution refuses a pattern (it names a set, not
/// a file); [`FileResolver::expand_pattern`] expands it.
///
/// This is a free function (not a method on `SpecAddress`) by the perimeter:
/// `address.rs` is out of scope, and co-locating the predicate with the matcher
/// keeps the glob vocabulary in one module. It is re-exported at the crate root
/// so the pipeline (its next consumer) reaches it as `vibe_spec::is_pattern`.
pub fn is_pattern(addr: &SpecAddress) -> bool {
    matches!(&addr.authority, Authority::Package { name, .. } if name.contains('*'))
}

impl FileResolver {
    /// Expand a pattern address (a `*` in the package name) into the concrete
    /// addresses it names — sorted by package name (byte order) and
    /// deterministic. Each member's version is the freshest installed (B-028);
    /// each member carries the pattern's group, doc-path, and anchor. A pattern
    /// that matches nothing is the empty vector (law 4), never an error.
    ///
    /// A non-pattern address is treated as a pattern that matches exactly
    /// itself, so this is the uniform "what addresses does this denote" oracle.
    pub fn expand_pattern(&self, addr: &SpecAddress) -> Result<Vec<SpecAddress>, ResolveError> {
        // A non-pattern address denotes exactly itself: return it before any
        // directory scan, so expansion is a total oracle (its docblock promise).
        // Scanning vibedeps/ instead would silently drop an address whose
        // document lives outside vibedeps/ — above all a self-coordinate
        // address, which resolves under ws_root/spec and never has a vibedeps/
        // slot. Silent source loss is exactly the defect class B-055 closes.
        // A point address still fails loudly where it belongs: in resolve_file.
        if !is_pattern(addr) {
            return Ok(vec![addr.clone()]);
        }
        // A pattern always carries a package name; an undotted host authority is
        // never a pattern (is_pattern is false for it), so it returned above.
        let Authority::Package { group, name, .. } = &addr.authority else {
            return Ok(Vec::new());
        };
        let pat = name.as_str();

        // Scan every vibedeps/ slot, split each directory name on its LAST
        // dot into `<group>.<name>` (РТ-3; the slot is identity-keyed and the
        // name is a single dot-free LDH label, so the last dot is always the
        // boundary), and keep the package name where it matches the pattern.
        // The group is not consulted (РТ-2): slots are matched by name, and
        // matching stays group-blind even now that the directory carries one.
        let vibedeps = vibedeps_root_under(&self.ws_root);
        let mut matched: Vec<(String, PathBuf)> = Vec::new();
        for entry in read_dir_or_empty(&vibedeps) {
            let slot_dir = entry.path();
            if !slot_dir.is_dir() {
                continue;
            }
            let Some(dir_name) = slot_dir.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            // Cut on the LAST dot: left = group, right = package name (the
            // name never contains a dot; the group may contain many).
            let Some((_group, pkg_name)) = dir_name.rsplit_once('.') else {
                continue; // not a `<group>.<name>` slot
            };
            if pkg_name.is_empty() || !name_matches(pat, pkg_name) {
                continue;
            }
            matched.push((pkg_name.to_string(), slot_dir));
        }

        // Sort BEFORE building the addresses (law 5). The key is the pair
        // (package name, slot directory name): a shared name under several
        // kinds is indistinguishable by address (it carries no kind), so the
        // names collapse either way — but sorting on the slot dir too means the
        // survivor is chosen by rule (the lexicographically-smallest slot),
        // never by the filesystem's read order. One tree ⇒ one result.
        matched.sort_by(|a, b| match a.0.cmp(&b.0) {
            std::cmp::Ordering::Equal => a.1.cmp(&b.1),
            ord => ord,
        });
        matched.dedup_by(|a, b| a.0 == b.0);

        let mut out: Vec<SpecAddress> = Vec::new();
        for (name, slot_dir) in matched {
            // Membership (law 3): name matched AND the document is present in
            // the freshest installed version (law 6 — B-028, no second version
            // rule). A matched package missing the doc is not a member; this is
            // not silent swallowing — both halves of the address define the set.
            let Some(spec_root) = freshest_spec_root(&slot_dir) else {
                continue; // no version installed → not a resolvable member
            };
            if resolve_doc(&spec_root, &addr.doc_path).is_ok()
                && let Some(concrete) = build_address(group, &name, &addr.doc_path, &addr.anchor)
            {
                out.push(concrete);
            }
            // (Kept as two conditions so a doc-miss never builds an address.)
        }
        Ok(out)
    }
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

/// The specs root of the freshest installed version under `slot_dir`, or
/// `None` when the slot holds no version directory. Reuses
/// `version_order::newest` (B-028) so there is one version rule, not two. A
/// directory whose name does not start with a digit is not a version candidate.
fn freshest_spec_root(slot_dir: &Path) -> Option<PathBuf> {
    let candidates: Vec<String> = read_dir_or_empty(slot_dir)
        .filter_map(|e| {
            let p = e.path();
            if !p.is_dir() {
                return None;
            }
            let n = p.file_name()?.to_str()?;
            n.chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
                .then_some(n.to_string())
        })
        .collect();
    let newest = version_order::newest(candidates.iter().map(String::as_str))?;
    Some(specs_root_under(&slot_dir.join(newest)))
}

/// Build the canonical concrete address for a matched package and round-trip it
/// through `SpecAddress::parse` so its `raw` is consistent (РТ-4 — construct the
/// canonical string and parse it, rather than assembling the struct field by
/// field). The address carries the pattern's group, doc-path, and anchor, the
/// matched name, and **no** version pin (law 6: the freshest is chosen at
/// resolve time) or revision pin. Returns `None` only if a matched directory
/// name cannot form a legal address (e.g. it carries whitespace) — which does
/// not occur in real vibedeps slots — so such a name is skipped rather than
/// failing the whole expansion (law 4 stays total).
fn build_address(
    group: &str,
    name: &str,
    doc_path: &str,
    anchor: &[String],
) -> Option<SpecAddress> {
    let mut s = String::from("spec://");
    s.push_str(group);
    s.push('/');
    s.push_str(name);
    s.push('/');
    s.push_str(doc_path);
    if !anchor.is_empty() {
        s.push('#');
        s.push_str(&anchor.join("."));
    }
    SpecAddress::parse(&s).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SelfCoordinate;
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
    fn g7_hyphen_split_on_first_hyphen() {
        // G7 (РТ-3): `plugin-*` matches the package `plugin-alpha` (directory
        // flow-plugin-alpha), NOT the package `alpha` (directory flow-alpha) —
        // the cut is on the FIRST hyphen: kind | name.
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
        // does not participate in matching — РТ-2).
        assert!(!is_pattern(
            &SpecAddress::parse("spec://org.*/abc/d").unwrap()
        ));
        // An undotted host authority is not a package, so never a pattern.
        assert!(!is_pattern(&SpecAddress::parse("spec://demo/a/b").unwrap()));
    }
}
