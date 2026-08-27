//! Unit oracles for exact relative spelling and lossless escaped rendering.

use std::ffi::OsString;
use std::path::PathBuf;

use super::*;

/// One `OsString` that is *not* valid UTF-8 on this host: raw invalid bytes
/// on Unix, an unpaired surrogate on Windows. `b?.md` in both cases, where
/// `?` is the unrepresentable unit.
#[cfg(test)]
fn non_utf8_name() -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(vec![b'b', 0xff, 0xfe, b'.', b'm', b'd'])
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        OsString::from_wide(&[0x0062, 0xd800, 0x002e, 0x006d, 0x0064])
    }
}

/// The exact escaped rendering of [`non_utf8_name`] on this host.
#[cfg(test)]
fn non_utf8_escaped() -> &'static str {
    #[cfg(unix)]
    {
        "b\\xFF\\xFE.md"
    }
    #[cfg(windows)]
    {
        "b\\uD800.md"
    }
}

#[test]
fn exact_components_round_trip_every_legal_utf8_spelling() {
    let base = PathBuf::from("base");
    for (relative, expected) in [
        ("SKILL.md", "SKILL.md"),
        ("references/guide.md", "references/guide.md"),
        ("references/Maße.md", "references/Maße.md"),
        ("スキル/説明.md", "スキル/説明.md"),
        // NFD stays exactly NFD — only the collision key normalizes.
        ("cafe\u{301}.md", "cafe\u{301}.md"),
    ] {
        let path = base.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        assert_eq!(exact_utf8_relative(&base, &path).unwrap(), expected);
    }
}

/// Legal text renders as itself; only the escape introducer and control
/// characters are escaped, and nothing is dropped.
#[test]
fn legal_names_escape_to_themselves() {
    for (name, expected) in [
        ("SKILL.md", "SKILL.md"),
        ("references/Maße.md", "references/Maße.md"),
        ("スキル.md", "スキル.md"),
        ("cafe\u{301}.md", "cafe\u{301}.md"),
        ("a\u{1}b.md", "a\\u0001b.md"),
        ("a\u{7f}b.md", "a\\u007Fb.md"),
        ("a\u{9f}b.md", "a\\u009Fb.md"),
        ("a\\b.md", "a\\\\b.md"),
    ] {
        let escaped = EscapedOsPath::new(OsStr::new(name));
        assert_eq!(escaped.as_str(), expected, "{name:?}");
        assert!(!escaped.as_str().contains('\u{fffd}'), "{name:?}");
    }
}

/// The unrepresentable units are named exactly, in place, and the rendering
/// never contains the replacement character it replaces.
#[test]
fn unrepresentable_units_escape_losslessly_without_a_replacement_char() {
    let name = non_utf8_name();
    assert!(
        name.to_str().is_none(),
        "the fixture must really be non-UTF-8"
    );
    let escaped = EscapedOsPath::new(&name);
    assert_eq!(escaped.as_str(), non_utf8_escaped());
    assert!(!escaped.as_str().contains('\u{fffd}'));
    // The lossy rendering this replaces really does destroy the identity.
    assert!(name.to_string_lossy().contains('\u{fffd}'));
    // Two different broken names must not render the same, which is exactly
    // what the lossy form does.
    let other = {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(vec![b'b', 0xfe, 0xff, b'.', b'm', b'd'])
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            OsString::from_wide(&[0x0062, 0xdc00, 0x002e, 0x006d, 0x0064])
        }
    };
    assert_ne!(EscapedOsPath::new(&other), escaped);
}

/// A non-UTF-8 entry name refuses, and the refusal — the whole rendered
/// `PackageSkillError`, not just its reason — carries the escaped identity
/// and no replacement character.
#[test]
fn non_utf8_entry_names_refuse_with_an_exact_diagnostic() {
    let name = non_utf8_name();
    let base = PathBuf::from("base");
    let path = base.join(&name);

    for error in [
        exact_utf8_component(&name, &path).unwrap_err(),
        exact_utf8_relative(&base, &path).unwrap_err(),
        exact_utf8_relative(&base, &base.join("references").join(&name)).unwrap_err(),
    ] {
        assert!(matches!(error, PackageSkillError::UnportablePath { .. }));
        let rendered = error.to_string();
        assert!(rendered.contains("is not valid UTF-8"), "{rendered}");
        assert!(
            rendered.contains(non_utf8_escaped()),
            "the exact escaped identity must appear: {rendered}"
        );
        assert!(
            !rendered.contains('\u{fffd}'),
            "no replacement character may reach the diagnostic: {rendered}"
        );
    }
}

#[test]
fn empty_and_non_normal_relatives_refuse() {
    let base = PathBuf::from("base");
    let error = exact_utf8_relative(&base, &base).unwrap_err();
    assert!(
        error.to_string().contains("relative path is empty"),
        "{error}"
    );
    let escaping = PathBuf::from("..").join("outside.md");
    let error = exact_utf8_relative(&base, &escaping).unwrap_err();
    assert!(
        error.to_string().contains("non-normal component"),
        "{error}"
    );
}
