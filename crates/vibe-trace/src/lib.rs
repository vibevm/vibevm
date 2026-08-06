//! `vibe-trace` — the host's traceability-explain capability (PROP-014 §2.6).
//!
//! The language stacks (`rust-ai-native trace explain`) and their MCP
//! `trace_explain` already answer the owner's canonical question — "which
//! test verifies this spec rule?" — over THEIR trees. This crate gives the
//! `vibe` host the same ability over ITS OWN tree, in ONE place, so the CLI
//! (`vibe explain`) and the MCP (`explain`) are two thin surfaces over a
//! single function: the config-load → build-fresh → render pipeline is not
//! duplicated between them.
//!
//! Explain answers for the tree AS IT IS: the index is built fresh in
//! memory on every call, never read from a stale committed `specmap.json`
//! — the same posture the stacks' `trace explain` takes. The text and JSON
//! forms match the stack's renderers exactly (they are the stack's
//! renderers); LLM prose is a separate presentation layer and is out of
//! scope here.

#![forbid(unsafe_code)]
specmark::scope!("spec://core-ai-native/mechanisms/PROP-014#queries");

use std::path::Path;

use anyhow::Result;
use serde_json::Value;

mod foreign;
mod fragment;
pub use fragment::{Fragment, fragment};

/// The simple-level map search — independent filters, AND-joined, over a hard
/// result ceiling (A5A-MAPSEARCH). A permanent grep-like floor over the
/// code↔spec map, not a degenerate case of a future query language.
pub mod search;

/// The query-language layer (E-A5B-QUERYLANG): a conjunctive predicate grammar
/// with undirected graph traversal, sitting ON TOP of [`search`] — it reuses
/// the floor's `uri`/`symbol`/`kind` predicate for seed selection and adds
/// `scope`, `has`/`lacks` (verb touch), and `depth` (BFS over the bipartite
/// code↔spec graph). The floor stays permanent and untouched; a broken parser
/// cannot reach it.
pub mod select;

/// One rendered explanation of a traceability target: the deterministic
/// text view, or the raw one-hop JSON subgraph. [`explain`] returns one of
/// these; a caller matches the form to decide how to render or pass it on.
///
/// ```
/// use vibe_trace::Explain;
///
/// // The two renderings `explain` can return — match the form to act on it.
/// let text = Explain::Text("spec unit …\n".to_string());
/// let json = Explain::Json(serde_json::json!({"target": "spec://x/Y#z"}));
/// match &text {
///     Explain::Text(s) => assert!(s.starts_with("spec unit")),
///     Explain::Json(_) => unreachable!("text form"),
/// }
/// match &json {
///     Explain::Json(v) => assert_eq!(v["target"], "spec://x/Y#z"),
///     Explain::Text(_) => unreachable!("json form"),
/// }
/// ```
#[derive(Debug)]
pub enum Explain {
    /// The deterministic text view — `specmap_core::explain::explain_text`.
    Text(String),
    /// The raw one-hop subgraph — `specmap_core::explain::explain_json`.
    Json(Value),
}

/// Answer a traceability question for `root` and render the subgraph around
/// `target` — a `spec://…#anchor` URI or a code symbol.
///
/// Two backends, picked by the address:
///
/// - **The project's own address** (or a code symbol, or anything no installed
///   package owns) builds the traceability index **FRESH** in memory and
///   renders it — the same posture as before, never from a stale committed
///   artefact (PROP-014 §2.6).
/// - **An installed package's address** — a `spec://` URI whose coordinate
///   `<group>/<name>` names a package materialised under `root/vibedeps/` that
///   carries a `package.specmap.json` — is answered from that carried map
///   (V6-FOREIGN-EXPLAIN). The committed project map is byte-stable and
///   deliberately excludes foreign sections, so the foreign answer comes from
///   a second, non-committed map built in memory at query time. The body is
///   the engine's own rendering; one provenance line marks that the data came
///   from a carried map, not a fresh build.
///
/// `json` selects the form: `true` → the raw subgraph ([`Explain::Json`]),
/// `false` → the deterministic text view ([`Explain::Text`]). Both surfaces
/// (CLI, MCP) call this one function, so the pipeline lives in one place.
///
/// Errors mirror the engine verbatim: a `target` that does not resolve — no
/// such spec unit, no matching code item, or an ambiguous suffix — is an
/// `Err` carrying the engine's own message. One new, distinct message is added
/// for the foreign half: an address owned by an installed package that carries
/// no map reports that the package "does not participate", so it is not
/// mistaken for a typo in the address (which surfaces the engine's generic
/// not-found). The caller decides how to surface an `Err` (the CLI prints and
/// exits non-zero; the MCP maps it to its not-found class).
///
/// The canonical use: point it at a tree root and a spec address, get the
/// code-side edges back. The example builds a one-unit tree so it does not
/// depend on any particular repository's content.
///
/// ```
/// use std::fs;
///
/// let root = tempfile::tempdir().unwrap();
/// let r = root.path();
/// fs::write(
///     r.join("specmap.toml"),
///     "namespace = \"demo\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"spec\"]\n",
/// )
/// .unwrap();
/// fs::create_dir_all(r.join("spec")).unwrap();
/// fs::write(
///     r.join("spec/D.md"),
///     "## The rule {#req-r}\n`req r1`\n\nIt MUST hold.\n",
/// )
/// .unwrap();
/// let src = r.join("crates/x/src");
/// fs::create_dir_all(&src).unwrap();
/// fs::write(
///     src.join("lib.rs"),
///     "#[verifies(\"spec://demo/D#req-r\")]\nfn t() {}\n",
/// )
/// .unwrap();
///
/// match vibe_trace::explain(r, "spec://demo/D#req-r", false).unwrap() {
///     vibe_trace::Explain::Text(text) => {
///         assert!(text.contains("verifies ← `x::t`"), "{text}")
///     }
///     vibe_trace::Explain::Json(_) => panic!("default is the text view"),
/// }
/// ```
pub fn explain(root: &Path, target: &str, json: bool) -> Result<Explain> {
    // A foreign `spec://` address — one owned by an installed package — is
    // answered from the carried map that package ships (V6-FOREIGN-EXPLAIN).
    // `try_foreign` returns `Ok(None)` for everything else: a code symbol, the
    // project's own address, or an address nothing owns. Those build the
    // project's map fresh below.
    if let Some(foreign) = foreign::try_foreign(root, target, json)? {
        return Ok(foreign);
    }
    explain_fresh(root, target, json)
}

/// The project's own backend: build the index FRESH in memory for `root` and
/// render the subgraph around `target` — never from a stale committed
/// artefact (PROP-014 §2.6 — the stacks' `trace explain` takes the same
/// posture).
fn explain_fresh(root: &Path, target: &str, json: bool) -> Result<Explain> {
    let cfg = specmap_core::config::Config::load(root)?.unwrap_or_default();
    let map = specmap_core::index::build(root, &cfg);
    let rendered = if json {
        Explain::Json(specmap_core::explain::explain_json(&map, target)?)
    } else {
        Explain::Text(specmap_core::explain::explain_text(&map, target)?)
    };
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A spec unit with both an implementing edge and a verifying edge
    /// into it — the canonical "what realises this rule?" shape. Mirrors
    /// the specmap engine's own `synthetic_tree` fixture format so the
    /// scanner actually parses it.
    const URI: &str = "spec://demo/D#req-r";

    fn tree() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(
            root.join("specmap.toml"),
            "namespace = \"demo\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"spec\"]\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("spec")).unwrap();
        std::fs::write(
            root.join("spec/D.md"),
            "## The rule {#req-r}\n`req r1`\n\nIt MUST hold.\n",
        )
        .unwrap();
        let src = root.join("crates/x/src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            concat!(
                "#[spec(implements = \"spec://demo/D#req-r\", r = 1)]\n",
                "pub fn f() {}\n\n",
                "#[verifies(\"spec://demo/D#req-r\")]\n",
                "fn t() {}\n",
            ),
        )
        .unwrap();
        tmp
    }

    #[test]
    fn text_view_lists_the_edges_into_a_unit() {
        let tmp = tree();
        let Explain::Text(text) = explain(tmp.path(), URI, false).unwrap() else {
            panic!("expected the text form");
        };
        assert!(text.contains("spec unit spec://demo/D#req-r"), "{text}");
        // The owner's canonical question, answered: the implementing and
        // verifying code-side locators travel as `file:line` edges.
        assert!(text.contains("implements ← `x::f`"), "{text}");
        assert!(text.contains("verifies ← `x::t`"), "{text}");
    }

    #[test]
    fn json_view_carries_both_edges_and_the_target() {
        let tmp = tree();
        let Explain::Json(v) = explain(tmp.path(), URI, true).unwrap() else {
            panic!("expected the json form");
        };
        let verbs: Vec<&str> = v["edges"]
            .as_array()
            .expect("edges array")
            .iter()
            .map(|e| e["verb"].as_str().unwrap())
            .collect();
        assert!(verbs.contains(&"implements"), "{verbs:?}");
        assert!(verbs.contains(&"verifies"), "{verbs:?}");
        assert_eq!(v["target"], URI);
    }

    #[test]
    fn a_code_symbol_target_renders_its_edges_out() {
        let tmp = tree();
        let Explain::Text(text) = explain(tmp.path(), "x::f", false).unwrap() else {
            panic!("expected the text form");
        };
        assert!(text.contains("code item `x::f`"), "{text}");
        assert!(text.contains("--implements-->"), "{text}");
    }

    /// §4: a target that does not resolve is an error with the engine's
    /// own message — the same behavior the stacks' `trace explain` has for
    /// a missing anchor or an unknown symbol.
    #[test]
    fn an_unresolvable_target_is_an_error() {
        let tmp = tree();
        let err = explain(tmp.path(), "spec://demo/D#no-such-anchor", false)
            .expect_err("an unknown anchor must error");
        assert!(format!("{err}").contains("no spec unit"), "{err}");
        explain(tmp.path(), "no::such::symbol", false).expect_err("an unknown symbol must error");
    }

    /// A tree with no `specmap.toml` still answers via the default policy
    /// (the placeholder namespace): `Config::load` returns `None`, the
    /// default applies, and an unknown target is the expected engine error
    /// rather than a config-time panic.
    #[test]
    fn an_absent_config_falls_back_to_the_default_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let err = explain(tmp.path(), "spec://project/X#y", false)
            .expect_err("no units ⇒ unknown target");
        assert!(format!("{err}").contains("no spec unit"), "{err}");
    }
}
