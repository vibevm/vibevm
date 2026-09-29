---
name: rust-ai-native-terraform
description: Adopt the AI-Native discipline on an existing (brownfield) Rust codebase — inventory-not-gate, the three registries, characterization goldens, then card raids crate by crate. Use once per codebase (or to resume a partial adoption); the recurring counterpart is /rust-ai-native-sweep.
---

<status stage="impl" state="done"/>

# Terraform a Rust codebase (brownfield adoption) {#root}

@fact:EXECUTING-THE-BROWNFIELD-PROTOCOL You are executing the BROWNFIELD protocol
(`spec://org.vibevm.ai-native/core-ai-native/mechanisms/BROWNFIELD-PROTOCOL-v0.1` — shipped
copy under `vibedeps/flow-core-ai-native/<version>/spec/mechanisms/`;
read it before the first phase, and skim `03-RAID-PLAYBOOK` +
`05-CAMPAIGN-FORM` for the campaign machinery). @status:impl/done

@fact:founding-principles-lead The founding principles: @status:impl/done

- @fact:PRINCIPLE-INVENTORY-NOT-GATE **inventory, not gate** (the only precondition is "the workspace
  compiles"); @status:impl/done
- @fact:PRINCIPLE-ASPIRATION-MUST-BE-LABELED **aspiration is legal only when labeled**; @status:impl/done
- @fact:PRINCIPLE-CONTRADICTION-IS-DATA **contradiction is
  data**; @status:impl/done
- @fact:PRINCIPLE-CHARACTERIZATION-IS-THE-TRUTH-OF-RECORD **characterization is the truth-of-record**; @status:impl/done
- @fact:PRINCIPLE-MONOTONE-UTILITY **monotone utility**. @status:impl/done

@fact:DO-NOT-BULLDOZE-AN-INHABITED-WORLD Do not bulldoze an inhabited world. @status:impl/done

@fact:TOOLCHAIN-INSTALL-OR-RUN-IN-PLACE Toolchain: `rust-ai-native` (install once:
`cargo install --path vibedeps/<stack-slot>/crates/rust-ai-native-cli`, or run
via `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
rust-ai-native-cli --bin rust-ai-native -- <args>`). @status:impl/done

## Phase −1 — inventory (record reality; change nothing) {#phase-inventory}

1. @fact:INVENTORY-PRECONDITION Precondition: `cargo build --workspace` succeeds. (Red build → fix the
   build first; that is the one true gate.) @status:impl/done
2. @fact:INVENTORY-INIT `rust-ai-native init` — policies + empty registries. Every crate
   starts exempt-with-a-reason; nothing is gated yet, and that is correct. @status:impl/done
3. @fact:INVENTORY-TESTS-BASELINE **Fill `discipline/registry/tests-baseline.json` with reality:** run
   the suite once (`cargo nextest run --workspace --no-fail-fast`), record
   every failing test as `failing-known` with a `since` date and a debt id
   — do NOT fix them now (drive-by repairs destroy the accounting). @status:impl/done
4. @fact:INVENTORY-HARVEST-INTENT **Harvest intent** into `discipline/registry/intent.json`: WAL "Next" /
   TASKS / ROADMAP items, `<!-- REVIEW -->` markers, load-bearing
   TODO/FIXME. The carry-over guarantee: at exit every harvested intention
   is done | rescoped | rejected(reason) — zero unaccounted. @status:impl/done
5. @fact:INVENTORY-FILE-DEBT **File debt** into `discipline/registry/debt.json`: failing tests,
   known-unimplemented specs, contradictions found while reading — each
   with severity, evidence, disposition, and `touch:` tripwires on its
   watched paths. @status:impl/done
6. @fact:INVENTORY-CHARACTERIZE **Characterize** currently-passing observable behavior (golden
   transcripts under `discipline/golden/`, normalized for volatile
   fields): these pin "don't break it" independently of whether tests or
   docs are trustworthy. A pinned bug is visible debt; an unpinned bug is
   a landmine. @status:impl/done
7. @fact:INVENTORY-SPECMAP `rust-ai-native specmap` — mint the (initially small) index; commit
   the whole inventory as its own topic commits. @status:impl/done

## Phase 0 — the first spec units {#phase-first-spec-units}

@fact:WRITE-THE-FIRST-SPEC-DOCUMENTS Write the project's first `spec/` documents for the subsystems you will
touch first: anchored headings (`{#req-…}`), kind lines (`` `req r1` ``). @status:impl/done

@fact:UNIMPLEMENTED-BY-PLAN-UNITS-ARE-MARKED-PLANNED Unimplemented-by-plan units are marked `planned` — zero coverage there is
expected, not red. @status:impl/done

@fact:TAG-IMPLEMENTING-MODULES-AS-YOU-GO Tag the implementing modules
(`specmark::scope!("spec://<ns>/…")`) as you go; `rust-ai-native specmap`
after each batch keeps the index green. @status:impl/done

## Phases 1…N — card raids, crate by crate {#card-raids}

@fact:raid-per-crate-in-dependency-order-lead Per the Raid Playbook skeleton (scope & freeze → card order → phases →
batches → differential safety → exit criteria), and per crate in
dependency order: @status:impl/done

1. @fact:RAID-DRAIN-THE-CRATE **Drain** the crate: unwraps out of domain code (restructure beats
   testify), one thiserror enum per layer with REQ-citing messages, a
   doctest per public seam, cells with oracles where variance exists.
   `rust-ai-native conform check --scope <crate>` is the per-crate lens;
   `rust-ai-native fast-loop --cell <crate>` keeps the loop under budget. @status:impl/done
2. @fact:RAID-FLIP **Flip**: add the crate to conform.toml's `[rust] gated` (remove its
   `[[rust.exempt]]` entry) — legal only at zero findings; a flip must never
   widen the baseline. @status:impl/done
3. @fact:RAID-BEHAVIOR-CHANGES-CARRY-AN-ORACLE **Behavior changes carry a differential oracle** (card scaffold-d);
   golden transcripts must fail loudly when stale, never auto-update. @status:impl/done
4. @fact:RAID-PHASE-CLOSES-WITH-THE-FLOOR-GREEN Each phase closes with the floor green (`rust-ai-native floor`) and
   its own topic commits, per the Campaign Form's phase-gate discipline. @status:impl/done

@fact:TRACK-THE-CAMPAIGN-PER-CAMPAIGN-FORM Track the campaign per `05-CAMPAIGN-FORM`: a cold-executable PLAN, the
BASELINE numbers you started from, PREDICTIONS, a LOG, and a closing
REPORT. @status:impl/done

@fact:CAMPAIGN-RESUME-POINTER Resume pointer: with a WAL — the standing line at each phase
boundary; without — the PLAN's status line + LOG tail
(`06-WAL-CONVENTION` §4). @status:impl/done

## Exit criteria {#exit-criteria}

@fact:EXIT-CRITERIA `rust-ai-native floor` green with every crate either gated or
exempt-with-a-living-reason; the test-gate baseline shrunk truthfully
(promotions, not silence); the carry-over guarantee met (every intent
done | rescoped | rejected); the REPORT written. @status:impl/done

@fact:HELD-BY-THE-RECURRING-SWEEP From here the tree is
held by the recurring sweep: /rust-ai-native-sweep. @status:impl/done

## The generation-time assistant (before you edit, not instead of the floor) {#generation-time-assistant}

@fact:STACK-SHIPS-AN-AGENTIC-TYPE-ORACLE The stack ships an agentic type oracle. @status:impl/done

@fact:check-the-hypothetical-content-lead Before writing a nontrivial `.rs`
edit, check the HYPOTHETICAL content instead of paying a red floor
iteration: @status:impl/done

```sh
vibe bin exec rust-ai-native-tcg -- validate src/cells/<cell>.rs \
    --content-from - --root .   # the edit on stdin; exit 1 = would fail
```

@fact:MCP-ALTERNATIVE or, when the vibevm MCP server is mounted, call `tcg_validate` with
`language: "rust"` and the `content` argument (plus `tcg_scope` /
`tcg_complete` / `tcg_type` for in-scope symbols, type-valid
completions, and quick info). @status:impl/done

@fact:RESPONSES-CARRY-THE-SAME-CONFORM-FINDINGS Responses carry the SAME conform findings
as the gate, flagged `baselined` or new, with guide-citing advice — a
new finding in the answer means the floor WILL go red if you write that
edit. @status:impl/done

@fact:ORACLE-PREREQUISITE Prerequisite: rust-analyzer on the machine (`rustup component add
rust-analyzer` — a stack obligation). @status:impl/done

@fact:ORACLE-HONESTY Honesty: the oracle is
rust-analyzer, not rustc; a clean answer shortens the distance to
green, and the floor stays the truth (TCG-ORACLE-RUST §5). @status:impl/done
