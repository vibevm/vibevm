//! The scanner: the whole variable grammar, its escapes and its refusals.

use super::{Piece, Var, scan};

fn vars(text: &str) -> Vec<Var> {
    scan(text, false)
        .expect("scans")
        .into_iter()
        .filter_map(|piece| match piece {
            Piece::Var(var) => Some(var),
            Piece::Literal(_) => None,
        })
        .collect()
}

fn literals(text: &str) -> Vec<String> {
    scan(text, false)
        .expect("scans")
        .into_iter()
        .filter_map(|piece| match piece {
            Piece::Literal(literal) => Some(literal),
            Piece::Var(_) => None,
        })
        .collect()
}

#[test]
fn every_variable_of_the_closed_set_scans() {
    assert_eq!(
        vars("{version}{version_raw}{major}{minor}{patch}{prerelease}{platform}{arch}"),
        vec![
            Var::Version,
            Var::VersionRaw,
            Var::Major,
            Var::Minor,
            Var::Patch,
            Var::Prerelease,
            Var::Platform,
            Var::Arch,
        ]
    );
    assert_eq!(vars("{meta.lts}"), vec![Var::Meta("lts".into())]);
    assert_eq!(
        vars("{meta.a.b.c}"),
        vec![Var::Meta("a.b.c".into())],
        "a dotted path is one variable"
    );
}

#[test]
fn doubled_braces_are_the_literal_braces() {
    assert_eq!(literals("{{version}}"), vec!["{version}".to_string()]);
    assert!(vars("{{version}}").is_empty(), "nothing was substituted");
    assert_eq!(
        scan("a{{b{version}c}}d", false).expect("scans"),
        vec![
            Piece::Literal("a{b".into()),
            Piece::Var(Var::Version),
            Piece::Literal("c}d".into()),
        ]
    );
}

#[test]
fn one_level_of_nesting_is_allowed_only_under_a_digest_variable() {
    assert_eq!(
        vars("{sha256:node-v{version}-linux-x64.tar.gz}"),
        vec![Var::Sha256(vec![
            Piece::Literal("node-v".into()),
            Piece::Var(Var::Version),
            Piece::Literal("-linux-x64.tar.gz".into()),
        ])]
    );
    assert_eq!(
        vars("{size:t-{version}-{platform}-{arch}.zip}"),
        vec![Var::Size(vec![
            Piece::Literal("t-".into()),
            Piece::Var(Var::Version),
            Piece::Literal("-".into()),
            Piece::Var(Var::Platform),
            Piece::Literal("-".into()),
            Piece::Var(Var::Arch),
            Piece::Literal(".zip".into()),
        ])]
    );
    let refusal = scan("{sha256:a{sha256:b}c}", false).expect_err("refuses a second level");
    assert!(refusal.contains("one level of nesting"), "{refusal}");
    // And nesting an ordinary variable inside an ordinary variable is not
    // a nesting at all — it is an unknown variable name.
    let refusal = scan("{meta.{version}}", false).expect_err("refuses");
    assert!(refusal.contains("not a known variable"), "{refusal}");
    // A digest argument whose inner variable ends where the outer one does
    // closes both braces in the right order.
    assert_eq!(
        vars("{sha256:t-{version}}"),
        vec![Var::Sha256(vec![
            Piece::Literal("t-".into()),
            Piece::Var(Var::Version),
        ])]
    );
}

#[test]
fn a_brace_that_opens_nothing_refuses_rather_than_passing_through() {
    for (text, marker) in [
        ("{nope}", "not a known variable"),
        ("{}", "not a known variable"),
        ("{version", "is never closed"),
        ("a } b", "closes no variable"),
        ("{sha256:}", "names no file"),
    ] {
        let refusal = scan(text, false).expect_err(text);
        assert!(refusal.contains(marker), "{text}: {refusal}");
    }
}

#[test]
fn a_string_without_a_brace_scans_to_one_literal() {
    assert_eq!(
        scan("plain text", false).expect("scans"),
        vec![Piece::Literal("plain text".into())]
    );
    assert_eq!(
        scan("", false).expect("scans"),
        Vec::new(),
        "an empty string has no pieces"
    );
}

#[test]
fn a_multibyte_literal_survives_the_scan() {
    assert_eq!(
        scan("Node.js — вер. {version}", false).expect("scans"),
        vec![
            Piece::Literal("Node.js — вер. ".into()),
            Piece::Var(Var::Version),
        ]
    );
}
