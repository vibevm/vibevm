//! Focused tests for the consumer-side `[compile]` table.
//!
//! The atom under proof is small enough that every assertion is a concrete
//! TOML round-trip: what the reader accepts, what the writer emits back, and
//! what both refuse — never a substring-only happy path.

use specmark::verifies;

use super::CompileSection;
use crate::Error;
use crate::manifest::{Manifest, NodeRole};

const OBS_TRACE: &str = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE";
const ROLES_EQUIPOTENT: &str =
    "spec://org.vibevm.core/vibevm/common/PROP-024#MANIFEST-ROLES-ARE-EQUIPOTENT";

const PROJECT: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";
const PACKAGE: &str = concat!(
    "[package]\n",
    "group = \"org.vibevm\"\n",
    "name = \"wal\"\n",
    "kind = \"flow\"\n",
    "version = \"0.3.0\"\n",
);
const VIRTUAL: &str = "[workspace]\nmembers = []\n";

/// Every root shape the consumer table is legal on: the two roles, the
/// virtual coordinator, and each role combined with `[workspace]`.
fn roots() -> Vec<(&'static str, String)> {
    vec![
        ("project", PROJECT.to_string()),
        ("package", PACKAGE.to_string()),
        ("virtual workspace", VIRTUAL.to_string()),
        ("project + workspace", format!("{PROJECT}\n{VIRTUAL}")),
        ("package + workspace", format!("{PACKAGE}\n{VIRTUAL}")),
    ]
}

fn parse(body: &str) -> Manifest {
    Manifest::parse_str(body).unwrap()
}

fn parse_error(body: &str) -> String {
    Manifest::parse_str(body).unwrap_err().to_string()
}

// --- 1. absent table ------------------------------------------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn missing_table_is_false_and_stays_off_the_page() {
    for (label, root) in roots() {
        let manifest = parse(&root);
        assert!(
            !manifest.compile.trace,
            "{label}: absent [compile] must read false"
        );
        assert!(manifest.compile.is_default(), "{label}");

        let rendered = toml::to_string_pretty(&manifest).unwrap();
        assert!(
            !rendered.contains("compile"),
            "{label}: default table leaked into the write:\n{rendered}"
        );
        assert_eq!(parse(&rendered), manifest, "{label}");
    }
}

// --- 2. explicit `false` --------------------------------------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn explicit_false_equals_absent_and_canonicalises_the_table_away() {
    for (label, root) in roots() {
        let explicit = parse(&format!("{root}\n[compile]\ntrace = false\n"));
        let absent = parse(&root);
        assert!(!explicit.compile.trace, "{label}");
        assert_eq!(
            explicit, absent,
            "{label}: `trace = false` and an absent table are one value"
        );

        let rendered = toml::to_string_pretty(&explicit).unwrap();
        assert!(
            !rendered.contains("compile"),
            "{label}: canonical write must drop the default table:\n{rendered}"
        );
    }
}

// --- 3. `true` round-trips on every root ----------------------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn true_round_trips_for_every_root_shape() {
    for (label, root) in roots() {
        let manifest = parse(&format!("{root}\n[compile]\ntrace = true\n"));
        assert!(manifest.compile.trace, "{label}");

        let rendered = toml::to_string_pretty(&manifest).unwrap();
        assert!(
            rendered.contains("[compile]\ntrace = true\n"),
            "{label}: the asked-for table must be written back:\n{rendered}"
        );

        // Parse → write → parse → write is a fixed point, values and bytes.
        let reparsed = parse(&rendered);
        assert_eq!(reparsed, manifest, "{label}");
        assert!(reparsed.compile.trace, "{label}");
        assert_eq!(
            toml::to_string_pretty(&reparsed).unwrap(),
            rendered,
            "{label}"
        );
    }
}

// --- 4. the package role accepts the consumer table -----------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-024#MANIFEST-ROLES-ARE-EQUIPOTENT")]
fn package_root_is_not_a_package_role_offender() {
    let package = parse(&format!("{PACKAGE}\n[compile]\ntrace = true\n"));
    // `validate` already ran inside `parse_str`; run it again explicitly so
    // the claim under test is the validator's verdict, not the parser's.
    package.validate().unwrap();
    assert_eq!(package.consumer_node().unwrap().role, NodeRole::Package);
    assert!(package.compile_trace_enabled());

    // And the mirror image: a project-rooted checkout says exactly the same.
    let project = parse(&format!("{PROJECT}\n[compile]\ntrace = true\n"));
    project.validate().unwrap();
    assert_eq!(
        project.compile_trace_enabled(),
        package.compile_trace_enabled(),
        "the two roles must answer the consumer control identically \
         (violates {ROLES_EQUIPOTENT})"
    );

    // A package-role section on a non-package root still refuses, so the
    // new table did not soften the offender list it sits beside.
    let offender = parse_error(&format!(
        "{PROJECT}\n[compile]\ntrace = true\n[compatibility]\nmin_vibe_version = \"0.1.0\"\n"
    ));
    assert!(
        offender.contains("[compatibility]") && offender.contains("without a [package] table"),
        "{offender}"
    );
}

// --- 5. the XOR diagnostic is untouched -----------------------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest")]
fn project_package_xor_diagnostics_are_unchanged() {
    let both = format!("{PROJECT}\n{PACKAGE}\n[compile]\ntrace = true\n");
    let with_table = parse_error(&both);
    assert!(
        with_table.contains("[project] and [package] are mutually exclusive"),
        "{with_table}"
    );

    // Byte-identical to the diagnostic the same document produces without
    // the new table — `[compile]` is invisible to the role law.
    let without_table = parse_error(&format!("{PROJECT}\n{PACKAGE}"));
    assert_eq!(with_table, without_table);

    // The no-role refusal is likewise untouched.
    let roleless = parse_error("[compile]\ntrace = true\n");
    assert!(roleless.contains("manifest declares no role"), "{roleless}");
}

// --- 6. strict refusals ---------------------------------------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn unknown_member_and_wrong_type_refuse() {
    for (body, expected) in [
        ("[compile]\ntrace = true\nverbose = true\n", "unknown field"),
        ("[compile]\nmystery = 1\n", "unknown field"),
        ("[compile]\ntrace = \"yes\"\n", "invalid type"),
        ("[compile]\ntrace = 1\n", "invalid type"),
    ] {
        let raw = format!("{PROJECT}\n{body}");
        match Manifest::parse_str(&raw).unwrap_err() {
            Error::ParseToml { diagnostic, .. } => {
                let detail = diagnostic.to_string();
                assert!(
                    detail.contains(expected),
                    "expected `{expected}` for `{body}`, got: {detail}"
                );
            }
            other => panic!("expected ParseToml for `{body}`, got {other:?}"),
        }
    }

    // The table is a strict struct, not a free map: a nested sub-table is
    // refused the same way a stray scalar is.
    let nested = parse_error(&format!("{PROJECT}\n[compile.extra]\nk = 1\n"));
    assert!(nested.contains("unknown field"), "{nested}");
}

// --- 7. the role-blind read seam ------------------------------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-024#MANIFEST-ROLES-ARE-EQUIPOTENT")]
fn accessor_returns_exactly_the_root_value_for_every_role() {
    for (label, root) in roots() {
        for asked in [false, true] {
            let manifest = parse(&format!("{root}\n[compile]\ntrace = {asked}\n"));
            assert_eq!(
                manifest.compile_trace_enabled(),
                asked,
                "{label}: the accessor must report the root's own value \
                 (violates {OBS_TRACE})"
            );
            assert_eq!(manifest.compile_trace_enabled(), manifest.compile.trace);
        }
        // Absent table — the accessor is total, no role branch, no unwrap.
        assert!(!parse(&root).compile_trace_enabled(), "{label}");
    }
}

// --- the setting stays where it was declared ------------------------------

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn a_dependency_manifest_carries_its_own_switch_only() {
    // Two independently read manifests: the "dependency" asks for tracing,
    // the "host" does not. Nothing in this cell copies one into the other —
    // the host answers from its own root and only from there.
    let dependency = parse(&format!("{PACKAGE}\n[compile]\ntrace = true\n"));
    let host = parse(&format!(
        "{PROJECT}\n[requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n"
    ));
    assert!(dependency.compile_trace_enabled());
    assert!(!host.compile_trace_enabled());

    // The dependency's own write keeps the table; the host's write has none,
    // so the setting cannot ride an install into the host's manifest.
    assert!(
        toml::to_string_pretty(&dependency)
            .unwrap()
            .contains("[compile]")
    );
    assert!(
        !toml::to_string_pretty(&host).unwrap().contains("compile"),
        "the host manifest must stay free of the dependency's control"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
fn section_default_and_is_default_agree_with_the_wire() {
    assert_eq!(CompileSection::default(), CompileSection { trace: false });
    assert!(CompileSection::default().is_default());
    assert!(!CompileSection { trace: true }.is_default());
    assert_eq!(
        toml::to_string(&CompileSection { trace: true }).unwrap(),
        "trace = true\n"
    );
}
