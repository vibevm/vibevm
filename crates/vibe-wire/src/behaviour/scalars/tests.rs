//! The shared grammars, proven once. Both validation cells lean on
//! these, so an edge case fixed here is fixed on both wires — which is
//! the whole reason the predicates stopped being written twice.

use super::{
    ProviderKeyDefect, RelativePathDefect, canonical_decimal_at_most, has_control_bytes,
    is_canonical_decimal, is_portable_token, is_sha256, provider_key_defect, relative_path_defect,
};

#[test]
fn every_relative_path_defect_is_independently_reachable() {
    for (value, defect) in [
        ("", RelativePathDefect::Blank),
        ("   ", RelativePathDefect::Blank),
        ("src\\lib.rs", RelativePathDefect::Backslash),
        ("src/lib.rs\n", RelativePathDefect::ControlByte),
        ("/etc/passwd", RelativePathDefect::Absolute),
        ("C:/work/demo", RelativePathDefect::DriveLetter),
        ("../sibling", RelativePathDefect::ParentSegment),
        ("src/../lib.rs", RelativePathDefect::ParentSegment),
        ("./src", RelativePathDefect::DotSegment),
        ("src/./lib.rs", RelativePathDefect::DotSegment),
        ("src//lib.rs", RelativePathDefect::EmptySegment),
        ("src/", RelativePathDefect::EmptySegment),
    ] {
        assert_eq!(
            relative_path_defect(value),
            Some(defect),
            "{value:?} must refuse as {defect:?}"
        );
        assert!(!defect.phrase().is_empty());
    }
}

#[test]
fn ordinary_project_relative_paths_and_globs_hold() {
    for value in [
        "src/lib.rs",
        "src/**",
        "docs/guide.md",
        "Cargo.toml",
        "crates/demo/src/build.rs",
        "a.b/c-d/e_f.rs",
        // A bare `..`-looking NAME is not a `..` SEGMENT.
        "src/..hidden/x.rs",
        // A single leading character that is not followed by `:` is not
        // a drive prefix.
        "C/work",
    ] {
        assert_eq!(relative_path_defect(value), None, "{value:?} must hold");
    }
}

#[test]
fn control_bytes_are_the_three_a_reader_cannot_print() {
    for value in ["a\rb", "a\nb", "a\0b"] {
        assert!(has_control_bytes(value), "{value:?}");
    }
    assert!(!has_control_bytes("a\tb"), "a tab is legal text");
    assert!(!has_control_bytes(""));
}

#[test]
fn the_sha256_spelling_is_scheme_plus_sixty_four_lowercase_hex() {
    let hex = "0123456789abcdef".repeat(4);
    assert_eq!(hex.len(), 64);
    assert!(is_sha256(&format!("sha256:{hex}")));
    assert!(!is_sha256(&hex), "the scheme is part of the spelling");
    assert!(!is_sha256(&format!("sha256:{}", hex.to_uppercase())));
    assert!(!is_sha256(&format!("sha256:{}", &hex[..63])));
    assert!(!is_sha256(&format!("sha256:{hex}0")));
    assert!(!is_sha256(&format!("blake3:{hex}")));
    assert!(!is_sha256(""));
}

#[test]
fn a_canonical_decimal_is_lossless_and_never_narrowed() {
    // Zero, a u32 boundary, a u64 boundary, and a value past u64::MAX
    // are all canonical: the string carries what no machine integer
    // could, which is exactly why the wire uses one.
    for value in [
        "0",
        "1234",
        "4294967295",
        "4294967296",
        "18446744073709551615",
        "18446744073709551616",
    ] {
        assert!(is_canonical_decimal(value), "{value:?} is canonical");
    }
    for value in ["", " ", "01", "0x10", "1_000", "-1", "1.0", "12 ", "١٢٣"] {
        assert!(!is_canonical_decimal(value), "{value:?} is not canonical");
    }
}

#[test]
fn canonical_decimals_compare_by_length_then_lexicographically() {
    assert!(canonical_decimal_at_most("0", "0"));
    assert!(canonical_decimal_at_most("9", "10"));
    assert!(!canonical_decimal_at_most("10", "9"));
    assert!(canonical_decimal_at_most(
        "18446744073709551615",
        "18446744073709551616"
    ));
    assert!(!canonical_decimal_at_most(
        "18446744073709551617",
        "18446744073709551616"
    ));
}

#[test]
fn every_provider_key_defect_is_independently_reachable() {
    for (value, defect) in [
        ("", ProviderKeyDefect::Blank),
        ("   ", ProviderKeyDefect::Blank),
        ("org.demo/tools\n", ProviderKeyDefect::Blank),
        ("org.demo/tools", ProviderKeyDefect::NoHash),
        ("org.demo/tools#x#y", ProviderKeyDefect::DuplicateHash),
        ("tools#announce", ProviderKeyDefect::NoSlash),
        ("org.demo/a/b#x", ProviderKeyDefect::DuplicateSlash),
        ("org.demo#tools/x", ProviderKeyDefect::HashBeforeSlash),
        ("/tools#x", ProviderKeyDefect::BlankComponent),
        ("org.demo/#x", ProviderKeyDefect::BlankComponent),
        ("org.demo/tools#", ProviderKeyDefect::BlankComponent),
        ("org.demo/  #x", ProviderKeyDefect::BlankComponent),
    ] {
        assert_eq!(
            provider_key_defect(value),
            Some(defect),
            "{value:?} must refuse as {defect:?}"
        );
        assert!(!defect.phrase().is_empty());
    }
}

#[test]
fn the_extension_key_spellings_hold() {
    for value in [
        "org.vibevm/vibe#cargo",
        "org.demo/tools#announce",
        "__host__/demo#x",
        "org.example/build-tools#cargo-v2",
        "org.acme/deploy#server",
    ] {
        assert_eq!(provider_key_defect(value), None, "{value:?} must hold");
    }
}

/// The mechanism plane's portable-token law, pinned case for case against
/// its manifest authority (`vibe-core` `manifest::mechanism::
/// is_portable_token`) — the parity that keeps the two copies honest until
/// the manifest side exports one. Note the two laws deliberately differ:
/// `_` and a trailing `-`/`.` are backend-id-legal and portable-token
/// ILLEGAL, and the portable token carries no length cap.
#[test]
fn the_portable_token_law_matches_its_manifest_authority() {
    for value in [
        "vibe-helper.exe",
        "cargo",
        "a",
        "0",
        "x.y-z",
        "team-plugin",
        &"x".repeat(65),
    ] {
        assert!(is_portable_token(value), "{value:?} is a portable token");
    }
    for value in [
        "",
        ".hidden",
        "-leading",
        "trailing-",
        "trailing.",
        "Upper",
        "with space",
        "with/slash",
        "under_score",
        "dot..run",
    ] {
        assert!(
            !is_portable_token(value),
            "{value:?} is not a portable token"
        );
    }
}
