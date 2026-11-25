//! Delegation routing (Campaign 2, plan D6): the verdict procedure of the
//! delegation decision matrix, executed as a pure function over four flat
//! axes. The boss scores a task and `route()` returns a KEEP verdict (with
//! the gate that fired) or a DELEGATE verdict (slot, scenario, and a
//! discretionary flag). No I/O, no clock, no state of its own — every
//! behaviour is a plain unit test, exactly like the rest of this crate.
//!
//! The matrix is encoded twice, by design (D6 — a single source would
//! split one fact across two artifacts that would drift). `matrix.toml`
//! is the human-readable data shape (the four axes, the three ordered
//! KEEP gates, the size × verify delegate routes); the constant tables
//! below are its parsed executable form, which `route()` reads its
//! verdict text from so the data and the procedure cannot drift apart.
//! A tripwire test asserts the TOML text mentions every axis value
//! (present) and every rule name (exactly once). The crate gains no toml
//! parser — the parsed form is the source the procedure uses.

use std::str::FromStr;

specmark::scope!("spec://delegation-rules/DECISION-MATRIX#verdict");

// ---- the canonical value lists (shared by FromStr + the drift tripwire) --

/// Legal `error_cost` values (§axes) — the single source for `FromStr`
/// error messages and the `matrix.toml` drift tripwire.
const ERROR_COST_VALUES: [&str; 2] = ["reversible", "irreversible"];
/// Legal `context` values (§axes).
const CONTEXT_VALUES: [&str; 3] = ["compilable", "boot-loadable", "untransferable"];
/// Legal `verify` values (§axes).
const VERIFY_VALUES: [&str; 2] = ["mechanical", "judgment"];
/// Legal `size` values (§axes).
const SIZE_VALUES: [&str; 3] = ["S", "M", "L"];

// ---- the four axes (§axes) ----------------------------------------------

/// Error cost — what a wrong result destroys (reversible vs irreversible).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCost {
    Reversible,
    Irreversible,
}

/// Context transferability — what the worker must know to do the task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    Compilable,
    BootLoadable,
    Untransferable,
}

/// Verifiability — how the result is proven (mechanical vs judgment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verify {
    Mechanical,
    Judgment,
}

/// Size — honest boss-time to do it directly (S ≤ ~15 min, M ≤ ~1 h, L > 1 h).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    S,
    M,
    L,
}

/// A task scored on the four axes — the sole input to `route()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Axes {
    pub error_cost: ErrorCost,
    pub context: Context,
    pub verify: Verify,
    pub size: Size,
}

// ---- the verdict (§verdict) ---------------------------------------------

/// The routing decision. `route()` returns one variant; the `rule` field
/// names the gate or route that decided it, `reason` quotes the matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// A KEEP gate fired (§verdict steps 1–3); the task stays with the boss.
    Keep {
        rule: &'static str,
        reason: &'static str,
    },
    /// The task delegates (§verdict step 4): `slot` is "big" | "small",
    /// `scenario` is "1" | "2", and `discretionary` marks the L × judgment
    /// draft cell where the judgment residue stays with the boss.
    Delegate {
        slot: &'static str,
        scenario: &'static str,
        discretionary: bool,
        rule: &'static str,
        reason: &'static str,
    },
}

// ---- the parsed form of matrix.toml -------------------------------------

/// A parsed `[[gate]]` row — a KEEP gate from §verdict steps 1–3.
#[derive(Debug, Clone, Copy)]
struct GateRow {
    name: &'static str,
    reason: &'static str,
}

/// The three KEEP gates in §verdict order. `route()` reads its KEEP
/// verdict text from here; this is the parsed form of the `[[gate]]`
/// entries in matrix.toml.
const GATES: &[GateRow] = &[
    GateRow {
        name: "never-delegate",
        reason: "Error cost irreversible → KEEP. This rule enumerates the hard \
                 boundary the boss never hands off: secrets, publishes, \
                 force-pushes, deletions of shared state.",
    },
    GateRow {
        name: "untransferable-context",
        reason: "Context untransferable → KEEP. No prompt or corpus order can \
                 carry the context, so the delegation would ship a guess.",
    },
    GateRow {
        name: "judgment-small",
        reason: "Verifiability judgment AND size S|M → KEEP. Deep review of a \
                 small result costs as much as doing it — no margin.",
    },
];

/// A parsed `[[route]]` row — a delegate outcome. The size × verify
/// key is positional (`route()`'s total match indexes the table; a
/// test pins index ↔ cell by rule name), and scenario is derived from
/// context at call time, so both live off the row.
#[derive(Debug, Clone, Copy)]
struct RouteRow {
    slot: &'static str,
    discretionary: bool,
    name: &'static str,
    reason: &'static str,
}

/// The delegate routes (§verdict step 4), expanded to one row per
/// surviving size × verify cell: (S, mechanical), (M, mechanical),
/// (L, mechanical), and (L, judgment). The M and L mechanical cells
/// share `route-big-mechanical` exactly as matrix.toml groups them
/// (`size = ["M", "L"]`); both rows carry the same name and reason. The
/// (S, judgment) and (M, judgment) cells are kept by `judgment-small`
/// and never reach here, so the table is exhaustive.
const ROUTES: &[RouteRow] = &[
    RouteRow {
        slot: "small",
        discretionary: false,
        name: "route-small-mechanical",
        reason: "S × mechanical → small slot (model = small; one-shot, \
                 scenario 1). Bounded mechanical transforms where the spec is \
                 longer than the thinking.",
    },
    RouteRow {
        slot: "big",
        discretionary: false,
        name: "route-big-mechanical",
        reason: "M|L × mechanical → big slot (model = big; coarse one-shot, \
                 scenario 1 when compilable, scenario 2 when boot-loadable).",
    },
    RouteRow {
        slot: "big",
        discretionary: false,
        name: "route-big-mechanical",
        reason: "M|L × mechanical → big slot (model = big; coarse one-shot, \
                 scenario 1 when compilable, scenario 2 when boot-loadable).",
    },
    RouteRow {
        slot: "big",
        discretionary: true,
        name: "route-big-judgment-draft",
        reason: "L × judgment → big slot, draft only (the one discretionary \
                 cell). Delegate a first draft when a mechanical acceptance \
                 slice exists; the judgment residue stays with the boss, or KEEP.",
    },
];

/// Scenario for a delegation, set by context (§verdict step 4):
/// compilable → "1" (precision-compiled), boot-loadable → "2" (boot from
/// named files). Untransferable is unreachable here — gate 2 keeps it.
fn scenario(context: Context) -> &'static str {
    match context {
        Context::Compilable => "1",
        Context::BootLoadable => "2",
        Context::Untransferable => "1",
    }
}

/// The verdict procedure (§verdict), applied exactly: gates 1–3 in order,
/// the first hit decides; a task that survives them delegates by the
/// size × verify table, with scenario set by context (the small one-shot
/// is always scenario 1).
pub fn route(axes: &Axes) -> Verdict {
    // Gate 1 — error cost irreversible (the never-delegate set).
    if axes.error_cost == ErrorCost::Irreversible {
        let g = &GATES[0];
        return Verdict::Keep {
            rule: g.name,
            reason: g.reason,
        };
    }
    // Gate 2 — context untransferable (delegation would ship a guess).
    if axes.context == Context::Untransferable {
        let g = &GATES[1];
        return Verdict::Keep {
            rule: g.name,
            reason: g.reason,
        };
    }
    // Gate 3 + the judgment column of step 4, as one total match: S|M
    // are kept (no margin), L is the discretionary draft row. Totality
    // by construction — no unreachable cell, no escape hatch needed.
    if axes.verify == Verify::Judgment {
        let g = &GATES[2];
        let r = &ROUTES[3];
        return match axes.size {
            Size::S | Size::M => Verdict::Keep {
                rule: g.name,
                reason: g.reason,
            },
            Size::L => Verdict::Delegate {
                slot: r.slot,
                scenario: scenario(axes.context),
                discretionary: r.discretionary,
                rule: r.name,
                reason: r.reason,
            },
        };
    }
    // Step 4, mechanical column — every size has a route row.
    let r = match axes.size {
        Size::S => &ROUTES[0],
        Size::M => &ROUTES[1],
        Size::L => &ROUTES[2],
    };
    let scenario = if r.slot == "small" {
        "1"
    } else {
        scenario(axes.context)
    };
    Verdict::Delegate {
        slot: r.slot,
        scenario,
        discretionary: r.discretionary,
        rule: r.name,
        reason: r.reason,
    }
}

// ---- FromStr: the kebab/plain lowercase forms the CLI passes ------------

impl FromStr for ErrorCost {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "reversible" => Ok(ErrorCost::Reversible),
            "irreversible" => Ok(ErrorCost::Irreversible),
            _ => Err(format!(
                "unknown error_cost {s:?}; legal values: {}",
                ERROR_COST_VALUES.join(", ")
            )),
        }
    }
}

impl FromStr for Context {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "compilable" => Ok(Context::Compilable),
            "boot-loadable" => Ok(Context::BootLoadable),
            "untransferable" => Ok(Context::Untransferable),
            _ => Err(format!(
                "unknown context {s:?}; legal values: {}",
                CONTEXT_VALUES.join(", ")
            )),
        }
    }
}

impl FromStr for Verify {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mechanical" => Ok(Verify::Mechanical),
            "judgment" => Ok(Verify::Judgment),
            _ => Err(format!(
                "unknown verify {s:?}; legal values: {}",
                VERIFY_VALUES.join(", ")
            )),
        }
    }
}

impl FromStr for Size {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "S" | "s" => Ok(Size::S),
            "M" | "m" => Ok(Size::M),
            "L" | "l" => Ok(Size::L),
            _ => Err(format!(
                "unknown size {s:?}; legal values: {}",
                SIZE_VALUES.join(", ")
            )),
        }
    }
}
