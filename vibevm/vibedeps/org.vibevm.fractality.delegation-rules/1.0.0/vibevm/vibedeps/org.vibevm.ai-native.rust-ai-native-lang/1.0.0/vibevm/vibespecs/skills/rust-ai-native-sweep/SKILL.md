---
name: rust-ai-native-sweep
description: Run the recurring AI-Native discipline sweep on this Rust project — floor gates first, then the health collector's ratchet items, weekly drift and judgment tiers. Use daily or several times a day on an active tree; any single item is a safe stop.
---

<status stage="impl" state="done"/>

# The discipline sweep (Rust stack) {#root}

@fact:RUNNING-THE-STANDING-SWEEP You are running the standing sweep from the Discipline's Sweep Playbook
(`spec://org.vibevm.ai-native/core-ai-native/04-SWEEP-PLAYBOOK` — the shipped copy is at
`vibedeps/flow-core-ai-native/<version>/spec/04-SWEEP-PLAYBOOK.md`; read it
once per session if you have not). @status:impl/done

@fact:two-truths-lead The two truths: @status:impl/done

- @fact:TRUTH-GATES-ARE-THE-FLOOR **the gates are the
  floor, the sweep is the ceiling**, @status:impl/done
- @fact:TRUTH-GATE-IS-TRUTH and **the gate is truth, the collector is
  a guide**. @status:impl/done

@fact:NEVER-SWEEP-ON-A-RED-TREE Never sweep on a red tree. @status:impl/done

@fact:ACT-ON-COLLECTOR-FACTS Act on collector facts, never on
memory. @status:impl/done

@fact:ALL-COMMANDS-ARE-THE-SHIPPED-TOOLCHAIN All commands below are the shipped toolchain. @status:impl/done

@fact:IF-NOT-ON-PATH-INSTALL-OR-RUN-IN-PLACE If `rust-ai-native` is not
on PATH, either install it once —
`cargo install --path vibedeps/<stack-slot>/crates/rust-ai-native-cli` — or run
it in place: `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
rust-ai-native-cli --bin rust-ai-native -- <args>`. @status:impl/done

@fact:PROJECT-MAY-KEEP-ITS-OWN-WRAPPER (A project may also keep
its own wrapper, e.g. a dev repo's `cargo xtask` — same engine, either way.) @status:impl/done

## Tier 0 — the hard floor (ALWAYS first) {#tier-zero}

```sh
rust-ai-native floor
```

@fact:RED-FLOOR-ADMITS-ONLY-GREENING-WORK Red? The only legal work is making it green — fix, do not proceed. @status:impl/done

@fact:CHECK-THE-PRINTED-POLICY-ORIGIN-LINES Check
the printed policy-origin lines: `conform: NO conform.toml — topology
default in force, nothing is gated` means the project is not
bootstrapped (`rust-ai-native init`), and a green on a defaulted
policy is vacuous. @status:impl/done

## Tier 1 — the ratchet (every run) {#tier-one}

```sh
rust-ai-native health
```

@fact:READ-THE-HEALTH-SUMMARY Read the summary (the JSON at `discipline/health/latest.json` is the
work-list; its git diff is the trend). @status:impl/done

@fact:take-cheapest-wins-lead Take one or two cheapest wins, in
this order: @status:impl/done

1. @fact:RATCHET-DANGER-BAND-FILES **`danger_band_files`** — split any file at the top of the [540,600)
   band before an edit trips the 600 budget. Idioms: tests-out to a sibling
   `foo/tests.rs` (`#[cfg(test)] #[path] mod tests;`) first, responsibility
   split second; every new module keeps the parent's `scope!` URI (GUIDE
   §14 has the gotchas). @status:impl/done
2. @fact:RATCHET-PUB-DOCTEST-PROMOTION-CANDIDATES **`pub_doctest_promotion_candidates`** — a gated crate at 0 typed-gap
   enters `gated_pub_doctest` in conform.toml for free; run
   `rust-ai-native conform check` to confirm the collector's prediction. @status:impl/done
3. @fact:RATCHET-PUB-DOCTEST-DRAIN-BACKLOG **`pub_doctest_drain_backlog`** — document the smallest-gap crate's
   types (the four doctest idioms, GUIDE §14), then promote it. @status:impl/done
4. @fact:RATCHET-DEVIATION-DEBT **`deviation_debt`** — re-justify each `#[spec(deviates)]`: a deviation
   whose invariant is now encodable in a type is removed and restructured. @status:impl/done
5. @fact:RATCHET-CENSUS-REGRESSIONS **Census regressions** (`unwrap_domain` / `env_nonroot` /
   `unsafe_nonaudit` / `error_enums_missing_req` non-zero on a gated
   crate) — drain immediately; restructure beats testify. On an ungated
   crate they are the adoption backlog: **flip a crate into `[rust] gated`
   only after it drains to zero.** @status:impl/done

## Tier 2 — drift (weekly) {#tier-two}

- @fact:DRIFT-TRIPWIRE `rust-ai-native tripwire` — re-disposition every touched-and-open debt
  entry; file new deficiencies into `discipline/registry/debt.json`, never
  leave them as prose. @status:impl/done
- @fact:DRIFT-LEDGER-RENDER `rust-ai-native ledger render --check` — the human views
  (`discipline/DEBT.md` / `INTENT.md`) match their registries; stale →
  re-render and commit (a registry edit without a re-render is exactly
  the drift this catches). @status:impl/done
- @fact:DRIFT-DOC-CODE Doc/code drift: WAL freshness (if the project keeps one — see
  `06-WAL-CONVENTION`), architecture docs vs the real tree, roadmap
  staleness. File `stale-doc` debt. @status:impl/done
- @fact:DRIFT-MARKER-CENSUS Marker census: `rg -n 'TODO|FIXME|REVIEW|XXX|HACK'` over the source
  roots — graduate load-bearing markers into the registries, delete
  trivial ones. @status:impl/done
- @fact:DRIFT-GOLDEN-TRANSCRIPTS Golden transcripts (`discipline/golden/`): must fail loudly, re-captured
  deliberately (`capture.sh`), never auto-updated. @status:impl/done

## Tier 3 — deep judgment (weekly) {#tier-three}

@fact:WALK-THE-WISH-RULES Walk the WISH rules over the week's diff (typed seams, cell
isolation/oracles, uniformity, contract-first ordering, lying prose,
closed-vocabulary naming — GUIDE §1–§10). @status:impl/done

@fact:CAMPAIGN-SIZED-BACKLOG-BECOMES-A-RAID If a Tier-1 backlog has grown
campaign-sized, plan a raid instead: `03-RAID-PLAYBOOK` +
`05-CAMPAIGN-FORM`. @status:impl/done

## Closing a sweep {#closing-a-sweep}

@fact:TOPIC-GROUPED-COMMITS Topic-grouped commits, one logical unit each, citing the sweep item. @status:impl/done

@fact:COMMIT-THE-REFRESHED-HEALTH-JSON Commit the refreshed `discipline/health/latest.json` in the same run. @status:impl/done

@fact:RESUME-POINTER Resume pointer: **with a WAL** — bump its standing line at any milestone
move; **without** — the closing commit message carries the summary (floor
state, items taken, next candidate). @status:impl/done

@fact:NEVER-LEAVE-STATE-ONLY-IN-THE-CONVERSATION Never leave the sweep's state only in
this conversation. @status:impl/done

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
