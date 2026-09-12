//! The schema-side refusals — what the pass declines to build, and what
//! it says when it declines.
//!
//! Sliced out of `tests.rs` along that seam when the parent crossed the
//! 600-line budget. A4.1's rule is that the policy is never derivable
//! from the Rust: an optional scalar without `x-default` is a generation
//! error, and a literal the pass cannot represent is refused by name
//! rather than silently approximated.
//!
//! Helpers and sample emissions come from the parent module; this file
//! declares none of its own.

use super::*;

/// A4.1's red: an optional scalar without `x-default` is a generation
/// error, not a default — the policy is not derivable from the Rust.
#[test]
fn a_scalar_without_an_annotation_refuses() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "superseded_by": { "type": "string" }
        }
    });
    let err = shapes(doc).expect_err("an optional scalar must carry its policy");
    let msg = err.to_string();
    assert!(msg.contains("schema.jtd.json"), "names the schema: {msg}");
    assert!(
        msg.contains("optionalProperties.superseded_by"),
        "names the site's path: {msg}"
    );
    assert!(msg.contains("x-default"), "names the missing key: {msg}");
    assert!(
        msg.contains("cargo xtask codegen"),
        "gives the recipe: {msg}"
    );
    Ok(())
}

/// A4.2's red: a non-boolean literal — `"stable"` — refuses loudly,
/// naming the site, the value, and exactly what building it would take:
/// a NAMED default function and a NAMED skip predicate.
#[test]
fn a_non_boolean_literal_refuses_naming_what_it_would_take() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "latest_stable": {
                "ref": "version",
                "metadata": { "x-default": "stable" }
            }
        },
        "definitions": {
            "version": { "type": "string" }
        }
    });
    let err = shapes(doc).expect_err("the pass builds no string literals");
    let msg = err.to_string();
    assert!(
        msg.contains("optionalProperties.latest_stable"),
        "names the site: {msg}"
    );
    assert!(msg.contains("\"stable\""), "names the value: {msg}");
    assert!(
        msg.contains("#[serde(default = \"…\")]"),
        "names what it would take to build: {msg}"
    );
    assert!(
        msg.contains("cargo xtask codegen"),
        "gives the recipe: {msg}"
    );
    Ok(())
}

/// The boolean twin of the same limit: `true` needs a named default
/// function (`serde(default)` spells `false`), so it refuses exactly
/// like `"stable"`.
#[test]
fn a_true_literal_refuses_like_any_other_named_default() -> Result<()> {
    let doc = json!({
        "optionalProperties": {
            "yanked": {
                "type": "boolean",
                "metadata": { "x-default": true }
            }
        }
    });
    let err = shapes(doc).expect_err("serde(default) cannot spell true");
    let msg = err.to_string();
    assert!(msg.contains("true"), "names the value: {msg}");
    assert!(
        msg.contains("#[serde(default = \"…\")]"),
        "names what it would take to build: {msg}"
    );
    Ok(())
}
