//! Source guards: no second wire shape exists. A grep over the conversion's
//! own source pins the forbidden carriers.

use std::path::PathBuf;

fn wire_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/compiler/wire")
}

/// Product sources only: the conversion itself, never its test modules (a
/// guard test quoting the forbidden token is not a violation).
fn sources() -> Vec<PathBuf> {
    let mut files = vec![wire_root().with_file_name("wire.rs")];
    let mut stack = vec![wire_root()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && !path.to_string_lossy().contains("tests")
            {
                files.push(path);
            }
        }
    }
    files
}

#[test]
fn no_value_carrier_no_unsafe_no_domain_serde_in_the_conversion() {
    for path in sources() {
        let text = std::fs::read_to_string(&path).unwrap();
        let shown = path.display();
        assert!(
            !text.contains("serde_json::Value"),
            "{shown}: no Value carrier"
        );
        assert!(!text.contains("unsafe"), "{shown}: no unsafe");
        assert!(!text.contains("transmute"), "{shown}: no transmute");
        assert!(
            !text.contains("derive(") || !text.contains("Serialize"),
            "{shown}: no serde derive on a conversion type"
        );
    }
}

/// The domain IR itself stays serde-free: the generated wire types are the
/// only projection surface.
#[test]
fn the_domain_ir_carries_no_serde_derives() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![manifest];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && !path.to_string_lossy().contains("tests")
            {
                let text = std::fs::read_to_string(&path).unwrap();
                assert!(
                    !text.contains("#[derive(") || !text.contains("Serialize"),
                    "{}: the domain IR must not grow serde derives",
                    path.display()
                );
            }
        }
    }
}

// ── the spelling preflights allocate only on the failure path ───────────────

fn emitted_source() -> String {
    std::fs::read_to_string(wire_root().with_file_name("wire").join("emitted.rs")).unwrap()
}

/// The body of `name`, up to the helper that follows it.
fn function_body<'a>(text: &'a str, name: &str, follower: &str) -> &'a str {
    let start = text
        .find(name)
        .unwrap_or_else(|| panic!("`{name}` is in emitted.rs"));
    let end = text[start..]
        .find(follower)
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("`{follower}` follows `{name}`"));
    &text[start..end]
}

/// Prose cannot satisfy — or break — this guard: it judges CODE, so every
/// line comment goes before anything is matched.
fn code_only(body: &str) -> String {
    body.lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Everything OUTSIDE the function's `return Err(…)` statements: the path a
/// VALID spelling actually takes. Parenthesis-balanced, so a multi-line
/// refusal is removed whole.
fn success_path(code: &str) -> String {
    const OPENER: &str = "return Err(";
    let mut out = String::new();
    let mut rest = code;
    while let Some(at) = rest.find(OPENER) {
        out.push_str(&rest[..at]);
        rest = &rest[at + OPENER.len()..];
        let mut depth = 1usize;
        let mut end = rest.len();
        for (index, ch) in rest.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = index + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        assert_eq!(depth, 0, "a refusal statement must close its parentheses");
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// The strict reader MOVES a unique key into its set. A `clone()` on that
/// path doubles an attacker-sized key's footprint before anything has decided
/// the document is even well formed (repair 3).
#[test]
fn the_strict_reader_never_clones_a_unique_key() {
    let text = std::fs::read_to_string(wire_root().join("json.rs")).unwrap();
    let body = code_only(function_body(&text, "fn visit_map", "\n}\n"));
    assert!(
        body.contains("seen.insert(key)"),
        "the key is moved: {body}"
    );
    assert!(
        !body.contains(".clone()"),
        "no clone rides the unique-key path: {body}"
    );
}

/// A verifier refusal must be rendered through the BOUNDED `Debug` sink and
/// the unbounded typed error must not be retained. `VerificationError`'s
/// cycle variant joins its whole path inside `Display`, so calling
/// `to_string()` here would allocate the full text before any sink could
/// truncate it (repair 4).
#[test]
fn a_verifier_refusal_is_rendered_bounded_and_never_retained() {
    let text = std::fs::read_to_string(wire_root().with_file_name("wire.rs")).unwrap();
    let body = code_only(function_body(
        &text,
        "fn verification(source: &VerificationError)",
        "
}
",
    ));
    assert!(
        body.contains("bounded::debug(source)"),
        "the refusal renders through the bounded Debug sink: {body}"
    );
    for token in ["to_string()", "{source}", "format!"] {
        assert!(
            !body.contains(token),
            "`{token}` must not ride the verifier refusal: {body}"
        );
    }
    // The variant carries a rendered String, never the typed error itself.
    assert!(
        text.contains("Verification(String)"),
        "IrWireError::Verification must not retain the unbounded source"
    );
}

/// Gate 5's spelling passes judge a canonical value with zero proportional
/// heap: no `Vec`, `collect`, `String` or `format!` survives on the success
/// path, and the bounded preview is built only inside a refusal (repair 2).
#[test]
fn the_spelling_preflights_allocate_only_on_failure() {
    let text = emitted_source();
    for (name, follower, witness) in [
        (
            "pub(super) fn check_canonical_base64",
            "\npub(super) fn encode_base64",
            "sextet(",
        ),
        (
            "pub(super) fn parse_digest",
            "\nfn hex_nibble",
            "is_ascii_digit()",
        ),
    ] {
        let code = code_only(function_body(&text, name, follower));
        assert!(
            code.contains(witness),
            "{name}: the guard must have sliced the real body"
        );
        let success = success_path(&code);
        assert!(
            success.contains(witness),
            "{name}: the success path is what remains outside the refusals"
        );
        for token in ["Vec", ".collect(", "String", "format!", "bounded_preview"] {
            assert!(
                !success.contains(token),
                "{name}: `{token}` must not ride the success path"
            );
        }
    }
}
