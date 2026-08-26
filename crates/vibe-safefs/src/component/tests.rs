//! The one portable-name table, judged by counterexample.

use super::{
    SelectionFault, UnsafeComponent, classify_component, ensure_lexically_contained, identity_key,
    judge_selection, paths_overlap, split_relative,
};
use std::path::Path;
use vibe_core::manifest::DeclarantPathFault;

#[test]
fn ordinary_components_pass() {
    for component in ["guide.md", "docs", "a-b_c.1", "Ünicode.md"] {
        assert_eq!(classify_component(component), None, "{component}");
    }
}

/// Every row is a spelling that reached disk in some subsystem before one
/// shared table existed.
#[test]
fn the_shared_table_refuses_every_hostile_spelling() {
    for (component, expected) in [
        ("", DeclarantPathFault::EmptySegment),
        (".", DeclarantPathFault::DotSegment),
        ("..", DeclarantPathFault::DotSegment),
        ("a\\b", DeclarantPathFault::Backslash),
        ("C:", DeclarantPathFault::Colon),
        // NTFS alternate data stream: the colon is interior, not a drive.
        ("guide.md:ads", DeclarantPathFault::Colon),
        ("guide.md:$DATA", DeclarantPathFault::Colon),
        ("a/b", DeclarantPathFault::Separator),
        ("con", DeclarantPathFault::WindowsDevice),
        ("CON", DeclarantPathFault::WindowsDevice),
        ("COM1", DeclarantPathFault::WindowsDevice),
        ("com1.json", DeclarantPathFault::WindowsDevice),
        ("LPT9.log", DeclarantPathFault::WindowsDevice),
        ("NUL", DeclarantPathFault::WindowsDevice),
        ("CONIN$", DeclarantPathFault::WindowsDevice),
        ("trailing.", DeclarantPathFault::TrailingDotOrSpace),
        ("trailing ", DeclarantPathFault::TrailingDotOrSpace),
        ("bell\u{7}.md", DeclarantPathFault::Control),
    ] {
        assert_eq!(
            classify_component(component),
            Some(UnsafeComponent::Declarant(expected)),
            "`{component}` must be refused as {expected:?}",
        );
    }
}

/// The complete Win32 forbidden set, character by character. `/`, `\` and `:`
/// answer with their own more specific reasons; the remaining six share one.
/// Every character is asserted individually so a future edit cannot silently
/// drop one from the table.
#[test]
fn every_win32_forbidden_character_is_refused() {
    for (character, expected) in [
        ('<', DeclarantPathFault::InvalidCharacter),
        ('>', DeclarantPathFault::InvalidCharacter),
        ('"', DeclarantPathFault::InvalidCharacter),
        ('|', DeclarantPathFault::InvalidCharacter),
        ('?', DeclarantPathFault::InvalidCharacter),
        ('*', DeclarantPathFault::InvalidCharacter),
        (':', DeclarantPathFault::Colon),
        ('/', DeclarantPathFault::Separator),
        ('\\', DeclarantPathFault::Backslash),
    ] {
        for spelling in [
            format!("{character}guide.md"),
            format!("gui{character}de.md"),
            format!("guide.md{character}"),
        ] {
            assert_eq!(
                classify_component(&spelling),
                Some(UnsafeComponent::Declarant(expected)),
                "`{spelling}` must be refused as {expected:?}",
            );
        }
    }
}

/// Every C0 and C1 control, not a sample of them.
#[test]
fn every_control_character_is_refused() {
    for code in (0x00_u32..=0x1f).chain(0x7f..=0x9f) {
        let character = char::from_u32(code).expect("a valid scalar");
        let spelling = format!("gui{character}de.md");
        assert_eq!(
            classify_component(&spelling),
            Some(UnsafeComponent::Declarant(DeclarantPathFault::Control)),
            "U+{code:04X} must be refused",
        );
    }
}

/// Whole-path identity: component-wise folding, Windows prefix/root semantics,
/// and empty segments dropped so a doubled separator is not a different file.
#[test]
fn whole_path_identity_folds_prefix_root_and_components() {
    use super::path_identity_key as key;
    for (left, right, why) in [
        ("docs/A.md", "Docs/a.md", "component case"),
        ("C:/p/x.md", "c:/p/x.md", "drive letter case"),
        ("C:/P/x.md", "c:/p/X.MD", "drive and components"),
        ("docs//a.md", "docs/a.md", "doubled separator"),
        ("docs\\a.md", "docs/a.md", "backslash separator"),
        (
            "docs/\u{e9}dition.md",
            "docs/e\u{301}dition.md",
            "nfc vs nfd",
        ),
        ("docs/Ma\u{df}e.md", "docs/MASSE.MD", "sharp s"),
        ("docs/\u{3c2}.md", "docs/\u{3a3}.md", "final sigma"),
    ] {
        assert_eq!(key(left), key(right), "`{left}` vs `{right}` ({why})");
    }
    for (left, right) in [
        ("docs/a.md", "docs/b.md"),
        ("/docs/a.md", "docs/a.md"),
        ("C:/p/x.md", "D:/p/x.md"),
    ] {
        assert_ne!(key(left), key(right), "{left} vs {right}");
    }
}

/// The superscript port spellings Windows also honours — proof this delegates
/// to the shared manifest table rather than carrying a weaker ASCII list.
#[test]
fn superscript_device_ports_are_refused_through_the_shared_table() {
    for component in ["COM\u{b9}", "LPT\u{b2}", "com\u{b3}.txt"] {
        assert_eq!(
            classify_component(component),
            Some(UnsafeComponent::Declarant(
                DeclarantPathFault::WindowsDevice,
            )),
            "{component}",
        );
    }
}

#[test]
fn split_relative_judges_every_component_not_only_the_last() {
    let (parents, name) = split_relative("docs/nested/guide.md").unwrap();
    assert_eq!(parents, ["docs", "nested"]);
    assert_eq!(name, "guide.md");
    for relative in [
        "docs/../escape.md",
        "COM1/guide.md",
        "docs/guide.md:ads",
        "docs//guide.md",
        "trailing./guide.md",
    ] {
        assert!(split_relative(relative).is_err(), "{relative}");
    }
}

/// The fold table is filesystem identity, so its Unicode version is part of
/// the contract, not an implementation detail: APFS case-insensitive volumes
/// match on Unicode 9.0.0. A dependency bump that moved this would silently
/// change which two spellings are "one file".
#[test]
fn the_fold_table_stays_pinned_to_the_unicode_version_apfs_uses() {
    assert_eq!(
        unicode_casefold::UNICODE_VERSION,
        (9, 0, 0),
        "filesystem identity is defined on Unicode 9.0.0; re-audit before bumping",
    );
}

/// The identity law, by the cases a *lowercase* or an *uppercase* fold gets
/// wrong. Each row is a pair a case-insensitive filesystem stores as ONE file.
#[test]
fn identity_folds_every_case_a_simpler_key_would_miss() {
    for (left, right, why) in [
        ("Docs", "docs", "ascii"),
        ("GUIDE.MD", "guide.md", "ascii with extension"),
        ("\u{c4}PFEL", "\u{e4}pfel", "precomposed non-ascii"),
        // `ß` folds to `ss` — a length-changing mapping simple lowercasing
        // cannot express, so `Maße` and `MASSE` would be two keys.
        ("Ma\u{df}e", "MASSE", "sharp s"),
        // Capital sharp S. Uppercasing maps `ß` to `SS` but leaves `ẞ` alone,
        // so under it these are two keys — and one file on APFS.
        ("Ma\u{1e9e}e", "Ma\u{df}e", "capital sharp s vs sharp s"),
        ("MA\u{1e9e}E", "MASSE", "capital sharp s vs SS"),
        // Greek sigma: final, medial and capital are one letter.
        ("\u{3c2}", "\u{3c3}", "greek final vs medial sigma"),
        ("\u{3a3}", "\u{3c2}", "greek capital vs final sigma"),
        // Theta and the theta SYMBOL. Uppercasing maps `θ` to `Θ` but leaves
        // `ϴ` (U+03F4) alone, so under it these are two keys.
        ("\u{3b8}", "\u{3f4}", "theta vs theta symbol"),
        ("\u{398}", "\u{3f4}", "capital theta vs theta symbol"),
        // A ligature: the full mapping decomposes it, a simple one does not.
        ("\u{fb01}le", "file", "fi ligature"),
        // NFC vs NFD spellings of one name.
        ("\u{e9}dition.md", "e\u{301}dition.md", "nfc vs nfd"),
        ("\u{c5}ngstrom", "A\u{30a}ngstrom", "nfc vs nfd, cased"),
    ] {
        assert_eq!(
            identity_key(left),
            identity_key(right),
            "`{left}` and `{right}` are one file ({why})",
        );
    }
}

/// The law is idempotent — feeding a key back through it must not move it, or
/// "the same file" would depend on how many times the question was asked.
#[test]
fn the_identity_key_is_a_fixed_point() {
    for value in [
        "Ma\u{df}e",
        "MA\u{1e9e}E",
        "\u{3f4}",
        "\u{fb01}le",
        "e\u{301}dition.md",
    ] {
        let once = identity_key(value);
        assert_eq!(identity_key(&once), once, "{value}");
    }
}

#[test]
fn identity_keeps_genuinely_distinct_names_apart() {
    for (left, right) in [
        ("guide.md", "guide2.md"),
        ("docs", "doc"),
        ("\u{e4}pfel", "apfel"),
        ("\u{3b1}", "\u{3b2}"),
    ] {
        assert_ne!(identity_key(left), identity_key(right), "{left} vs {right}");
    }
}

/// Legal Unicode stays legal: the law folds identities, it does not restrict
/// output names to ASCII.
#[test]
fn ordinary_unicode_names_are_not_refused() {
    for component in [
        "\u{dc}nicode.md",
        "\u{4f7f}\u{7528}\u{6cd5}.md",
        "\u{440}\u{443}\u{43a}\u{43e}\u{432}\u{43e}\u{434}.md",
        "\u{e9}dition.md",
    ] {
        assert_eq!(classify_component(component), None, "{component}");
    }
}

/// The staging reservation is an identity, not a spelling: on a case-folding
/// filesystem `.VIBE-STAGE-…` would alias an in-flight stage.
#[test]
fn the_stage_reservation_is_case_insensitive() {
    for component in [
        ".vibe-stage-1234-0",
        ".VIBE-STAGE-1234-0",
        ".Vibe-Stage-anything",
    ] {
        assert_eq!(
            classify_component(component),
            Some(UnsafeComponent::StagePrefix),
            "{component}",
        );
    }
}

#[test]
fn complete_sets_share_the_identity_and_literal_path_laws() {
    assert!(judge_selection(["SKILL.md", "references/guide.md"]).is_ok());
    assert!(matches!(
        judge_selection(["SKILL.md", "skill.md"]),
        Err(SelectionFault::Collision { .. })
    ));
    assert!(matches!(
        judge_selection(["SKILL.md", "references/NUL.txt"]),
        Err(SelectionFault::Unsafe(_))
    ));
    assert!(matches!(
        judge_selection(["SKILL.md", ".VIBE-STAGE-foreign"]),
        Err(SelectionFault::Unsafe(_))
    ));
    assert!(matches!(
        judge_selection(["Maße.md", "MASSE.md"]),
        Err(SelectionFault::Collision { .. })
    ));
}

#[test]
fn overlap_and_containment_use_the_same_component_boundaries() {
    let target = Path::new(r"C:\project\.claude\skills\demo");
    let alias = Path::new(r"c:\PROJECT\.CLAUDE\skills\demo\nested");
    let sibling = Path::new(r"C:\project\.claude\skills\other");
    assert!(paths_overlap(target, alias));
    assert!(!paths_overlap(target, sibling));

    let scope = tempfile::tempdir().unwrap();
    let root = scope.path().join("project");
    let nested = root.join(".claude/skills/demo");
    assert!(ensure_lexically_contained(&root, &nested).is_ok());
    assert!(ensure_lexically_contained(&root, &scope.path().join("project-sibling/x")).is_err());
}

#[cfg(unix)]
#[test]
fn non_utf8_overlap_keys_are_reversible_and_ascii_folded() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;

    let path = |bytes: Vec<u8>| PathBuf::from(OsString::from_vec(bytes));
    let upper = path(vec![b'x', 0xff, b'A']);
    let lower = path(vec![b'x', 0xff, b'a']);
    let distinct = path(vec![b'x', 0xfe, b'a']);
    assert!(paths_overlap(&upper, &lower));
    assert!(!paths_overlap(&upper, &distinct));
}

#[cfg(windows)]
#[test]
fn unpaired_wide_overlap_keys_are_reversible_and_ascii_folded() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;

    let path = |units: &[u16]| PathBuf::from(OsString::from_wide(units));
    let upper = path(&[0xd800, b'A' as u16]);
    let lower = path(&[0xd800, b'a' as u16]);
    let distinct = path(&[0xd801, b'a' as u16]);
    assert!(paths_overlap(&upper, &lower));
    assert!(!paths_overlap(&upper, &distinct));
}
