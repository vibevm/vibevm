//! The deep-merge algorithm (PROP-040 §4 `#merge-algorithm`). One
//! responsibility: fold a higher-precedence TOML table onto a lower one,
//! recording the winner ([`Origin`]) on every leaf. Pure, recursive, no
//! ambient state, no panic — kind conflicts (R2) surface as diagnostics and
//! the conflicting value is skipped.
//!
//! Kind rules (PROP-040 §4 `#merge-algorithm`):
//! - **Scalars** (string/int/bool/float/datetime) → last-wins.
//! - **Tables** → recursive deep-merge (children merge by the same rules).
//! - **Arrays** → merge per the schema's declared
//!   [`MergeStrategy`](crate::schema::MergeStrategy) (default `Replace` — the
//!   non-obvious VSCode "arrays replace, never concatenate" semantics, made
//!   explicit per §4).
//!
//! Spec: [PROP-040 §4](../../../../../spec/modules/vibe-settings/PROP-040-settings.md#merge).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#merge-algorithm");

use std::collections::BTreeMap;

use crate::schema::{MergeStrategy, Schema};

use super::Origin;

/// Deep-merge `over` onto `merged` under `path_prefix`, recording the winner
/// (`origin`) on every leaf (PROP-040 §4 `#merge-algorithm`).
///
/// Kind rules:
/// - **Scalar** in `over` → last-wins (replaces `merged[key]`); provenance =
///   `origin`. A scalar-vs-table clash is a non-fatal diagnostic (R2) and the
///   scalar is skipped.
/// - **Array** in `over` → merges onto `merged[key]` per the schema's
///   [`MergeStrategy`] for `dotted` (default [`MergeStrategy::Replace`]);
///   provenance = `origin`. An array-vs-non-array clash is a diagnostic (R2).
/// - **Table** in `over` → recursive deep-merge into `merged[key]` (creating
///   it when absent); only the nested leaves record provenance.
///
/// `path_prefix` is the dotted path to `over`'s root inside the composed tree
/// (empty at the top level); `dotted = prefix + "." + key` is the path used
/// for both schema lookup and provenance.
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#merge-algorithm")]
pub(super) fn merge_layer(
    merged: &mut toml::Table,
    provenance: &mut BTreeMap<String, Origin>,
    over: &toml::Table,
    origin: Origin,
    schema: &Schema,
    path_prefix: &str,
    diagnostics: &mut Vec<String>,
) {
    for (key, over_value) in over {
        let dotted = dotted_path(path_prefix, key);

        match over_value {
            // Table → recurse into merged[key] (creating when absent).
            toml::Value::Table(over_tbl) => {
                if let Some(toml::Value::Table(dest)) = merged.get_mut(key) {
                    merge_layer(
                        dest,
                        provenance,
                        over_tbl,
                        origin,
                        schema,
                        &dotted,
                        diagnostics,
                    );
                } else if !merged.contains_key(key) {
                    // No prior value — recursively merge into a fresh table so
                    // nested leaves record provenance and arrays apply their
                    // declared strategies.
                    let mut dest = toml::Table::new();
                    merge_layer(
                        &mut dest,
                        provenance,
                        over_tbl,
                        origin,
                        schema,
                        &dotted,
                        diagnostics,
                    );
                    merged.insert(key.clone(), toml::Value::Table(dest));
                } else {
                    push_conflict(diagnostics, &dotted, origin, "a table", "a non-table");
                }
            }

            // Array → merge onto any existing array per the declared strategy.
            toml::Value::Array(over_arr) => {
                let strategy = schema.get(&dotted).map(|m| m.merge).unwrap_or_default();
                if let Some(toml::Value::Array(base)) = merged.get_mut(key) {
                    *base = merge_array(base, over_arr, strategy, &dotted, diagnostics);
                    provenance.insert(dotted.clone(), origin);
                } else if !merged.contains_key(key) {
                    merged.insert(key.clone(), over_value.clone());
                    provenance.insert(dotted.clone(), origin);
                } else {
                    push_conflict(diagnostics, &dotted, origin, "an array", "a non-array");
                }
            }

            // Scalar (string/int/bool/float/datetime) → last-wins.
            _ => {
                if matches!(merged.get(key), Some(toml::Value::Table(_))) {
                    push_conflict(diagnostics, &dotted, origin, "a scalar", "a table");
                } else {
                    merged.insert(key.clone(), over_value.clone());
                    provenance.insert(dotted.clone(), origin);
                }
            }
        }
    }
}

/// Merge `over` onto `base` per `strategy` (PROP-040 §4 `#merge-strategy-opt-in`).
/// `Replace` is the default (the higher layer's array fully replaces the
/// lower — the non-obvious VSCode semantics, made explicit per §4
/// `#merge-algorithm`).
fn merge_array(
    base: &[toml::Value],
    over: &[toml::Value],
    strategy: MergeStrategy,
    dotted: &str,
    diagnostics: &mut Vec<String>,
) -> Vec<toml::Value> {
    match strategy {
        // The higher layer's array fully replaces the lower's.
        MergeStrategy::Replace => over.to_vec(),
        // base ++ over.
        MergeStrategy::Append => {
            let mut out = Vec::with_capacity(base.len() + over.len());
            out.extend(base.iter().cloned());
            out.extend(over.iter().cloned());
            out
        }
        // over ++ base.
        MergeStrategy::Prepend => {
            let mut out = Vec::with_capacity(base.len() + over.len());
            out.extend(over.iter().cloned());
            out.extend(base.iter().cloned());
            out
        }
        // REVIEW(phase 2.4): `MergeByKey` carries no identifying field yet —
        // the spec (§4 #merge-strategy-opt-in) leaves the key-field open. The
        // interim semantics here are identity-by-index: zip base with over,
        // `over` wins index-for-index; extras from either side are kept. A
        // future PROP adds `key_field` to `KeyMeta` and rewrites this arm.
        MergeStrategy::MergeByKey => merge_by_index(base, over, dotted, diagnostics),
    }
}

/// Identity-by-index merge: pairwise `over` wins; trailing elements from
/// either side are appended. Interim for [`MergeStrategy::MergeByKey`] —
/// documented via the REVIEW marker in [`merge_array`].
fn merge_by_index(
    base: &[toml::Value],
    over: &[toml::Value],
    dotted: &str,
    diagnostics: &mut Vec<String>,
) -> Vec<toml::Value> {
    if !base.is_empty() && !over.is_empty() && base.len() != over.len() {
        diagnostics.push(format!(
            "merge-by-key at `{dotted}` falls back to identity-by-index (base={}, over={}) — \
             REVIEW(phase 2.4): a key-field will make this a real keyed merge \
             (spec://vibevm/modules/vibe-settings/PROP-040#merge-strategy-opt-in)",
            base.len(),
            over.len(),
        ));
    }
    let max_len = base.len().max(over.len());
    let mut out = Vec::with_capacity(max_len);
    for i in 0..max_len {
        match (base.get(i), over.get(i)) {
            (_, Some(o)) => out.push(o.clone()),
            (Some(b), None) => out.push(b.clone()),
            // i is bounded by max_len; (None, None) is unreachable.
            (None, None) => {}
        }
    }
    out
}

/// Append a non-fatal merge-conflict diagnostic (risk R2 — mixed kinds at one
/// path). The conflicting value from `origin` is skipped; the prior layer's
/// value stands. The message cites the REQ so a surface can point at the
/// contract clause.
fn push_conflict(
    diagnostics: &mut Vec<String>,
    dotted: &str,
    origin: Origin,
    over_kind: &str,
    prior_kind: &str,
) {
    diagnostics.push(format!(
        "merge conflict at `{dotted}`: {origin} has {over_kind} but a prior layer set {prior_kind}; \
         the {over_kind} is skipped \
         (REVIEW: spec://vibevm/modules/vibe-settings/PROP-040#merge-algorithm)"
    ));
}

/// Build a child dotted path from a prefix and a key.
fn dotted_path(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_owned()
    } else {
        format!("{prefix}.{key}")
    }
}

#[cfg(test)]
mod tests {
    //! Algorithm-level golden tests — scalar/array/object merge, opt-in
    //! strategies, and R2 mixed-kind conflicts (predictions P2; risk R2).
    //! These exercise [`merge_layer`] end-to-end through [`super::super::resolve`]
    //! because that is the real integration path.
    use super::super::{Origin, resolve};
    use crate::loader::LayeredRaw;
    use crate::schema::{KeyMeta, KeyType, MergeStrategy, Schema, Scope};

    /// Parse a TOML body into a table (test helper — body is fixture data).
    fn tbl(body: &str) -> toml::Table {
        toml::from_str(body).unwrap()
    }

    /// Build `LayeredRaw` from three bodies (L1, L2, L3).
    fn layered(l1: &str, l2: &str, l3: &str) -> LayeredRaw {
        LayeredRaw {
            l1: tbl(l1),
            l2: tbl(l2),
            l3: tbl(l3),
        }
    }

    /// Build a KeyMeta for an array key with a merge strategy (test helper).
    fn array_key(path: &str, strategy: MergeStrategy) -> KeyMeta {
        KeyMeta::new(path, KeyType::Array, Scope::User, "a test array")
            .unwrap()
            .with_merge(strategy)
    }

    // ── P2: scalar last-wins (no per-key special-casing) ────────────────────

    #[test]
    fn scalar_last_wins_records_higher_layer_as_origin() {
        let raw = layered("a = 1\n", "a = 2\n", "");
        let rp = resolve(raw, &Schema::new(), toml::Table::new(), toml::Table::new());
        assert_eq!(rp.get("a").and_then(|v| v.as_integer()), Some(2));
        assert_eq!(rp.origin("a"), Some(Origin::L2));
    }

    #[test]
    fn object_deep_merge_unions_then_overrides_leaves() {
        // Union: L1 contributes x, L2 contributes y.
        let raw = layered("[tree]\nx = 1\n", "[tree]\ny = 2\n", "");
        let rp = resolve(raw, &Schema::new(), toml::Table::new(), toml::Table::new());
        let tree = rp.get_section("tree").unwrap();
        assert_eq!(tree.get("x").and_then(|v| v.as_integer()), Some(1));
        assert_eq!(tree.get("y").and_then(|v| v.as_integer()), Some(2));

        // Override: both set x; L2 wins.
        let raw = layered("[tree]\nx = 1\n", "[tree]\nx = 2\n", "");
        let rp = resolve(raw, &Schema::new(), toml::Table::new(), toml::Table::new());
        assert_eq!(rp.get("tree.x").and_then(|v| v.as_integer()), Some(2));
        assert_eq!(rp.origin("tree.x"), Some(Origin::L2));
    }

    // ── P2: arrays default to Replace; opt-in Append/Prepend/MergeByKey ─────

    #[test]
    fn array_replace_is_the_default_strategy() {
        // No schema entry for `arr` ⇒ default strategy = Replace.
        let raw = layered("arr = [1, 2, 3]\n", "arr = [4]\n", "");
        let rp = resolve(raw, &Schema::new(), toml::Table::new(), toml::Table::new());
        let got: Vec<i64> = rp
            .get("arr")
            .and_then(|v| v.as_array())
            .unwrap()
            .iter()
            .map(|v| v.as_integer().unwrap())
            .collect();
        assert_eq!(got, vec![4]);
        assert_eq!(rp.origin("arr"), Some(Origin::L2));
    }

    #[test]
    fn array_append_concatenates_base_then_over() {
        let mut schema = Schema::new();
        schema
            .register(array_key("arr", MergeStrategy::Append))
            .unwrap();
        let raw = layered("arr = [1, 2]\n", "arr = [3]\n", "");
        let rp = resolve(raw, &schema, toml::Table::new(), toml::Table::new());
        let got: Vec<i64> = rp
            .get("arr")
            .and_then(|v| v.as_array())
            .unwrap()
            .iter()
            .map(|v| v.as_integer().unwrap())
            .collect();
        assert_eq!(got, vec![1, 2, 3]);
    }

    #[test]
    fn array_prepend_concatenates_over_then_base() {
        let mut schema = Schema::new();
        schema
            .register(array_key("arr", MergeStrategy::Prepend))
            .unwrap();
        let raw = layered("arr = [1, 2]\n", "arr = [3]\n", "");
        let rp = resolve(raw, &schema, toml::Table::new(), toml::Table::new());
        let got: Vec<i64> = rp
            .get("arr")
            .and_then(|v| v.as_array())
            .unwrap()
            .iter()
            .map(|v| v.as_integer().unwrap())
            .collect();
        assert_eq!(got, vec![3, 1, 2]);
    }

    #[test]
    fn array_merge_by_key_uses_identity_by_index_interim() {
        // REVIEW(phase 2.4): identity-by-index; over wins pairwise.
        let mut schema = Schema::new();
        schema
            .register(array_key("arr", MergeStrategy::MergeByKey))
            .unwrap();
        let raw = layered("arr = [1, 2, 3]\n", "arr = [10, 20]\n", "");
        let rp = resolve(raw, &schema, toml::Table::new(), toml::Table::new());
        let got: Vec<i64> = rp
            .get("arr")
            .and_then(|v| v.as_array())
            .unwrap()
            .iter()
            .map(|v| v.as_integer().unwrap())
            .collect();
        assert_eq!(got, vec![10, 20, 3]);
        // Length mismatch surfaces the REVIEW diagnostic.
        assert!(
            rp.diagnostics()
                .iter()
                .any(|d| d.contains("identity-by-index")),
            "expected a merge-by-key diagnostic, got: {:?}",
            rp.diagnostics(),
        );
    }

    // ── R2: mixed kind at one path is a non-fatal diagnostic ────────────────

    #[test]
    fn mixed_scalar_then_table_records_diagnostic_and_skips() {
        // L1 sets `k` as a scalar; L2 tries to make it a table — the table is
        // skipped, the L1 scalar stands, and a diagnostic is recorded.
        let raw = layered("k = 1\n", "[k]\nx = 2\n", "");
        let rp = resolve(raw, &Schema::new(), toml::Table::new(), toml::Table::new());
        assert_eq!(rp.get("k").and_then(|v| v.as_integer()), Some(1));
        assert_eq!(
            rp.origin("k"),
            Some(Origin::L1),
            "L2's conflicting table did not override L1's scalar",
        );
        assert_eq!(rp.diagnostics().len(), 1);
        assert!(rp.diagnostics()[0].contains("merge conflict"));
        assert!(rp.diagnostics()[0].contains("merge-algorithm"));
    }

    #[test]
    fn mixed_table_then_scalar_records_diagnostic_and_skips() {
        // L1 sets `k` as a table; L2 tries a scalar — the scalar is skipped.
        let raw = layered("[k]\nx = 1\n", "k = 2\n", "");
        let rp = resolve(raw, &Schema::new(), toml::Table::new(), toml::Table::new());
        let table = rp.get("k").and_then(|v| v.as_table()).unwrap();
        assert_eq!(table.get("x").and_then(|v| v.as_integer()), Some(1));
        assert_eq!(rp.diagnostics().len(), 1);
    }
}
