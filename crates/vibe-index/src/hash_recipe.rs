//! Hash-recipe catalogue — the two ways a package-tree content hash is
//! computed and labelled (PROP-044 §4.7 `##M-BREAK-WINDOW`).
//!
//! - Recipe 0 ([`RecipeId::Legacy0`]): the pre-2026-08 behaviour, frozen
//!   verbatim in *code* (its exclude list is [`LEGACY0_EXCLUDES`]). It exists
//!   only so hashes written before recipes were named stay readable; it is
//!   NOT configurable, because a frozen recipe that can be edited is not
//!   frozen. Its wire label is the bare `sha256:`.
//! - Recipe 1 ([`RecipeId::Tree1`]): the live tree recipe, its parameters
//!   carried as *data* in `formats/hash_recipes/1.toml` (parsed once via a
//!   process-global [`OnceLock`]). Its wire label is `sha256-tree/1:`.
//!
//! This module is duplicated verbatim-in-intent in `vibe-registry`; the two
//! MUST stay in lockstep (PROP-005 §3.2) so a package indexed here and
//! materialised there hashes identically for any given recipe. The parity
//! test `tests/content_hash_parity.rs` gates that divergence at CI time.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#trust");

use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;

/// Which recipe computes (and labels) a content hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeId {
    /// Recipe 0 — the pre-2026-08 behaviour, frozen verbatim. It exists so
    /// hashes written before recipes were named stay readable and
    /// reproducible; it is NOT configurable, because a frozen recipe that can
    /// be edited is not frozen. Its wire label is the bare `sha256:`.
    Legacy0,
    /// Recipe 1 — the live tree recipe, its parameters carried as data in
    /// `formats/hash_recipes/1.toml`. Its wire label is `sha256-tree/1:`.
    Tree1,
}

impl RecipeId {
    /// The wire label this recipe stamps in front of the hex digest.
    pub fn label(self) -> &'static str {
        match self {
            RecipeId::Legacy0 => "sha256:",
            RecipeId::Tree1 => "sha256-tree/1:",
        }
    }
}

/// Recipe 0's frozen exclude set — the pre-2026-08 constants, byte-identical
/// in `vibe-registry`'s copy of this module. A frozen recipe that can be
/// edited is not frozen, so this is a `const`, not data.
pub const LEGACY0_EXCLUDES: &[&str] = &[".git", ".vibe", "target", "node_modules", ".vibeignore"];

/// Recipe 1's parameters, parsed once from `formats/hash_recipes/1.toml`.
/// Every field is read in [`recipe1`] so the file is genuinely the source of
/// truth for recipe 1's documented behaviour: a drift in any declared
/// parameter fails loudly the first time the recipe is used.
#[derive(Debug, Deserialize)]
struct Recipe1File {
    schema: u32,
    excludes: Vec<String>,
    path_normalisation: String,
    sort: String,
}

static RECIPE1: OnceLock<Recipe1File> = OnceLock::new();

/// The parsed recipe-1 file, materialised exactly once per process.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "no-unwrap-gate: the recipe file is embedded by `include_str!`, so its \
              well-formedness is a property of the BUILD, not of any run — there is no \
              runtime input that can make this parse fail, and threading a Result through \
              every caller would add a branch no test can reach. The machinery that makes \
              the panic unreachable rather than merely unlikely: the `assert_eq!`s below \
              pin every field this recipe depends on, and the frozen goldens in \
              `tests/content_hash_parity.rs` fail the moment the file's MEANING changes — \
              demonstrated by editing the recipe's exclude list and watching the golden \
              break, then reverting."
)]
fn recipe1() -> &'static Recipe1File {
    RECIPE1.get_or_init(|| {
        let text = include_str!("../../../formats/hash_recipes/1.toml");
        let parsed: Recipe1File = toml::from_str(text).expect(
            "formats/hash_recipes/1.toml must parse as recipe 1 (PROP-044 §4.7); \
             carrying the recipe as data means a malformed file fails loudly",
        );
        assert_eq!(parsed.schema, 1, "recipe 1 file must declare schema = 1");
        assert_eq!(
            parsed.path_normalisation, "backslash-to-slash",
            "recipe 1 normalises separators before ordering"
        );
        assert_eq!(
            parsed.sort, "bytewise-normalised",
            "recipe 1 orders the normalised string"
        );
        parsed
    })
}

/// The dir/file names recipe 1 prunes from the walk, read from the recipe
/// file. The strings are `'static` because the file is parsed exactly once
/// into a process-global [`OnceLock`].
pub fn recipe1_excludes() -> Vec<&'static str> {
    recipe1().excludes.iter().map(String::as_str).collect()
}

/// Order the relative-path strings for hashing under `recipe`, returning the
/// normalised (`\` → `/`) strings in hash order.
///
/// Recipe 1 normalises separators BEFORE ordering, so its order is the same
/// byte sequence on every host. Recipe 0 reproduces the pre-2026-08 order,
/// which sorted the platform `PathBuf` first and normalised afterwards — on
/// a `\` host that order depends on the separator whenever a sibling's name
/// shares a directory's prefix and continues with a byte between `0x2F`
/// (`/`) and `0x5C` (`\`). That is the defect recipe 1 fixes, and the reason
/// this function is callable in isolation from the filesystem: the property
/// "recipe 1's order does not depend on the input separator" is provable on
/// every host without touching disk (see
/// `tests/content_hash_parity.rs::normalisation_precedes_ordering`).
pub fn order_paths(recipe: RecipeId, rels: &[String]) -> Vec<String> {
    order_entries(recipe, rels.iter().map(|r| (r.clone(), ())).collect())
        .into_iter()
        .map(|(norm, ())| norm)
        .collect()
}

/// [`order_paths`] with a payload riding along: order `(raw_rel, payload)`
/// pairs by the recipe's rule and return each **normalised** rel beside its
/// payload, in hash order.
///
/// The payload travels with its path so a hasher never has to look the path
/// back up after ordering. That lookup is the reason this function exists in
/// this shape: a map from normalised rel back to file cannot fail on a real
/// filesystem, and a branch that cannot fail is exactly the branch that gets
/// written as `.expect()` and then has to be argued about. Carrying the file
/// through removes the branch instead of annotating it.
pub fn order_entries<T>(recipe: RecipeId, items: Vec<(String, T)>) -> Vec<(String, T)> {
    // Keep the raw input beside its normalised form: recipe 0 orders by the
    // raw platform string, recipe 1 by the normalised one, and both emit the
    // normalised bytes the hash consumes.
    let mut triples: Vec<(String, String, T)> = items
        .into_iter()
        .map(|(raw, payload)| {
            let norm = raw.replace('\\', "/");
            (raw, norm, payload)
        })
        .collect();
    match recipe {
        RecipeId::Tree1 => triples.sort_by(|a, b| a.1.cmp(&b.1)),
        // Recipe 0 ordered a `Vec<PathBuf>`, and `Path`'s `Ord` is
        // **component-wise**, not a byte compare of the whole string. Ordering
        // the raw string instead would silently re-order any tree holding a
        // directory whose name prefixes a sibling file — i.e. it would change
        // hashes that recipe 0 exists to keep reproducible. Measured, not
        // assumed: sorting `["spec\inner\a.md", "specX.md"]` as paths yields
        // `spec/inner/a.md` first, as strings yields `specX.md` first.
        RecipeId::Legacy0 => {
            triples.sort_by(|a, b| Path::new(a.0.as_str()).cmp(Path::new(b.0.as_str())))
        }
    }
    triples
        .into_iter()
        .map(|(_, norm, payload)| (norm, payload))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `order_entries` is what the hasher runs; `order_paths` is what the
    /// platform-independence proof runs
    /// (`tests/content_hash_parity.rs::normalisation_precedes_ordering`). If
    /// the two could disagree, that proof would be about code nobody executes.
    /// `vibe-registry` carries the same pair of tests over its own copy — two
    /// implementations mean two proofs.
    #[test]
    fn order_paths_is_order_entries_without_a_payload() {
        let rels = vec![
            r"spec\inner\a.md".to_string(),
            "specX.md".to_string(),
            "README.md".to_string(),
        ];
        for recipe in [RecipeId::Legacy0, RecipeId::Tree1] {
            let items: Vec<(String, u8)> = rels.iter().map(|r| (r.clone(), 0u8)).collect();
            let via_entries: Vec<String> = order_entries(recipe, items)
                .into_iter()
                .map(|(norm, _)| norm)
                .collect();
            assert_eq!(order_paths(recipe, &rels), via_entries, "{recipe:?}");
        }
    }
}
