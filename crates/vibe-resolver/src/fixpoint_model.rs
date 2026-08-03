//! A runnable reference model of the conditional-dependency fixpoint
//! (card scaffold-h-simulators; PROP-003 §2.6 semantics, the
//! solve → probe → add → re-solve loop `vibe install` orchestrates).
//!
//! Execution-prediction is where weak readers are weakest: nobody
//! should have to mentally simulate the fixpoint to modify code near
//! it. This model IS the simulation — feed it a world, step it, watch
//! the present-set grow monotonically until stability. It is a
//! reference, deliberately tiny: packages are names, capabilities are
//! tags, conditional deps are `(trigger-tag → dep)` pairs. The
//! production loop (vibe-cli's install, driving the real solver and
//! `ConditionalPredicate`) must agree with it on representative
//! worlds — the conformance test in `tests/fixpoint_conformance.rs`
//! holds the two together.
//!
//! The monotone-lattice property is the load-bearing fact: the
//! present-set only ever grows, so the loop terminates (a fixpoint
//! exists) — witnessed by a contract at every step, not described in
//! prose (card scaffold-c).

use std::collections::{BTreeMap, BTreeSet};

use specmark::spec;

/// One package in the model world.
#[derive(Debug, Clone)]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#conditional-deps"
)]
pub struct ModelPackage {
    /// Package name; doubles as its graph tag (`flow:<name>` in the
    /// production shape, plain `name` here — the model abstracts the
    /// namespace).
    pub name: String,
    /// Tags this package contributes to the present-set when it is in
    /// the graph (capabilities / interfaces it provides).
    pub provides: Vec<String>,
    /// Conditional dependencies: when `trigger` is present in the
    /// context, `dep` joins the graph on the next re-solve.
    pub conditional: Vec<ConditionalEdge>,
}

/// `context(<trigger>) → <dep>` — the model's rendering of one
/// `[target."context(...)".dependencies]` manifest entry.
#[derive(Debug, Clone)]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#conditional-deps"
)]
pub struct ConditionalEdge {
    pub trigger: String,
    pub dep: String,
}

/// One observed iteration of the loop — what a reader inspects
/// instead of mentally simulating.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#conditional-deps"
)]
pub struct FixpointStep {
    /// 1-based iteration number.
    pub iteration: usize,
    /// The present-set the predicates were evaluated against.
    pub present: BTreeSet<String>,
    /// Edges that fired this iteration: `(owner, trigger, dep)`.
    pub fired: Vec<(String, String, String)>,
    /// Packages newly added to the graph by those firings.
    pub added: BTreeSet<String>,
    /// True iff nothing was added — the fixpoint is reached.
    pub stable: bool,
}

/// The runnable model. Construct with the world and the root set,
/// then [`step`](Self::step) until [`stable`](Self::is_stable) — or
/// [`run`](Self::run) to completion.
///
/// ```
/// use vibe_resolver::fixpoint_model::{ConditionalEdge, FixpointModel, ModelPackage};
///
/// // rust-stack provides `stack:rust`; the project conditionally
/// // pulls `clippy-flow` when `stack:rust` is present.
/// let world = vec![
///     ModelPackage {
///         name: "rust-stack".into(),
///         provides: vec!["stack:rust".into()],
///         conditional: vec![],
///     },
///     ModelPackage {
///         name: "project-base".into(),
///         provides: vec![],
///         conditional: vec![ConditionalEdge {
///             trigger: "stack:rust".into(),
///             dep: "clippy-flow".into(),
///         }],
///     },
///     ModelPackage {
///         name: "clippy-flow".into(),
///         provides: vec![],
///         conditional: vec![],
///     },
/// ];
/// let mut model = FixpointModel::new(world, ["project-base", "rust-stack"]);
/// let steps = model.run(5);
/// // Iteration 1 fires the edge and adds clippy-flow; iteration 2 is stable.
/// assert_eq!(steps.len(), 2);
/// assert!(steps[0].added.contains("clippy-flow"));
/// assert!(steps[1].stable);
/// ```
#[derive(Debug, Clone)]
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#conditional-deps"
)]
pub struct FixpointModel {
    world: BTreeMap<String, ModelPackage>,
    graph: BTreeSet<String>,
    iteration: usize,
    stable: bool,
}

impl FixpointModel {
    pub fn new<I, S>(world: Vec<ModelPackage>, roots: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let world: BTreeMap<String, ModelPackage> =
            world.into_iter().map(|p| (p.name.clone(), p)).collect();
        let graph: BTreeSet<String> = roots.into_iter().map(Into::into).collect();
        FixpointModel {
            world,
            graph,
            iteration: 0,
            stable: false,
        }
    }

    /// The present-set induced by the current graph: every member's
    /// name tag plus everything it provides.
    pub fn present(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for name in &self.graph {
            out.insert(name.clone());
            if let Some(p) = self.world.get(name) {
                out.extend(p.provides.iter().cloned());
            }
        }
        out
    }

    /// Packages currently in the graph.
    pub fn graph(&self) -> &BTreeSet<String> {
        &self.graph
    }

    pub fn is_stable(&self) -> bool {
        self.stable
    }

    /// One iteration of solve → probe → add. Returns the observable
    /// record of what happened.
    pub fn step(&mut self) -> FixpointStep {
        self.iteration += 1;
        let present = self.present();

        let mut fired: Vec<(String, String, String)> = Vec::new();
        let mut added: BTreeSet<String> = BTreeSet::new();
        for name in &self.graph {
            let Some(p) = self.world.get(name) else {
                continue;
            };
            for edge in &p.conditional {
                if present.contains(&edge.trigger) {
                    fired.push((name.clone(), edge.trigger.clone(), edge.dep.clone()));
                    if !self.graph.contains(&edge.dep) {
                        added.insert(edge.dep.clone());
                    }
                }
            }
        }
        fired.sort();

        self.graph.extend(added.iter().cloned());
        self.stable = added.is_empty();

        // The monotone-lattice contract: the present-set never shrinks
        // across a step. This is WHY the loop terminates — the lattice
        // of (graph, present) is finite and growth-only, so a fixpoint
        // exists (PROP-003 §2.6; card scaffold-c witness, not prose).
        let after = self.present();
        debug_assert!(
            after.is_superset(&present),
            "monotonicity violated: present-set shrank across a step"
        );

        FixpointStep {
            iteration: self.iteration,
            present,
            fired,
            added,
            stable: self.stable,
        }
    }

    /// Step to stability or `max_iterations`, whichever first; returns
    /// every step taken. The production loop caps at 5 iterations and
    /// errors loudly past the cap — mirror that shape when comparing.
    pub fn run(&mut self, max_iterations: usize) -> Vec<FixpointStep> {
        let mut steps = Vec::new();
        while !self.stable && steps.len() < max_iterations {
            steps.push(self.step());
        }
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(name: &str, provides: &[&str], conditional: &[(&str, &str)]) -> ModelPackage {
        ModelPackage {
            name: name.into(),
            provides: provides.iter().map(|s| s.to_string()).collect(),
            conditional: conditional
                .iter()
                .map(|(t, d)| ConditionalEdge {
                    trigger: t.to_string(),
                    dep: d.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn no_conditionals_is_immediately_stable() {
        let mut m = FixpointModel::new(vec![pkg("a", &[], &[])], ["a"]);
        let steps = m.run(5);
        assert_eq!(steps.len(), 1);
        assert!(steps[0].stable);
        assert!(steps[0].fired.is_empty());
    }

    #[test]
    fn chained_conditionals_cascade_one_per_iteration() {
        // a --(a present)--> b --(b present)--> c: two growth steps,
        // then the stable observation.
        let world = vec![
            pkg("a", &[], &[("a", "b")]),
            pkg("b", &[], &[("b", "c")]),
            pkg("c", &[], &[]),
        ];
        let mut m = FixpointModel::new(world, ["a"]);
        let steps = m.run(5);
        assert_eq!(steps.len(), 3);
        assert!(steps[0].added.contains("b"));
        assert!(steps[1].added.contains("c"));
        assert!(steps[2].stable);
        assert!(m.graph().contains("c"));
    }

    #[test]
    fn provides_tags_trigger_edges() {
        let world = vec![
            pkg("rust-stack", &["stack:rust"], &[]),
            pkg("base", &[], &[("stack:rust", "clippy")]),
            pkg("clippy", &[], &[]),
        ];
        let mut m = FixpointModel::new(world, ["base", "rust-stack"]);
        let steps = m.run(5);
        assert!(steps[0].added.contains("clippy"));
        assert!(m.is_stable());
    }

    #[test]
    fn mutual_triggers_reach_a_joint_fixpoint() {
        // a pulls b when x; b provides x... but x only appears once b
        // is in: a's edge cannot fire before b arrives some other way.
        // Here c provides x and is a root — both edges fire in one
        // pass, b joins, stability follows.
        let world = vec![
            pkg("a", &[], &[("x", "b")]),
            pkg("b", &["y"], &[]),
            pkg("c", &["x"], &[("y", "d")]),
            pkg("d", &[], &[]),
        ];
        let mut m = FixpointModel::new(world, ["a", "c"]);
        let steps = m.run(5);
        // Step 1: x present (c) → b added. Step 2: y present (b) → d
        // added. Step 3: stable.
        assert_eq!(steps.len(), 3);
        assert!(steps[0].added.contains("b"));
        assert!(steps[1].added.contains("d"));
        assert!(steps[2].stable);
    }

    #[test]
    fn run_respects_the_iteration_cap() {
        // A self-growing chain longer than the cap: run() stops at the
        // cap un-stable, exactly like the production loop's loud exit.
        let world = vec![
            pkg("p0", &[], &[("p0", "p1")]),
            pkg("p1", &[], &[("p1", "p2")]),
            pkg("p2", &[], &[("p2", "p3")]),
            pkg("p3", &[], &[("p3", "p4")]),
            pkg("p4", &[], &[("p4", "p5")]),
            pkg("p5", &[], &[("p5", "p6")]),
            pkg("p6", &[], &[]),
        ];
        let mut m = FixpointModel::new(world, ["p0"]);
        let steps = m.run(3);
        assert_eq!(steps.len(), 3);
        assert!(!m.is_stable());
    }
}
