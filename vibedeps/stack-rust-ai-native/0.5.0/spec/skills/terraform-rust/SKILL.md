---
name: terraform-rust
description: Adopt the AI-Native discipline on an existing (brownfield) Rust codebase — inventory-not-gate, the three registries, characterization goldens, then card raids crate by crate. Use once per codebase (or to resume a partial adoption); the recurring counterpart is /discipline-sweep.
---

# Terraform a Rust codebase (brownfield adoption)

You are executing the BROWNFIELD protocol
(`spec://discipline-core/mechanisms/BROWNFIELD-PROTOCOL-v0.1` — shipped
copy under `vibedeps/flow-discipline-core/<version>/spec/mechanisms/`;
read it before the first phase, and skim `03-RAID-PLAYBOOK` +
`05-CAMPAIGN-FORM` for the campaign machinery). The founding principles:
**inventory, not gate** (the only precondition is "the workspace
compiles"); **aspiration is legal only when labeled**; **contradiction is
data**; **characterization is the truth-of-record**; **monotone utility**.
Do not bulldoze an inhabited world.

Toolchain: `discipline-rust` (install once:
`cargo install --path vibedeps/<stack-slot>/crates/discipline-cli-rust`, or run
via `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
discipline-cli-rust --bin discipline-rust -- <args>`).

## Phase −1 — inventory (record reality; change nothing)

1. Precondition: `cargo build --workspace` succeeds. (Red build → fix the
   build first; that is the one true gate.)
2. `discipline-rust init` — policies + empty registries. Every crate
   starts exempt-with-a-reason; nothing is gated yet, and that is correct.
3. **Fill `discipline/registry/tests-baseline.json` with reality:** run
   the suite once (`cargo nextest run --workspace --no-fail-fast`), record
   every failing test as `failing-known` with a `since` date and a debt id
   — do NOT fix them now (drive-by repairs destroy the accounting).
4. **Harvest intent** into `discipline/registry/intent.json`: WAL "Next" /
   TASKS / ROADMAP items, `<!-- REVIEW -->` markers, load-bearing
   TODO/FIXME. The carry-over guarantee: at exit every harvested intention
   is done | rescoped | rejected(reason) — zero unaccounted.
5. **File debt** into `discipline/registry/debt.json`: failing tests,
   known-unimplemented specs, contradictions found while reading — each
   with severity, evidence, disposition, and `touch:` tripwires on its
   watched paths.
6. **Characterize** currently-passing observable behavior (golden
   transcripts under `discipline/golden/`, normalized for volatile
   fields): these pin "don't break it" independently of whether tests or
   docs are trustworthy. A pinned bug is visible debt; an unpinned bug is
   a landmine.
7. `discipline-rust specmap` — mint the (initially small) index; commit
   the whole inventory as its own topic commits.

## Phase 0 — the first spec units

Write the project's first `spec/` documents for the subsystems you will
touch first: anchored headings (`{#req-…}`), kind lines (`` `req r1` ``).
Unimplemented-by-plan units are marked `planned` — zero coverage there is
expected, not red. Tag the implementing modules
(`specmark::scope!("spec://<ns>/…")`) as you go; `discipline-rust specmap`
after each batch keeps the index green.

## Phases 1…N — card raids, crate by crate

Per the Raid Playbook skeleton (scope & freeze → card order → phases →
batches → differential safety → exit criteria), and per crate in
dependency order:

1. **Drain** the crate: unwraps out of domain code (restructure beats
   testify), one thiserror enum per layer with REQ-citing messages, a
   doctest per public seam, cells with oracles where variance exists.
   `discipline-rust conform check --scope <crate>` is the per-crate lens;
   `discipline-rust fast-loop --cell <crate>` keeps the loop under budget.
2. **Flip**: add the crate to conform.toml's `gated_crates` (remove its
   `[[exempt]]` entry) — legal only at zero findings; a flip must never
   widen the baseline.
3. **Behavior changes carry a differential oracle** (card scaffold-d);
   golden transcripts must fail loudly when stale, never auto-update.
4. Each phase closes with the floor green (`discipline-rust floor`) and
   its own topic commits, per the Campaign Form's phase-gate discipline.

Track the campaign per `05-CAMPAIGN-FORM`: a cold-executable PLAN, the
BASELINE numbers you started from, PREDICTIONS, a LOG, and a closing
REPORT. Resume pointer: with a WAL — the standing line at each phase
boundary; without — the PLAN's status line + LOG tail
(`06-WAL-CONVENTION` §4).

## Exit criteria

`discipline-rust floor` green with every crate either gated or
exempt-with-a-living-reason; the test-gate baseline shrunk truthfully
(promotions, not silence); the carry-over guarantee met (every intent
done | rescoped | rejected); the REPORT written. From here the tree is
held by the recurring sweep: /discipline-sweep.

## The generation-time assistant (before you edit, not instead of the floor)

The stack ships an agentic type oracle. Before writing a nontrivial `.rs`
edit, check the HYPOTHETICAL content instead of paying a red floor
iteration:

```sh
vibe bin exec tcg-rust -- validate src/cells/<cell>.rs \
    --content-from - --root .   # the edit on stdin; exit 1 = would fail
```

or, when the vibevm MCP server is mounted, call `tcg_validate` with
`language: "rust"` and the `content` argument (plus `tcg_scope` /
`tcg_complete` / `tcg_type` for in-scope symbols, type-valid
completions, and quick info). Responses carry the SAME conform findings
as the gate, flagged `baselined` or new, with guide-citing advice — a
new finding in the answer means the floor WILL go red if you write that
edit. Prerequisite: rust-analyzer on the machine (`rustup component add
rust-analyzer` — a stack obligation). Honesty: the oracle is
rust-analyzer, not rustc; a clean answer shortens the distance to
green, and the floor stays the truth (TCG-ORACLE-RUST §5).
