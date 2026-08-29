//! The T10B visibility pin (R4 architecture §5.3): exactly what this atom
//! widened across the crate boundary, and nothing beside it.
//!
//! Its own cell rather than a fourth test in `plan_fence_tests`, because it
//! asks a different question. That file asks what a cell may NAME; this one
//! asks what a cell may EXPORT — an AST question about visibility, not about
//! imports, and the one a future atom is most likely to answer by accident.

use std::collections::BTreeSet;

use syn::{File, Item, Visibility};

/// Every `pub` item name one source declares at its top level, plus every
/// `pub` method on an `impl` block, as a flat set.
fn public_names(source: &str) -> BTreeSet<String> {
    let file: File = syn::parse_file(source).expect("the cell parses");
    let is_pub = |visibility: &Visibility| matches!(visibility, Visibility::Public(_));
    let mut names = BTreeSet::new();
    for item in &file.items {
        match item {
            Item::Struct(value) if is_pub(&value.vis) => {
                names.insert(value.ident.to_string());
            }
            Item::Enum(value) if is_pub(&value.vis) => {
                names.insert(value.ident.to_string());
            }
            Item::Fn(value) if is_pub(&value.vis) => {
                names.insert(value.sig.ident.to_string());
            }
            Item::Type(value) if is_pub(&value.vis) => {
                names.insert(value.ident.to_string());
            }
            Item::Impl(value) => {
                for item in &value.items {
                    if let syn::ImplItem::Fn(method) = item
                        && is_pub(&method.vis)
                    {
                        names.insert(method.sig.ident.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    names
}

/// The T10B visibility pin: EXACTLY what this atom widened, and nothing
/// beside it.
///
/// T2 promised "public crate-root construction waits for T10's real
/// workspace consumer", and the R4 architecture §5.3 named the four names
/// that may cross when it arrives. This test is that promise made
/// mechanical: the plan cell exports the plan VALUE plus three accessors
/// that lend no crate-private type, and the lowering cell exports the entry
/// and its refusal. A future atom that widens a seed, an entry, a provider,
/// an implementation, a config table or a digest fails here and has to say
/// so out loud.
#[test]
fn t10b_widened_exactly_the_plan_value_its_accessors_and_the_lowering_entry() {
    assert_eq!(
        public_names(include_str!("plan.rs")),
        BTreeSet::from([
            "TransformPlan".to_owned(),
            // Scalar-returning accessors only: `entries`, `digest`,
            // `build` and every seed/provider/implementation constructor
            // stay crate- or module-private.
            "empty".to_owned(),
            "len".to_owned(),
            "is_empty".to_owned(),
        ]),
        "plan.rs widens the plan value and three scalar accessors — nothing else"
    );
    assert_eq!(
        public_names(include_str!("lowering.rs")),
        BTreeSet::from(["from_effective_rows".to_owned()]),
        "lowering.rs widens ONE entry; the injected-catalog seam beside it is \
         `#[cfg(test)]` and never public"
    );
    assert_eq!(
        public_names(include_str!("fault.rs")),
        BTreeSet::from([
            // T6b's opaque execution refusal, and T10B's opaque lowering
            // refusal beside it — one cell, one idiom, both typed inside.
            "TransformCompileError".to_owned(),
            "TransformLoweringError".to_owned(),
        ]),
        "the fault cell exports its two opaque errors and no fault taxonomy"
    );
    for cell in [
        include_str!("config.rs"),
        include_str!("config_lowering.rs"),
        include_str!("plan_digest.rs"),
        include_str!("plan_validate.rs"),
        include_str!("registry.rs"),
    ] {
        assert!(
            public_names(cell).is_empty(),
            "the config, digest, refusal and behavior-registry cells stay entirely crate-internal"
        );
    }
}
