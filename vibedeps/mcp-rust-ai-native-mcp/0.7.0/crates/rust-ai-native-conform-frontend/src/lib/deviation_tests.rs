//! v11 deviation-reason extraction tests — out-of-line in their own file
//! because [`super`]'s `lib/tests.rs` sits at the file-length budget. Same
//! `#[cfg(test)] #[path] mod` inclusion shape, so `use super::*` reaches the
//! crate-root bindings unchanged. [`extract`] is the test harness, mirrored
//! here so this module is self-contained.

use super::*;

#[cfg(test)]
fn extract(src: &str) -> Vec<Fact> {
    RustFrontend.extract("crates/x/src/m.rs", "x", "x::m", src)
}

/// `reason = "…"` rides on all three deviation facts. They read the one
/// [`deviation::DeviationStack`], so the extraction path is COMMON — this
/// single test covers `UnsafeUse`, `UnwrapUse`, and `EnvRead`, each drawn from
/// the same deviating fn (the explicit "all three go through it" check).
#[test]
fn deviation_reason_threads_into_unsafe_unwrap_and_envread() {
    let facts = extract(
        r#"
        #[spec(deviates = "spec://p/d#a", reason = "FFI boundary")]
        pub fn testified() {
            unsafe { std::hint::black_box(()) }
            Some(1).unwrap();
            let _ = std::env::var("HOME");
        }
        "#,
    );
    let unsafe_reason = facts.iter().find_map(|f| match f {
        Fact::UnsafeUse {
            in_deviation: true,
            reason,
            ..
        } => reason.clone(),
        _ => None,
    });
    let unwrap_reason = facts.iter().find_map(|f| match f {
        Fact::UnwrapUse {
            in_deviation: true,
            reason,
            ..
        } => reason.clone(),
        _ => None,
    });
    let env_reason = facts.iter().find_map(|f| match f {
        Fact::EnvRead {
            in_deviation: true,
            reason,
            ..
        } => reason.clone(),
        _ => None,
    });
    assert_eq!(unsafe_reason, Some("FFI boundary".into()), "{facts:?}");
    assert_eq!(unwrap_reason, Some("FFI boundary".into()), "{facts:?}");
    assert_eq!(env_reason, Some("FFI boundary".into()), "{facts:?}");
}

/// The degenerate case AND regression gate: a fn that deviates on record
/// WITHOUT a `reason = "…"` key still grants amnesty (`in_deviation = true`)
/// and carries `reason: None`. v4's fn-grain scoping is unchanged by v11.
#[test]
fn deviates_without_reason_key_carries_none_but_still_in_deviation() {
    let facts = extract(
        r#"
        #[spec(deviates = "spec://p/d#a")]
        pub fn testified() {
            Some(1).unwrap();
        }
        "#,
    );
    let unwrap = facts
        .iter()
        .find_map(|f| match f {
            Fact::UnwrapUse {
                in_deviation,
                reason,
                ..
            } => Some((*in_deviation, reason.clone())),
            _ => None,
        })
        .unwrap();
    assert_eq!(unwrap, (true, None), "{facts:?}");
}

/// Measured behavior (refinement #2): the reason value is parsed as a
/// `syn::LitStr`, so it unescapes EXACTLY as rustc parses a string literal —
/// `\"` → `"`, `\t` → TAB, `\\` → `\` — NOT the raw source text with the
/// backslashes left intact. Pinned as measured, not "fixed".
#[test]
fn deviation_reason_unescapes_as_a_rust_string_literal() {
    let facts = extract(
        r#"
        #[spec(deviates = "spec://p/d#a", reason = "quote:\" tab:\t back:\\")]
        pub fn testified() {
            Some(1).unwrap();
        }
        "#,
    );
    let reason = facts
        .iter()
        .find_map(|f| match f {
            Fact::UnwrapUse { reason, .. } => reason.clone(),
            _ => None,
        })
        .expect("a deviating UnwrapUse carries the reason");
    assert_eq!(reason, "quote:\" tab:\t back:\\", "{facts:?}");
}
