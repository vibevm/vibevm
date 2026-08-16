//! Round-trip proofs for the open-vocabulary layer of codegen
//! post-processing (PROP-044 §4.2a), driven against the really
//! generated types — not against text. They guard three properties the
//! pass promises the wire: an unfamiliar value survives a read/write
//! cycle verbatim, every known value still lands in its named variant
//! and writes the same bytes, and a vocabulary annotated `closed`
//! (`NamingConvention`) keeps refusing unfamiliar values. If the pass
//! ever cross-wires a rename or drops the `Unknown` arm, one of these
//! fires.

use vibe_wire::generated::index::e1::entry::PackageKind;
use vibe_wire::generated::index::e1::repomd::NamingConvention;

/// Guards the tolerance that is the point of the open form: a wire value
/// this build did not know reads into `Unknown` carrying the string, and
/// writes back the identical bytes — an older reader neither drops nor
/// rewrites a newer writer's vocabulary.
#[test]
fn an_unfamiliar_value_survives_the_read_write_cycle() {
    let value: PackageKind = serde_json::from_str("\"plugin\"")
        .expect("an open vocabulary accepts a value it does not know");
    match &value {
        PackageKind::Unknown(seen) => assert_eq!(seen, "plugin"),
        _ => panic!("`plugin` must read into Unknown, not a named variant"),
    }
    let wire = serde_json::to_string(&value).expect("Unknown serialises as the string itself");
    assert_eq!(wire, "\"plugin\"");
}

/// Guards the known half of the stitch: all six wire values of
/// `PackageKind` read into their NAMED variants (not `Unknown`) and write
/// back the very bytes they came as — the wire-parity property the five
/// oracles check indirectly, pinned here on the type itself.
#[test]
fn every_known_value_reads_named_and_writes_identical_bytes() {
    for wire in ["feat", "flow", "lang", "mcp", "stack", "tool"] {
        let value: PackageKind =
            serde_json::from_str(&format!("\"{wire}\"")).expect("a known value parses");
        let canonical = match &value {
            PackageKind::Feat => "feat",
            PackageKind::Flow => "flow",
            PackageKind::Lang => "lang",
            PackageKind::Mcp => "mcp",
            PackageKind::Stack => "stack",
            PackageKind::Tool => "tool",
            PackageKind::Unknown(_) => {
                panic!("`{wire}` is a known value and must read into its named variant")
            }
        };
        assert_eq!(canonical, wire, "the variant matches the wire string");
        let written = serde_json::to_string(&value).expect("a known value serialises");
        assert_eq!(written, format!("\"{wire}\""));
    }
}

/// Guards the boundary of the policy: `naming_convention` is annotated
/// `closed` (repository paths are built from its values, so an
/// unfamiliar one has no safe behaviour), and the pass must leave it a
/// plain derived enum — an unfamiliar value is a parse error, never an
/// `Unknown` variant.
#[test]
fn a_closed_vocabulary_still_refuses_unfamiliar_values() {
    let parsed = serde_json::from_str::<NamingConvention>("\"bogus\"");
    assert!(
        parsed.is_err(),
        "NamingConvention is closed by `x-vocabulary`: an unfamiliar value \
         must be a parse error, not an Unknown variant"
    );
}
