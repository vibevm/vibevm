//! The shared declarant-path law: one table, every authored surface.

use std::path::Path;

use specmark::verifies;

use super::{
    DeclarantPathFault, declarant_path, declarant_path_pattern, is_windows_device_name,
    is_windows_unsafe_component,
};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#REF-HANDLER-TABLES")]
fn portable_literal_spellings_pass() {
    for spelling in [
        "Cargo.toml",
        "schemas/cargo-build-v1.jtd.json",
        "dir.with.dot/prepare",
        "hooks/prepare",
        "skills/review-code",
        "a/b/c/d/e",
        // Near-miss device spellings that are ordinary names.
        "context/console/component",
        "com/lpt/com10/com0",
        "confidence.md",
        // Legal Unicode is preserved: the law is spellability, not ASCII.
        "документы/файл.md",
        "スキル/本文.md",
        "café/naïve.txt",
    ] {
        assert_eq!(
            declarant_path(Path::new(spelling)),
            Ok(spelling),
            "{spelling}"
        );
        // A literal path is also a legal pattern — patterns only *add* syntax.
        assert_eq!(
            declarant_path_pattern(Path::new(spelling)),
            Ok(spelling),
            "{spelling}"
        );
    }
}

/// Wildcards are glob *syntax*, legal only in pattern mode and only when
/// well-formed. A literal caller (skill / handler / config_schema / prebuilt)
/// never gets glob capability.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
fn pattern_mode_permits_well_formed_globs_and_nothing_else() {
    for spelling in [
        "crates/*/Cargo.toml",
        "crates/**/src/*.rs",
        "crates/helper/**",
        "**",
        "**/*.md",
        "*.rs",
        "src/*_test.rs",
    ] {
        assert_eq!(
            declarant_path_pattern(Path::new(spelling)),
            Ok(spelling),
            "{spelling}"
        );
        // The same spelling is a literal fault: `*` is not a name character.
        assert_eq!(
            declarant_path(Path::new(spelling)),
            Err(DeclarantPathFault::InvalidCharacter),
            "{spelling}"
        );
    }

    // Malformed wildcard runs refuse in pattern mode.
    for spelling in [
        "crates/a**b/x",
        "crates/**x/y",
        "crates/x**/y",
        "crates/***/y",
        "a***b",
    ] {
        assert_eq!(
            declarant_path_pattern(Path::new(spelling)),
            Err(DeclarantPathFault::MalformedGlob),
            "{spelling}"
        );
    }

    // A pattern is no way past the literal law: every non-wildcard segment
    // still answers to device / trailing / character / escape rules.
    for (spelling, fault) in [
        ("crates/**/nul", DeclarantPathFault::WindowsDevice),
        ("crates/*/CON.txt", DeclarantPathFault::WindowsDevice),
        ("crates/*/trailing.", DeclarantPathFault::TrailingDotOrSpace),
        ("crates/*.", DeclarantPathFault::TrailingDotOrSpace),
        ("crates/**/../x", DeclarantPathFault::DotSegment),
        ("crates/**/a:b", DeclarantPathFault::Colon),
        (r"crates/**/a\b", DeclarantPathFault::Backslash),
        ("crates/?/x", DeclarantPathFault::InvalidCharacter),
        ("crates/<x>/y", DeclarantPathFault::InvalidCharacter),
        ("crates/\"x\"/y", DeclarantPathFault::InvalidCharacter),
        ("crates/x|y/z", DeclarantPathFault::InvalidCharacter),
        ("/crates/**", DeclarantPathFault::Rooted),
        ("crates//**", DeclarantPathFault::EmptySegment),
    ] {
        assert_eq!(
            declarant_path_pattern(Path::new(spelling)),
            Err(fault),
            "{spelling}"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#REF-HANDLER-TABLES")]
fn every_fault_class_is_named_exactly() {
    for (spelling, fault) in [
        ("", DeclarantPathFault::Empty),
        ("a\u{0}b", DeclarantPathFault::Control),
        ("a\nb", DeclarantPathFault::Control),
        ("a\u{7f}b", DeclarantPathFault::Control),
        ("a\u{9b}b", DeclarantPathFault::Control),
        (r"skills\escape", DeclarantPathFault::Backslash),
        ("C:/outside", DeclarantPathFault::Colon),
        ("c:hooks/prepare", DeclarantPathFault::Colon),
        // Alternate data streams: the colon is nowhere near a drive prefix.
        ("skills/file:stream", DeclarantPathFault::Colon),
        ("crates/h/x.rs:zone.identifier", DeclarantPathFault::Colon),
        ("/outside", DeclarantPathFault::Rooted),
        ("//server/share", DeclarantPathFault::Rooted),
        ("skills//escape", DeclarantPathFault::EmptySegment),
        ("skills/escape/", DeclarantPathFault::EmptySegment),
        (".", DeclarantPathFault::DotSegment),
        ("..", DeclarantPathFault::DotSegment),
        ("../outside", DeclarantPathFault::DotSegment),
        ("skills/./escape", DeclarantPathFault::DotSegment),
        ("hooks/../prepare", DeclarantPathFault::DotSegment),
        ("nul", DeclarantPathFault::WindowsDevice),
        ("dist/aux", DeclarantPathFault::WindowsDevice),
        ("skills/COM1", DeclarantPathFault::WindowsDevice),
        ("skills/CON.txt", DeclarantPathFault::WindowsDevice),
        ("skills/LPT²/body", DeclarantPathFault::WindowsDevice),
        ("skills/CLOCK$", DeclarantPathFault::WindowsDevice),
        ("skills/trailing.", DeclarantPathFault::TrailingDotOrSpace),
        ("out/secret.txt. ", DeclarantPathFault::TrailingDotOrSpace),
        ("skills/trailing ", DeclarantPathFault::TrailingDotOrSpace),
        // Windows-invalid characters, refused literally in both modes.
        ("bad*.md", DeclarantPathFault::InvalidCharacter),
        ("bad?.md", DeclarantPathFault::InvalidCharacter),
        ("bad<.md", DeclarantPathFault::InvalidCharacter),
        ("bad>.md", DeclarantPathFault::InvalidCharacter),
        ("bad\".md", DeclarantPathFault::InvalidCharacter),
        ("bad|.md", DeclarantPathFault::InvalidCharacter),
    ] {
        assert_eq!(
            declarant_path(Path::new(spelling)),
            Err(fault),
            "{spelling:?}"
        );
        assert!(!fault.reason().is_empty());
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-018#skill-decl")]
fn device_table_judges_stems_extensions_and_superscripts() {
    for device in [
        "con",
        "CON",
        "PRN",
        "aux",
        "nul",
        "com1",
        "LPT9",
        "CON.txt",
        "NUL.md",
        "COM1.json",
        "LPT9.log",
        "con.in.txt",
        "CONIN$",
        "conout$",
        "CLOCK$",
        "clock$.tmp",
        "COM¹",
        "com²",
        "LPT³.cfg",
    ] {
        assert!(is_windows_device_name(device), "{device}");
        assert!(is_windows_unsafe_component(device), "{device}");
    }
    for ordinary in [
        "context",
        "console",
        "component",
        "com",
        "lpt",
        "com10",
        "com0",
        "lpt0",
        "confidence.md",
        "com¹⁰",
        "KON",
        "ＣＯＮ",
    ] {
        assert!(!is_windows_device_name(ordinary), "{ordinary}");
        assert!(!is_windows_unsafe_component(ordinary), "{ordinary}");
    }
    assert!(is_windows_unsafe_component("trailing "));
    assert!(is_windows_unsafe_component("trailing."));
}

/// `is_windows_unsafe_component` is a delegation point for receipt
/// containment today and `vibe-safefs` at R7, so it must not report a weaker
/// literal truth than the manifest law enforces.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-018#skill-decl")]
fn unsafe_component_reports_the_whole_literal_truth() {
    for unsafe_component in [
        "nul",
        "CON.txt",
        "trailing.",
        "trailing ",
        "a<b",
        "a>b",
        "a\"b",
        "a|b",
        "a?b",
        "a*b",
        "a:b",
        r"a\b",
        "a\u{0}b",
        "a\u{7f}b",
    ] {
        assert!(
            is_windows_unsafe_component(unsafe_component),
            "{unsafe_component:?}"
        );
        // Whatever the component-level answer says is unsafe, the path law
        // refuses as a segment — the two cannot drift apart.
        assert!(
            declarant_path(Path::new(&format!("dir/{unsafe_component}"))).is_err(),
            "{unsafe_component:?}"
        );
    }
    for safe in [
        "helper.exe",
        "review-code",
        "dir.with.dot",
        "файл.md",
        "a b",
    ] {
        assert!(!is_windows_unsafe_component(safe), "{safe:?}");
        assert!(
            declarant_path(Path::new(&format!("dir/{safe}"))).is_ok(),
            "{safe:?}"
        );
    }
}

#[test]
fn non_utf8_and_non_normal_spellings_refuse_without_panicking() {
    // Non-ASCII that is nonetheless valid UTF-8 stays legal — the law is
    // about spellability, not about ASCII.
    assert!(declarant_path(Path::new("документы/файл.md")).is_ok());
    #[cfg(windows)]
    assert_eq!(
        declarant_path(Path::new(r"\\?\C:\x")),
        Err(DeclarantPathFault::Backslash)
    );
}

/// Only the artifact input row is glob-bearing. Every other authored path
/// surface keeps the literal law, so a wildcard cannot be smuggled in.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#REF-HANDLER-TABLES")]
fn only_artifact_inputs_are_glob_capable() {
    use crate::manifest::{Manifest, SkillDecl};

    const PROJECT: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";
    let mechanism = concat!(
        "[[mechanism]]\nid = \"x\"\nrole = \"build\"\nname = \"cargo\"\n",
        "handler = { kind = \"native\", crate_dir = \"crates/x\" }\n",
        "protocol = 1\nconfig_schema = \"schemas/*.json\"\nfreshness = \"engine\"\n",
    );
    for literal_surface in [
        mechanism,
        "[[extension]]\nid = \"e\"\npoint = \"phase:build\"\nhandler = { kind = \"script\", base = \"hooks/*\" }\n",
        "[[extension]]\nid = \"e\"\npoint = \"phase:build\"\nhandler = { kind = \"native\", crate_dir = \"crates/*\" }\n",
    ] {
        let error = Manifest::parse_str(&format!("{PROJECT}\n{literal_surface}"))
            .expect_err("a wildcard is not a literal name")
            .to_string();
        assert!(error.contains("literal `*`"), "{error}");
    }

    let skill = SkillDecl {
        name: "s".into(),
        path: "skills/*".into(),
        description: None,
        agents: Vec::new(),
        include: Vec::new(),
    };
    assert!(skill.validate().is_err(), "a skill path is never a glob");

    // The artifact input row is the one place a wildcard is syntax.
    let inputs = concat!(
        "[[artifacts.build]]\nid = \"helper\"\nmechanism = \"build:cargo\"\n",
        "inputs = [{ path = \"crates/*/Cargo.toml\" }, { path = \"crates/**/src/*.rs\" }]\n",
        "outputs = [{ id = \"helper.exe\", kind = \"executable\" }]\n",
    );
    assert!(Manifest::parse_str(&format!("{PROJECT}\n{inputs}")).is_ok());
}
