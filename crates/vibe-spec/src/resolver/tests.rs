//! Resolver tests, out-of-line so the module stays within the file-length
//! budget (the pattern `mdspec/tests.rs` sets in the engine; the inline form
//! this file carried until the XML-serialisation tests outgrew it).

use std::fs;

use super::*;

// ----- B-031: the host is a package coordinate (Т1–Т6) ------------------

/// The self coordinate the host project carries since B-031.
fn host_coord() -> SelfCoordinate {
    SelfCoordinate::new(Some("org.vibevm.core".into()), "vibevm".into())
}

#[test]
fn t1_self_coordinate_resolves_to_the_authored_spec_tree() {
    // Т1: `spec://<self_group>/<self_name>/…` resolves under ws_root/spec,
    // ahead of any vibedeps/ slot lookup (B-031 — the self-match is first).
    let ws = tempfile::TempDir::new().unwrap();
    let doc = specs_root_under(ws.path()).join("common").join("TARGET.md");
    fs::create_dir_all(doc.parent().unwrap()).unwrap();
    fs::write(&doc, "# Target\n").unwrap();
    let r = FileResolver::new(ws.path(), host_coord());
    let addr = SpecAddress::parse("spec://org.vibevm.core/vibevm/common/TARGET").unwrap();
    let file = r.resolve_file(&addr).unwrap();
    assert!(file.ends_with("TARGET.md"), "{file:?}");
}

#[test]
fn t2_legacy_host_authority_names_the_self_coordinate_and_b031() {
    // Т2: an undotted (legacy-host-shaped) authority no longer resolves;
    // the error points at the actual self coordinate and cites B-031. The
    // input is built by concatenation so the retired literal never sits in
    // source where the B-031 migrator (or a reader) could take it for a
    // live address; the arm and the hint are identical for every undotted
    // token.
    let r = FileResolver::new(Path::new("."), host_coord());
    let legacy = concat!("spec://", "vibevm", "/common/PROP-000#commits");
    let addr = SpecAddress::parse(legacy).unwrap();
    let err = r.resolve_file(&addr).unwrap_err();
    let ResolveError::LegacyHostAuthority { given, hint } = &err else {
        panic!("expected LegacyHostAuthority, got {err:?}");
    };
    assert_eq!(given, "vibevm");
    assert!(hint.contains("org.vibevm.core/vibevm"), "{hint}");
    assert!(hint.contains("B-031"), "{hint}");
}

#[test]
fn t3_any_undotted_authority_never_resolves() {
    // Т3: a fixture-style undotted authority (`spec://demo/…`) parses but
    // never resolves — the same legacy-host error as the real token.
    let r = FileResolver::new(Path::new("."), host_coord());
    let addr = SpecAddress::parse("spec://demo/x/y#z").unwrap();
    let err = r.resolve_file(&addr).unwrap_err();
    assert!(matches!(err, ResolveError::LegacyHostAuthority { .. }));
    // The hint still points at the self coordinate, not at `demo`.
    let hint = err.to_string();
    assert!(hint.contains("org.vibevm.core/vibevm"), "{hint}");
}

#[test]
fn t4_a_non_self_package_resolves_to_its_vibedeps_slot() {
    // Т4: a package coordinate that is NOT the self coordinate falls through
    // to the vibedeps/ slot lookup, unchanged from before B-031.
    let ws = tempfile::TempDir::new().unwrap();
    let doc = specs_root_under(&vibedeps_root_under(ws.path()).join("org.vibevm.demo.demo/1.0.0"))
        .join("contract")
        .join("API.md");
    fs::create_dir_all(doc.parent().unwrap()).unwrap();
    fs::write(&doc, "# API\n").unwrap();
    let r = FileResolver::new(ws.path(), host_coord());
    let addr = SpecAddress::parse("spec://org.vibevm.demo/demo/contract/API#root").unwrap();
    let file = r.resolve_file(&addr).unwrap();
    assert!(file.ends_with("API.md"), "{file:?}");
}

#[test]
fn t5_a_groupless_project_has_no_self_coordinate() {
    // Т5: a project with no `group` declares no self coordinate. Its own
    // name in package form does NOT resolve to spec/ (it falls through to a
    // vibedeps slot lookup that finds nothing), and an undotted authority
    // errors "no self coordinate".
    let ws = tempfile::TempDir::new().unwrap();
    let coord = SelfCoordinate::new(None, "solo".into());
    let r = FileResolver::new(ws.path(), coord);

    // Package form of the project's own name → slot lookup, not vibevm/vibespecs/.
    let pkg = SpecAddress::parse("spec://org.foo/solo/x/y").unwrap();
    assert!(matches!(
        r.resolve_file(&pkg).unwrap_err(),
        ResolveError::PackageSlotNotFound(_)
    ));

    // Undotted authority → "no self coordinate".
    let undotted = SpecAddress::parse("spec://solo/x/y").unwrap();
    let err = r.resolve_file(&undotted).unwrap_err();
    assert!(err.to_string().contains("no self coordinate"), "{}", err);
}

#[test]
fn t6_a_versioned_self_coordinate_is_an_error() {
    // Т6 (У2): a self-coordinate address carrying `@version` is an error —
    // the self coordinate is unversioned, so the pin is never dropped.
    let r = FileResolver::new(Path::new("."), host_coord());
    let addr = SpecAddress::parse("spec://org.vibevm.core/vibevm@0.1/common/TARGET").unwrap();
    let err = r.resolve_file(&addr).unwrap_err();
    let ResolveError::SelfCoordinateVersioned {
        self_group,
        self_name,
        version,
    } = &err
    else {
        panic!("expected SelfCoordinateVersioned, got {err:?}");
    };
    assert_eq!(self_group, "org.vibevm.core");
    assert_eq!(self_name, "vibevm");
    assert_eq!(version, "0.1");
}

#[test]
fn id_stem_recognition() {
    assert!(is_id_stem("PROP-042"));
    assert!(is_id_stem("FEAT-7"));
    assert!(!is_id_stem("PROP"));
    assert!(!is_id_stem("PROP-"));
    assert!(!is_id_stem("README"));
    assert!(!is_id_stem("PROP-00x"));
    assert!(!is_id_stem("DESIGN-1")); // only PROP / FEAT truncate
}

#[test]
fn id_file_match() {
    assert!(id_file_matches(
        Path::new("PROP-042-example-thing.md"),
        "PROP-042"
    ));
    assert!(id_file_matches(Path::new("PROP-042.md"), "PROP-042"));
    // The XML serialisation of the same document matches the same id.
    assert!(id_file_matches(
        Path::new("PROP-042-example-thing.xml"),
        "PROP-042"
    ));
    assert!(id_file_matches(Path::new("PROP-042.xml"), "PROP-042"));
    // A different number sharing a prefix does not match.
    assert!(!id_file_matches(Path::new("PROP-0420-x.md"), "PROP-042"));
    assert!(!id_file_matches(
        Path::new("PROP-042-example.txt"),
        "PROP-042"
    ));
}

// ----- XML serialisation (PROP-045 ##ADDRESSING-UNCHANGED) ---------------

#[test]
fn an_xml_document_resolves_under_the_same_address() {
    // The plain-stem branch: `spec/common/DEP` finds `DEP.xml` when no
    // `DEP.md` sits beside it — the address never names the form.
    let ws = tempfile::TempDir::new().unwrap();
    let dir = specs_root_under(ws.path()).join("common");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("DEP.xml"), "<spec/>").unwrap();
    let r = FileResolver::new(ws.path(), host_coord());
    let addr = SpecAddress::parse("spec://org.vibevm.core/vibevm/common/DEP").unwrap();
    let file = r.resolve_file(&addr).unwrap();
    assert!(file.ends_with("DEP.xml"), "{file:?}");
}

#[test]
fn an_id_stem_resolves_its_xml_slug_form() {
    // The `PROP-NNN` truncation inverts over the XML form too:
    // `PROP-045` finds `PROP-045-xml-spec-sources.xml`.
    let ws = tempfile::TempDir::new().unwrap();
    let dir = specs_root_under(ws.path()).join("common");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("PROP-045-xml-spec-sources.xml"), "<spec/>").unwrap();
    let r = FileResolver::new(ws.path(), host_coord());
    let addr = SpecAddress::parse("spec://org.vibevm.core/vibevm/common/PROP-045").unwrap();
    let file = r.resolve_file(&addr).unwrap();
    assert!(file.ends_with("PROP-045-xml-spec-sources.xml"), "{file:?}");
}

#[test]
fn a_document_in_both_forms_is_a_loud_pair_collision() {
    // `DEP.md` + `DEP.xml` beside each other: one logical document in
    // two forms — the resolver reports both paths and refuses to guess.
    let ws = tempfile::TempDir::new().unwrap();
    let dir = specs_root_under(ws.path()).join("common");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("DEP.md"), "# Dep\n").unwrap();
    fs::write(dir.join("DEP.xml"), "<spec/>").unwrap();
    let r = FileResolver::new(ws.path(), host_coord());
    let addr = SpecAddress::parse("spec://org.vibevm.core/vibevm/common/DEP").unwrap();
    let err = r.resolve_file(&addr).unwrap_err();
    let ResolveError::PairCollision { markdown, xml } = &err else {
        panic!("expected PairCollision, got {err:?}")
    };
    assert!(markdown.ends_with("DEP.md"), "{markdown:?}");
    assert!(xml.ends_with("DEP.xml"), "{xml:?}");
    let msg = err.to_string();
    assert!(msg.contains("one document, one form"), "{msg}");
    assert!(msg.contains("DEP.md") && msg.contains("DEP.xml"), "{msg}");
}

// ----- B-028: an absent version resolves to the freshest (F1–F6) --------

/// Build a vibedeps slot `<kind>-<name>` holding the given installed
/// versions, each with `spec/API.md` (the doc every F-test resolves).
/// Returns the slot path so a caller may add non-version neighbours.
fn make_versions(ws: &Path, group: &str, name: &str, versions: &[&str]) -> PathBuf {
    let slot = vibedeps_root_under(ws).join(format!("{group}.{name}"));
    for v in versions {
        let dir = specs_root_under(&slot.join(v));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("API.md"), "# API\n").unwrap();
    }
    slot
}

#[test]
fn f1_a_single_installed_version_is_taken() {
    // F1: one installed version — an absent `@version` takes it (as before).
    let ws = tempfile::TempDir::new().unwrap();
    make_versions(ws.path(), "org.demo", "widget", &["1.0.0"]);
    let r = FileResolver::new(ws.path(), host_coord());
    let addr = SpecAddress::parse("spec://org.demo/widget/API").unwrap();
    let file = r.resolve_file(&addr).unwrap();
    assert!(
        file.ends_with(format!("1.0.0/{}/API.md", LEGACY_SPECS_ROOT)),
        "{file:?}"
    );
}

#[test]
fn f2_two_versions_compare_numerically_not_lexicographically() {
    // F2: `0.9.0` and `0.10.0` — the freshest is `0.10.0` (numeric segment
    // compare, not lexicographic).
    let ws = tempfile::TempDir::new().unwrap();
    make_versions(ws.path(), "org.demo", "widget", &["0.9.0", "0.10.0"]);
    let r = FileResolver::new(ws.path(), host_coord());
    let addr = SpecAddress::parse("spec://org.demo/widget/API").unwrap();
    let file = r.resolve_file(&addr).unwrap();
    assert!(
        file.ends_with(format!("0.10.0/{}/API.md", LEGACY_SPECS_ROOT)),
        "{file:?}"
    );
}

#[test]
fn f3_a_release_beats_its_pre_release() {
    // F3: `1.0.0` and `1.0.0-alpha` — the release `1.0.0` is fresher.
    let ws = tempfile::TempDir::new().unwrap();
    make_versions(ws.path(), "org.demo", "widget", &["1.0.0", "1.0.0-alpha"]);
    let r = FileResolver::new(ws.path(), host_coord());
    let addr = SpecAddress::parse("spec://org.demo/widget/API").unwrap();
    let file = r.resolve_file(&addr).unwrap();
    assert!(
        file.ends_with(format!("1.0.0/{}/API.md", LEGACY_SPECS_ROOT)),
        "{file:?}"
    );
}

#[test]
fn f4_an_explicit_version_pins_even_the_non_newest() {
    // F4: an explicit `@version` names the exact slot — including one that
    // is NOT the freshest (pinning `1.0.0` under a newer `2.0.0`).
    let ws = tempfile::TempDir::new().unwrap();
    make_versions(ws.path(), "org.demo", "widget", &["1.0.0", "2.0.0"]);
    let r = FileResolver::new(ws.path(), host_coord());
    let addr = SpecAddress::parse("spec://org.demo/widget@1.0.0/API").unwrap();
    let file = r.resolve_file(&addr).unwrap();
    assert!(
        file.ends_with(format!("1.0.0/{}/API.md", LEGACY_SPECS_ROOT)),
        "{file:?}"
    );
}

#[test]
fn f5_no_version_directories_is_slot_not_found() {
    // F5: a slot with no version directories → PackageSlotNotFound. The slot
    // holds only a non-version folder (`notes`, B-028 У1): such a directory
    // is not a version candidate, so the candidate set is empty.
    let ws = tempfile::TempDir::new().unwrap();
    let slot = make_versions(ws.path(), "org.demo", "widget", &[]);
    fs::create_dir_all(slot.join("notes")).unwrap();
    let r = FileResolver::new(ws.path(), host_coord());
    let addr = SpecAddress::parse("spec://org.demo/widget/API").unwrap();
    let err = r.resolve_file(&addr).unwrap_err();
    assert!(
        matches!(err, ResolveError::PackageSlotNotFound(_)),
        "{err:?}"
    );
}

#[test]
fn f6_more_segments_at_equal_prefix_is_newer() {
    // F6: `1.2` and `1.2.1` — the freshest is `1.2.1` (more segments).
    let ws = tempfile::TempDir::new().unwrap();
    make_versions(ws.path(), "org.demo", "widget", &["1.2", "1.2.1"]);
    let r = FileResolver::new(ws.path(), host_coord());
    let addr = SpecAddress::parse("spec://org.demo/widget/API").unwrap();
    let file = r.resolve_file(&addr).unwrap();
    assert!(
        file.ends_with(format!("1.2.1/{}/API.md", LEGACY_SPECS_ROOT)),
        "{file:?}"
    );
}

#[test]
fn canonical_doc_path_is_the_forward_half_of_resolution() {
    use crate::resolver::canonical_doc_path;
    // The docstring examples, pinned: slug truncation, extension strip,
    // spec/-relativity, and the no-id full-stem cases.
    assert_eq!(
        canonical_doc_path("spec/modules/x/PROP-003-dep-evolution.md"),
        "modules/x/PROP-003"
    );
    assert_eq!(
        canonical_doc_path("spec/common/PROP-046-adoption-facts-registry.md"),
        "common/PROP-046"
    );
    assert_eq!(
        canonical_doc_path("spec/common/FEAT-012-thing.xml"),
        "common/FEAT-012"
    );
    assert_eq!(canonical_doc_path("spec/boot/00-core.md"), "boot/00-core");
    assert_eq!(canonical_doc_path("spec/WAL.md"), "WAL");
    // PROP without an all-digit number is not an id stem.
    assert_eq!(
        canonical_doc_path("spec/PROP-abc-notes.md"),
        "PROP-abc-notes"
    );
}

// ----- selected world: exact roots, exact versions ---------------------

/// A selected world is a claim about *which instance* each coordinate is, so
/// an address that pins a version is checked against it rather than having the
/// pin quietly dropped. Four rows, one for each way an address can relate to
/// the lock.
#[test]
fn a_selected_world_enforces_the_exact_locked_version() {
    use std::collections::BTreeMap;

    use crate::SelectedPackage;

    let ws = tempfile::tempdir().unwrap();
    let b_one = ws.path().join("b-1.0.0");
    let doc = b_one.join(NEW_SPECS_ROOT).join("common/note.md");
    fs::create_dir_all(doc.parent().unwrap()).unwrap();
    fs::write(&doc, "# Note {#root}\n\nselected\n").unwrap();

    let mut selected = BTreeMap::new();
    selected.insert(
        ("org.demo".to_string(), "b".to_string()),
        SelectedPackage::new("1.0.0", &b_one),
    );
    let resolver = FileResolver::with_selected_world(
        ws.path().join("a-1.0.0"),
        ws.path(),
        selected,
        SelfCoordinate::new(Some("org.demo".into()), "a".into()),
    );

    // Unversioned: whatever the lock chose.
    let unversioned = SpecAddress::parse("spec://org.demo/b/common/note").unwrap();
    assert_eq!(resolver.resolve_file(&unversioned).unwrap(), doc);

    // Agreeing pin: the author's claim and the lock say the same thing.
    let agreeing = SpecAddress::parse("spec://org.demo/b@1.0.0/common/note").unwrap();
    assert_eq!(resolver.resolve_file(&agreeing).unwrap(), doc);

    // Disagreeing pin: refuse, and name BOTH numbers — the only interesting
    // question is "which one did I get", and silence answers it wrongly.
    let disagreeing = SpecAddress::parse("spec://org.demo/b@2.0.0/common/note").unwrap();
    let error = resolver.resolve_file(&disagreeing).unwrap_err().to_string();
    assert!(
        error.contains("2.0.0") && error.contains("1.0.0"),
        "{error}"
    );

    // Unselected coordinate: still refused, never scanned for.
    let unselected = SpecAddress::parse("spec://org.demo/c/common/note").unwrap();
    let error = resolver.resolve_file(&unselected).unwrap_err().to_string();
    assert!(error.contains("selected world"), "{error}");
}

/// Even with a newer instance sitting on disk beside the selected one, the
/// selected world never scans: the lock's answer is the only answer.
#[test]
fn a_selected_world_never_falls_back_to_the_freshest_installed_slot() {
    use std::collections::BTreeMap;

    use crate::SelectedPackage;

    let ws = tempfile::tempdir().unwrap();
    for (version, body) in [("1.0.0", "selected\n"), ("2.0.0", "newer\n")] {
        let slot = ws
            .path()
            .join(NEW_VIBEDEPS_ROOT)
            .join("org.demo.b")
            .join(version);
        let doc = slot.join(NEW_SPECS_ROOT).join("common/note.md");
        fs::create_dir_all(doc.parent().unwrap()).unwrap();
        fs::write(&doc, format!("# Note {{#root}}\n\n{body}")).unwrap();
    }
    let chosen = ws
        .path()
        .join(NEW_VIBEDEPS_ROOT)
        .join("org.demo.b")
        .join("1.0.0");

    let mut selected = BTreeMap::new();
    selected.insert(
        ("org.demo".to_string(), "b".to_string()),
        SelectedPackage::new("1.0.0", &chosen),
    );
    let resolver = FileResolver::with_selected_world(
        ws.path().join("a"),
        ws.path(),
        selected,
        SelfCoordinate::new(Some("org.demo".into()), "a".into()),
    );
    let address = SpecAddress::parse("spec://org.demo/b/common/note").unwrap();
    let resolved = resolver.resolve_file(&address).unwrap();
    assert!(
        fs::read_to_string(&resolved).unwrap().contains("selected"),
        "the lock-selected instance must answer, not the newest installed",
    );
}

/// Every refusal an operator can be shown is one sentence, not a source
/// literal's indentation. A multi-line `#[error]` written without a `\`
/// continuation bakes the leading whitespace of the next line into the
/// message; nothing else in the suite would notice, because every other
/// assertion is a `contains` of a fragment on one side of the seam.
#[test]
fn resolve_refusals_render_as_single_spaced_sentences() {
    let rendered = [
        ResolveError::UnselectedPackage {
            given: "org.demo/b".into(),
        }
        .to_string(),
        ResolveError::SelectedVersionMismatch {
            given: "org.demo/b".into(),
            requested: "2.0.0".into(),
            selected: "1.0.0".into(),
        }
        .to_string(),
        ResolveError::SelfCoordinateVersioned {
            self_group: "org.demo".into(),
            self_name: "a".into(),
            version: "1.0.0".into(),
        }
        .to_string(),
    ];
    assert_eq!(
        rendered[0],
        "package `org.demo/b` is not in this resolver's selected world; only coordinates the \
         lock selected can be reached",
    );
    assert_eq!(
        rendered[1],
        "address pins `org.demo/b@2.0.0`, but the selected world holds `org.demo/b@1.0.0`; \
         the pin and the lock disagree",
    );
    for message in &rendered {
        assert!(
            !message.contains("  "),
            "a run of spaces is baked source indentation: {message}",
        );
        assert!(!message.is_empty());
    }
}
