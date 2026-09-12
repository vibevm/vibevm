//! The manifest and lock-file surface — every key `vibe.toml` and
//! `vibe.lock` accept (PROP-057 `##OBS-SURFACE-SNAPSHOTS`).
//!
//! The list is obtained by ASKING THE PARSER, not by reading the structs
//! it is built from. A probe document carrying one key nobody declared is
//! handed to the real deserialiser, and the refusal names every key that
//! level does accept — which is what `deny_unknown_fields` is for. The
//! walk repeats a level down for each key that turns out to be a table,
//! and the result is the dotted paths a manifest may carry.
//!
//! Why this way round. The surface of a version is what the product
//! ACCEPTS, and a struct's name, its module and its documentation are not
//! part of that. A scan of the source would also have to model serde's
//! renames, flattening and aliases — a second implementation of the thing
//! being described, drifting from it the first time an attribute changed.
//! Here the answer comes from the one parser a manifest actually meets,
//! so a snapshot cannot disagree with the product it describes.
//!
//! What it does not reach: a table whose keys are free-form (`[features]`
//! and its kind), a value the parser validates before it looks at any
//! key, and a table that ACCEPTS what it does not know — a level with no
//! `deny_unknown_fields` answers the probe with silence, and silence is
//! not a field list. All three are leaves in the result rather than
//! silent gaps: the path itself is recorded, and only the level below it
//! is unopened.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-SURFACE-SNAPSHOTS");

use std::collections::BTreeSet;

use serde::de::DeserializeOwned;
use vibe_core::manifest::{Lockfile, Manifest};

/// The key the probe document carries. It has to be a key nothing
/// accepts, in every table of both documents, for the probe to be a
/// question rather than a value.
const PROBE: &str = "vibe_doc_surface_probe";

/// How deep the walk opens tables. The deepest real path in either
/// document today is three segments; the bound is here because a
/// self-referential shape (a condition that carries conditions) would
/// otherwise be walked until the stack ran out.
const MAX_DEPTH: usize = 5;

/// A ceiling on how many questions one walk may ask. It is a guard and
/// not a budget: reaching it means the shape being walked is not the
/// shape this module was written for, and stopping with a partial answer
/// beats running until somebody notices.
const MAX_PROBES: usize = 50_000;

/// Every dotted key path `vibe.toml` accepts, sorted.
///
/// ```
/// let fields = vibe_doc::surface::fields::manifest_fields();
/// assert!(fields.iter().any(|f| f == "package"));
/// assert!(fields.iter().any(|f| f == "package.title"));
/// ```
pub fn manifest_fields() -> Vec<String> {
    crawl::<Manifest>()
}

/// Every dotted key path `vibe.lock` accepts, sorted.
///
/// A path carries the bracket form of the table it names, so the two
/// documents do not read alike where they are not alike: `vibe.toml`
/// declares `[package]` once, `vibe.lock` declares `[[package]]` per
/// locked package, and only the second spelling ends in `[]`.
///
/// ```
/// let fields = vibe_doc::surface::fields::lock_fields();
/// // `[meta]` is a REQUIRED member, and it does not hide the list: the
/// // refusal names every key this level takes, not the one it misses.
/// assert!(fields.iter().any(|f| f == "meta"));
/// assert!(fields.iter().any(|f| f == "meta.schema_version"));
/// // The locked packages are an array of tables, and the path says so.
/// assert!(fields.iter().any(|f| f == "package[]"));
/// assert!(fields.iter().any(|f| f == "package[].name"));
/// assert!(!fields.iter().any(|f| f == "package"));
/// ```
pub fn lock_fields() -> Vec<String> {
    crawl::<Lockfile>()
}

/// One segment of a path under construction: the key, and whether the
/// table it opens is an array of tables.
#[derive(Debug, Clone)]
struct Segment {
    name: String,
    repeated: bool,
}

/// Walk one document type.
fn crawl<T: DeserializeOwned>() -> Vec<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut queue: Vec<Vec<Segment>> = vec![Vec::new()];
    let mut probes = 0usize;
    while let Some(path) = queue.pop() {
        let Some(names) = accepted::<T>(&path, &mut probes) else {
            continue;
        };
        for name in names {
            let mut table = path.clone();
            table.push(Segment {
                name: name.clone(),
                repeated: false,
            });
            let mut array = path.clone();
            array.push(Segment {
                name,
                repeated: true,
            });
            // A key is written down before the walk asks what is under
            // it: a field that turns out to be a scalar is still part of
            // the surface, and the question below it is a different one.
            let opens_table = path.len() < MAX_DEPTH
                && accepted::<T>(&table, &mut probes).is_some_and(|fields| !fields.is_empty());
            let opens_array = !opens_table
                && path.len() < MAX_DEPTH
                && accepted::<T>(&array, &mut probes).is_some_and(|fields| !fields.is_empty());
            let reached = if opens_array { array } else { table };
            if !out.insert(spell(&reached)) {
                continue;
            }
            if opens_table || opens_array {
                queue.push(reached);
            }
        }
    }
    out.into_iter().collect()
}

/// `package.binary[].name` — the path as a person would write it, with
/// `[]` marking the segments that are arrays of tables.
fn spell(path: &[Segment]) -> String {
    path.iter()
        .map(|s| {
            if s.repeated {
                format!("{}[]", s.name)
            } else {
                s.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// Ask the parser what it accepts at `path`.
///
/// `Some(fields)` means the level is a struct that refuses what it does
/// not know — the answer is its field list. `None` means it is not: a
/// scalar, a free-form map, a sequence of scalars, or a value whose own
/// deserialiser refused before any key was read. Both are legitimate
/// states of a real document, so neither is an error.
fn accepted<T: DeserializeOwned>(path: &[Segment], probes: &mut usize) -> Option<Vec<String>> {
    if *probes >= MAX_PROBES {
        return None;
    }
    *probes += 1;
    let document = probe_document(path);
    let error = toml::from_str::<T>(&document).err()?;
    expected(&error.to_string())
}

/// The probe document for one path: every ancestor declared in order with
/// its own bracket form, then the key nobody accepts.
///
/// The chain is spelled out rather than dotted, because `[a.b]` after no
/// `[[a]]` would make `a` a table — and the level being asked about would
/// refuse for a reason that has nothing to do with its keys.
fn probe_document(path: &[Segment]) -> String {
    let mut out = String::new();
    let mut prefix: Vec<&str> = Vec::new();
    for segment in path {
        prefix.push(&segment.name);
        let dotted = prefix.join(".");
        if segment.repeated {
            out.push_str(&format!("[[{dotted}]]\n"));
        } else {
            out.push_str(&format!("[{dotted}]\n"));
        }
    }
    out.push_str(PROBE);
    out.push_str(" = 1\n");
    out
}

/// The field list out of a refusal, or `None` when the refusal is not
/// about an unknown key.
///
/// serde writes the list four ways, by how many names are in it: «there
/// are no fields», «expected `a`», «expected `a` or `b`», and «expected
/// one of `a`, `b`, `c`». All four are the same answer — this level is a
/// struct, and here is what it takes — so all four are read here rather
/// than three of them reading as «not a struct».
///
/// ```
/// use vibe_doc::surface::fields::expected_for_test as expected;
///
/// assert_eq!(
///     expected("unknown field `vibe_doc_surface_probe`, expected one of `a`, `b`, `c`"),
///     Some(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
/// );
/// assert_eq!(
///     expected("unknown field `vibe_doc_surface_probe`, expected `a` or `b` at line 1"),
///     Some(vec!["a".to_string(), "b".to_string()]),
/// );
/// assert_eq!(expected("invalid type: integer, expected a string"), None);
/// ```
fn expected(message: &str) -> Option<Vec<String>> {
    let marker = format!("unknown field `{PROBE}`");
    let rest = message.split_once(&marker).map(|(_, rest)| rest)?;
    let Some(list) = rest.split_once("expected ").map(|(_, list)| list) else {
        return Some(Vec::new());
    };
    let mut out = Vec::new();
    let mut cursor = list.strip_prefix("one of ").unwrap_or(list);
    while let Some(after) = cursor.strip_prefix('`') {
        let Some((name, rest)) = after.split_once('`') else {
            break;
        };
        out.push(name.to_owned());
        cursor = rest
            .strip_prefix(", or ")
            .or_else(|| rest.strip_prefix(", "))
            .or_else(|| rest.strip_prefix(" or "))
            .unwrap_or("");
    }
    Some(out)
}

/// [`expected`] under a name a doctest can reach. The reader of a
/// refusal is the part of this module most likely to be broken by an
/// upgrade of serde, so it is the part with a test of its own.
#[doc(hidden)]
pub fn expected_for_test(message: &str) -> Option<Vec<String>> {
    expected(message)
}

#[cfg(test)]
mod tests;
