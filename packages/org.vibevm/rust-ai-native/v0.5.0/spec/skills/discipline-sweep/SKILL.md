---
name: discipline-sweep
description: Run the recurring AI-Native discipline sweep on this Rust project — floor gates first, then the health collector's ratchet items, weekly drift and judgment tiers. Use daily or several times a day on an active tree; any single item is a safe stop.
---

# The discipline sweep (Rust stack)

You are running the standing sweep from the Discipline's Sweep Playbook
(`spec://discipline-core/04-SWEEP-PLAYBOOK` — the shipped copy is at
`vibedeps/flow-discipline-core/<version>/spec/04-SWEEP-PLAYBOOK.md`; read it
once per session if you have not). The two truths: **the gates are the
floor, the sweep is the ceiling**, and **the gate is truth, the collector is
a guide**. Never sweep on a red tree. Act on collector facts, never on
memory.

All commands below are the shipped toolchain. If `discipline-rust` is not
on PATH, either install it once —
`cargo install --path vibedeps/<stack-slot>/crates/discipline-cli` — or run
it in place: `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
discipline-cli --bin discipline-rust -- <args>`. (A project may also keep
its own wrapper, e.g. a dev repo's `cargo xtask` — same engine, either way.)

## Tier 0 — the hard floor (ALWAYS first)

```sh
discipline-rust floor
```

Red? The only legal work is making it green — fix, do not proceed. Check
the printed policy-origin lines: a `Defaulted` policy means the project is
not bootstrapped (`discipline-rust init`), and a green on a defaulted
policy is vacuous.

## Tier 1 — the ratchet (every run)

```sh
discipline-rust health
```

Read the summary (the JSON at `discipline/health/latest.json` is the
work-list; its git diff is the trend). Take one or two cheapest wins, in
this order:

1. **`danger_band_files`** — split any file at the top of the [540,600)
   band before an edit trips the 600 budget. Idioms: tests-out to a sibling
   `foo/tests.rs` (`#[cfg(test)] #[path] mod tests;`) first, responsibility
   split second; every new module keeps the parent's `scope!` URI (GUIDE
   §14 has the gotchas).
2. **`pub_doctest_promotion_candidates`** — a gated crate at 0 typed-gap
   enters `gated_pub_doctest` in conform.toml for free; run
   `discipline-rust conform check` to confirm the collector's prediction.
3. **`pub_doctest_drain_backlog`** — document the smallest-gap crate's
   types (the four doctest idioms, GUIDE §14), then promote it.
4. **`deviation_debt`** — re-justify each `#[spec(deviates)]`: a deviation
   whose invariant is now encodable in a type is removed and restructured.
5. **Census regressions** (`unwrap_domain` / `env_nonroot` /
   `unsafe_nonaudit` / `error_enums_missing_req` non-zero on a gated
   crate) — drain immediately; restructure beats testify. On an ungated
   crate they are the adoption backlog: **flip a crate into `gated_crates`
   only after it drains to zero.**

## Tier 2 — drift (weekly)

- `discipline-rust tripwire` — re-disposition every touched-and-open debt
  entry; file new deficiencies into `discipline/registry/debt.json`, never
  leave them as prose.
- `discipline-rust ledger render --check` — the human views
  (`discipline/DEBT.md` / `INTENT.md`) match their registries; stale →
  re-render and commit (a registry edit without a re-render is exactly
  the drift this catches).
- Doc/code drift: WAL freshness (if the project keeps one — see
  `06-WAL-CONVENTION`), architecture docs vs the real tree, roadmap
  staleness. File `stale-doc` debt.
- Marker census: `rg -n 'TODO|FIXME|REVIEW|XXX|HACK'` over the source
  roots — graduate load-bearing markers into the registries, delete
  trivial ones.
- Golden transcripts (`discipline/golden/`): must fail loudly, re-captured
  deliberately (`capture.sh`), never auto-updated.

## Tier 3 — deep judgment (weekly)

Walk the WISH rules over the week's diff (typed seams, cell
isolation/oracles, uniformity, contract-first ordering, lying prose,
closed-vocabulary naming — GUIDE §1–§10). If a Tier-1 backlog has grown
campaign-sized, plan a raid instead: `03-RAID-PLAYBOOK` +
`05-CAMPAIGN-FORM`.

## Closing a sweep

Topic-grouped commits, one logical unit each, citing the sweep item.
Commit the refreshed `discipline/health/latest.json` in the same run.
Resume pointer: **with a WAL** — bump its standing line at any milestone
move; **without** — the closing commit message carries the summary (floor
state, items taken, next candidate). Never leave the sweep's state only in
this conversation.
