//! Model-vs-production conformance for the conditional-dependency
//! fixpoint (card scaffold-h-simulators, routine step 4: "model vs
//! production agree on representative inputs").
//!
//! The production loop lives in vibe-cli's install orchestration; its
//! primitives — `ConditionalPredicate::evaluate` over an
//! `ActivationContext`, re-solve, repeat — are all in this crate.
//! This test rebuilds the loop from those production primitives and
//! steps it in lockstep with [`FixpointModel`]: same world, same
//! roots, the same packages must join on the same iteration. If the
//! model ever drifts from what the primitives actually do, this fails
//! — a model that drifts from production misleads (the card's named
//! risk), so the conformance test is the model's license to exist.

use std::collections::BTreeSet;

use specmark::verifies;
use vibe_resolver::fixpoint_model::{ConditionalEdge, FixpointModel, ModelPackage};
use vibe_resolver::{ActivationContext, CapabilityTag, conditional::ConditionalPredicate};

/// One world package for the production side: mirrors ModelPackage
/// but evaluated through the real predicate / context types.
struct ProdPackage {
    name: &'static str,
    provides: &'static [&'static str],
    conditional: &'static [(&'static str, &'static str)],
}

/// Run the production-primitive loop: build the context from the
/// graph exactly the way `build_activation_context` does (name tag +
/// provides), evaluate every member's conditional predicates through
/// the real `ConditionalPredicate`, add fired deps, repeat to
/// stability. Returns the set added per iteration.
fn production_loop(
    world: &[ProdPackage],
    roots: &[&str],
    max_iterations: usize,
) -> Vec<BTreeSet<String>> {
    let mut graph: BTreeSet<String> = roots.iter().map(|s| s.to_string()).collect();
    let mut history = Vec::new();
    for _ in 0..max_iterations {
        // The model worlds use bare names as tags; qualify them the
        // way the production context does (`flow:<name>`), and use
        // the same qualified form in the predicates below.
        let mut ctx = ActivationContext::default();
        for name in &graph {
            ctx.add_present(CapabilityTag::parse(format!("flow:{name}")).unwrap());
            if let Some(p) = world.iter().find(|p| p.name == name) {
                for tag in p.provides {
                    ctx.add_present(CapabilityTag::parse(*tag).unwrap());
                }
            }
        }
        let mut added: BTreeSet<String> = BTreeSet::new();
        for name in &graph {
            let Some(p) = world.iter().find(|p| p.name == name) else {
                continue;
            };
            for (trigger, dep) in p.conditional {
                let pred = ConditionalPredicate::parse(&format!("context({trigger})"))
                    .expect("trigger parses as a context predicate");
                if pred.evaluate(&ctx) && !graph.contains(*dep) {
                    added.insert(dep.to_string());
                }
            }
        }
        let stable = added.is_empty();
        graph.extend(added.iter().cloned());
        history.push(added);
        if stable {
            break;
        }
    }
    history
}

/// The model side of the same world. Name tags are qualified
/// identically so both sides evaluate the same trigger strings.
fn model_world(world: &[ProdPackage]) -> Vec<ModelPackage> {
    world
        .iter()
        .map(|p| ModelPackage {
            name: p.name.to_string(),
            provides: p
                .provides
                .iter()
                .map(|s| s.to_string())
                .chain(std::iter::once(format!("flow:{}", p.name)))
                .collect(),
            conditional: p
                .conditional
                .iter()
                .map(|(t, d)| ConditionalEdge {
                    trigger: t.to_string(),
                    dep: d.to_string(),
                })
                .collect(),
        })
        .collect()
}

fn assert_conformance(world: &[ProdPackage], roots: &[&str]) {
    let prod = production_loop(world, roots, 5);
    let mut model = FixpointModel::new(model_world(world), roots.iter().copied());
    let steps = model.run(5);
    let model_added: Vec<BTreeSet<String>> = steps.iter().map(|s| s.added.clone()).collect();
    assert_eq!(
        prod, model_added,
        "model and production primitives disagree on the per-iteration added sets"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#conditional-deps")]
fn stack_trigger_world_conforms() {
    // The canonical PROP-003 §2.6.1 shape: a stack's presence pulls a
    // conditional flow in on the second pass.
    let world = [
        ProdPackage {
            name: "rust-stack",
            provides: &["stack:rust"],
            conditional: &[],
        },
        ProdPackage {
            name: "base",
            provides: &[],
            conditional: &[("stack:rust", "clippy")],
        },
        ProdPackage {
            name: "clippy",
            provides: &[],
            conditional: &[],
        },
    ];
    assert_conformance(&world, &["base", "rust-stack"]);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#conditional-deps")]
fn chained_triggers_conform() {
    // Two-stage cascade: each join changes the context the next
    // iteration probes — the case mental simulation gets wrong.
    let world = [
        ProdPackage {
            name: "a",
            provides: &[],
            conditional: &[("flow:a", "b")],
        },
        ProdPackage {
            name: "b",
            provides: &["interface:y"],
            conditional: &[],
        },
        ProdPackage {
            name: "c",
            provides: &[],
            conditional: &[("interface:y", "d")],
        },
        ProdPackage {
            name: "d",
            provides: &[],
            conditional: &[],
        },
    ];
    assert_conformance(&world, &["a", "c"]);
}

#[test]
fn already_stable_world_conforms() {
    let world = [ProdPackage {
        name: "solo",
        provides: &[],
        conditional: &[],
    }];
    assert_conformance(&world, &["solo"]);
}
