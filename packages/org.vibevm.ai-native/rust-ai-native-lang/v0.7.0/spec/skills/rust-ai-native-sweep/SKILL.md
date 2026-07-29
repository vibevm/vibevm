---
name: rust-ai-native-sweep
description: Run the recurring AI-Native discipline sweep on this Rust project — floor gates first, then the health collector's ratchet items, weekly drift and judgment tiers. Use daily or several times a day on an active tree; any single item is a safe stop.
---

<status stage="impl" state="done"/>

# The discipline sweep (Rust stack) {#root}

##RUNNING-THE-STANDING-SWEEP You are running the standing sweep from the Discipline's Sweep Playbook
(`spec://org.vibevm.ai-native/core-ai-native/04-SWEEP-PLAYBOOK` — the shipped copy is at
`vibedeps/flow-core-ai-native/<version>/spec/04-SWEEP-PLAYBOOK.md`; read it
once per session if you have not). @impl/done

##two-truths-lead The two truths: @impl/done

- ##TRUTH-GATES-ARE-THE-FLOOR **the gates are the
  floor, the sweep is the ceiling**, @impl/done
- ##TRUTH-GATE-IS-TRUTH and **the gate is truth, the collector is
  a guide**. @impl/done

##NEVER-SWEEP-ON-A-RED-TREE Never sweep on a red tree. @impl/done

##ACT-ON-COLLECTOR-FACTS Act on collector facts, never on
memory. @impl/done

##ALL-COMMANDS-ARE-THE-SHIPPED-TOOLCHAIN All commands below are the shipped toolchain. @impl/done

##IF-NOT-ON-PATH-INSTALL-OR-RUN-IN-PLACE If `rust-ai-native` is not
on PATH, either install it once —
`cargo install --path vibedeps/<stack-slot>/crates/rust-ai-native-cli` — or run
it in place: `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
rust-ai-native-cli --bin rust-ai-native -- <args>`. @impl/done

##PROJECT-MAY-KEEP-ITS-OWN-WRAPPER (A project may also keep
its own wrapper, e.g. a dev repo's `cargo xtask` — same engine, either way.) @impl/done

## Tier 0 — the hard floor (ALWAYS first) {#tier-zero}

```sh
rust-ai-native floor
```

##RED-FLOOR-ADMITS-ONLY-GREENING-WORK Red? The only legal work is making it green — fix, do not proceed. @impl/done

##CHECK-THE-PRINTED-POLICY-ORIGIN-LINES Check
the printed policy-origin lines: a `Defaulted` policy means the project is
not bootstrapped (`rust-ai-native init`), and a green on a defaulted
policy is vacuous. @impl/done

## Tier 1 — the ratchet (every run) {#tier-one}

```sh
rust-ai-native health
```

##READ-THE-HEALTH-SUMMARY Read the summary (the JSON at `discipline/health/latest.json` is the
work-list; its git diff is the trend). @impl/done

##take-cheapest-wins-lead Take one or two cheapest wins, in
this order: @impl/done

1. ##RATCHET-DANGER-BAND-FILES **`danger_band_files`** — split any file at the top of the [540,600)
   band before an edit trips the 600 budget. Idioms: tests-out to a sibling
   `foo/tests.rs` (`#[cfg(test)] #[path] mod tests;`) first, responsibility
   split second; every new module keeps the parent's `scope!` URI (GUIDE
   §14 has the gotchas). @impl/done
2. ##RATCHET-PUB-DOCTEST-PROMOTION-CANDIDATES **`pub_doctest_promotion_candidates`** — a gated crate at 0 typed-gap
   enters `gated_pub_doctest` in conform.toml for free; run
   `rust-ai-native conform check` to confirm the collector's prediction. @impl/done
3. ##RATCHET-PUB-DOCTEST-DRAIN-BACKLOG **`pub_doctest_drain_backlog`** — document the smallest-gap crate's
   types (the four doctest idioms, GUIDE §14), then promote it. @impl/done
4. ##RATCHET-DEVIATION-DEBT **`deviation_debt`** — re-justify each `#[spec(deviates)]`: a deviation
   whose invariant is now encodable in a type is removed and restructured. @impl/done
5. ##RATCHET-CENSUS-REGRESSIONS **Census regressions** (`unwrap_domain` / `env_nonroot` /
   `unsafe_nonaudit` / `error_enums_missing_req` non-zero on a gated
   crate) — drain immediately; restructure beats testify. On an ungated
   crate they are the adoption backlog: **flip a crate into `gated_crates`
   only after it drains to zero.** @impl/done

## Tier 2 — drift (weekly) {#tier-two}

- ##DRIFT-TRIPWIRE `rust-ai-native tripwire` — re-disposition every touched-and-open debt
  entry; file new deficiencies into `discipline/registry/debt.json`, never
  leave them as prose. @impl/done
- ##DRIFT-LEDGER-RENDER `rust-ai-native ledger render --check` — the human views
  (`discipline/DEBT.md` / `INTENT.md`) match their registries; stale →
  re-render and commit (a registry edit without a re-render is exactly
  the drift this catches). @impl/done
- ##DRIFT-DOC-CODE Doc/code drift: WAL freshness (if the project keeps one — see
  `06-WAL-CONVENTION`), architecture docs vs the real tree, roadmap
  staleness. File `stale-doc` debt. @impl/done
- ##DRIFT-MARKER-CENSUS Marker census: `rg -n 'TODO|FIXME|REVIEW|XXX|HACK'` over the source
  roots — graduate load-bearing markers into the registries, delete
  trivial ones. @impl/done
- ##DRIFT-GOLDEN-TRANSCRIPTS Golden transcripts (`discipline/golden/`): must fail loudly, re-captured
  deliberately (`capture.sh`), never auto-updated. @impl/done

## Tier 3 — deep judgment (weekly) {#tier-three}

##WALK-THE-WISH-RULES Walk the WISH rules over the week's diff (typed seams, cell
isolation/oracles, uniformity, contract-first ordering, lying prose,
closed-vocabulary naming — GUIDE §1–§10). @impl/done

##CAMPAIGN-SIZED-BACKLOG-BECOMES-A-RAID If a Tier-1 backlog has grown
campaign-sized, plan a raid instead: `03-RAID-PLAYBOOK` +
`05-CAMPAIGN-FORM`. @impl/done

## Closing a sweep {#closing-a-sweep}

##TOPIC-GROUPED-COMMITS Topic-grouped commits, one logical unit each, citing the sweep item. @impl/done

##COMMIT-THE-REFRESHED-HEALTH-JSON Commit the refreshed `discipline/health/latest.json` in the same run. @impl/done

##RESUME-POINTER Resume pointer: **with a WAL** — bump its standing line at any milestone
move; **without** — the closing commit message carries the summary (floor
state, items taken, next candidate). @impl/done

##NEVER-LEAVE-STATE-ONLY-IN-THE-CONVERSATION Never leave the sweep's state only in
this conversation. @impl/done

## The generation-time assistant (before you edit, not instead of the floor) {#generation-time-assistant}

##STACK-SHIPS-AN-AGENTIC-TYPE-ORACLE The stack ships an agentic type oracle. @impl/done

##check-the-hypothetical-content-lead Before writing a nontrivial `.rs`
edit, check the HYPOTHETICAL content instead of paying a red floor
iteration: @impl/done

```sh
vibe bin exec rust-ai-native-tcg -- validate src/cells/<cell>.rs \
    --content-from - --root .   # the edit on stdin; exit 1 = would fail
```

##MCP-ALTERNATIVE or, when the vibevm MCP server is mounted, call `tcg_validate` with
`language: "rust"` and the `content` argument (plus `tcg_scope` /
`tcg_complete` / `tcg_type` for in-scope symbols, type-valid
completions, and quick info). @impl/done

##RESPONSES-CARRY-THE-SAME-CONFORM-FINDINGS Responses carry the SAME conform findings
as the gate, flagged `baselined` or new, with guide-citing advice — a
new finding in the answer means the floor WILL go red if you write that
edit. @impl/done

##ORACLE-PREREQUISITE Prerequisite: rust-analyzer on the machine (`rustup component add
rust-analyzer` — a stack obligation). @impl/done

##ORACLE-HONESTY Honesty: the oracle is
rust-analyzer, not rustc; a clean answer shortens the distance to
green, and the floor stays the truth (TCG-ORACLE-RUST §5). @impl/done
