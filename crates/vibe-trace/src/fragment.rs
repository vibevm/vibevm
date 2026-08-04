//! The code-fragment view: the **source text** of one code element, read by its
//! position in the map — and a **drift verdict**, because the text is re-read
//! fresh from the file and re-fingerprinted with the *same* calculator that
//! built the map, so a body the map does not know about is surfaced before a
//! person reads it (V7-FRAGMENT-DRIFT).
//!
//! The fingerprint is recomputed by re-scanning the element's current file
//! with [`specmap_core::rscan::scan_source`] — the public entry that internally
//! calls the engine's `fingerprint_of`. There is no second hash here: the same
//! token-walk that minted the map's fingerprint mints the comparison one, so
//! the two can only differ when the source genuinely changed. (A second
//! implementation of the hash would be a guaranteed future false-drift —
//! V7-FRAGMENT-DRIFT §3.)
//!
//! Own tree vs installed package mirror [`explain`](crate::explain): the own
//! tree builds its map fresh (so the recorded fingerprint is the current one —
//! a Same verdict here proves the re-scan seam reproduces the build's hash
//! exactly, the invariant a second calculator would break); a carried map is a
//! genuine checkpoint built at publish time, so a body edited in the installed
//! slot since then surfaces as drift.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#addressing-code");

use std::fs;
use std::path::Path;

use anyhow::{Result, bail};
use serde_json::{Value, json};
use specmap_core::generated::specmap::{CodeItem, Specmap};
use specmap_core::rscan::scan_source;

use crate::foreign;

/// One rendered code fragment of a traceability target: the deterministic text
/// view, or the raw structured JSON. [`fragment`] returns one of these; a
/// caller matches the form to decide how to render or pass it on — the same
/// shape [`explain`](crate::explain) takes, so the two surfaces stay parallel.
///
/// ```
/// use vibe_trace::Fragment;
///
/// // The two renderings `fragment` can return — match the form to act on it.
/// let text = Fragment::Text("pub fn f() {}".to_string());
/// let json = Fragment::Json(serde_json::json!({"text": "pub fn f() {}"}));
/// match text {
///     Fragment::Text(s) => assert!(s.contains("pub fn f")),
///     Fragment::Json(_) => unreachable!("text form"),
/// }
/// match json {
///     Fragment::Json(v) => assert_eq!(v["text"], "pub fn f() {}"),
///     Fragment::Text(_) => unreachable!("json form"),
/// }
/// ```
#[derive(Debug)]
pub enum Fragment {
    /// The human-readable view — the element's source text plus a drift
    /// verdict line.
    Text(String),
    /// The structured view — `text`, span, drift verdict, and provenance, for
    /// an agent or script to consume.
    Json(Value),
}

/// Where the answer came from — the fresh build of this tree, or a package's
/// carried map. Carried as a small enum (not a string) so the provenance cue
/// is typed at the render boundary.
enum Source<'a> {
    Fresh,
    Carried { coordinate: &'a str },
}

/// The drift verdict for one element: the fingerprint the map records, versus
/// the one recomputed from the element's current source text.
enum Drift {
    /// Recomputed fingerprint matches the recorded one — the code under this
    /// link is unchanged since the map was built.
    Same { fingerprint: String },
    /// The code under this link changed since the map was built. The fragment
    /// is still returned — the person came for the piece, not for a refusal.
    Changed { recorded: String, current: String },
    /// No verdict is possible: either the map recorded no fingerprint for this
    /// element (its scanner does not produce one), or one was recorded but the
    /// current source could not be re-fingerprinted (the spec tag is gone, or
    /// the file no longer parses). `recorded` is the fingerprint the map
    /// carries, if any.
    Unchecked {
        recorded: Option<String>,
        reason: &'static str,
    },
}

const NO_FINGERPRINT_REASON: &str = "the map recorded no fingerprint for this element (its \
                                      scanner does not produce one) — shown without verification";
const UNRECOMPUTABLE_REASON: &str = "could not recompute the fingerprint from the current source \
                                     — the spec tag may be gone, or the file no longer parses; \
                                     shown from the recorded range";

/// Return the source text of the code element behind `target` — a code symbol,
/// or a `spec://` unit linked to exactly one — and, when the map recorded a
/// fingerprint, recompute it from the element's current source and report
/// drift.
///
/// Two backends, picked exactly as [`explain`](crate::explain):
///
/// - **The project's own address / a code symbol** builds the traceability map
///   **FRESH** in memory. The recorded fingerprint is therefore the current
///   one, so the verdict is `Same` — and that match is the proof the re-scan
///   seam reproduces the build's hash (the invariant a second calculator would
///   silently break).
/// - **An installed package's address** is answered from its carried map — a
///   checkpoint built at publish time — so a body edited in the installed slot
///   since then surfaces as `Changed`: the map notices the edit before a
///   person does.
///
/// `json` selects the form: `true` → [`Fragment::Json`], `false` →
/// [`Fragment::Text`]. Drift is **information, not refusal**: the fragment is
/// returned either way, the verdict is baked into both forms, and the exit
/// code stays zero (the request was fulfilled). Errors — an unresolvable
/// target, an unreadable source file, a range past the end of a shortened file
/// — propagate as `Err` for the caller to surface.
///
/// The canonical use: point it at a tree root and a code symbol, get the
/// element's text back. The example builds a one-unit tree so it does not
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
/// fs::write(r.join("spec/D.md"), "## The rule {#req-r}\n`req r1`\n\nIt MUST hold.\n").unwrap();
/// let src = r.join("crates/x/src");
/// fs::create_dir_all(&src).unwrap();
/// fs::write(
///     src.join("lib.rs"),
///     "#[spec(implements = \"spec://demo/D#req-r\", r = 1)]\npub fn f() -> u32 { 7 }\n",
/// )
/// .unwrap();
///
/// match vibe_trace::fragment(r, "x::f", false).unwrap() {
///     vibe_trace::Fragment::Text(text) => {
///         // The element's own source is shown …
///         assert!(text.contains("pub fn f"), "the element's source is shown: {text}");
///         // … and the fresh build's fingerprint matches the re-scan's (Same).
///         assert!(text.contains("fingerprint ok"), "fresh build => Same: {text}");
///     }
///     vibe_trace::Fragment::Json(_) => panic!("default is the text view"),
/// }
/// ```
pub fn fragment(root: &Path, target: &str, json: bool) -> Result<Fragment> {
    // A foreign `spec://` address — owned by an installed package — is answered
    // from the carried map that package ships, exactly as `explain` does; the
    // slot is the base directory the element's source file is read from. The
    // own-tree path (a symbol, the project's own address, or an unowned
    // address) builds fresh below.
    if let Some(fr) = foreign::resolve_foreign(root, target)? {
        return fragment_of(
            &fr.map,
            &fr.slot,
            Source::Carried {
                coordinate: &fr.coordinate,
            },
            target,
            json,
        );
    }
    let cfg = specmap_core::config::Config::load(root)?.unwrap_or_default();
    let map = specmap_core::index::build(root, &cfg);
    fragment_of(&map, root, Source::Fresh, target, json)
}

/// The shared body once the map and its source base are known: resolve the
/// target to one code item, read its current text, analyse (slice + drift),
/// and render. Pure of the CLI's output styling.
fn fragment_of(
    map: &Specmap,
    base: &Path,
    source: Source,
    target: &str,
    json: bool,
) -> Result<Fragment> {
    let item = resolve_item(map, target)?;
    let path = base.join(&item.file);
    let text = fs::read_to_string(&path).map_err(|e| {
        anyhow::anyhow!(
            "could not read source `{}` for `{}`: {e}",
            path.display(),
            item.symbol
        )
    })?;
    let analysis = analyse(item, &text)?;
    Ok(if json {
        Fragment::Json(render_json(&analysis, item, source, target))
    } else {
        Fragment::Text(render_text(&analysis, item, source))
    })
}

/// Resolve `target` to a single code item in `map`. A code symbol matches
/// exactly, then by suffix (mirroring `specmap_core::explain::explain_symbol`);
/// a `spec://` URI resolves to the unique code item linked to it, or names the
/// candidates when more than one is. Fragment needs ONE element, so an
/// ambiguous target is an error rather than a multi-item render.
fn resolve_item<'a>(map: &'a Specmap, target: &str) -> Result<&'a CodeItem> {
    if target.starts_with("spec://") {
        let mut symbols: Vec<&str> = map
            .edges
            .iter()
            .filter(|e| e.uri == target)
            .map(|e| e.fromSymbol.as_str())
            .collect();
        symbols.sort_unstable();
        symbols.dedup();
        return match symbols.len() {
            0 => bail!(
                "no code element is linked to `{target}` — nothing to fragment. \
                 A spec unit with no implementing/verifying code has no fragment."
            ),
            1 => find_exact(map, symbols[0]),
            _ => bail!(
                "`{target}` is linked from {} code elements; fragment one by symbol:\n  {}",
                symbols.len(),
                symbols.join("\n  ")
            ),
        };
    }
    let exact: Vec<&CodeItem> = map
        .codeItems
        .iter()
        .filter(|i| i.symbol == target)
        .collect();
    if exact.len() == 1 {
        return Ok(exact[0]);
    }
    if exact.len() > 1 {
        return ambiguous(target, exact);
    }
    let suffix: Vec<&CodeItem> = map
        .codeItems
        .iter()
        .filter(|i| i.symbol.ends_with(target))
        .collect();
    match suffix.len() {
        0 => bail!(
            "no tagged code item matches `{target}` (neither exactly nor as a suffix); \
             untagged items are outside the migrated frontier — facts only"
        ),
        1 => Ok(suffix[0]),
        _ => ambiguous(target, suffix),
    }
}

/// One exact-symbol hit in `map`.
fn find_exact<'a>(map: &'a Specmap, symbol: &str) -> Result<&'a CodeItem> {
    map.codeItems
        .iter()
        .find(|i| i.symbol == symbol)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "`{symbol}` carries an edge but no code item — the map is inconsistent; regenerate it"
            )
        })
}

/// The ambiguous-target error, listing the candidates alphabetically.
fn ambiguous<'a>(target: &str, items: Vec<&'a CodeItem>) -> Result<&'a CodeItem> {
    let mut candidates: Vec<&str> = items.iter().map(|i| i.symbol.as_str()).collect();
    candidates.sort();
    bail!(
        "`{target}` is ambiguous; fragment one by symbol:\n  {}",
        candidates.join("\n  ")
    );
}

/// The Rust module path the scanner assigns to `file` inside `crate_name` —
/// mirroring `specmap_core::rscan::module_path` (the path *naming* rule, not
/// the fingerprint hash). Needed only to re-scan a file and re-pin the same
/// item by symbol; if `rscan::module_path` is ever exposed, this collapses to
/// a call.
fn module_path_of(crate_name: &str, file: &str) -> Option<String> {
    let crate_ident = crate_name.replace('-', "_");
    let comps: Vec<&str> = file.split('/').collect();
    let idx = comps.iter().position(|c| *c == crate_name)?;
    let rel = &comps[idx + 1..];
    let (head, rest) = rel.split_first()?;
    let mut parts: Vec<String> = vec![crate_ident];
    match *head {
        "src" => {}
        "tests" => parts.push("tests".to_string()),
        _ => return None,
    }
    for (i, comp) in rest.iter().enumerate() {
        if i + 1 == rest.len() {
            let stem = comp.strip_suffix(".rs")?;
            match stem {
                "lib" | "main" | "mod" => {}
                other => parts.push(other.to_string()),
            }
        } else {
            parts.push(comp.to_string());
        }
    }
    Some(parts.join("::"))
}

/// Re-scan the element's current file and return the freshly-computed item
/// matching `item.symbol` — carrying its current span AND a fingerprint minted
/// by the *same* `fingerprint_of` that built the map. `None` when the module
/// path cannot be derived, the file no longer parses, or the spec tag is gone
/// (so the item is no longer emitted).
fn rescan_item(item: &CodeItem, text: &str) -> Option<CodeItem> {
    let module = module_path_of(&item.crateName, &item.file)?;
    let (items, _edges, _warnings) = scan_source(&item.file, &item.crateName, &module, text);
    items.into_iter().find(|i| i.symbol == item.symbol)
}

/// What the analysis produced for one element: the sliced source body, the
/// span actually shown, and the drift verdict.
struct Analysis {
    body: String,
    line_count: usize,
    start: u32,
    /// The end line shown (clamped to the file when the recorded end ran past
    /// it).
    end: u32,
    /// The end line the map asked for, before clamping — for the truncation
    /// note.
    requested_end: u32,
    truncated: bool,
    drift: Drift,
}

/// Read the element's current text and decide its drift verdict. The text is
/// sliced from the **current** span when the item could be re-pinned by symbol
/// (so a body that grew or shrank is shown in full), and from the recorded
/// span otherwise. A start line past the end of a shortened file is a hard,
/// clear error; an end past it is clamped and flagged.
fn analyse(item: &CodeItem, text: &str) -> Result<Analysis> {
    let lines: Vec<&str> = text.lines().collect();
    let n = lines.len() as u32;

    let (start, requested_end, drift) = match item.fingerprint.as_ref().map(|b| b.as_str()) {
        None => (
            item.line,
            item.endLine.as_deref().copied().unwrap_or(n),
            Drift::Unchecked {
                recorded: None,
                reason: NO_FINGERPRINT_REASON,
            },
        ),
        Some(recorded) => match rescan_item(item, text) {
            Some(cur) => {
                let current = cur
                    .fingerprint
                    .as_ref()
                    .map(|b| b.as_str())
                    .unwrap_or("")
                    .to_string();
                let end = cur.endLine.as_deref().copied().unwrap_or(cur.line);
                let drift = if current.as_str() == recorded {
                    Drift::Same {
                        fingerprint: recorded.to_string(),
                    }
                } else {
                    Drift::Changed {
                        recorded: recorded.to_string(),
                        current,
                    }
                };
                (cur.line, end, drift)
            }
            None => (
                item.line,
                item.endLine.as_deref().copied().unwrap_or(n),
                Drift::Unchecked {
                    recorded: Some(recorded.to_string()),
                    reason: UNRECOMPUTABLE_REASON,
                },
            ),
        },
    };

    if start > n {
        bail!(
            "the element's recorded span starts at line {start}, but `{}` now has only {n} \
             line(s) — the file is shorter than when the map was built (the element moved or the \
             file shrank)",
            item.file
        );
    }
    if requested_end < start {
        bail!(
            "the recorded range for `{}` is empty (line {start}..{requested_end})",
            item.symbol
        );
    }
    let truncated = requested_end > n;
    let end = requested_end.min(n);
    let body: String = lines[((start - 1) as usize)..=(end as usize - 1)]
        .iter()
        .map(|l| {
            let mut s = (*l).to_string();
            s.push('\n');
            s
        })
        .collect();
    Ok(Analysis {
        body,
        line_count: n as usize,
        start,
        end,
        requested_end,
        truncated,
        drift,
    })
}

/// The text view: a header (symbol, file, span, kind), an optional carried-map
/// note, the source body, and the drift verdict.
fn render_text(a: &Analysis, item: &CodeItem, source: Source) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "fragment of `{}` — {} ({}:{}) [{}]\n",
        item.symbol, item.file, a.start, a.end, item.itemKind
    ));
    if let Source::Carried { coordinate } = source {
        out.push_str(&format!(
            "note: source read from the installed package `spec://{coordinate}/` \
             (vibedeps/…), not this tree\n"
        ));
    }
    out.push('\n');
    out.push_str(&a.body);
    out.push_str(&drift_line(&a.drift));
    if a.truncated {
        out.push_str(&format!(
            "note: the recorded span ends at line {}, beyond the {}-line file; \
             showing the lines that remain\n",
            a.requested_end, a.line_count
        ));
    }
    out
}

/// The one-line (or few-line) drift verdict for the text view.
fn drift_line(d: &Drift) -> String {
    match d {
        Drift::Same { fingerprint } => format!(
            "fingerprint ok — the code under this link is unchanged since the map was built \
             ({fingerprint})\n"
        ),
        Drift::Changed { recorded, current } => format!(
            "DRIFT: the code under this link changed since the map was built.\n  \
             recorded: {recorded}\n  current:  {current}\n  \
             (the map was built against an earlier body; the fragment above is the current source.)\n"
        ),
        Drift::Unchecked { reason, .. } => format!("note: {reason}\n"),
    }
}

/// The JSON view: target, symbol, span, text, the drift verdict, and
/// provenance — structured for an agent or script.
fn render_json(a: &Analysis, item: &CodeItem, source: Source, target: &str) -> Value {
    let (verdict, recorded, current, reason): (&str, Option<&str>, Option<&str>, Option<&str>) =
        match &a.drift {
            Drift::Same { fingerprint } => ("same", Some(fingerprint.as_str()), None, None),
            Drift::Changed { recorded, current } => (
                "changed",
                Some(recorded.as_str()),
                Some(current.as_str()),
                None,
            ),
            Drift::Unchecked { recorded, reason } => {
                ("unchecked", recorded.as_deref(), None, Some(reason))
            }
        };
    json!({
        "target": target,
        "symbol": item.symbol,
        "item_kind": item.itemKind,
        "file": item.file,
        "line": a.start,
        "end_line": a.end,
        "truncated": a.truncated,
        "text": a.body.trim_end_matches('\n'),
        "drift": {
            "verdict": verdict,
            "recorded": recorded,
            "current": current,
            "reason": reason,
        },
        "source": match source {
            Source::Fresh => json!({"from": "fresh-build"}),
            Source::Carried { coordinate } => json!({"from": "carried-map", "coordinate": coordinate}),
        },
    })
}

#[cfg(test)]
mod tests;
